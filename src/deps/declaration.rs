//! Dependency declaration parsing and signature normalization.
//!
//! deps! reads headers and signatures, then schema emits the trait, production
//! implementation and reusable metadata.

/// Declare the dependencies your code uses and how production calls them.
///
/// With production bodies, you get a trait named `Deps` and a zero-sized
/// production value with the same name. Take `&impl Deps` in the function using these dependencies, then pass
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
/// - `bounds = [Send + Sync];` sets trait bounds. Standard `Clone`, `Copy` and
///   `Default` bounds are also supported when stored values meet those bounds.
/// - Methods accept bracketed generics (`copy[T: Clone](value: T)`), optional
///   `call [T, N]` forwarding arguments, and bracketed `where [...]` predicates.
/// - Omitted return types mean `()`. Method attributes are passed through unchanged.
///
/// Cross-crate use adds `export = unique_name; target = Name; scope = crate::module;`
/// before the declarations; `target` is optional.
/// Exported names must be unique and signature types publicly accessible.
/// Types in signatures use prelude/built-in names, qualified standard-library paths,
/// or `source::path::Type` for types in the declaring crate. Private signature
/// imports are not carried into callers. Production closures can use local imports.
///
/// # Binding callables and specializing generic functions
///
/// Omit production bodies to get a `Deps(...)` constructor instead of the
/// production constant. Bind real functions or borrowed context where available.
/// `stubs!` still supplies every shared default. `defs!` stores captured `Fn`
/// replacements directly.
///
/// Function generics use plain names with their bounds in `where [...]`.
/// The nongeneric collection trait forwards each function through its own call
/// contract, such as `calls::query<H>`. A replacement only needs to support the
/// generic arguments used by its calls, rather than every possible type.
///
/// ```
/// traitstub::deps! {
///     query[H](host: &H, id: u64) -> String;
///     enabled() -> bool;
/// }
/// traitstub::stubs! {
///     query => |_, _| String::new();
///     enabled => || false;
/// }
/// fn read<H>(deps: &impl calls::query<H>, host: &H) -> String {
///     deps.query(host, 7)
/// }
/// # fn main() {
/// let production = Deps(|host: &String, id| format!("{host}-{id}"), || true);
/// assert_eq!(read(&production, &"real".into()), "real-7");
/// let name = String::from("test");
/// let test = traitstub::defs! {
///     query => |_: &String, _| name.clone();
/// };
/// assert_eq!(read(&test, &String::new()), "test");
/// assert!(!test.enabled());
/// # }
/// ```
///
/// Use the host's existing associated types in return signatures, such as
/// `Result<Option<H::Value>, H::Error>` with `where [H: source::Host + ?Sized]`.
/// Traitstub adds no associated types to the collection.
/// If a generic appears only in the result, select it explicitly when calling:
/// `deps.query::<H>(id)`. Concrete signature types infer callable arguments;
/// generic replacements may need closure input or return annotations.
///
/// For a local contract used from another module, import both `Deps` and `calls`.
/// Exported contracts carry their module path automatically. One collection per
/// module reserves the generated `calls` module in this form.
///
/// Constructor-bound functions currently support plain type parameters, owned
/// results and ordinary input references. Use production-body declarations for
/// lifetime/const generics and results borrowed from inputs or `self`.
/// Associated-type declarations, collection generics, async/unsafe/foreign
/// methods, `impl Trait` results, bare `Self` arguments, custom supertraits and
/// `&mut self` methods are outside the syntax.
#[macro_export]
macro_rules! deps {
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
    (@headers [$d:tt] $origin:tt $contract:tt $export:tt $scope:tt [] [];
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) $(-> $out:ty)? $(where [$($where:tt)*])?; $($rest:tt)*) => {
        $crate::__traitstub_functions! { @methods [[$d] $origin $contract $export $scope []] [];
            $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
            ($($arg:$ty),*) $(-> $out)? $(where [$($where)*])?; $($rest)* }
    };
    // --- @headers: collect optional collection bounds ---
    (@headers [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] [$($scope:ident)*] [] $types:tt;
     bounds = [$($bounds:tt)+]; $($rest:tt)*) => {
        $crate::__traitstub_deps! { @headers [$d] [$origin] [$contract] [$export] [$($scope)*] [$($bounds)+] $types; $($rest)* }
    };
    (@headers $d:tt $origin:tt $contract:tt $export:tt $scope:tt $bounds:tt $types:tt;
     type $($rest:tt)*) => { compile_error!("deps! wraps functions; put types in function signatures"); };
    (@headers [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] $scope:tt $bounds:tt $types:tt; $($entries:tt)*) => {
        $crate::__traitstub_deps! { @entries [[$d] [$origin] [$contract] [$export] $scope $bounds $types] []; $($entries)* }
    };

    // Parse complete declarations in one repetition. Result types remain opaque;
    // generated Rust signatures handle their lifetime elision.
    (@entries [$($context:tt)*] [];
     $( $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
        ($($arg:ident: $ty:ty),* $(,)?) $(-> $out:ty)? $(where [$($where:tt)*])?, $production:expr; )+) => {
        $crate::__traitstub_schema! { @schema $($context)*
            $( $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
               ($($arg:$ty),*) -> $crate::__traitstub_functions!(@output $($out)?)
               $(where [$($where)*])?, $production; )+ }
    };
    (@entries $context:tt []; ) => { compile_error!("deps! requires at least one dependency declaration"); };
    (@entries $context:tt []; $($entries:tt)+) => {
        $crate::__traitstub_deps! { @terminated $context; $($entries)+ ; }
    };
    (@terminated $context:tt;
     $( $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
        ($($arg:ident: $ty:ty),* $(,)?) $(-> $out:ty)? $(where [$($where:tt)*])?, $production:expr; )+) => {
        $crate::__traitstub_deps! { @entries $context [];
            $( $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
               ($($arg:$ty),*) $(-> $out)? $(where [$($where)*])?, $production; )+ }
    };
    (@terminated $($bad:tt)*) => {
        compile_error!("deps! expects name(arguments), optional -> Output, then a comma and a production closure");
    };

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
