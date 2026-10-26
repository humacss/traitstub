//! Custom types, adapted results and function-level generics.
//!
//! This file shows custom types in signatures, wrapping return values in
//! Result/Option and giving individual functions generic parameters. Tests
//! supply successive responses with an iterator, capture changing values and
//! check that failed loads skip saving.
//!
//! Read common.rs first for the basic setup.
//! Run with `cargo run --example types` or `cargo test --example types`.

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    id: u64,
}
mod storage {
    pub fn load(id: u64) -> super::User {
        super::User { id }
    }
    pub fn save(user: super::User) -> u64 {
        user.id
    }
}

traitstub::deps! {
    // Custom return types: source::User refers to this crate's User. The real
    // function returns User; the wrapper adds Result<Option<User>, _>, so tests
    // can supply a missing value or an error without changing storage::load.
    load(id: u64) -> Result<Option<source::User>, &'static str>, |id| Ok(Some(storage::load(id)));

    // Owned custom inputs: saving consumes User and returns its stored ID.
    // Result adapts the real function's return type to a fallible dependency.
    save(user: source::User) -> Result<u64, &'static str>, |user| Ok(storage::save(user));

    // Function-level generics: T: Clone applies only to this method. process
    // keeps one copy for its caller and passes the other to the consuming save.
    // Tests also call this method with different types on the same instance.
    duplicate[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
}

// No dependency parameter escapes into the production API.
pub fn process(id: u64, attempts: usize) -> Result<Option<(User, u64)>, &'static str> {
    process_with_deps(&Deps, id, attempts)
}

fn process_with_deps(
    deps: &impl Deps,
    id: u64,
    attempts: usize,
) -> Result<Option<(User, u64)>, &'static str> {
    for _ in 0..attempts {
        if let Some(user) = deps.load(id)? {
            let (saved, retained) = deps.duplicate(user);
            let stored_id = deps.save(saved)?;
            return Ok(Some((retained, stored_id)));
        }
    }
    Ok(None)
}

fn main() {
    println!("{:?}", process(7, 3));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    traitstub::stubs! {
        load => |id| Ok(Some(User { id }));
        save => |user| Ok(user.id);
        duplicate => |value| (value.clone(), value);
    }

    #[test]
    fn public_and_internal_calls_use_the_same_processing_logic() {
        let expected = Ok(Some((User { id: 7 }, 7)));
        assert_eq!(process(7, 3), expected);
        assert_eq!(process_with_deps(&Defs, 7, 3), expected);
    }

    #[test]
    fn overrides_compose_custom_inputs_and_results() {
        let deps = traitstub::defs! {
            load(id) => Ok(Some(User { id: id + 1 }));
            save(user) => Ok(user.id * 2);
        };
        assert_eq!(
            process_with_deps(&deps, 7, 3),
            Ok(Some((User { id: 8 }, 16)))
        );
        assert_eq!(
            process_with_deps(&Defs, 7, 3),
            Ok(Some((User { id: 7 }, 7)))
        );
    }

    #[test]
    fn captured_overrides_keep_concrete_return_types() {
        let adjustment = Cell::new(1_u64);
        let deps = traitstub::defs! {
            load(id: u64) -> Result<Option<User>, &'static str> => |id| {
                Ok(Some(User { id: id + adjustment.get() }))
            };
        };
        assert_eq!(
            process_with_deps(&deps, 7, 3),
            Ok(Some((User { id: 8 }, 8)))
        );
        adjustment.set(2);
        assert_eq!(
            process_with_deps(&deps, 7, 3),
            Ok(Some((User { id: 9 }, 9)))
        );
    }

    #[rstest::rstest]
    #[case::no_attempts(0, None, 0, 0)]
    #[case::not_ready(2, None, 2, 0)]
    #[case::ready(3, Some((User { id: 7 }, 7)), 3, 1)]
    #[case::stops_after_success(4, Some((User { id: 7 }, 7)), 3, 1)]
    fn successive_responses_use_a_test_owned_iterator(
        #[case] attempts: usize,
        #[case] expected: Option<(User, u64)>,
        #[case] expected_reads: usize,
        #[case] expected_writes: usize,
    ) {
        // Ordinary Rust owns the sequence and counters; there is no queue helper
        // or hidden state in Traitstub. Each call advances this test's iterator.
        let responses = RefCell::new([None, None, Some(User { id: 7 })].into_iter());
        let reads = Cell::new(0);
        let writes = Cell::new(0);
        let deps = traitstub::defs! {
            load(_id: u64) -> Result<Option<User>, &'static str> => |_| {
                reads.set(reads.get() + 1);
                Ok(responses.borrow_mut().next().flatten())
            };
            save(user: User) -> Result<u64, &'static str> => |user| {
                writes.set(writes.get() + 1);
                Ok(user.id)
            };
        };
        assert_eq!(process_with_deps(&deps, 7, attempts), Ok(expected));
        assert_eq!(
            (reads.get(), writes.get()),
            (expected_reads, expected_writes)
        );
    }

    #[test]
    fn failed_load_skips_save_in_a_later_override_layer() {
        let deps = traitstub::defs! { load(_id) => Err("load failed"); };
        let deps = traitstub::defs! {
            target = deps;
            save(_user) => panic!("save must not run after loading fails");
        };
        assert_eq!(process_with_deps(&deps, 7, 3), Err("load failed"));
    }

    #[test]
    fn save_errors_are_preserved() {
        let deps = traitstub::defs! { save(_user) => Err("storage unavailable"); };
        assert_eq!(process_with_deps(&deps, 7, 3), Err("storage unavailable"));
    }

    #[test]
    fn each_generic_call_can_choose_a_different_type() {
        let deps = traitstub::defs! { duplicate(value) => (value.clone(), value); };
        assert_eq!(deps.duplicate("hello"), ("hello", "hello"));
        assert_eq!(deps.duplicate(42_u64), (42, 42));
    }
}
