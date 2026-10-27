//! Start here: replace dependencies in tests without exposing them to callers.
//!
//! This file shows forwarding, renamed dependencies, mutable arguments and
//! borrowed results. Its tests define complete shared stubs, override a subset,
//! layer changes and capture test-owned data. Each dependency has a comment
//! explaining what it demonstrates.
//!
//! See types.rs for custom values and retries, or workers.rs for generic tasks.
//! Run with `cargo run --example common` or `cargo test --example common`.

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

// One dependency collection serves this file's one function under test.
traitstub::deps! {
    // Basic forwarding: use the existing function's name, inputs and output.
    load_name(id: u64) -> String, |id| users::load_name(id);

    // Renaming: both modules export load_name, but this dependency is called
    // team_name. Names in the collection need not match the original functions.
    team_name(id: u64) -> String, |id| teams::load_name(id);

    // Shared and mutable inputs: &self on the generated method does not prevent
    // passing &mut String. Result lets tests also exercise a failing writer.
    write(message: &str, output: &mut String) -> Result<(), &'static str>,
        |message, output| { output.push_str(message); Ok(()) };

    // Input-borrowed results: the lifetime ties the returned slice to value.
    // Overrides can change how it is trimmed while preserving that relationship.
    trim['a](value: &'a str) -> &'a str, |value| value.trim();

    // Implementation-borrowed results: no input supplies this reference.
    // Production returns a literal; tests can borrow a test-owned salutation.
    salutation() -> &str, || "Hello";
}

// Production callers see no dependency argument. This ordinary Rust wrapper
// selects the allocation-free defaults; Traitstub does not generate it.
pub fn greet(id: u64) -> Result<String, &'static str> {
    greet_with_deps(&Deps, id)
}

// Production and tests exercise this same implementation. The dependency trait
// stays out of the public function signature.
fn greet_with_deps(deps: &impl Deps, id: u64) -> Result<String, &'static str> {
    let user = deps.load_name(id);
    let team = deps.team_name(id);
    let message = format!(
        "{}, {} from {}!",
        deps.salutation(),
        deps.trim(&user),
        deps.trim(&team)
    );
    let mut output = String::new();
    deps.write(&message, &mut output)?;
    Ok(output)
}

fn main() {
    println!("{:?}", greet(7));
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every dependency has an explicit test implementation. Nothing falls back
    // to production. The writer records the message in the caller's buffer.
    traitstub::stubs! {
        load_name => |_| "Test User".into();
        team_name => |_| "Test Team".into();
        write => |message, output| { output.push_str(message); Ok(()) };
        trim => |value| value;
        salutation => || "Hello";
    }

    #[test]
    fn production_callers_need_no_dependency_argument() {
        assert_eq!(greet(7), Ok("Hello, user-7 from team-7!".into()));
    }

    #[test]
    fn uses_complete_shared_test_defaults() {
        assert_eq!(
            greet_with_deps(&Stubs, 7),
            Ok("Hello, Test User from Test Team!".into())
        );
    }

    #[test]
    fn overrides_only_the_user() {
        // Unlisted functions keep their shared test implementations.
        let defs = traitstub::defs! {
            load_name => |_| "Ada".into();
        };

        assert_eq!(
            greet_with_deps(&defs, 7),
            Ok("Hello, Ada from Test Team!".into())
        );
    }

    #[test]
    fn multiple_overrides_can_be_layered_without_changing_the_base() {
        let base = traitstub::defs! {
            load_name => |_| "Ada".into();
            team_name => |_| "Admins".into();
        };
        let defs = traitstub::defs! { target = base; team_name => |_| "Reviewers".into(); };
        assert_eq!(
            greet_with_deps(&defs, 7),
            Ok("Hello, Ada from Reviewers!".into())
        );
        assert_eq!(
            greet_with_deps(&base, 7),
            Ok("Hello, Ada from Admins!".into())
        );
        assert_eq!(
            greet_with_deps(&Stubs, 7),
            Ok("Hello, Test User from Test Team!".into())
        );
    }

    #[test]
    fn captured_overrides_record_borrowed_and_mutable_inputs() {
        // A typed Fn closure borrows test-owned state. RefCell handles mutation;
        // the mutable output argument is independent of the closure's captures.
        let messages = std::cell::RefCell::new(Vec::new());
        let defs = traitstub::defs! {
            write => |message, output| {
                messages.borrow_mut().push(message.to_owned());
                output.push_str(&format!("recorded: {message}"));
                Ok(())
            };
        };
        assert_eq!(
            greet_with_deps(&defs, 7),
            Ok("recorded: Hello, Test User from Test Team!".into())
        );
        assert_eq!(*messages.borrow(), ["Hello, Test User from Test Team!"]);
    }

    #[test]
    fn dependency_errors_reach_the_caller() {
        let defs = traitstub::defs! { write => |_, _| Err("output unavailable"); };
        assert_eq!(greet_with_deps(&defs, 7), Err("output unavailable"));
        assert!(greet_with_deps(&Stubs, 7).is_ok());
    }

    #[rstest::rstest]
    #[case::start(str::trim_start, "Ada ", 1)]
    #[case::end(str::trim_end, " Ada", 0)]
    fn overrides_preserve_input_borrows(
        #[case] clean: fn(&str) -> &str,
        #[case] expected: &str,
        #[case] offset: usize,
    ) {
        let defs = traitstub::defs! {
            load_name => |_| " Ada ".into();
            trim => |value| clean(value);
        };
        let input = String::from(" Ada ");
        let result = defs.trim(&input);
        assert_eq!(result, expected);
        assert_eq!(result.as_ptr(), input[offset..].as_ptr());
        assert_eq!(
            greet_with_deps(&defs, 7),
            Ok(format!("Hello, {expected} from Test Team!"))
        );
    }

    #[test]
    fn returned_values_can_borrow_from_the_test() {
        // capture names a lifetime, not storage. Rust keeps this dependency
        // implementation from escaping the lifetime of salutation.
        let salutation = String::from("Welcome");
        let defs = traitstub::defs! {
            capture ['test];
            salutation() -> &'test str => || salutation.as_str();
        };
        assert_eq!(
            greet_with_deps(&defs, 7),
            Ok("Welcome, Test User from Test Team!".into())
        );
        assert_eq!(Stubs.salutation(), "Hello");
    }
}
