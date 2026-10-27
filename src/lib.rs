//! # Traitstub
//! Simple trait stubs for tests that still run your code.
//!
//! Testing code with external dependencies should make it clear which behavior
//! comes from your implementation and which behavior the test supplies. A test
//! setup that recreates the logic under test can pass while the real code still
//! has the problem you meant to catch.
//!
//! Rust already gives us a simple approach: put the dependencies behind a trait
//! and supply a different implementation in the test. The function under test
//! keeps the same body. Its calls go through the trait, and the test decides how
//! those dependencies behave.
//!
//! Doing this by hand works. Repeating all the trait implementations soon becomes
//! the part that makes the tests hard to read, especially when each test only
//! needs one small change.
//!
//! `traitstub` uses declarative macros to handle that boilerplate. The result is
//! ordinary Rust traits and implementations. You write the dependency functions
//! and the test behavior; the macros generate the repeated declarations and
//! forwarding code. There is no separate mocking runtime or global override state.
//!
//! You can implement the generated trait by hand wherever that is clearer. The
//! goal is to make this Rust pattern easier to use and the tests easier to read.
//!
//! # Usage
//!
//! - Connect dependency signatures to your existing production functions with [`deps!`].
//! - Pass the generated trait into the code using those dependencies.
//! - Define every shared test stub with [`stubs!`].
//! - Replace one or more functions in a test with [`defs!`].
//!
//! Production uses `&Deps`. Shared test stubs use `&Defs`. Individual overrides
//! return a value you can pass to the same function. You can keep this argument
//! private: a public wrapper supplies production defaults, while tests inject
//! their dependencies into the internal function. The examples below show this
//! split without exposing dependencies to production callers.
//!
//! Production-body defaults and shared stubs are zero-sized. Constructor-bound
//! dependencies store their callables and captured context. Overrides store their
//! base and closures directly. Traitstub adds no heap allocations, locks or global
//! state; dependency bodies and captured values can still allocate normally.
//!
//! A collection can also declare function signatures without production bodies.
//! Its constructor then binds the real callables where their context is known.
//! The collection trait has no type parameters; generic methods keep their own.
//! In this form, a helper can take `&impl calls::query<H>` to require just the
//! call its implementation needs. Test closures may support one concrete host,
//! while other calls retain their base implementations. Types remain in function
//! signatures; Traitstub has no associated-type declaration syntax.
//!
//! Short closure overrides reuse the declared argument and result types and can
//! capture test-owned data. Explicit signatures remain available when captures
//! need a different result lifetime. Use a stateless body override such as
//! `value() => 7;` when the implementation needs `Default`. See [`defs!`] for
//! generic and borrowed cases.
//!
//! # Examples
//!
//! Each file has one public entry point that uses production defaults, one private
//! implementation accepting dependencies, and a complete test setup. Comments on
//! the dependency declarations explain why each function is included.
//! Run them with `cargo run --example common`, `cargo run --example types` or
//! `cargo run --example workers`. Test all three with `cargo test --examples`.
//!
//! ## Common dependencies
//!
//! A greeting shows regular and renamed functions, borrowed results, mutable
//! arguments, test-owned captures and independent override layers.
#![doc = concat!("\n```rust\n", include_str!("../examples/common.rs"), "\n```\n")]
//!
//! ## Types and results
//!
//! Loading and saving a user shows custom values, adapting return types, generic
//! methods, errors and successive responses from a test-owned iterator.
#![doc = concat!("\n```rust\n", include_str!("../examples/types.rs"), "\n```\n")]
//!
//! ## Workers
//!
//! Callback arguments show `Fn`, `FnMut` and `FnOnce` bounds with generic results.
//! Worker tests also share captured state between concurrent callers. These
//! callback bounds do not change the `Fn` requirement on captured overrides.
#![doc = concat!("\n```rust\n", include_str!("../examples/workers.rs"), "\n```\n")]

mod defs;
mod deps;
mod shared;
mod stubs;

#[doc(hidden)]
pub use defs::results::SignatureResult as __TraitstubSignatureResult;
