//! Reuse declared or explicit Fn signatures and generate inline capture storage.
//!
//! Signature helpers process one override at a time. Separate storage prevents
//! dependency names from shadowing types in callable signatures.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_captures {
    (@inferred [[select $mode:tt $d:tt $lifetimes:tt $signature:tt]] $replacement:tt) => {
        $crate::__traitstub_entries! { @select $mode $d $lifetimes $signature $replacement }
    };
    // The result family is normalized by Rust using a receiver-like reference.
    // Static receiver lifetimes preserve the explicit-fixture rule for results;
    // input lifetimes remain higher-ranked for functions such as trim.
    (@signature $state:tt [$name:ident] [] $args:tt $where:tt [$value:expr]) => {
        $crate::__traitstub_captures! { @inferred $state
            [capture [] $args [__callers::$name::__StaticResult] [$value]] }
    };
    (@signature $state:tt [$name:ident] [$($lifetime:lifetime),+ $(,)?] [] [] [$value:expr]) => {
        $crate::__traitstub_captures! { @inferred $state
            [capture [] [] [__callers::$name::__StaticResult] [$value]] }
    };
    (@signature $state:tt [$name:ident] [$($lifetime:lifetime),+ $(,)?] $args:tt [] [$value:expr]) => {
        $crate::__traitstub_captures! { @inferred $state
            [capture [for<$($lifetime),+>] $args [__callers::$name::__Result<'static, $($lifetime),+>] [$value]] }
    };
    (@signature $state:tt $name:tt $gen:tt $args:tt $where:tt [$value:expr]) => {
        $crate::__traitstub_captures! { @inferred $state [body $value] }
    };
    // --- @storage: store closures without shadowing signature types with field names ---
    // The only generic parameters named after dependencies live in this small
    // storage impl. Typed Fn signatures elsewhere refer to projected types, so a
    // dependency named `String` or `usize` cannot shadow a signature's type.
    (@storage [$($name:ident)*]) => {
        #[allow(non_camel_case_types, clippy::builtin_type_shadow)]
        #[derive(Clone, Copy, Default)]
        struct __Captures<$($name,)*> { $($name: $name,)* }
        #[allow(non_camel_case_types, clippy::builtin_type_shadow)]
        trait __CaptureCalls {
            $(type $name; fn $name(&self) -> &Self::$name;)*
        }
        #[allow(non_camel_case_types, clippy::builtin_type_shadow)]
        impl<$($name,)*> __CaptureCalls for __Captures<$($name,)*> {
            $(type $name = $name;
              fn $name(&self) -> &Self::$name { &self.$name })*
        }
    };
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod inferred_signatures {
        use std::cell::RefCell;

        crate::deps! {
            write(value: &str, output: &mut String) -> Result<usize, &'static str>,
                |value, output| { output.push_str(value); Ok(value.len()) };
            count() -> usize, || 0;
            label() -> &str, || "production";
            copy[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
        }
        crate::stubs! {
            write => |_, _| Ok(0);
            count => || 0;
            label => || "stub";
            copy => |value| (value.clone(), value);
        }

        #[test]
        fn shorthand_move_captures_can_own_test_data() {
            fn configured() -> impl Deps {
                let values = String::from("abc");
                crate::defs! { count => move || values.len(); }
            }
            assert_eq!(configured().count(), 3);
        }

        #[test]
        fn shorthand_captures_reuse_declared_types_and_preserve_other_methods() {
            let calls = RefCell::new(Vec::new());
            let deps = crate::defs! {
                write => |value, output| {
                    calls.borrow_mut().push(value.to_owned());
                    output.push_str(value);
                    Ok(value.len())
                };
                count => || calls.borrow().len();
                label => || "override";
                copy => |value| (value.clone(), value);
            };
            let mut output = String::new();
            assert_eq!(deps.write("first", &mut output), Ok(5));
            let next = crate::defs! { target = deps; count => || 99; };
            assert_eq!(next.write("second", &mut output), Ok(6));
            assert_eq!(output, "firstsecond");
            assert_eq!(*calls.borrow(), ["first", "second"]);
            assert_eq!((deps.count(), next.count(), Defs.count()), (2, 99, 0));
            assert_eq!(next.label(), "override");
            assert_eq!(next.copy(String::from("x")), ("x".into(), "x".into()));
        }
    }

    mod borrowed_batches {

        crate::deps! {
            current() -> &str, || "production";
            trim['a](value: &'a str) -> &'a str, |value| value.trim();
            active() -> bool, || false;
        }
        crate::stubs! {
            current => || "";
            trim => |value| value;
            active => || false;
        }
        #[test]
        fn multiple_borrowed_closures_keep_input_and_fixture_lifetimes() {
            let fixture = String::from("fixture");
            let calls = std::cell::Cell::new(0);
            {
                let deps = crate::defs! {
                    capture ['fixture];
                    current() -> &'fixture str => || fixture.as_str();
                    active() => true;
                    for ['input] trim(value: &'input str) -> &'input str => |value| {
                        calls.set(calls.get() + 1);
                        value.trim_start()
                    };
                };
                let input = String::from(" hello ");
                assert_eq!(deps.current(), "fixture");
                assert!(deps.active());
                assert_eq!(deps.trim(&input), "hello ");
            }
            assert_eq!(calls.get(), 1);
        }
    }

    mod capture_names {

        crate::deps! {
            base() -> usize, || 0;
            marker() -> usize, || 0;
            replacement() -> usize, || 0;
        }
        crate::stubs! {
            base => || 0;
            marker => || 0;
            replacement => || 0;
        }
        #[test]
        fn dependency_names_do_not_collide_with_capture_storage() {
            let calls = std::cell::Cell::new(0);
            let deps = crate::defs! {
                base() -> usize => || { calls.set(calls.get() + 1); calls.get() };
                marker() -> usize => || 2;
                replacement() -> usize => || 3;
            };
            assert_eq!((deps.base(), deps.marker(), deps.replacement()), (1, 2, 3));
        }
    }

    #[allow(non_snake_case)]
    mod type_named_dependencies {

        crate::deps! {
            usize() -> usize, || 0;
            String() -> String, String::new;
        }
        crate::stubs! {
            usize => || 0;
            String => String::new;
        }
        #[test]
        fn capture_names_do_not_shadow_signature_types() {
            let deps = crate::defs! {
                usize() -> usize => || 20;
                String() -> String => || "value".to_owned();
            };
            assert_eq!(deps.usize(), 20);
            assert_eq!(deps.String(), "value");
        }
    }

    mod borrowed {

        crate::deps! {
            trim['a](value: &'a str) -> &'a str, |value| value.trim();
            current() -> &str, || "production";
        }
        crate::stubs! {
            current => || "shared";
            trim => |value| value;
        }
        #[test]
        fn shorthand_captures_reuse_declared_input_lifetimes() {
            let calls = std::cell::Cell::new(0);
            let deps = crate::defs! {
                trim => |value| {
                    calls.set(calls.get() + 1);
                    value.trim_start()
                };
            };
            let input = String::from(" hi ");
            let result = deps.trim(&input);
            assert_eq!(result, "hi ");
            assert_eq!(result.as_ptr(), input[1..].as_ptr());
            assert_eq!(calls.get(), 1);
        }
        #[test]
        fn captured_closures_can_preserve_input_lifetimes() {
            let calls = std::cell::Cell::new(0);
            {
                let deps = crate::defs! {
                    for ['input] trim(value: &'input str) -> &'input str => |value| {
                        calls.set(calls.get() + 1);
                        value.trim_start()
                    };
                };
                let input = String::from(" hi ");
                assert_eq!(deps.trim(&input), "hi ");
            }
            assert_eq!(calls.get(), 1);
        }
        #[test]
        fn borrows_inputs_and_test_owned_results() {
            let name = String::from("fixture");
            let deps = crate::defs! {
                capture ['fixture];
                current() -> &'fixture str => || name.as_str();
            };
            assert_eq!(deps.current(), "fixture");
            let deps = crate::defs! { target = deps; trim(value) => value.trim_start(); };
            let input = String::from(" hi ");
            assert_eq!(deps.trim(&input), "hi ");
            assert_eq!(deps.current(), "fixture");
        }
    }

    mod workers {
        use std::sync::atomic::{AtomicUsize, Ordering};

        crate::deps! {
            bounds = [Send + Sync];
            spawn[T, F](run: F) -> std::thread::JoinHandle<T>
            where [T: Send + 'static, F: FnOnce() -> T + Send + 'static], |run| std::thread::spawn(run);
            observe(value: usize) -> usize, |value| value;
        }
        crate::stubs! { spawn => |_run| panic!("spawn is not configured"); observe => |value| value; }
        #[test]
        fn shares_typed_captures_between_workers() {
            let calls = AtomicUsize::new(0);
            let deps = crate::defs! {
                spawn(run) => std::thread::spawn(run);
                observe => |value| { calls.fetch_add(1, Ordering::Relaxed); value };
            };
            std::thread::scope(|scope| {
                let a = scope.spawn(|| deps.observe(1));
                let b = scope.spawn(|| deps.observe(2));
                assert_eq!((a.join().unwrap(), b.join().unwrap()), (1, 2));
            });
            assert_eq!(calls.load(Ordering::Relaxed), 2);
            assert_eq!(deps.spawn(|| 7).join().unwrap(), 7);
            let deps = crate::defs! { spawn(run) => std::thread::spawn(run); };
            assert_eq!(deps.spawn(|| 9).join().unwrap(), 9);
        }
    }
}
