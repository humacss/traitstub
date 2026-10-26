//! Generate dependency traits, production implementations and schema metadata.
//!
//! Metadata replays signatures for stubs!/defs!, selects methods by literal name,
//! and initializes signature-checked captures. Local metadata stays crate-visible;
//! exported metadata uses the declaring crate even when its dependency is renamed.
//! `$d` carries a dollar token into generated macro_rules! definitions; `$runtime`
//! identifies Traitstub in the crate invoking those definitions.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_schema {

    (@production [$contract:ident] [__functions] []
     $($(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
       ($($arg:ident:$ty:ty),*) -> $out:ty $(where [$($where:tt)*])?, $ignored:expr;)*) => {
        $crate::__traitstub_functions! { @production [$contract]
            [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?]
                ($($arg:$ty),*) [$out] [$($($where)*)?]) )*] }
    };
    // --- @production: emit the trait and its zero-sized real implementation ---
    (@production [$contract:ident] [$($bounds:tt)*] []
     $($(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
       ($($arg:ident: $ty:ty),*) -> $out:ty $(where [$($where:tt)*])?, $production:expr;)*) => {
        pub trait $contract: $($bounds)* {
            $($(#[$($attr)*])* fn $name $(<$($gen)*>)? (&self, $($arg: $ty),*) -> $out $(where $($where)*)?;)*
        }
        #[doc(hidden)]
        #[derive(Clone, Copy, Default)]
        pub struct __TraitstubProduction;
        impl $contract for __TraitstubProduction {
            $($(#[$($attr)*])* fn $name $(<$($gen)*>)? (&self, $($arg: $ty),*) -> $out $(where $($where)*)? {
                let call: fn($($ty),*) -> _ = $production;
                call($($arg),*)
            })*
        }
        #[allow(non_upper_case_globals)]
        pub const $contract: __TraitstubProduction = __TraitstubProduction;
    };

    // --- @schema: emit exported metadata, or dispatch to crate-local metadata ---
    (@schema [$d:tt] [__local] $contract:tt $export:tt $scope:tt $bounds:tt $types:tt $($entries:tt)*) => {
        $crate::__traitstub_schema! { @schema_local [$d] [__local] $contract $export $scope $bounds $types $($entries)* }
    };

    (@schema [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] [$($scope:ident)*]
     [$($bounds:tt)*] []
     $($(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
       ($($arg:ident: $ty:ty),* $(,)?) -> $out:ty $(where [$($where:tt)*])?, $production:expr;)*) => {
        $crate::__traitstub_schema! { @source [$origin] }

        $crate::__traitstub_schema! { @production
            [$contract] [$($bounds)*] []
            $($(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
              ($($arg: $ty),*) -> $out $(where [$($where)*])?, $production;)*
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! $export {

            (@defaults [$d($d runtime:tt)*] [$d output:ident] [$d metadata:ident] [$d default_origin:ident] [$d($d default_scope:ident)*]
             [] [$d($d overrides:tt)*]) => {
                $d($d runtime)*::__traitstub_stubs! { @defaults [$d] [$d $origin] [$contract] [$($scope)*] [$($bounds)*]
                    []
                    [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($where)*)?]) )*]
                    [$d output] [$d metadata] [$d default_origin] [$d($d default_scope)*]
                    [] [$d($d overrides)*]
                }
            };
            (@instance [$d($d runtime:tt)*] [$d($d request:tt)*]) => {
                $d($d runtime)*::__traitstub_defs! { @instance [$d $origin] [$contract] [$($scope)*] [$($bounds)*] []
                    [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($where)*)?]) )*]
                    [$d $origin] [$($scope ::)* __TraitstubProduction] [$d($d request)*]
                }
            };
            (@functions) => { #[allow(unused_imports)] use source $(::$scope)* :: calls as __function_schema; };
            $(
                (@select [$d($d runtime:tt)*] [$d kind:ident] [$d attrs:tt $name $d($d signature:tt)*]
                 [$name] [$d($d value:tt)*] [$d($d rest:tt)*]) => {
                    $d($d runtime)*::__traitstub_methods! { @emit [$d kind] [$d attrs $name $d($d signature)*] [$d($d value)*] }
                };
            )*
            (@select [$d($d runtime:tt)*] [$d kind:ident] [$d($d signature:tt)*]
             [$d other:ident] [$d($d value:tt)*] [$d($d rest:tt)*]) => {
                $d($d runtime)*::__traitstub_methods! { @find [$d kind] [$d($d signature)*] [$d($d rest)*] }
            };
            $(
                (@infer [$d($d runtime:tt)*] $name [$d value:expr]) => {
                    $d($d runtime)*::__traitstub_functions! { @infer [$($($gen)*)?] [$name] [$d value] }
                };
            )*
            $( (@valid_method $name) => {}; )*
            (@valid_method $d bad:ident) => { compile_error!(concat!("unknown dependency: ", stringify!($d bad))); };
        }
        #[doc(hidden)]
        pub use $export as $contract;
    };

    // --- @schema_local: same schema protocol, with crate-local visibility and paths ---
    (@schema_local [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] [$($scope:ident)*]
     [$($bounds:tt)*] []
     $($(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
       ($($arg:ident: $ty:ty),* $(,)?) -> $out:ty $(where [$($where:tt)*])?, $production:expr;)*) => {
        $crate::__traitstub_schema! { @source [$origin] }

        $crate::__traitstub_schema! { @production
            [$contract] [$($bounds)*] []
            $($(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
              ($($arg: $ty),*) -> $out $(where [$($where)*])?, $production;)*
        }
        macro_rules! $export {

            (@defaults [$d($d runtime:tt)*] [$d output:ident] [$d metadata:ident] [$d default_origin:ident] [$d($d default_scope:ident)*]
             [] [$d($d overrides:tt)*]) => {
                $d($d runtime)*::__traitstub_stubs! { @defaults [$d] [$origin] [$contract] [$($scope)*] [$($bounds)*]
                    []
                    [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($where)*)?]) )*]
                    [$d output] [$d metadata] [$d default_origin] [$d($d default_scope)*]
                    [] [$d($d overrides)*]
                }
            };
            (@instance [$d($d runtime:tt)*] [$d($d request:tt)*]) => {
                $d($d runtime)*::__traitstub_defs! { @instance [$origin] [$contract] [$($scope)*] [$($bounds)*] []
                    [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($where)*)?]) )*]
                    [$origin] [$($scope ::)* __TraitstubProduction] [$d($d request)*]
                }
            };
            (@functions) => { #[allow(unused_imports)] use calls as __function_schema; };
            $(
                (@select [$d($d runtime:tt)*] [$d kind:ident] [$d attrs:tt $name $d($d signature:tt)*]
                 [$name] [$d($d value:tt)*] [$d($d rest:tt)*]) => {
                    $d($d runtime)*::__traitstub_methods! { @emit [$d kind] [$d attrs $name $d($d signature)*] [$d($d value)*] }
                };
            )*
            (@select [$d($d runtime:tt)*] [$d kind:ident] [$d($d signature:tt)*]
             [$d other:ident] [$d($d value:tt)*] [$d($d rest:tt)*]) => {
                $d($d runtime)*::__traitstub_methods! { @find [$d kind] [$d($d signature)*] [$d($d rest)*] }
            };
            $(
                (@infer [$d($d runtime:tt)*] $name [$d value:expr]) => {
                    $d($d runtime)*::__traitstub_functions! { @infer [$($($gen)*)?] [$name] [$d value] }
                };
            )*
            $( (@valid_method $name) => {}; )*
            (@valid_method $d bad:ident) => { compile_error!(concat!("unknown dependency: ", stringify!($d bad))); };
        }
        #[allow(unused_imports)]
        pub(crate) use $export as $contract;
    };

    // --- @source: give production closures access to the declaring crate as source ---
    (@source [__local]) => {
        #[allow(unused_imports)]
        use crate as source;
    };
    (@source [$origin:ident]) => {
        #[allow(unused_imports)]
        use $origin as source;
    };
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod contracts {
        //! Generics, signature scope and handwritten bases.

        mod method_generics {

            crate::deps! {
                echo['a, T](value: &'a T) -> &'a T, |value| value;
                array[T, const N: usize] call [T, N](value: T) -> [T; N]
                where [T: Clone], |value| std::array::from_fn(|_| value.clone());
            }
            crate::stubs! {
                echo => |value| value;
                array => |value| std::array::from_fn(|_| value.clone());
            }
            #[test]
            fn method_lifetimes_consts_and_where_clauses_survive_forwarding() {
                let value = String::from("value");
                let deps = crate::defs! {
                    array(value) => std::array::from_fn(|_| value.clone());
                };
                assert_eq!(
                    deps.array::<_, 2>(value.clone()),
                    [value.clone(), value.clone()]
                );
                assert!(std::ptr::eq(deps.echo(&value), &value));
            }
        }

        mod owned_fixture {

            crate::deps! {
                name() -> &str, || "production";
                replace(value: String) -> String, |value| value;
            }
            struct Named(String);
            impl Deps for Named {
                fn name(&self) -> &str {
                    &self.0
                }
                fn replace(&self, value: String) -> String {
                    value
                }
            }
            #[test]
            fn inherited_results_can_borrow_from_a_handwritten_base() {
                let deps = crate::defs! {
                    contract = Deps;
                    target = Named("Ada".into());
                    replace(value) => value.to_uppercase();
                };
                assert_eq!(deps.replace("Grace".into()), "GRACE");
                assert_eq!(deps.name(), "Ada");
            }
        }

        mod signature_scope {

            pub type Count = u64;
            pub struct Value(pub Count);
            pub struct View<'a>(pub &'a [u64]);
            fn helper() -> Count {
                1
            }
            crate::deps! {
                path() -> std::path::PathBuf, std::path::PathBuf::new;
                value() -> source::deps::schema::tests::contracts::signature_scope::Value, || Value(helper());
                number() -> source::deps::schema::tests::contracts::signature_scope::Count, helper;
                read(view: &source::deps::schema::tests::contracts::signature_scope::View<'_>) -> usize, |view| view.0.len();
                formatted[T: std::fmt::Debug](value: T) -> String, |value| format!("{value:?}");
                #[cfg(any())]
                unavailable[T: DoesNotExist](value: T) -> (), |_value| ();
            }
        }
        mod caller_scope {
            use super::signature_scope::Deps;
            // Caller names intentionally differ from the declaration's types/helpers.
            struct Value(String);
            fn helper() -> u64 {
                99
            }
            crate::stubs! {
                path => std::path::PathBuf::new;
                value => || source::deps::schema::tests::contracts::signature_scope::Value(42);
                number => helper;
                read => |view| view.0.len();
                formatted => |value| format!("{value:?}");
            }
            #[test]
            fn signature_paths_and_disabled_methods_survive_a_different_scope() {
                let deps = crate::defs! {
                    number() => Value("test".into()).0.len() as u64;
                };
                assert_eq!(Deps.number(), 1);
                assert_eq!(Deps.value().0, 1);
                assert_eq!(Defs.number(), 99);
                assert_eq!(deps.number(), 4);
                assert_eq!(deps.value().0, 42);
                assert!(deps.path().as_os_str().is_empty());
                assert_eq!(deps.read(&super::signature_scope::View(&[1, 2])), 2);
                assert_eq!(deps.formatted(19), "19");
            }
        }

        mod default_and_clone {

            crate::deps! {
                bounds = [Default + Clone];
                value() -> u64, || 1;
            }
            crate::stubs! { value => || 0; }
            #[test]
            fn stateless_layers_preserve_clone_and_default_bounds() {
                let deps = crate::defs! { value() => 7; };
                assert_eq!(deps.clone().value(), 7);
                fn fresh<D: Deps>(_: &D) -> D {
                    D::default()
                }
                assert_eq!(fresh(&deps).value(), 7);
            }
        }

        mod production_bindings {

            crate::deps! {
                value(n: u64) -> u64, |mut n| { n += 1; n };
                other() -> u64, || 0;
            }
            crate::stubs! { value => |n| n * 2; other => || 0; }
            #[test]
            fn mutable_argument_bindings_stay_inside_production_closures() {
                let deps = crate::defs! { other() => 5; };
                assert_eq!(Deps.value(2), 3);
                assert_eq!(deps.value(2), 4);
                assert_eq!(deps.other(), 5);
            }
        }
    }
}
