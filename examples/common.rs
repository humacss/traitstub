//! A tour of Traitstub in one dependency collection: production defaults,
//! complete test stubs, individual overrides, composition, captured state,
//! borrowed values, associated types, generic methods and worker threads.
//!
//! Run production paths with `cargo run --example common`.
//! Run the test stubs and overrides with `cargo test --example common`.

// Real dependencies remain ordinary functions. These modules show how a
// collection can adapt functions from different namespaces without changing them.
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

// Production can query any Host. A test can select its concrete host through
// a closure's argument annotation, without repeating the dependency signature.
pub trait Host {
    type Value;
    type Error;
    fn load(&self, id: u64) -> Result<Option<Self::Value>, Self::Error>;
}
pub struct ProductionHost;
impl Host for ProductionHost {
    type Value = String;
    type Error = &'static str;
    fn load(&self, id: u64) -> Result<Option<String>, Self::Error> {
        Ok(Some(format!("user-{id}")))
    }
}

// One deps! defines the entire collection. Deps is both the trait and its
// allocation-free production value. Closures connect each method to real code.
// H is chosen once per implementation; associated types describe shared values.
// Generic methods such as spawn and duplicate choose their types per call.
traitstub::deps! {
    for [H] where [H: source::Host + ?Sized];
    type Value = source::User;
    type Error = &'static str;
    load_name(id: u64) -> String, |id| users::load_name(id);
    team_name(id: u64) -> String, |id| teams::load_name(id);
    write(message: &str, output: &mut Vec<String>) -> Result<(), &'static str>,
        |message, output| { output.push(message.into()); Ok(()) };
    poll_name(id: u64) -> Option<String>, |id| Some(users::load_name(id));

    // Brackets around generics and where clauses keep this declarative.
    spawn[T, F](name: String, run: F) -> std::io::Result<std::thread::JoinHandle<T>>
    where [T: Send + 'static, F: FnOnce() -> T + Send + 'static],
        |name, run| std::thread::Builder::new().name(name).spawn(run);
    observe(value: usize) -> usize, |value| value;
    trim['a](value: &'a str) -> &'a str, |value| value.trim();
    current() -> &str, || "production";
    load(id: u64) -> Result<Self::Value, Self::Error>, |id| Ok(load_user(id));
    save(value: Self::Value) -> Result<u64, Self::Error>, |value| Ok(save_user(value));
    duplicate[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
    host_load(host: &H, id: u64) -> Result<Option<H::Value>, H::Error>,
        |host, id| host.load(id);
}

// Consumer functions take the trait directly, using ordinary static dispatch.
// These consumers use ProductionHost; lookup below accepts any Host instead.
// Without the host example, the collection needs no H and these take impl Deps.
pub fn greet(deps: &impl Deps<ProductionHost>, id: u64) -> String {
    let user = deps.load_name(id);
    let team = deps.team_name(id);
    format!("Hello, {user} from {team}!")
}
fn write_greeting(deps: &impl Deps<ProductionHost>, id: u64) -> Result<Vec<String>, &'static str> {
    let message = greet(deps, id);
    let mut output = Vec::new();
    deps.write(&message, &mut output)?;
    Ok(output)
}
fn wait_for_name(deps: &impl Deps<ProductionHost>, id: u64, attempts: usize) -> Option<String> {
    for _ in 0..attempts {
        if let Some(name) = deps.poll_name(id) {
            return Some(name);
        }
    }
    None
}
fn work(deps: &impl Deps<ProductionHost>) -> std::io::Result<u64> {
    let worker = deps.spawn("example-worker".into(), || 42u64)?;
    worker
        .join()
        .map_err(|_| std::io::Error::other("worker panicked"))
}
fn process<D: Deps<ProductionHost>>(deps: &D, id: u64) -> Result<u64, D::Error> {
    deps.save(deps.load(id)?)
}
fn lookup<H: Host + ?Sized>(
    deps: &impl Deps<H>,
    host: &H,
    id: u64,
) -> Result<Option<H::Value>, H::Error> {
    deps.host_load(host, id)
}
// A public wrapper can keep the dependency argument internal to the application.
fn host_name() -> String {
    lookup(&Deps, &ProductionHost, 7).unwrap().unwrap()
}

fn main() {
    println!("{}", greet(&Deps, 7));
    println!("{:?}", write_greeting(&Deps, 7));
    println!("{:?}", wait_for_name(&Deps, 7, 3));
    println!(
        "{}: {}",
        Deps::<ProductionHost>::current(&Deps),
        Deps::<ProductionHost>::trim(&Deps, " hello ")
    );
    println!("{:?}", process(&Deps, 7));
    println!("{:?}", work(&Deps));
    println!("{}", host_name());
}

// Every case below uses the same complete test implementation. Each test owns
// its overrides; neither production functions nor other tests are modified.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct TestUser {
        id: u64,
    }

    // stubs! requires every dependency, including ones a test may never call.
    // Nothing silently falls back to production. The output is named Defs.
    traitstub::stubs! {
        type Value = TestUser;
        load_name => |_| "Test User".into();
        team_name => |_| "Test Team".into();
        poll_name => |_| None;
        write => |_, _| Ok(());
        spawn => |_name, _run| Err(std::io::Error::other("spawn is not configured"));
        observe => |value| value;
        current => || "test user";
        trim => |value| value;
        load => |id| Ok(TestUser { id });
        save => |value| Ok(value.id);
        duplicate => |value| (value.clone(), value);
        host_load => |_, _| Ok(None);
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
            let a = scope.spawn(|| Deps::<ProductionHost>::observe(&deps, 7));
            let b = scope.spawn(|| Deps::<ProductionHost>::observe(&deps, 9));
            assert_eq!((a.join().unwrap(), b.join().unwrap()), (7, 9));
        });
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(work(&deps).unwrap(), 42);
    }
    #[test]
    fn failing_spawns_are_local_to_the_override() {
        let deps = traitstub::defs! {
            spawn(_name, _run) => Err(std::io::Error::other("injected failure"));
        };
        assert_eq!(work(&deps).unwrap_err().to_string(), "injected failure");
        assert_eq!(
            work(&Defs).unwrap_err().to_string(),
            "spawn is not configured"
        );
    }

    #[test]
    fn returned_values_can_borrow_from_inputs() {
        // The shorthand reuses the declaration's lifetime and signature.
        let deps = traitstub::defs! { trim(value) => value.trim_start(); };
        let input = String::from(" hello ");
        let result = Deps::<ProductionHost>::trim(&deps, &input);
        assert_eq!(result, "hello ");
        assert_eq!(result.as_ptr(), input[1..].as_ptr());
        assert_eq!(Deps::<ProductionHost>::current(&deps), "test user");
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
        assert_eq!(Deps::<ProductionHost>::current(&deps), "local fixture");
        assert_eq!(Deps::<ProductionHost>::current(&Defs), "test user");
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
        assert_eq!(
            Deps::<ProductionHost>::duplicate(&deps, "hello"),
            ("hello", "hello")
        );
        assert_eq!(Deps::<ProductionHost>::duplicate(&deps, 42u64), (42, 42));
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

    #[test]
    fn a_closure_annotation_selects_the_host_and_result_types() {
        let name = String::from("Ada");
        // &ProductionHost selects H. Rust infers String and &'static str
        // from that host's associated types; no return annotation or
        // repeated dependency signature is needed. The closure borrows name.
        let deps = traitstub::defs! {
            host_load => |_: &ProductionHost, _| Ok(Some(name.clone()));
        };
        assert_eq!(lookup(&deps, &ProductionHost, 7), Ok(Some("Ada".into())));

        // A later layer inherits H from its base, so even the argument
        // annotation can be omitted. Earlier values keep their behavior.
        let missing = traitstub::defs! {
            target = deps;
            host_load => |_, _| Ok(None);
        };
        assert_eq!(lookup(&missing, &ProductionHost, 7), Ok(None));
        assert_eq!(lookup(&deps, &ProductionHost, 7), Ok(Some("Ada".into())));
        assert_eq!(lookup(&Defs, &ProductionHost, 7), Ok(None));
        assert_eq!(host_name(), "user-7");
    }
}
