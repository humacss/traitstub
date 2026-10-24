//! Dependency declaration parsing and signature normalization.
//!
//! deps! reads headers and signatures, then conditions filters attributes before
//! schema emits the trait, production implementation and reusable metadata.

/// Declare the dependencies your code uses and how production calls them.
///
/// You get a trait named `Deps` and a zero-sized production value with the same
/// name. Take `&impl Deps` in the function using these dependencies, then pass
/// `&Deps` when running production code. Declare one collection per module.
/// The closures connect signatures to your existing functions and cannot capture
/// data. They can adapt the inputs or outputs when needed.
///
/// Write the function using these dependencies once. Production and tests call
/// that same function with different implementations of the generated trait.
///
/// ```
/// fn load(id: u64) -> String { format!("user-{id}") }
/// traitstub::deps! { load(id: u64) -> String, |id| load(id); }
/// fn greet(deps: &impl Deps) -> String { deps.load(7) }
/// assert_eq!(greet(&Deps), "user-7");
/// ```
///
/// # More control
///
/// - `target = Name;` selects the trait and production value name.
/// - `type Value = Type;` declares an associated type with a production selection.
/// - `bounds = [Send + Sync];` sets trait bounds. Standard `Clone`, `Copy` and
///   `Default` bounds are also supported when stored values meet those bounds.
/// - Methods accept bracketed generics (`copy[T: Clone](value: T)`), optional
///   `call [T, N]` forwarding arguments, and bracketed `where [...]` predicates.
/// - Omitted return types mean `()`. `cfg` and `cfg_attr` are resolved in the
///   declaring crate before signatures are exported; other attributes remain.
///
/// Cross-crate use adds `export = unique_name; target = Name; scope = crate::module;`
/// before the declarations; `target` is optional.
/// Exported names must be unique and signature types publicly accessible.
/// Types in signatures use prelude/built-in names, qualified standard-library paths,
/// or `source::path::Type` for types in the declaring crate. Private signature
/// imports are not carried into callers. Production closures can use local imports.
///
/// `for [H] where [H: source::Host + ?Sized];` makes the collection generic,
/// producing `Deps<H>`. Its methods use those collection parameters directly.
/// Production defaults work for every valid H; `defs!` can infer a concrete H
/// from a captured closure's argument annotation. Associated types, generic
/// methods and borrowed results can share that same collection.
///
/// These macros are intended for synchronous methods taking `&self`. Lifetime/const collection parameters,
/// associated type bounds/GATs, async/unsafe/foreign methods, `impl Trait` results,
/// bare `Self` arguments, custom supertraits and `&mut self` methods are outside
/// its scope. See the crate-level annotated example for generics and borrowing.
#[macro_export]
macro_rules! deps {
    (export = $export:ident; target = $contract:ident; scope = $origin:ident $(::$scope:ident)*;
     for [$($param:ident),+ $(,)?] $(where [$($where:tt)*])?; $($body:tt)*) => {
        $crate::__traitstub_parameter_schema! { @headers
            [[$] [$origin] [$contract] [$export] [$($scope)*] [$($param),+] [$($($where)*)?]] []; $($body)* }
    };
    (export = $export:ident; scope = $origin:ident $(::$scope:ident)*;
     for [$($param:ident),+ $(,)?] $(where [$($where:tt)*])?; $($body:tt)*) => {
        $crate::deps! { export = $export; target = Deps; scope = $origin $(::$scope)*;
            for [$($param),+] where [$($($where)*)?]; $($body)* }
    };
    (target = $contract:ident; for [$($param:ident),+ $(,)?] $(where [$($where:tt)*])?; $($body:tt)*) => {
        $crate::__traitstub_parameter_schema! { @headers
            [[$] [__local] [$contract] [__traitstub_schema] [] [$($param),+] [$($($where)*)?]] []; $($body)* }
    };
    (for [$($param:ident),+ $(,)?] $(where [$($where:tt)*])?; $($body:tt)*) => {
        $crate::deps! { target = Deps; for [$($param),+] where [$($($where)*)?]; $($body)* }
    };

    (export = $export:ident; target = $contract:ident; scope = $origin:ident $(::$scope:ident)*; $($body:tt)*) => {
        $crate::__traitstub_deps! { @headers [$] [$origin] [$contract] [$export] [$($scope)*] [] []; $($body)* }
    };
    (export = $export:ident; scope = $origin:ident $(::$scope:ident)*; $($body:tt)*) => {
        $crate::__traitstub_deps! { @headers [$] [$origin] [Deps] [$export] [$($scope)*] [] []; $($body)* }
    };
    (target = $contract:ident; $($body:tt)*) => {
        $crate::__traitstub_deps! { @headers [$] [__local] [$contract] [__traitstub_schema] [] [] []; $($body)* }
    };
    ($($body:tt)*) => {
        $crate::__traitstub_deps! { @headers [$] [__local] [Deps] [__traitstub_schema] [] [] []; $($body)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_deps {
    // --- @headers: collect optional bounds and associated-type defaults ---
    (@headers [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] [$($scope:ident)*] [] $types:tt;
     bounds = [$($bounds:tt)+]; $($rest:tt)*) => {
        $crate::__traitstub_deps! { @headers [$d] [$origin] [$contract] [$export] [$($scope)*] [$($bounds)+] $types; $($rest)* }
    };
    (@headers [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] $scope:tt $bounds:tt [$($types:tt)*];
     type $name:ident = $ty:ty; $($rest:tt)*) => {
        $crate::__traitstub_deps! { @headers [$d] [$origin] [$contract] [$export] $scope $bounds [$($types)* type $name = $ty;]; $($rest)* }
    };
    (@headers [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] $scope:tt $bounds:tt $types:tt; $($entries:tt)*) => {
        $crate::__traitstub_deps! { @entries [[$d] [$origin] [$contract] [$export] $scope $bounds $types] []; $($entries)* }
    };

    // --- @entries: normalize signatures, adding unit output and a final semicolon ---
    (@entries $context:tt []; ) => {
        compile_error!("deps! requires at least one dependency declaration");
    };
    (@entries $context:tt [$($done:tt)+];) => {
        $crate::__traitstub_conditions! { @active $context []; $($done)+ }
    };
    (@entries $context:tt [$($done:tt)*];
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident: $ty:ty),* $(,)?) -> $out:ty $(where [$($where:tt)*])?, $production:expr; $($rest:tt)*) => {
        $crate::__traitstub_deps! { @entries $context
            [$($done)* $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
             ($($arg: $ty),*) -> $out $(where [$($where)*])?, $production;]; $($rest)* }
    };
    (@entries $context:tt [$($done:tt)*];
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident: $ty:ty),* $(,)?) $(where [$($where:tt)*])?, $production:expr; $($rest:tt)*) => {
        $crate::__traitstub_deps! { @entries $context
            [$($done)* $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
             ($($arg: $ty),*) -> () $(where [$($where)*])?, $production;]; $($rest)* }
    };
    (@entries $context:tt $done:tt; $($tail:tt)+) => {
        $crate::__traitstub_deps! { @entries_last $context $done; $($tail)+ }
    };
    (@entries_last $context:tt $done:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident: $ty:ty),* $(,)?) $(-> $out:ty)? $(where [$($where:tt)*])?, $production:expr) => {
        $crate::__traitstub_deps! { @entries $context $done;
            $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
            ($($arg: $ty),*) $(-> $out)? $(where [$($where)*])?, $production; }
    };
    (@entries_last $($bad:tt)*) => {
        compile_error!("deps! expects name(arguments), optional -> Output, then a comma and a production closure");
    };
    (@entries $($bad:tt)*) => { compile_error!("deps! expects named arguments, optional bracketed generics and return type, then a comma and a production closure"); };
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod injection {
        crate::deps! { target = TestDeps;
        load_name(id: u64) -> String, |id| load_name(id);
        decorate(name: &str) -> String, |name| decorate(name);
        remember(values: &mut Vec<u64>, id: u64) , |values, id| helpers::remember(values, id);
        size(value: u64) -> u32, |value| u64::count_ones(value);
        }
        fn load_name(id: u64) -> String {
            format!("user-{id}")
        }
        fn decorate(name: &str) -> String {
            format!("Hello, {name}!")
        }
        mod helpers {
            pub fn remember(values: &mut Vec<u64>, id: u64) {
                values.push(id);
            }
        }
        fn greet(deps: &impl TestDeps, id: u64) -> String {
            deps.decorate(&deps.load_name(id))
        }
        fn formatted(deps: &impl TestDeps, id: u64) -> String {
            format!("Hello, {}!", deps.load_name(id))
        }
        #[test]
        fn production_and_tests_use_the_same_consumer_function() {
            let production = TestDeps;
            let stubs = crate::defs! { contract = TestDeps; target = TestDeps; load_name => |id| format!("guest-{id}"); };
            assert_eq!(greet(&production, 7), "Hello, user-7!");
            assert_eq!(greet(&stubs, 7), "Hello, guest-7!");
            assert_eq!(greet(&stubs, 8), "Hello, guest-8!");
            assert_eq!(greet(&production, 7), "Hello, user-7!");
        }
        #[test]
        fn one_instance_supplies_multiple_functions() {
            let deps = crate::defs! { contract = TestDeps; target = TestDeps; load_name => |_| "Ada".into(); };
            assert_eq!(greet(&deps, 1), "Hello, Ada!");
            assert_eq!(formatted(&deps, 1), "Hello, Ada!");
        }
        #[test]
        fn separate_instances_and_replacements_remain_independent() {
            let first = crate::defs! { contract = TestDeps; target = TestDeps; load_name => |_| "Ada".into(); };
            let second = TestDeps;
            assert_eq!(greet(&first, 1), "Hello, Ada!");
            assert_eq!(greet(&second, 1), "Hello, user-1!");
            let first = crate::defs! { contract = TestDeps; target = first; load_name => |_| "Grace".into(); };
            assert_eq!(greet(&first, 1), "Hello, Grace!");
            assert_eq!(greet(&first, 2), "Hello, Grace!");
        }
        #[test]
        fn mutable_inputs_and_unit_results_forward_through_layers() {
            let base = TestDeps;
            let mut values = Vec::new();
            base.remember(&mut values, 6);
            let deps = crate::defs! { contract = TestDeps; target = base; remember => |v, id| v.push(id * 2); };
            deps.remember(&mut values, 8);
            assert_eq!(values, [6, 16]);
            assert_eq!(deps.size(7), 3);
        }
        #[test]
        fn independent_instances_work_in_parallel_tests() {
            let threads: Vec<_> = (0..4)
                .map(|id| {
                    std::thread::spawn(move || {
                        let deps =
                            crate::defs! { contract = TestDeps; target = TestDeps; load_name(_id: u64) -> String => move |_| format!("thread-{id}"); };
                        greet(&deps, 0)
                    })
                })
                .collect();
            for (id, thread) in threads.into_iter().enumerate() {
                assert_eq!(thread.join().unwrap(), format!("Hello, thread-{id}!"));
            }
        }
        fn scope_value() -> u64 {
            10
        }
        mod relative_paths {
            pub struct Deps(pub u64);
            pub fn provider(value: &Deps) -> Deps {
                Deps(value.0 + 1)
            }
            crate::deps! { target = Contract;
            scope_value() -> u64, || super::scope_value();
            load(value: &self::Deps) -> self::Deps, |value| self::provider(value);
            }
            #[test]
            fn declaration_paths_and_types_keep_the_original_module_scope() {
                let base = Contract;
                assert_eq!(base.scope_value(), 10);
                assert_eq!(base.load(&Deps(2)).0, 3);
                let deps = crate::defs! { contract = Contract; target = base; load => |value| Deps(value.0 * 10); };
                assert_eq!(deps.load(&Deps(2)).0, 20);
                assert_eq!(deps.scope_value(), 10);
            }
        }
        mod colliding_names {
            mod first {
                pub fn value(id: u64) -> u64 {
                    id + 1
                }
            }
            mod second {
                pub fn value(id: u64) -> u64 {
                    id * 2
                }
            }
            crate::deps! { target = Contract;
            value(id: u64) -> u64, |id| first::value(id);
            other_value(id: u64) -> u64, |id| second::value(id);
            }
            fn calculate(deps: &impl Contract, id: u64) -> u64 {
                deps.value(id) + deps.other_value(id)
            }
            #[test]
            fn aliases_distinguish_same_named_functions() {
                let base = Contract;
                assert_eq!(calculate(&base, 3), 10);
                let deps = crate::defs! { contract = Contract; target = base; other_value => |id| id * 10; };
                assert_eq!(calculate(&deps, 3), 34);
                let deps = crate::defs! { contract = Contract; target = deps; value => |_| 100; };
                assert_eq!(calculate(&deps, 3), 130);
            }
        }
    }
}
