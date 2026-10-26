//! Generate an override layer and its forwarding or replacement methods.
//! The target is evaluated once and retained as the base of the new value.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_layer {
    // --- @emit: generate the layer and its forwarding or replacement methods ---
    (@emit [[$root:ident] [$contract:ident] [$($scope:ident)*] [$($bounds:tt)*] [$($assoc:ident)*] $methods:tt
      [$default_root:ident] [$($default_path:ident)::+]]
     [$target:expr] $choices:tt [$($lifetime:lifetime),*] [$($predicate:tt)*] [$($field:tt)*] [$($init:tt)*]
     [$(($changed:ident [$($replacement:tt)*]))*]) => {{
        $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
        $(__schema!(@valid_method $changed);)*
        $crate::__traitstub_captures! { @storage [$($field)*] }
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Default)]
        struct __Layer<$($lifetime,)* __Base, __Functions>
        where __Base: __schema, __Functions: __CaptureCalls + $($bounds)*, $($predicate)* {
            base: __Base,
            marker: ::core::marker::PhantomData<($(&$lifetime (),)*)>,
            replacement: __Functions,
        }
        #[allow(non_camel_case_types)]
        impl<$($lifetime,)* __Base, __Functions> __schema for __Layer<$($lifetime,)* __Base, __Functions>
        where __Base: __schema, __Functions: __CaptureCalls + $($bounds)*, $($predicate)* {
            $crate::__traitstub_methods! { @implement [layer] $methods [$(($changed [$($replacement)*]))*] }
        }
        let __base = $target;
        __Layer {
            base: __base,
            marker: ::core::marker::PhantomData,
            replacement: __Captures { $($init)* },
        }
    }};
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod layers {
        use rstest::rstest;

        crate::deps! { target = GreetDeps;
            load_name(id: u64) -> String, |id| format!("user-{id}");
            team_name(id: u64) -> String, |id| format!("team-{id}");
        }
        crate::stubs! { target = GreetDeps; name = NamespaceDefs;
            load_name => |_| "Test User".into(); team_name => |_| "Test Team".into();
        }
        fn greet(deps: &impl GreetDeps, id: u64) -> String {
            format!("Hello, {} from {}!", deps.load_name(id), deps.team_name(id))
        }
        fn namespace_defaults() -> impl GreetDeps {
            NamespaceDefs
        }
        #[derive(Clone, Copy)]
        struct Custom;
        impl GreetDeps for Custom {
            fn load_name(&self, id: u64) -> String {
                format!("custom-{id}")
            }
            fn team_name(&self, _: u64) -> String {
                "Custom Team".into()
            }
        }

        #[rstest]
        #[case::production(GreetDeps)]
        #[case::shared(NamespaceDefs)]
        #[case::handwritten(Custom)]
        fn overrides_preserve_other_methods_and_leave_the_base_unchanged<D: GreetDeps + Copy>(
            #[case] base: D,
            #[values(0, 7, 8, u64::MAX)] id: u64,
        ) {
            let original = greet(&base, id);
            let user = crate::defs! { contract = GreetDeps; target = base;
                load_name => |id| format!("guest-{id}");
            };
            let team = crate::defs! { contract = GreetDeps; target = base;
                team_name => |_| "Admins".into();
            };
            assert_eq!(user.load_name(id), format!("guest-{id}"));
            assert_eq!(user.team_name(id), base.team_name(id));
            assert_eq!(team.load_name(id), base.load_name(id));
            assert_eq!(team.team_name(id), "Admins");
            assert_eq!(greet(&base, id), original);
        }

        #[rstest]
        #[case::names("Ada", "Grace")]
        #[case::replacements("first", "second")]
        fn layers_replace_previous_behavior_instead_of_queuing_it(
            #[case] first: &'static str,
            #[case] latest: &'static str,
        ) {
            let first = crate::defs! { contract = GreetDeps; target = namespace_defaults();
                load_name(_id: u64) -> String => move |_| first.into();
            };
            let team = crate::defs! { contract = GreetDeps; target = first; team_name => |_| "Admins".into(); };
            let final_layer = crate::defs! { contract = GreetDeps; target = team;
                load_name(_id: u64) -> String => move |_| latest.into();
            };
            assert_eq!(
                greet(&final_layer, 7),
                format!("Hello, {latest} from Admins!")
            );
            assert_eq!(
                greet(&final_layer, 8),
                format!("Hello, {latest} from Admins!")
            );
            assert_eq!(greet(&NamespaceDefs, 7), "Hello, Test User from Test Team!");
        }

        #[rstest]
        #[case::different_inputs(3, 4)]
        #[case::repeated_input(7, 7)]
        fn inherited_captures_keep_their_borrows_and_mutable_state(
            #[case] first_id: u64,
            #[case] next_id: u64,
        ) {
            let seen = std::cell::RefCell::new(Vec::new());
            {
                let base = crate::defs! { contract = GreetDeps; target = GreetDeps;
                    load_name(id: u64) -> String => |id| {
                        let mut seen = seen.borrow_mut(); seen.push(id); format!("seen-{}", seen.len())
                    };
                };
                let deps = crate::defs! { contract = GreetDeps; target = base; team_name => |_| "Admins".into(); };
                assert_eq!(greet(&deps, first_id), "Hello, seen-1 from Admins!");
                assert_eq!(greet(&deps, next_id), "Hello, seen-2 from Admins!");
            }
            assert_eq!(*seen.borrow(), [first_id, next_id]);
        }

        #[test]
        fn an_opaque_target_is_evaluated_once() {
            let mut calls = 0;
            let deps = crate::defs! { contract = GreetDeps;
                target = { calls += 1; namespace_defaults() };
                team_name => |_| "Admins".into();
            };
            assert_eq!(calls, 1);
            assert_eq!(greet(&deps, 7), "Hello, Test User from Admins!");
        }

        #[test]
        fn qualified_contract_paths_work_in_definition_layers() {
            let defaults = crate::defs::layer::tests::layers::GreetDeps;
            let deps = crate::defs! { contract = crate::defs::layer::tests::layers::GreetDeps; target = defaults; load_name => |_| "Ada".into(); };
            assert_eq!(greet(&deps, 7), "Hello, Ada from team-7!");
        }

        mod ordinary_trait_names {
            fn value() -> u64 {
                7
            }
            crate::deps! {
            value() -> u64, || value();
            }
            fn read(deps: &impl Deps) -> u64 {
                deps.value()
            }
            #[test]
            fn trait_names_do_not_collide_with_generated_instance_types() {
                let base = Deps;
                let deps = crate::defs! { contract = Deps; target = base; value => || 42; };
                assert_eq!(read(&Deps), 7);
                assert_eq!(read(&deps), 42);
            }
        }
    }

    mod ordinary {
        use std::cell::Cell;
        crate::deps! {
            first() -> usize, || panic!("production must not run");
            second() -> usize, || panic!("production must not run");
            untouched() -> usize, || panic!("production must not run");
        }
        crate::stubs! {
            first => || 1;
            second => || 2;
            untouched => || 3;
        }
        #[test]
        fn several_overrides_keep_other_stubs_and_evaluate_the_base_once() {
            let evaluations = Cell::new(0);
            let first = Cell::new(10);
            let second = Cell::new(20);
            let deps = crate::defs! {
                target = { evaluations.set(evaluations.get() + 1); Defs };
                first() -> usize => || { first.set(first.get() + 1); first.get() };
                second() -> usize => || { second.set(second.get() + 1); second.get() };
            };
            assert_eq!(evaluations.get(), 1);
            assert_eq!((deps.first(), deps.second(), deps.untouched()), (11, 21, 3));
            let deps = crate::defs! { target = deps; untouched() => 30; first() => 40; };
            assert_eq!(
                (deps.first(), deps.second(), deps.untouched()),
                (40, 22, 30)
            );
            assert_eq!((Defs.first(), Defs.second(), Defs.untouched()), (1, 2, 3));
        }
    }

    mod generic_calls {
        use std::cell::Cell;

        crate::deps! {
            first() -> usize, || panic!("production must not run");
            second(value: usize) -> usize, |value| value;
            unchanged() -> usize, || panic!("production must not run");
            duplicate[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
        }
        crate::stubs! {
            first => || 1;
            second => |value| value + 2;
            unchanged => || 3;
            duplicate => |value| (value.clone(), value);
        }
        #[test]
        fn several_bodies_preserve_the_remaining_stubs_and_generic_methods() {
            let mut evaluations = 0;
            let deps = crate::defs! {
                target = { evaluations += 1; Defs };
                first() => 10;
                duplicate(value) => (value.clone(), value);
            };
            assert_eq!(evaluations, 1);
            assert_eq!((deps.first(), deps.second(5), deps.unchanged()), (10, 7, 3));
            assert_eq!(deps.duplicate("hello"), ("hello", "hello"));
        }
        #[rstest::rstest]
        #[case::method_first(true)]
        #[case::method_last(false)]
        fn raw_methods_see_sibling_overrides_in_the_same_implementation(
            #[case] method_first: bool,
        ) {
            let actual = if method_first {
                let deps = crate::defs! {
                    first { fn first(&self) -> usize { self.second(10) } }
                    second(value) => value + 20;
                };
                deps.first()
            } else {
                let deps = crate::defs! {
                    second(value) => value + 20;
                    first { fn first(&self) -> usize { self.second(10) } }
                };
                deps.first()
            };
            assert_eq!(actual, 30);
        }
        #[test]
        fn multiple_captures_and_bodies_work_in_either_order() {
            let first = Cell::new(10);
            let second = Cell::new(20);
            let deps = crate::defs! {
                first() -> usize => || { first.set(first.get()+1); first.get() };
                second(value: usize) -> usize => |value| { second.set(second.get() + value); second.get() };
                unchanged() => 30;
            };
            assert_eq!(
                (deps.first(), deps.second(5), deps.unchanged()),
                (11, 25, 30)
            );
            let deps = crate::defs! {
                target = deps;
                unchanged() => 40;
                first() -> usize => || 50;
            };
            assert_eq!(
                (deps.first(), deps.second(2), deps.unchanged()),
                (50, 27, 40)
            );
        }
    }

    mod generic_arguments {

        crate::deps! {
            count[const N: usize] call [N]() -> usize, || N;
            size[T] call [T]() -> usize, || std::mem::size_of::<T>();
        }
        crate::stubs! { count => || 0; size => || 0; }
        #[test]
        fn a_complete_method_can_override_the_generated_body() {
            let deps = crate::defs! {
                size { fn size<T>(&self) -> usize { 42 } }
            };
            assert_eq!(deps.size::<String>(), 42);
            assert_eq!(deps.count::<3>(), 0);
        }
        #[test]
        fn forwards_generics_that_arguments_cannot_infer() {
            assert_eq!(Defs.count::<3>(), 0);
            assert_eq!(Defs.size::<u64>(), 0);
            let deps = crate::defs! { count() => N * 2; };
            assert_eq!(deps.count::<3>(), 6);
            assert_eq!(deps.size::<u64>(), 0);
        }
    }

    mod custom_local_names {

        crate::deps! {
            target = WorkerDeps;
            value() -> usize, || 1;
        }
        crate::stubs! {
            contract = WorkerDeps;
            name = WorkerDefs;
            value => || 2;
        }
        #[test]
        fn local_collections_accept_custom_names_without_metadata_headers() {
            // The chosen defaults name is imported under the conventional name for
            // header-free individual overrides.
            use WorkerDefs as Defs;
            let deps = crate::defs! { value() => 3; };
            assert_eq!(
                (WorkerDeps.value(), WorkerDefs.value(), deps.value()),
                (1, 2, 3)
            );
        }
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
    mod typed_dependencies {
        pub struct User(pub u64);
        crate::deps! {
            load(id: u64) -> source::defs::layer::tests::typed_dependencies::User, |id| User(id);
            save(value: source::defs::layer::tests::typed_dependencies::User) -> u64, |value| value.0;
            duplicate[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
        }
        mod tests {
            use super::*;
            crate::stubs! {
                load => |id| User(id + 1);
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
            use crate::defs::layer::tests::typed_dependencies::tests::Defs;
            use crate::defs::layer::tests::typed_dependencies::Deps;

            #[test]
            fn imported_defaults_keep_their_function_signatures() {
                let deps = crate::defs! { save(value) => value.0 * 3; };
                assert_eq!(deps.save(deps.load(1)), 6);
            }
        }
        mod manual_base {
            use crate::defs::layer::tests::typed_dependencies::{Deps, User};

            struct Fixture;
            impl Deps for Fixture {
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
                    contract = crate::defs::layer::tests::typed_dependencies::Deps;
                    target = { evaluations+=1; Fixture };
                    save(value) => value.0 * 3;
                };
                assert_eq!(evaluations, 1);
                assert_eq!(deps.save(deps.load(1)), 6);
            }
        }
    }
}
