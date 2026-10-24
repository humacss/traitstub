//! Parse instance-local overrides and select their base, types and lifetimes.
//!
//! defs! loads schema metadata, then @headers and @parse normalize the request.
//! Captures collects typed closures; layer emits the new implementation using
//! type_defaults for associated-type resolution.
//! Replacements are `(name [body expression])`, `(name [method RustItem])` or
//! `(name [capture [binder] [argument types] [result type] [closure]])`.

/// Replace the dependencies an individual test cares about.
///
/// Start from `Defs`, or choose an existing implementation with `target = value;`.
/// Replace one function or several. The rest keep using the base implementation,
/// and the result is a new value owned by this test.
///
/// Keep the changes here focused on the dependency behavior this test needs.
/// The function under test still runs its own implementation.
///
/// You can build on that result again. Earlier implementations and production
/// functions keep their own behavior.
///
/// ```
/// traitstub::deps! { value() -> u64, || 1; other() -> u64, || 2; }
/// traitstub::stubs! { value => || 0; other => || 0; }
/// let first = traitstub::defs! { value() => 7; };
/// let second = traitstub::defs! { target = first; other() => 9; };
/// assert_eq!((second.value(), second.other(), Defs.value()), (7, 9, 0));
/// ```
///
/// # Using data from the test
///
/// `name => closure;` and `name(arguments) => body;` reuse the schema signature
/// and must be stateless in ordinary collections. A captured closure uses an explicit callable signature:
/// `value() -> u64 => || calls.get();`. Captures implement `Fn`; use test-owned
/// `Cell`/`RefCell` for mutable state, or atomics/synchronization across threads.
/// Captured signatures use concrete types (or accessible type aliases), not
/// `Self::Value`. Generic type/const methods use body overrides or full methods.
///
/// `capture ['fixture];` introduces a borrowed fixture lifetime, without creating
/// storage. `for ['a] trim(value: &'a str) -> &'a str => closure;` ties a captured
/// result to each input's lifetime. See the annotated example in the crate docs.
///
/// For a collection declared with `for [H] where [...];`, closures instead use
/// the schema's callable signature: `load => |host: &ConcreteHost, id| ...;`.
/// Rust infers H and the result/error types from the annotation and the call.
/// These closures may capture data directly. Later layers retain the same H.
/// Shared stubs must implement the contract for every valid H. Nested elided
/// references in results (such as `Option<&str>`) need an explicit callable
/// signature naming the result lifetime, just like captured fixture results.
///
/// # Choosing a base
///
/// `contract = api::Deps; target = instance;` explicitly selects the contract,
/// including for handwritten implementations. Associated types then use the
/// production defaults unless leading `type Value = Type;` selections are supplied;
/// selections must agree with the base. Header-free layers use the shared stubs'
/// selected types. For custom stub names, import the metadata as `Defs` or give
/// both `contract` and `target`.
///
/// The base is evaluated once, before captured closures, which are evaluated in
/// declaration order. Layers implement `Clone`, `Copy` and `Default` when their
/// stored values support them. Rust checks any bounds required by the contract.
/// Raw methods can call sibling overrides through `self`; inherited methods
/// continue calling their base's implementations.
///
/// # Invalid overrides
///
/// Rust checks closure bounds, lifetimes and types. Traitstub also rejects empty
/// definitions and duplicate dependency names:
///
/// ```compile_fail
/// traitstub::deps! { value() -> u64, || 0; }
/// traitstub::stubs! { value => || 0; }
/// # fn main() {
/// let mut calls = 0;
/// let deps = traitstub::defs! { value() -> u64 => || { calls += 1; calls }; };
/// let _ = deps.value();
/// # }
/// ```
///
/// ```compile_fail
/// traitstub::deps! { current() -> &str, || "production"; }
/// traitstub::stubs! { current => || ""; }
/// # fn main() {
/// let result;
/// {
///     let fixture = String::from("temporary");
///     let deps = traitstub::defs! {
///         capture ['fixture];
///         current() -> &'fixture str => || fixture.as_str();
///     };
///     result = deps.current();
/// }
/// assert_eq!(result,"temporary");
/// # }
/// ```
///
/// ```compile_fail
/// traitstub::deps! {
///     value() -> u64, || 1;
/// }
/// traitstub::stubs! { value => || 0; }
/// # fn main() {
/// let value = std::rc::Rc::new(7);
/// let deps = traitstub::defs! { value() -> u64 => move || *value; };
/// std::thread::scope(|scope| { scope.spawn(|| deps.value()); });
/// # }
/// ```
///
/// ```compile_fail
/// traitstub::deps! { value() -> u64, || 1; }
/// traitstub::stubs! { value => || 0; }
/// # fn main() {
/// let deps = traitstub::defs! { value() => 2; value() => 3; };
/// # }
/// ```
///
/// ```compile_fail
/// traitstub::deps! { value() -> u64, || 1; }
/// traitstub::stubs! { value => || 0; }
/// # fn main() {
/// let deps = traitstub::defs! {};
/// # }
/// ```
#[macro_export]
macro_rules! defs {
    (target = $target:expr; contract = $($contract:ident)::+; $($rest:tt)*) => {
        $crate::defs! { contract = $($contract)::+; target = $target; $($rest)* }
    };

    (contract = $($contract:ident)::+; target = $target:expr; $($request:tt)*) => {
        $($contract)::+! { @instance [$crate] [target = $target; $($request)*] }
    };
    (contract = $($contract:ident)::+; $($bad:tt)*) => {
        compile_error!("an explicit contract needs target = value;");
    };
    ($($request:tt)*) => { Defs! { [$crate] $($request)* } };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_defs {
    // --- @instance: receive the schema and start with Defs as the default base ---
    (@instance $root:tt $contract:tt $scope:tt $bounds:tt $assoc:tt $methods:tt $defaults:tt $path:tt [$($request:tt)*]) => {
        $crate::__traitstub_defs! { @headers [$root $contract $scope $bounds $assoc $methods $defaults $path] [Defs] [] []; $($request)* }
    };

    // --- @headers: read the target, type selections and captured lifetimes ---
    (@headers $context:tt $target:tt $choices:tt $lifetime:tt; target = $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @headers $context [$value] $choices $lifetime; $($rest)* }
    };
    (@headers $context:tt $target:tt [$($choices:tt)*] $lifetime:tt; type $name:ident = $ty:ty; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @headers $context $target [$($choices)* type $name = $ty;] $lifetime; $($rest)* }
    };
    (@headers $context:tt $target:tt $choices:tt []; capture [$($lifetime:lifetime),+]; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @headers $context $target $choices [$($lifetime),+]; $($rest)* }
    };
    (@headers $context:tt $target:tt $choices:tt $lifetimes:tt; $($entries:tt)*) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes []; $($entries)* }
    };

    // --- @parse: normalize heterogeneous overrides; reject empty/duplicate batches ---
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$($done:tt)*];
     $name:ident($($arg:ident),*) => $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes
            [$($done)* ($name [body |$($arg),*| $body])]; $($rest)* }
    };
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$($done:tt)*];
     $name:ident => $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes
            [$($done)* ($name [body $body])]; $($rest)* }
    };
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$($done:tt)*];
     $name:ident { $method:item } $($rest:tt)*) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes
            [$($done)* ($name [method $method])]; $($rest)* }
    };
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$($done:tt)*];
     for [$($binder:lifetime),+] $name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes
            [$($done)* ($name [capture [for<$($binder),+>] [$($ty),*] [$out] [$value]])]; $($rest)* }
    };
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$($done:tt)*];
     $name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes
            [$($done)* ($name [capture [] [$($ty),*] [$out] [$value]])]; $($rest)* }
    };
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$($done:tt)*]; $($tail:tt)+) => {
        $crate::__traitstub_defs! { @parse_last $context $target $choices $lifetimes [$($done)*]; $($tail)+ }
    };
    (@parse_last $context:tt $target:tt $choices:tt $lifetimes:tt $done:tt;
     $name:ident($($arg:ident),*) => $body:expr) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes $done; $name($($arg),*) => $body; }
    };
    (@parse_last $context:tt $target:tt $choices:tt $lifetimes:tt $done:tt;
     $name:ident => $body:expr) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes $done; $name => $body; }
    };
    (@parse_last $context:tt $target:tt $choices:tt $lifetimes:tt $done:tt;
     $(for [$($binder:lifetime),+])? $name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr) => {
        $crate::__traitstub_defs! { @parse $context $target $choices $lifetimes $done;
            $(for [$($binder),+])? $name($($arg: $ty),*) -> $out => $value; }
    };
    (@parse $context:tt $target:tt $choices:tt $lifetimes:tt [$(($name:ident [$($replacement:tt)*]))+];) => {{
        #[allow(dead_code)] struct __UniqueOverrides { $($name: (),)+ }
        $crate::__traitstub_captures! { @collect $context $target $choices $lifetimes [] [] [] [];
            $(($name [$($replacement)*]))+
        }
    }};
    (@parse_last $($bad:tt)*) => {
        compile_error!("defs! expects name => closure, name(arguments) => body, a raw method, or a typed Fn closure");
    };
    (@parse $($bad:tt)*) => { compile_error!("traitstub::defs! expects one or more body overrides, raw methods, or typed Fn closures"); };
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod declarative_core {
        mod caller_names {
            use std::rc::Rc as Box;
            trait Fn {}
            impl Fn for u64 {}
            crate::deps! { value(base: u64) -> u64, |base| base + 1; }
            crate::stubs! { value => |_| 0; }
            #[test]
            fn infrastructure_types_do_not_resolve_to_caller_names() {
                fn uses_local_trait(_: &impl Fn) {}
                uses_local_trait(&1u64);
                let _: Box<u64> = Box::new(1);
                let deps = crate::defs! { value => |base| base * 2; };
                assert_eq!(deps.value(3), 6);
            }
        }
        mod shorthand {
            crate::deps! {
                value(base: u64) -> u64, |base| base + 1;
                label() -> &'static str, || "production";
            }
            crate::stubs! { name = TestDefaults; value => |_| 0; label => || "test"; }
            #[test]
            fn body_shorthand_and_static_results_work_without_procedural_macros() {
                let deps = crate::defs! { contract = Deps; target = TestDefaults; value(call) => call * 3; };
                assert_eq!(deps.value(4), 12);
                assert_eq!(deps.label(), "test");
                assert_eq!(Deps.value(4), 5);
            }
        }
    }

    mod unified_api {
        //! One public macro set supports ordinary and generic dependency collections.
        crate::deps! {
            type Value = String;
            load(id: u64) -> Self::Value, |id| id.to_string();
            copy[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
            trim['a](value: &'a str) -> &'a str, |value| value.trim();
            record(value: u64), |_value| ();
        }
        crate::stubs! {
            load => |_| String::new();
            copy => |value| (value.clone(), value);
            trim => |value| value;
            record => |_| ();
        }
        #[test]
        fn root_macros_combine_types_generics_borrows_and_captured_state() {
            let recorded = std::cell::RefCell::new(Vec::new());
            let deps = crate::defs! {
                load(id) => format!("user-{id}");
                trim(value) => value.trim_start();
                record(value: u64) -> () => |value| recorded.borrow_mut().push(value);
            };
            assert_eq!(deps.load(7), "user-7");
            assert_eq!(deps.copy(3), (3, 3));
            let text = String::from(" hello ");
            assert_eq!(deps.trim(&text), "hello ");
            deps.record(7);
            assert_eq!(*recorded.borrow(), [7]);
            assert_eq!(Defs.load(7), "");
        }

        mod ordinary_shorthand {
            crate::deps! {
                record(values: &mut Vec<u64>), |values| values.push(1)
            }
            crate::stubs! {
                record => |values| values.push(2)
            }
            #[test]
            fn omitted_unit_results_and_final_semicolons_keep_working() {
                let deps = crate::defs! { record(values) => values.push(3) };
                let mut values = Vec::new();
                Deps.record(&mut values);
                Defs.record(&mut values);
                deps.record(&mut values);
                assert_eq!(values, [1, 2, 3]);
            }
        }
    }
}
