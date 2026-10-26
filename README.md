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
are excerpts from [common.rs](examples/common.rs), where `users` and `teams`
contain ordinary Rust functions. Omitted declarations are marked with `// ...`.
Each example has one dependency collection and one operation under test.

<!-- example: dependencies -->
```rust
traitstub::deps! {
    // ...
    load_name(id: u64) -> String, |id| users::load_name(id);
    // ...
    team_name(id: u64) -> String, |id| teams::load_name(id);
    // ...
}
```
<!-- endexample -->

Keep dependencies inside your implementation. The public function supplies
production defaults; a private function takes the trait so tests can inject stubs:

<!-- example: consumer -->
```rust
pub fn greet(id: u64) -> Result<String, &'static str> {
    greet_with_deps(&Deps, id)
}
// ...
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
```
<!-- endexample -->

Production callers just use `greet(7)`. Tests in the same module call
`greet_with_deps` instead. Use `stubs!` to define their shared dependencies:

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

assert_eq!(
    greet_with_deps(&deps, 7),
    Ok("Hello, Ada from Test Team!".into())
);
```
<!-- endexample -->

`defs!` returns a new implementation. Other tests keep their own defaults. Use
`target = existing;` to build on an implementation you have already configured.

### See it in action

Each example keeps one public entry point, one private implementation accepting
dependencies, and its own complete shared test stubs. Every dependency declaration
has a comment explaining the capability it demonstrates.

| Example | Operation | What it shows |
| --- | --- | --- |
| [common.rs](examples/common.rs) | `greet` | Forwarding, renaming, borrowed inputs/results, mutable arguments and captured overrides |
| [types.rs](examples/types.rs) | `process` | Custom types, adapted results, retry sequences, error propagation and function generics |
| [workers.rs](examples/workers.rs) | `work` | `Fn`/`FnMut`/`FnOnce` callbacks, generic results and counters shared between threads |

In the types example, a closure adapts an ordinary return value to the contract
the consumer needs. Tests can then supply a missing value or an error:

<!-- example: custom-types -->
```rust
load(id: u64) -> Result<Option<source::User>, &'static str>, |id| Ok(Some(storage::load(id)));
```
<!-- endexample -->

The workers example keeps the launcher's generic task and result types, with
Rust checking the bounds required to move the task between threads:

<!-- example: worker-generics -->
```rust
spawn[T, F](name: String, run: F) -> std::io::Result<std::thread::JoinHandle<T>>
where [T: Send + 'static, F: FnOnce() -> T + Send + 'static],
    |name, run| std::thread::Builder::new().name(name).spawn(run);
```
<!-- endexample -->

The same file compares `Fn` callbacks that read captured data repeatedly with
`FnMut` callbacks that change a captured counter. These are arguments passed to
dependencies. Captured dependency overrides themselves still require `Fn`.

Read the full files to see these declarations used by production code, shared
stubs and individual tests. Run them independently or test all three:

```sh
cargo run --example common
cargo run --example types
cargo run --example workers
cargo test --examples
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
synchronization. Captured signatures use concrete types or accessible aliases. Thread safety is checked where an instance is shared.
Generic type/const methods use body overrides or complete methods rather than
captured closures. All overrides in one `defs!` share an implementation, so raw
methods can call sibling overrides through `self`. Inherited methods continue
to call their base's implementations.

If you give shared stubs a custom name, import it as `Defs` or specify both
`contract` and `target`. An explicit contract also supports a handwritten base
or a function returning `impl Contract`.

## Functions that borrow production context

Omit production bodies to construct dependencies from callables where their
context is available. Function generics stay on the functions. Their signatures
can refer to the host's existing types:

```rust
traitstub::deps! {
    query[H](host: &H, id: u64) -> Result<Option<H::Value>, H::Error>
        where [H: source::Host + ?Sized];
}
```

`Host` is your existing integration trait. The collection declares no additional
associated types. Production binds a real callable:

```rust
let deps = Deps(|host: &MyHost, id| host.query(id));
```

A consumer requires the call it uses:

```rust
fn read<H: Host + ?Sized>(
    deps: &impl calls::query<H>, host: &H, id: u64,
) -> Result<Option<H::Value>, H::Error> {
    deps.query(host, id)
}
```

The collection trait `Deps` remains nongeneric. The generated `calls::query<H>`
contract checks that this implementation can handle this function's chosen host.
Shared stubs define every generic function; individual overrides can support one
concrete host:

```rust
traitstub::stubs! { query => |_, _| Ok(None); }

let value = 42;
let deps = traitstub::defs! {
    query => |_: &MyHost, id| Ok(Some(value + id));
};
```

Each override stores its closure directly. Other methods keep their base's
capabilities. Replacing a function can also select a different host than its
base supports. A mismatched host is a compile error.

The host can instead be captured by the production closure. Its type can still
appear in the function's result; call with `query::<H>(id)` when it cannot be
inferred from an argument. This lets exported functions keep their public API
and pass dependencies into private implementations.

See [tests/function_bindings.rs](tests/function_bindings.rs) for production
forwarding, shared stubs and captured overrides with numeric and text hosts.
Run it with `cargo test --test function_bindings`.

This constructor form uses plain type parameter names, with bounds in
`where [...]`. It supports owned results and ordinary input references.
Use the production-body form for lifetime/const generics and results borrowed
from inputs or `self`; [common.rs](examples/common.rs) shows the borrowing cases. Constructor-bound overrides use
closures; raw method overrides belong to the production-body form.
When using a local constructor-bound contract from another module, import both
`Deps` and `calls`; exported contracts carry their module path automatically.
This form reserves the generated `calls` module name.

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
optional return types. They support per-method generics, borrowed results and captured `Fn` closures. Generic parameters and `where`
clauses use brackets so declarative macros can parse them.

Collection type parameters, associated-type declarations, async/unsafe/foreign methods,
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
overrides, borrowing, generics and worker threads.
Integration tests cover a consumer using all three macros, real compiler and
crate boundaries, and allocations with an isolated counting allocator.
Each test file defines its own setup; repeated cases use `rstest`.

`tests/compilation.rs` defines small programs in the test file and compiles them
in temporary directories to check rejected syntax and real provider/consumer
boundaries. Those consumer programs also run their behavioral assertions.
The README snippets are checked against their respective example files.

`rstest` is a development dependency only. The library still has no dependencies.

Rust 1.90 or newer.

## License

MIT. See [LICENSE](LICENSE).
