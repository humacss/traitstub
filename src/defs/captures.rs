//! Collect typed Fn captures and generate their inline storage.
//!
//! Collection preserves declaration order and accumulates predicates, fields,
//! initializers and replacement markers. Separate capture storage prevents
//! dependency names from shadowing types in callable signatures.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_captures {
    // --- @collect: accumulate capture predicates, fields, initializers and replacements ---
    (@collect $context:tt $target:tt $choices:tt $lifetimes:tt
     [$($predicate:tt)*] [$($field:tt)*] [$($init:tt)*] [$($methods:tt)*];) => {
        $crate::__traitstub_layer! { @emit $context $target $choices $lifetimes [$($predicate)*]
            [$($field)*] [$($init)*] [$($methods)*]
        }
    };
    (@collect [$root:tt $contract:tt $scope:tt [$($bounds:tt)*] $assoc:tt $methods:tt $defaults:tt $path:tt]
     $target:tt $choices:tt [$($lifetime:lifetime),*]
     [$($predicate:tt)*] [$($field:tt)*] [$($init:tt)*] [$($overrides:tt)*]
     ; ($name:ident [capture [$($binder:tt)*] [$($args:ty),*] [$out:ty] [$value:expr]]) $($rest:tt)*) => {
        $crate::__traitstub_captures! { @collect [$root $contract $scope [$($bounds)*] $assoc $methods $defaults $path]
            $target $choices [$($lifetime),*]
            [$($predicate)* <__Functions as __CaptureCalls>::$name: $($binder)* ::core::ops::Fn($($args),*) -> $out + $($bounds)*,]
            [$($field)* $name]
            [$($init)* $name: {
                fn typed<$($lifetime,)* __Function>(value: __Function) -> __Function
                where __Function: $($binder)* ::core::ops::Fn($($args),*) -> $out + $($bounds)* { value }
                typed($value)
            },]
            [$($overrides)* ($name [capture])]; $($rest)*
        }
    };
    (@collect $context:tt $target:tt $choices:tt $lifetimes:tt
     $predicate:tt $field:tt $init:tt [$($methods:tt)*];
     ($name:ident [$($replacement:tt)*]) $($rest:tt)*) => {
        $crate::__traitstub_captures! { @collect $context $target $choices $lifetimes $predicate $field $init
            [$($methods)* ($name [$($replacement)*])]; $($rest)*
        }
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
                observe(value: usize) -> usize => |value| { calls.fetch_add(1, Ordering::Relaxed); value };
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
