//! Validate associated-type selections and resolve the selected base defaults.
//! Local production defaults need a type-only implementation so dependent and
//! qualified Self projections resolve even when the contract is imported.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_type_defaults {
    // --- @check_types: reject unknown or duplicate associated-type selections ---
    (@check_types [$(type $name:ident = $ty:ty;)*]) => {
        #[allow(dead_code, non_snake_case)] struct __UniqueTypes { $($name: (),)* }
        $(__schema!(@valid_type $name);)*
    };

    // --- @import_defaults: resolve associated types from the chosen defaults ---
    (@import_defaults [$d:tt] [__local] [__TraitstubProduction] [$($assoc:ident)*] $methods:tt) => {
        // Local contracts can be imported without a path to their hidden
        // production type. This implementation is used only for type projection;
        // no value is constructed or method called. Implement the actual contract
        // so both Self::Value and <Self as Contract>::Value resolve correctly.
        #[derive(Clone, Copy, Default)]
        struct __Defaults;
        impl __schema for __Defaults {
            $(type $assoc = __schema!(@production_type $assoc);)*
            $crate::__traitstub_methods! { @implement [type_defaults] $methods [] }
        }
        #[allow(unused_macros)]
        macro_rules! __traitstub_default_type {
            ($d assoc:ident) => { <__Defaults as __schema>::$d assoc };
        }
    };
    (@import_defaults [$d:tt] [__local] [$($path:ident)::+] $assoc:tt $methods:tt) => {
        use $($path)::+ as __Defaults;
        #[allow(unused_macros)]
        macro_rules! __traitstub_default_type {
            ($d assoc:ident) => { <__Defaults as __schema>::$d assoc };
        }
    };
    (@import_defaults [$d:tt] [$root:ident] [$($path:ident)::+] $assoc:tt $methods:tt) => {
        use $root::{self as defaults};
        use defaults::$($path)::+ as __Defaults;
        #[allow(unused_macros)]
        macro_rules! __traitstub_default_type {
            ($d assoc:ident) => { <__Defaults as __schema>::$d assoc };
        }
    };
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod associated_defaults {
        mod declaration {
            crate::deps! {
                type Value = u64;
                type Error = Option<Self::Value>;
                type Qualified = Option<<Self as Deps>::Value>;
                qualified() -> Result<Self::Value, Self::Qualified>, || Ok(42);
                action() -> Result<Self::Value, Self::Error>, || Ok(42);
            }
        }
        use declaration::Deps;
        crate::stubs! { action => || Ok(0); qualified => || Ok(0); }

        #[test]
        fn explicit_contract_resolves_dependent_defaults_from_another_module() {
            let production = crate::defs! {
                contract = Deps; target = Deps;
                action() => Ok(7);
            };
            let shared = crate::defs! {
                contract = Deps; target = Defs;
                action() => Err(Some(9));
            };
            assert_eq!(production.action(), Ok(7));
            assert_eq!(production.qualified(), Ok(42));
            assert_eq!(shared.qualified(), Ok(0));
            assert_eq!(shared.action(), Err(Some(9)));
        }

        mod copy_contract {
            crate::deps! {
                bounds = [Copy];
                value() -> u64, || 1;
            }
            crate::stubs! { value => || 0; }
            #[test]
            fn copy_bounds_work_for_stateless_and_copy_captured_overrides() {
                let deps = crate::defs! { value() => 7; };
                let copied = deps;
                assert_eq!((deps.value(), copied.value()), (7, 7));
                let increment = 2u64;
                let deps = crate::defs! { value() -> u64 => move || increment + 7; };
                let copied = deps;
                assert_eq!((deps.value(), copied.value()), (9, 9));
            }
        }
    }

    mod typed_dependencies {
        pub struct User(pub u64);
        crate::deps! {
            type Value = source::defs::type_defaults::tests::typed_dependencies::User;
            load(id: u64) -> Self::Value, |id| User(id);
            save(value: Self::Value) -> u64, |value| value.0;
            duplicate[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
        }
        mod tests {
            use super::*;
            pub struct TestUser(pub u64);
            crate::stubs! {
                type Value = crate::defs::type_defaults::tests::typed_dependencies::tests::TestUser;
                load => |id| TestUser(id + 1);
                save => |value| value.0;
                duplicate => |value| (value.clone(), value);
            }
            #[test]
            fn types_and_generic_functions_inherit_shared_defaults() {
                assert_eq!(Deps.save(Deps.load(1)), 1);
                let deps = crate::defs! { save(value) => value.0 * 2; };
                assert_eq!(deps.save(deps.load(1)), 4);
                let deps = crate::defs! { duplicate(value) => (value.clone(), value); };
                assert_eq!(deps.duplicate(3), (3, 3));
                assert_eq!(deps.duplicate("x"), ("x", "x"));
            }
        }

        mod across_modules {
            use crate::defs::type_defaults::tests::typed_dependencies::tests::Defs;
            use crate::defs::type_defaults::tests::typed_dependencies::Deps;

            #[test]
            fn imported_defaults_keep_their_concrete_type_choices() {
                let deps = crate::defs! { save(value) => value.0 * 3; };
                assert_eq!(deps.save(deps.load(1)), 6);
            }
        }
        mod manual_base {
            use crate::defs::type_defaults::tests::typed_dependencies::{Deps, User};

            struct Fixture;
            impl Deps for Fixture {
                type Value = User;
                fn load(&self, id: u64) -> User {
                    User(id + 1)
                }
                fn save(&self, value: User) -> u64 {
                    value.0
                }
                fn duplicate<T: Clone>(&self, value: T) -> (T, T) {
                    (value.clone(), value)
                }
            }
            #[test]
            fn explicit_contract_wraps_a_handwritten_implementation_once() {
                let mut evaluations = 0;
                let deps = crate::defs! {
                    contract = crate::defs::type_defaults::tests::typed_dependencies::Deps;
                    target = { evaluations+=1; Fixture };
                    save(value) => value.0 * 3;
                };
                assert_eq!(evaluations, 1);
                assert_eq!(deps.save(deps.load(1)), 6);
            }
        }
    }
}
