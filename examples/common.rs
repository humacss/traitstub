//! A tour of Traitstub: production defaults, complete test stubs, per-test overrides,
//! composed calls, captured state, borrowed results, shared types and worker threads.
//!
//! Run production paths with `cargo run --example common`.
//! Run the test stubs and overrides with `cargo test --example common`.

// 1. Keep real dependencies as ordinary functions. Their callers opt into a
// dependency collection; nothing replaces or modifies these functions globally.
mod users {
    pub fn load_name(id: u64) -> String {
        format!("user-{id}")
    }
}

mod teams {
    pub fn load_name(id: u64) -> String {
        format!("team-{id}")
    }
}

// One collection per module. `deps!` creates the Deps trait and a zero-sized
// production value also named Deps. Closures connect the contract to real code;
// they can adapt arguments/results, or distinguish functions with the same name.
traitstub::deps! {
    load_name(id: u64) -> String, |id| users::load_name(id);
    team_name(id: u64) -> String, |id| teams::load_name(id);
    write(message: &str, output: &mut Vec<String>) -> Result<(), &'static str>,
        |message, output| { output.push(message.into()); Ok(()) };
    poll_name(id: u64) -> Option<String>, |id| Some(users::load_name(id));
}

// Consumer functions take the trait directly: static dispatch, ordinary Rust,
// and no wrapper around the function under test. Pass &Deps in production.
pub fn greet(deps: &impl Deps, id: u64) -> String {
    let user = deps.load_name(id);
    let team = deps.team_name(id);
    format!("Hello, {user} from {team}!")
}

// Several consumer functions can use the same dependency collection.
fn write_greeting(deps: &impl Deps, id: u64) -> Result<Vec<String>, &'static str> {
    let message = greet(deps, id);
    let mut output = Vec::new();
    deps.write(&message, &mut output)?;
    Ok(output)
}

fn wait_for_name(deps: &impl Deps, id: u64, attempts: usize) -> Option<String> {
    for _ in 0..attempts {
        if let Some(name) = deps.poll_name(id) {
            return Some(name);
        }
    }
    None
}

fn main() {
    println!("{}", greet(&Deps, 7));
    println!("{:?}", write_greeting(&Deps, 7));
    println!("{:?}", wait_for_name(&Deps, 7, 3));
    println!("{}", borrowed::run());
    println!("{:?}", types::run());
    println!("{:?}", workers::run());
}

// 2. Test defaults are a separate implementation, owned by this test module.
#[cfg(test)]
mod tests {
    use super::*;

    // stubs! requires every dependency, including ones a test may never call.
    // Nothing silently falls back to production. The output is named Defs.
    traitstub::stubs! {
        load_name => |_| "Test User".into();
        team_name => |_| "Test Team".into();
        poll_name => |_| None;
        write => |_, _| Ok(());
    }

    #[test]
    fn uses_shared_test_defaults() {
        assert_eq!(greet(&Defs, 7), "Hello, Test User from Test Team!");
    }

    #[test]
    fn overrides_only_the_user() {
        // defs! produces a new value, starting from Defs unless target is given.
        // Unlisted methods retain their test defaults; Defs itself stays unchanged.
        let deps = traitstub::defs! {
            load_name => |_| "Ada".into();
        };

        assert_eq!(greet(&deps, 7), "Hello, Ada from Test Team!");
    }

    #[test]
    fn overrides_only_the_team() {
        let deps = traitstub::defs! {
            team_name => |_| "Admins".into();
        };

        assert_eq!(greet(&deps, 7), "Hello, Test User from Admins!");
    }

    #[test]
    fn ordinary_calls_use_production_dependencies() {
        assert_eq!(greet(&Deps, 7), "Hello, user-7 from team-7!");
    }
    #[test]
    fn one_definition_overrides_multiple_dependencies() {
        // A single layer can override several methods at once.
        let deps = traitstub::defs! {
            load_name => |_| "Ada".into();
            team_name => |_| "Admins".into();
        };
        assert_eq!(greet(&deps, 7), "Hello, Ada from Admins!");
        assert_eq!(greet(&Defs, 7), "Hello, Test User from Test Team!");
    }
    #[test]
    fn composed_calls_forward_results_and_record_mutable_inputs() {
        // Captures use explicit callable signatures. Closures implement Fn, so
        // tests own mutable captured data through Cell/RefCell (or atomics).
        // &mut arguments work normally; they do not require a mutable closure.
        let calls = std::cell::RefCell::new(Vec::new());
        let deps = traitstub::defs! {
            write(message: &str, output: &mut Vec<String>) -> Result<(), &'static str> => |message, output| {
                calls.borrow_mut().push(message.to_owned());
                output.push(format!("recorded: {message}"));
                Ok(())
            };
        };
        assert_eq!(
            write_greeting(&deps, 7),
            Ok(vec!["recorded: Hello, Test User from Test Team!".into()])
        );
        assert_eq!(*calls.borrow(), ["Hello, Test User from Test Team!"]);
    }
    #[test]
    fn a_dependency_can_fail_without_changing_the_defaults() {
        let deps = traitstub::defs! { write => |_, _| Err("output unavailable"); };
        assert_eq!(write_greeting(&deps, 7), Err("output unavailable"));
        assert_eq!(write_greeting(&Defs, 7), Ok(vec![]));
    }
    #[test]
    fn successive_responses_use_a_test_owned_iterator() {
        // Successive responses need only a normal iterator owned by the test.
        // Traitstub adds no queue, global registry or hidden fixture storage.
        let responses = std::cell::RefCell::new([None, None, Some("Ada".into())].into_iter());
        let deps = traitstub::defs! {
            poll_name(_id: u64) -> Option<String> => |_| responses.borrow_mut().next().unwrap_or_else(|| Some("fallback".into()));
        };
        assert_eq!(wait_for_name(&deps, 7, 2), None);
        assert_eq!(wait_for_name(&deps, 7, 1), Some("Ada".into()));
        assert_eq!(wait_for_name(&deps, 7, 1), Some("fallback".into()));
        assert_eq!(wait_for_name(&Defs, 7, 3), None);
    }
}

// 3. Generic methods and worker threads use the same three macros.
// Rust checks thread safety where an instance is shared. The dependency trait
// itself does not require Send + Sync; bounds = [Send + Sync]; can require both.
mod workers {

    // Generics and where clauses use brackets so macro_rules! can parse them.
    // T and F vary per call; Error is an associated type shared by the collection.
    use std::{io, thread::JoinHandle};
    traitstub::deps! {
        type Error = io::Error;
        spawn[T, F](name: String, run: F) -> Result<JoinHandle<T>, Self::Error>
        where [T: Send + 'static, F: FnOnce() -> T + Send + 'static],
            |name, run| std::thread::Builder::new().name(name).spawn(run);
        observe(value: usize) -> usize, |value| value;
    }
    fn work(deps: &impl Deps<Error = io::Error>) -> io::Result<u64> {
        let worker = deps.spawn("example-worker".into(), || 42u64)?;
        worker
            .join()
            .map_err(|_| io::Error::other("worker panicked"))
    }
    pub fn run() -> io::Result<u64> {
        work(&Deps)
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        traitstub::stubs! {
            spawn => |_name, _run| Err(io::Error::other("spawn is not configured"));
            observe => |value| value;
        }
        #[test]
        fn generic_overrides_can_be_shared_by_workers() {
            let deps = traitstub::defs! {
                spawn(name, run) => std::thread::Builder::new().name(name).spawn(run);
            };
            std::thread::scope(|scope| {
                let a = scope.spawn(|| work(&deps).unwrap());
                let b = scope.spawn(|| work(&deps).unwrap());
                assert_eq!((a.join().unwrap(), b.join().unwrap()), (42, 42));
            });
        }
        #[test]
        fn test_owned_counters_use_normal_closure_captures() {
            // Atomic state allows this Fn closure to be shared between threads.
            // RefCell would be rejected here by Rust's ordinary Sync checks.
            let calls = AtomicUsize::new(0);
            let deps = traitstub::defs! {
                spawn(name, run) => std::thread::Builder::new().name(name).spawn(run);
                observe(value: usize) -> usize => |value| { calls.fetch_add(1, Ordering::Relaxed); value };
            };
            std::thread::scope(|scope| {
                let a = scope.spawn(|| deps.observe(7));
                let b = scope.spawn(|| deps.observe(9));
                assert_eq!((a.join().unwrap(), b.join().unwrap()), (7, 9));
            });
            assert_eq!(calls.load(Ordering::Relaxed), 2);
            assert_eq!(work(&deps).unwrap(), 42);
        }
        #[test]
        fn failing_spawns_are_local_to_the_override() {
            let deps = traitstub::defs! {
                spawn(_name, _run) => Err(io::Error::other("injected failure"));
            };
            assert_eq!(work(&deps).unwrap_err().to_string(), "injected failure");
            assert_eq!(
                work(&Defs).unwrap_err().to_string(),
                "spawn is not configured"
            );
        }
    }
}

// 4. Borrowed results can come from arguments or fixtures owned by a test.
// There is no requirement to allocate or to return a 'static reference.
mod borrowed {

    traitstub::deps! {
        trim['a](value: &'a str) -> &'a str, |value| value.trim();
        current() -> &str, || "production";
    }
    pub fn run() -> String {
        format!("{}: {}", Deps.current(), Deps.trim(" hello "))
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        traitstub::stubs! {
            current => || "test user";
            trim => |value| value;
        }
        #[test]
        fn returned_values_can_borrow_from_inputs() {
            // The shorthand reuses the declaration's lifetime and signature.
            let deps = traitstub::defs! { trim(value) => value.trim_start(); };
            let input = String::from(" hello ");
            let result = deps.trim(&input);
            assert_eq!(result, "hello ");
            assert_eq!(result.as_ptr(), input[1..].as_ptr());
            assert_eq!(deps.current(), "test user");
        }
        #[test]
        fn fixtures_stay_in_the_test() {
            // capture declares a lifetime parameter, not storage. The typed
            // closure borrows name, and the resulting deps cannot outlive it.
            let name = String::from("local fixture");
            let deps = traitstub::defs! {
                capture ['fixture];
                current() -> &'fixture str => || name.as_str();
            };
            assert_eq!(deps.current(), "local fixture");
            assert_eq!(Defs.current(), "test user");
        }
    }
}

// 5. Associated types describe a shared contract; generic methods vary per call.
mod types {
    #[derive(Debug, PartialEq)]
    pub struct User {
        id: u64,
    }
    fn load_user(id: u64) -> User {
        User { id }
    }
    fn save_user(user: User) -> u64 {
        user.id
    }
    // source:: names the crate declaring this schema. Use a public, qualified
    // path for types that must also be visible when signatures are replayed.
    traitstub::deps! {
        type Value = source::types::User;
        type Error = &'static str;
        load(id: u64) -> Result<Self::Value, Self::Error>, |id| Ok(load_user(id));
        save(value: Self::Value) -> Result<u64, Self::Error>, |value| Ok(save_user(value));
        duplicate[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
    }
    fn process<D: Deps>(deps: &D, id: u64) -> Result<u64, D::Error> {
        deps.save(deps.load(id)?)
    }
    pub fn run() -> Result<u64, &'static str> {
        process(&Deps, 7)
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        pub struct TestUser {
            id: u64,
        }
        // Select a different concrete associated type once for all test methods.
        // Omitted associated types (Error here) retain their production selection.
        traitstub::stubs! {
            type Value = TestUser;
            load => |id| Ok(TestUser { id });
            save => |value| Ok(value.id);
            duplicate => |value| (value.clone(), value);
        }
        #[test]
        fn test_defaults_can_select_a_shared_type() {
            assert_eq!(process(&Defs, 7), Ok(7));
        }
        #[test]
        fn only_the_dependency_of_interest_needs_an_override() {
            let deps = traitstub::defs! {
                save(_value) => Err("storage unavailable");
            };
            assert_eq!(process(&deps, 7), Err("storage unavailable"));
        }
        #[test]
        fn each_generic_call_can_choose_its_own_type() {
            // Generic method parameters differ from associated types: every call
            // chooses T independently, even on the same dependency instance.
            let deps = traitstub::defs! {
                duplicate(value) => (value.clone(), value);
            };
            assert_eq!(deps.duplicate("hello"), ("hello", "hello"));
            assert_eq!(deps.duplicate(42u64), (42, 42));
        }
        #[test]
        fn multiple_overrides_keep_shared_test_types() {
            let deps = traitstub::defs! {
                load(id) => Ok(TestUser { id: id + 1 });
                save(value) => Ok(value.id * 2);
            };
            assert_eq!(process(&deps, 7), Ok(16));
            assert_eq!(process(&Defs, 7), Ok(7));
        }
        #[test]
        fn failed_load_skips_save() {
            let deps = traitstub::defs! { load(_id) => Err("load failed"); };
            // target consumes an earlier instance to add another override layer.
            // The failed load is retained; each test owns its entire chain.
            let deps = traitstub::defs! {
                target = deps;
                save(_value) => panic!("save must not run after loading fails");
            };
            assert_eq!(process(&deps, 7), Err("load failed"));
        }
    }
}
