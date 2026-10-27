//! A consumer uses the three exported macros through one ordinary entry point.
use rstest::rstest;

fn load_name(id: u64) -> String {
    format!("user-{id}")
}
traitstub::deps! {
    load_name(id: u64) -> String, |id| load_name(id);
    prefix() -> &'static str, || "Hello";
}
traitstub::stubs! {
    load_name => |_| "Test User".into();
    prefix => || "Welcome";
}
fn greet(deps: &impl Deps, id: u64) -> String {
    format!("{}, {}!", deps.prefix(), deps.load_name(id))
}

#[rstest]
#[case::zero(0)]
#[case::ordinary(7)]
#[case::maximum(u64::MAX)]
fn production_shared_and_individual_implementations_use_the_same_function(#[case] id: u64) {
    let individual = traitstub::defs! { load_name => |_| "Ada".into(); };
    let layered = traitstub::defs! { target = individual; prefix() => "Goodbye"; };

    assert_eq!(greet(&Deps, id), format!("Hello, user-{id}!"));
    assert_eq!(greet(&Stubs, id), "Welcome, Test User!");
    assert_eq!(greet(&individual, id), "Welcome, Ada!");
    assert_eq!(greet(&layered, id), "Goodbye, Ada!");
}
