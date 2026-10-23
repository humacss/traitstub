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
//! return a value you can pass to the same function.
//!
//! Production and shared defaults are zero-sized. Overrides store their base and
//! closures directly; the library adds no heap allocations, locks or global state.
//! Dependency bodies and captured values can still allocate normally.
//!
//! # Example
//!
//! The example starts with production functions and shared test stubs, then shows
//! captured data, borrowed values, associated types, generic methods and workers.
//! Run the tests with `cargo test --example common`.
#![doc = concat!("\n```rust\n", include_str!("../examples/common.rs"), "\n```\n")]

mod defs;
mod deps;
mod shared;
mod stubs;
