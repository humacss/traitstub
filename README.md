# traitstub

Simple trait stubs for tests that still run your code.

`traitstub` started with wanting the simplest possible way to unit test code
with external dependencies. Mocking and testing libraries can do a lot, but
sometimes the machinery makes it harder to see what you are actually testing.
You can end up recreating the behavior in the test setup instead of exercising
the implementation you meant to test. The tests pass, but the underlying code
still has problems you thought they covered.

Rust already gives us a straightforward way to handle this: define a trait for
the dependencies, use the production implementation normally, and provide a
different implementation in each test. The function under test runs the same
body. Its dependencies are what change.

That worked well by hand. The problem was the boilerplate. Repeating trait
implementations across tests made the code harder to read, especially when a
test only needed one dependency to behave differently.

`traitstub` uses declarative macros to write that repetitive code for you. It
is a small, zero-dependency crate that generates ordinary Rust traits and
implementations. You write the dependency functions and test behavior; the
macros handle the declarations, shared stubs and individual overrides.

More complete mocking libraries have their uses. The goal here is to keep the
mechanism simple enough that you can see what the test is doing, using Rust's
existing features and keeping the tests readable.

## Usage

There are three macros:

| Macro | What you do | What you get |
| --- | --- | --- |
| `deps!` | Declare function signatures and their production implementations | A `Deps` trait and production value |
| `stubs!` | Give every dependency a test implementation | Shared test defaults named `Defs` |
| `defs!` | Replace the dependencies this test cares about | A new implementation based on `Defs`, or a `target` you choose |

Declare the dependencies and connect them to your production functions. These
are excerpts from the [complete example](examples/common.rs), where `users` and
`teams` contain ordinary Rust functions. Omitted declarations are marked with
`// ...`; the full example keeps every case in one collection and one test module.

<!-- example: dependencies -->
```rust
traitstub::deps! {
    for [H] where [H: source::Host + ?Sized];
    // ...
    load_name(id: u64) -> String, |id| users::load_name(id);
    team_name(id: u64) -> String, |id| teams::load_name(id);
    // ...
}
```
<!-- endexample -->

The example also includes a generic host dependency, so its collection takes
`H`. This consumer chooses `ProductionHost`. A collection without type parameters
would simply take `&impl Deps`:

<!-- example: consumer -->
```rust
pub fn greet(deps: &impl Deps<ProductionHost>, id: u64) -> String {
    let user = deps.load_name(id);
    let team = deps.team_name(id);
    format!("Hello, {user} from {team}!")
}
```
<!-- endexample -->

Pass `&Deps` in production. In the test module, use `stubs!` to define the shared
test implementations:

<!-- example: shared-stubs -->
```rust
traitstub::stubs! {
    // ...
    load_name => |_| "Test User".into();
    team_name => |_| "Test Team".into();
    // ...
}
```
<!-- endexample -->

Every shared stub is explicit. Forgetting one is a compile error, so a test does
not accidentally call a production dependency that it forgot to replace.

Now an individual test can change just the user name. The team name still comes
from the shared stubs:

<!-- example: override -->
```rust
let deps = traitstub::defs! {
    load_name => |_| "Ada".into();
};

assert_eq!(greet(&deps, 7), "Hello, Ada from Test Team!");
```
<!-- endexample -->

`defs!` returns a new implementation. Other tests keep their own defaults. Use
`target = existing;` to build on an implementation you have already configured.

### See it in action

[examples/common.rs](examples/common.rs) contains the production functions,
shared test module and complete tests behind these snippets. It also shows
captured data, successive responses, borrowed results, associated types,
generic methods and worker threads.

```sh
cargo run --example common
cargo test --example common
```

## What the macros handle

With ten dependencies, replacing one should not mean writing implementations
for the other nine in every test. Define those shared stubs once, then keep each
test focused on the behavior it needs.

The generated trait is an ordinary Rust trait. You can implement it by hand
where that is clearer, or inspect the generated code to see exactly what the
macros produced.

## When to use `traitstub`

- Your code already has dependencies you want to pass in explicitly.
- Several tests share the same stubs, with small differences between tests.
- You want the test to own its data and dependency implementation.
- You want declarative macros and generated code you can inspect.

If a function only needs one closure, passing that closure directly may be all
you need. Traitstub is useful when managing the implementations becomes the
boilerplate.

Production and shared defaults are zero-sized. Overrides store their base and
closures directly. Traitstub adds no heap allocations, locks or global state;
your dependency implementations can still allocate or synchronize normally.

## More control

Declare one dependency collection per module. The closures connect it to your
production functions and can adapt their inputs or outputs. Pass the generated
trait directly to the code that uses it, such as `deps: &impl Deps`.

`stubs!` requires a stub for every active dependency, even if a test does not use
it. `defs!` replaces one or more functions and keeps the rest from its base.
Missing stubs, duplicate names and unknown dependencies fail to compile.
A collection with no active methods needs no stubs.

| Optional syntax | Purpose |
| --- | --- |
| `target = ContractName;` in `deps!` | Choose a contract name instead of `Deps` |
| `contract = api::Contract; name = TestStubs;` in `stubs!` | Select the contract and shared stub name (`target` also accepts a contract) |
| `target = instance;` in `defs!` | Layer overrides on an existing implementation |
| `contract = api::Contract; target = instance;` in `defs!` | Use a named contract with your own implementation or custom stubs |
| `type Value = Type;` | Choose a shared type for production or test stubs; overrides must agree with their base |
| `method[T: Clone](...)` | Give a method its own generic parameters |
| `where [T: Send, F: FnOnce() -> T]` | Supply bracketed Rust predicates |
| `call [T, N]` | Forward generic arguments that cannot be inferred |
| `bounds = [Send + Sync];` in `deps!` | Require these bounds on every implementation |
| `capture ['fixture];` in `defs!` | Name a captured fixture lifetime, without creating storage |
| `for ['a] trim(value: &'a str) -> &'a str => closure;` | Return a reference borrowed from each input |
| `method { fn method<T>(&self) -> usize { 42 } }` | Supply a complete Rust method instead of a closure |

`name(arguments) => body;` and `name => closure;` use the declared signature.
In ordinary collections they cannot capture test data. To borrow or own data from the test, include the
signature: `name(argument: Type) -> Output => closure;`. Captured closures implement
`Fn`; mutable state belongs to the test, using `Cell`, `RefCell`, atomics or other Rust
synchronization. Captured signatures use concrete types or accessible aliases,
not `Self::Value`. Thread safety is checked where an instance is shared.
Generic type/const methods use body overrides or complete methods rather than
captured closures. All overrides in one `defs!` share an implementation, so raw
methods can call sibling overrides through `self`. Inherited methods continue
to call their base's implementations.

If you give shared stubs a custom name, import it as `Defs` or specify both
`contract` and `target`. An explicit contract also supports a handwritten base
or a function returning `impl Contract`; its associated types use production choices
unless explicit selections are supplied.

## Concrete types inferred from closures

Use a collection parameter when production supports several host types but a
test override needs one concrete type:

```rust
traitstub::deps! {
    for [H] where [H: source::Host + ?Sized];
    host_load(host: &H, id: u64) -> Result<Option<H::Value>, H::Error>,
        |host, id| host.load(id);
}
```

The internal function takes `deps: &impl Deps<H>`. Its public wrapper can supply
`&Deps`, so callers do not need to pass dependencies themselves. Production
still calls the real host and preserves its target/result and error types.

Inside the test module, `stubs!` explicitly supplies every shared stub as usual.
Those shared stubs work for every host satisfying the declared bounds. A test
then selects its host through the closure annotation:

```rust
let deps = traitstub::defs! {
    host_load => |_: &ProductionHost, _| Ok(Some(name.clone()));
};
```

Rust infers the collection type and the closure's result/error types. The closure
can capture test data without repeating the dependency signature. Later layers
inherit their base's types and cannot change them. Where several types remain
possible, use another closure annotation or an expected result type, just as in
ordinary generic Rust code. The [complete example](examples/common.rs) combines
this with all the other cases in one collection.

`for [H, Other] where [predicates];` accepts type parameters, including `?Sized`
bounds. The same collection can include associated types, per-method generics
and borrowed results. Shared stubs remain stateless; captured test behavior
belongs in `defs!`. Layers inherit their base's associated types. Neither form
boxes closures or adds runtime dispatch.

For inferred captures, borrowed references nested inside an output such as
`Option<&str>` need an explicit callable signature, for example
`current() -> Option<&'static str> => || Some("stub");`. For a test-owned result,
use `capture ['fixture];` and that named lifetime in the signature instead.

## Sharing dependencies between modules and crates

Production closures can use private imports from their own module. Types in
method signatures must also make sense where the stubs are defined. Use
built-in/prelude names, qualified standard-library paths, or public paths starting
with `source::` for types in the crate declaring the dependencies. Traitstub does
not resolve private aliases for you. The name `source` is reserved in generated
code.

Within a crate, import `Deps` and `Defs` normally. To share a contract with
another crate, add `export = unique_name; scope = crate::module;` before the
declarations. Shared stubs need these headers only if they are also exported. Export names must be unique within the crate,
and exported items and signature types must be public. Method `cfg` and
`cfg_attr` conditions are evaluated in the declaring crate, so the consumer
does not need to mirror its features.

## Current limitations

The macros handle synchronous methods taking `&self`, with named arguments and
optional return types. They support per-method generics, shared associated types,
borrowed results and captured `Fn` closures. Generic parameters and `where`
clauses use brackets so declarative macros can parse them.

Lifetime/const collection parameters, associated type bounds/GATs, async/unsafe/foreign methods,
`impl Trait` results, bare `Self` arguments, custom supertraits and `&mut self`
methods are outside the current syntax. A handwritten trait or implementation
may be the simpler choice for those cases.

Rust rejects incompatible types, escaping references and captures that do not
meet the required callable or thread bounds. The individual macro docs include
examples of these checks.

## Inspecting the generated code

The macros generate ordinary traits and implementations. To inspect them, use
[cargo-expand](https://github.com/dtolnay/cargo-expand):

```sh
cargo expand --example common
```

## Tests

Run the checks from this repository:

```sh
cargo test --lib
cargo test --all-targets
cargo test --doc
cargo test --test compilation
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

Unit tests beside each macro cover declarations, shared stubs, individual
overrides, borrowing, associated types, generics and worker threads.
Integration tests cover a consumer using all three macros, real compiler and
crate boundaries, and allocations with an isolated counting allocator.
Each test file defines its own setup; repeated cases use `rstest`.

`tests/compilation.rs` defines small programs in the test file and compiles them
in temporary directories to check rejected syntax and real provider/consumer
boundaries. Those consumer programs also run their behavioral assertions.
The README snippets are checked against the example source.

`rstest` is a development dependency only. The library still has no dependencies.

Rust 1.90 or newer.

## License

MIT. See [LICENSE](LICENSE).
