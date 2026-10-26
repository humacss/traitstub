//! Complete stateless test stubs and their reusable metadata.
//!
//! Flow: stubs! -> @types -> contract's @defaults -> @parse -> @emit.
//! @implementation checks names and emits the shared zero-sized implementation;
//! @emit also creates the Defs metadata used later by defs!.
//!
//! Parsed replacements are `(name [body closure])` or `(name [method RustItem])`.
//! Every schema method must have a replacement: shared method emission reports
//! omissions. A repeated field-name check rejects duplicates at declaration time.
//! Local and exported metadata differ in visibility and $crate hygiene, while
//! both share the exact same implementation-generation arms.

/// Define the test stubs you want to share between tests.
///
/// Supply an implementation for every dependency, usually inside the test module.
/// You get a zero-sized value named `Defs` that each test can use directly or
/// build on with [`defs!`](crate::defs!). Forgetting a stub is a compile error;
/// it does not call the production function instead.
///
/// Write the shared test behavior here once. Each test can then replace only the
/// dependencies that matter to it, without repeating the rest of the implementation.
///
/// ```
/// traitstub::deps! { value() -> u64, || 1; other() -> u64, || 2; }
/// traitstub::stubs! { value => || 10; other => || 20; }
/// assert_eq!((Deps.value(), Defs.value(), Defs.other()), (1, 10, 20));
/// ```
///
/// Use `name => closure;`, `name(arguments) => body;` or a full method such as
/// `value { fn value(&self) -> u64 { 10 } }`. Bodies and closures are stateless;
/// captured fixtures belong in [`defs!`](crate::defs!).
///
/// # More control
///
/// `contract = path::Deps; name = TestStubs;` selects a contract and output name.
/// `target` is an alias for `contract`. Types belong in function signatures.
/// Shared stubs for generic constructor-bound functions implement the declared
/// generic contract; individual `defs!` closures can specialize it for a test.
///
/// Exported defaults use `contract = path::Deps; export = unique_name; name =
/// TestStubs; scope = crate::module;` (the default name may be omitted).
/// Export headers are needed only when those defaults cross a crate boundary.
///
/// # Missing or invalid stubs
///
/// Every dependency needs a stub, and each name must appear only once:
///
/// ```compile_fail
/// traitstub::deps! { value() -> u64, || 1; }
/// traitstub::stubs! { export = unknown_defaults; scope = crate; missing => || 2; }
/// ```
///
/// ```compile_fail
/// traitstub::deps! { value() -> u64, || 1; }
/// traitstub::stubs! {
///     export = duplicate_defaults; scope = crate;
///     value => || 2;
///     value => || 3;
/// }
/// ```
///
/// ```compile_fail
/// traitstub::deps! { first() -> u64, || 1; second() -> u64, || 2; }
/// traitstub::stubs! { first => || 0; }
/// ```
///
/// ```compile_fail
/// traitstub::deps! { first() -> u64, || 1; }
/// traitstub::stubs! {}
/// ```
#[macro_export]
macro_rules! stubs {
    (name = $name:ident; target = $($contract:ident)::+; $($rest:tt)*) => {
        $crate::stubs! { contract = $($contract)::+; name = $name; $($rest)* }
    };
    (target = $($contract:ident)::+; $($rest:tt)*) => {
        $crate::stubs! { contract = $($contract)::+; $($rest)* }
    };

    (contract = $($contract:ident)::+; export = $export:ident; name = $name:ident; scope = $default_origin:ident $(::$scope:ident)*; $($rest:tt)*) => {
        $crate::__traitstub_stubs! { @types [$($contract)::+] [$name] [$export] [$default_origin] [$($scope)*] []; $($rest)* }
    };
    (export = $export:ident; scope = $default_origin:ident $(::$scope:ident)*; $($rest:tt)*) => {
        $crate::stubs! { contract = Deps; export = $export; name = Defs; scope = $default_origin $(::$scope)*; $($rest)* }
    };
    (contract = $($contract:ident)::+; export = $export:ident; scope = $default_origin:ident $(::$scope:ident)*; $($rest:tt)*) => {
        $crate::stubs! { contract = $($contract)::+; export = $export; name = Defs; scope = $default_origin $(::$scope)*; $($rest)* }
    };
    (contract = $($contract:ident)::+; name = $name:ident; $($rest:tt)*) => {
        $crate::__traitstub_stubs! { @types [$($contract)::+] [$name] [__traitstub_defaults] [__local] [] []; $($rest)* }
    };
    (contract = $($contract:ident)::+; $($rest:tt)*) => {
        $crate::stubs! { contract = $($contract)::+; name = Defs; $($rest)* }
    };
    (name = $name:ident; $($rest:tt)*) => {
        $crate::stubs! { contract = Deps; name = $name; $($rest)* }
    };
    ($($rest:tt)*) => {
        $crate::stubs! { contract = Deps; name = Defs; $($rest)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_stubs {

    // --- @types: collect type selections, then ask the contract to replay signatures ---
    (@types [$($contract:ident)::+] $name:tt $export:tt $origin:tt $scope:tt $choices:tt; $($rest:tt)*) => {
        $($contract)::+! { @defaults [$crate] $name $export $origin $scope [] [$($rest)*] }
    };

    // --- @defaults: receive the schema and begin normalizing user stubs ---
    (@defaults [$d:tt] [$root:ident] [$contract:ident] [$($scope:ident)*] [$($bounds:tt)*]
     [] [$($methods:tt)*] [$name:ident] [$export:ident] [$default_origin:ident] [$($default_scope:ident)*]
     [$($choices:tt)*] [$($overrides:tt)*]) => {
        $crate::__traitstub_stubs! { @parse [[$d] [$root] [$contract] [$($scope)*] [$($bounds)*] [] [$($methods)*]
             [$name] [$export] [$default_origin] [$($default_scope)*] [$($choices)*]] []; $($overrides)* }
    };

    // Match the whole list; each entry is normalized independently later.
    (@parse [$($context:tt)*] [];
     $( $name:ident $(($($args:tt)*))? $({$method:item})? $(=> $value:expr;)? )* $(;)?) => {
        $crate::__traitstub_stubs! { @emit $($context)*
            [$(($name [entry [$name $(($($args)*))? $({$method})? $(=> $value;)?]]))*] }
    };
    (@parse $context:tt []; $($entries:tt)+) => {
        $crate::__traitstub_stubs! { @terminated $context; $($entries)+ ; }
    };
    (@terminated $context:tt;
     $( $name:ident $(($($args:tt)*))? $({$method:item})? $(=> $value:expr;)? )* $(;)?) => {
        $crate::__traitstub_stubs! { @parse $context [];
            $( $name $(($($args)*))? $({$method})? $(=> $value;)? )* }
    };
    (@terminated $($bad:tt)*) => { compile_error!("stubs! expects every dependency's stub"); };

    (@entry [$kind:ident] [$name:ident => $value:expr;]) => {
        $crate::__traitstub_methods! { @selector [$] [$kind] [$name] [body $value] }
    };
    (@entry [$kind:ident] [$name:ident($($arg:ident),*) => $value:expr;]) => {
        $crate::__traitstub_methods! { @selector [$] [$kind] [$name] [body |$($arg),*| $value] }
    };
    (@entry [$kind:ident] [$name:ident {$method:item}]) => {
        $crate::__traitstub_methods! { @selector [$] [$kind] [$name] [method $method] }
    };
    // --- @emit: generate the shared implementation and exported default metadata ---
    (@emit $d:tt $root:tt $contract:tt $scope:tt $bounds:tt $assoc:tt $methods:tt $name:tt $export:tt [__local] $default_scope:tt $choices:tt $overrides:tt) => {
        $crate::__traitstub_stubs! { @emit_local $d $root $contract $scope $bounds $assoc $methods $name $export [__local] $default_scope $choices $overrides }
    };

    (@emit [$d:tt] [$root:ident] [$contract:ident] [$($scope:ident)*] [$($bounds:tt)*]
     [] [$($methods:tt)*] [$name:ident] [$export:ident] [$default_origin:ident] [$($default_scope:ident)*]
     [] [$(($changed:ident [$($replacement:tt)*]))*]) => {
        $crate::__traitstub_stubs! { @implementation
            [$root] [$contract] [$($scope)*] [$name] [$($bounds)*] []
            [] [$($methods)*] [$(($changed [$($replacement)*]))*]
        }
        #[doc(hidden)]
        #[macro_export]
        macro_rules! $export {
            ([$d($d runtime:tt)*] $d($d request:tt)*) => {
                $d($d runtime)*::__traitstub_defs! { @instance [$root] [$contract] [$($scope)*] [$($bounds)*] [] [$($methods)*]
                    [$d $default_origin] [$($default_scope ::)* $name] [$d($d request)*]
                }
            };
        }
        #[doc(hidden)]
        pub use $export as $name;
    };

    // --- @emit_local: generate the same implementation with crate-local metadata ---
    (@emit_local [$d:tt] [$root:ident] [$contract:ident] [$($scope:ident)*] [$($bounds:tt)*]
     [] [$($methods:tt)*] [$name:ident] [$export:ident] [$default_origin:ident] [$($default_scope:ident)*]
     [] [$(($changed:ident [$($replacement:tt)*]))*]) => {
        $crate::__traitstub_stubs! { @implementation
            [$root] [$contract] [$($scope)*] [$name] [$($bounds)*] []
            [] [$($methods)*] [$(($changed [$($replacement)*]))*]
        }
        macro_rules! $export {
            ([$d($d runtime:tt)*] $d($d request:tt)*) => {
                $d($d runtime)*::__traitstub_defs! { @instance [$root] [$contract] [$($scope)*] [$($bounds)*] [] [$($methods)*]
                    [$default_origin] [$($default_scope ::)* $name] [$d($d request)*]
                }
            };
        }
        #[allow(unused_imports)]
        pub(crate) use $export as $name;
    };

    (@implementation $root:tt $contract:tt $scope:tt $name:tt [__functions] [] $lookup:tt $methods:tt $overrides:tt) => {
        $crate::__traitstub_function_stubs! { @implementation $root $contract $scope $name $lookup $methods $overrides }
    };
    // --- @implementation: validate names and build the zero-sized stub implementation ---
    (@implementation [$root:ident] [$contract:ident] [$($scope:ident)*] [$name:ident] $bounds:tt []
     [] [$($methods:tt)*] [$(($changed:ident [$($replacement:tt)*]))*]) => {
        #[derive(Clone, Copy, Default)]
        pub struct $name;
        const _: () = {
            $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
            $crate::__traitstub_methods! { @selectors [named] [$($methods)*] [$(($changed [$($replacement)*]))*] }
            // Rust rejects duplicate fields even if no method is ever called.
            #[allow(dead_code)] struct __UniqueMethods { $($changed: (),)* }
            $(__schema!(@valid_method $changed);)*
            $crate::__traitstub_stubs! { @trait_impl
                [$name] [] []
                [$($methods)*] [$(($changed [$($replacement)*]))*]
            }
        };
    };

    // Emit the ordinary shared implementation.
    (@trait_impl [$name:ident] [] $choices:tt $methods:tt $overrides:tt) => {
        impl __schema for $name {
            $crate::__traitstub_methods! { @implement [named] $methods $overrides }
        }
    };

}

#[cfg(test)]
mod tests {
    // Each case intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]
    use rstest::rstest;

    crate::deps! {
        load_name(id: u64) -> String, |id| format!("user-{id}");
        team_name(id: u64) -> String, |id| format!("team-{id}");
    }
    crate::stubs! { load_name => |_| "Test User".into(); team_name => |_| "Test Team".into(); }
    crate::stubs! { target = Deps; name = TestDeps; load_name => |_| "Test User".into(); team_name => |_| "Test Team".into(); }
    crate::stubs! { name = EmptyDeps; load_name => |_| String::new(); team_name => |_| String::new(); }
    crate::stubs! { name = AlternateDeps; load_name => |_| "Alternate".into(); team_name => |_| "Other Team".into(); }
    mod namespace {
        use super::*;
        crate::stubs! { target = Deps; name = LocalDeps; load_name => |_| String::new(); team_name => |_| "Local Team".into(); }
    }
    fn greet(deps: &impl Deps, id: u64) -> String {
        format!("Hello, {} from {}!", deps.load_name(id), deps.team_name(id))
    }

    #[rstest]
    #[case::production(Deps, "Hello, user-7 from team-7!")]
    #[case::default_name(Defs, "Hello, Test User from Test Team!")]
    #[case::custom_name(TestDeps, "Hello, Test User from Test Team!")]
    #[case::alternate(AlternateDeps, "Hello, Alternate from Other Team!")]
    #[case::empty(EmptyDeps, "Hello,  from !")]
    #[case::nested_module(namespace::LocalDeps, "Hello,  from Local Team!")]
    fn selected_defaults_supply_their_own_behavior<D: Deps>(
        #[case] base: D,
        #[case] expected: &str,
    ) {
        assert_eq!(greet(&base, 7), expected);
    }

    #[rstest]
    #[case::production(Deps)]
    #[case::default_name(Defs)]
    #[case::custom_name(TestDeps)]
    #[case::nested_module(namespace::LocalDeps)]
    fn overrides_keep_the_selected_base_and_leave_it_unchanged<D: Deps + Copy>(#[case] base: D) {
        let original = greet(&base, 7);
        let first = crate::defs! { target = base; load_name => |_| "Ada".into(); };
        let second =
            crate::defs! { contract = Deps; target = first; team_name => |_| "Admins".into(); };
        assert_eq!(first.team_name(7), base.team_name(7));
        assert_eq!(first.load_name(7), "Ada");
        assert_eq!(greet(&second, 7), "Hello, Ada from Admins!");
        assert_eq!(greet(&base, 7), original);
    }

    #[rstest]
    #[case::default_name(Defs)]
    #[case::custom_name(TestDeps)]
    fn named_defaults_keep_captured_state_across_layers<D: Deps>(#[case] base: D) {
        let mut evaluations = 0;
        let seen = std::cell::RefCell::new(Vec::new());
        let first = crate::defs! {
            contract = Deps; target = { evaluations += 1; base };
            load_name(id: u64) -> String => |id| {
                let mut seen = seen.borrow_mut(); seen.push(id); format!("seen-{}", seen.len())
            };
        };
        assert_eq!(evaluations, 1);
        assert_eq!(greet(&first, 7), "Hello, seen-1 from Test Team!");
        let second =
            crate::defs! { contract = Deps; target = first; team_name => |_| "Admins".into(); };
        assert_eq!(greet(&second, 8), "Hello, seen-2 from Admins!");
        assert_eq!(*seen.borrow(), [7, 8]);
        assert_eq!(greet(&TestDeps, 7), "Hello, Test User from Test Team!");
    }

    mod custom {
        use rstest::rstest;
        crate::deps! { target = CustomDeps; value(input: u64) -> u64, |input| input + 1; }
        crate::stubs! { target = CustomDeps; name = CustomDefs; value => |input| input + 10; }
        mod reordered_headers {
            use super::*;
            crate::stubs! { name = Reordered; target = super::CustomDeps; value => |_| 0; }
        }
        mod default_name {
            use super::CustomDeps;
            crate::stubs! { target = CustomDeps; value => |input| input * 3; }
            #[test]
            fn implicit_output_name_supplies_the_override_base() {
                let deps = crate::defs! { value => |input| input * 4; };
                assert_eq!((Defs.value(7), deps.value(7)), (21, 28));
            }
        }
        #[rstest]
        #[case::production(CustomDeps, 8)]
        #[case::named(CustomDefs, 17)]
        #[case::reordered_headers(reordered_headers::Reordered, 0)]
        #[case::default_output(default_name::Defs, 21)]
        fn custom_contracts_keep_the_selected_implementation<D: CustomDeps>(
            #[case] base: D,
            #[case] expected: u64,
        ) {
            assert_eq!(base.value(7), expected);
            let deps =
                crate::defs! { contract = CustomDeps; target = base; value(input) => input * 2; };
            assert_eq!(deps.value(7), 14);
        }
    }

    pub struct Input(pub u64);
    mod typed_api {
        crate::deps! { target = Contract;
            value(input: &source::stubs::declaration::tests::Input) -> u64, |input| input.0;
        }
        mod defaults {
            use super::*;
            crate::stubs! { target = Contract; name = Named; value => |input| input.0 + 1; }
            #[test]
            fn named_defaults_preserve_signature_types_from_the_contract_scope() {
                assert_eq!(Named.value(&crate::stubs::declaration::tests::Input(7)), 8);
                let deps = crate::defs! { contract = Contract; target = Named; value => |input| input.0 + 10; };
                assert_eq!(deps.value(&crate::stubs::declaration::tests::Input(7)), 17);
            }
        }
    }
}
