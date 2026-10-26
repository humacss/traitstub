//! Bind concrete callables while keeping generic parameters on each function.
//!
//! A collection is nongeneric. Each function has a separate call contract, so
//! an override need only implement the host/signature used at a particular call.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_functions {
    (@methods $context:tt [$($done:tt)*];
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) -> $out:ty $(where [$($where:tt)*])?; $($rest:tt)*) => {
        $crate::__traitstub_functions! { @methods $context
            [$($done)* $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
             ($($arg:$ty),*) -> $out $(where [$($where)*])?, || ();]; $($rest)* }
    };
    (@methods $context:tt $done:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) $(where [$($where:tt)*])?; $($rest:tt)*) => {
        $crate::__traitstub_functions! { @methods $context $done;
            $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])?
            ($($arg:$ty),*) -> () $(where [$($where)*])?; $($rest)* }
    };
    (@methods [$d:tt $origin:tt $contract:tt $export:tt $scope:tt []] [$($done:tt)+];) => {
        $crate::__traitstub_deps! { @entries [$d $origin $contract $export $scope [__functions] []] []; $($done)+ }
    };
    (@methods $($bad:tt)*) => {
        compile_error!("bound functions need signatures with plain generic names and bracketed where clauses");
    };
    (@production [$contract:ident]
     [$( ([$($attrs:tt)*] $name:ident [$($gen:ident),*] $call:tt
          ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]) )*]) => {
        pub trait $contract {
            $( $($attrs)* fn $name<$($gen),*>(&self, $($arg:$ty),*) -> $out
                where Self: calls::$name<$($gen),*>, $($where)* {
                <Self as calls::$name<$($gen),*>>::__call(self, $($arg),*)
            } )*
        }
        /// Per-function call contracts for constructor-bound dependencies.
        #[allow(non_camel_case_types)]
        pub mod calls {
            #[allow(unused_imports)] use super::*;
            // Keep signatures outside the re-exported trait names: a function
            // named String must not shadow String in its own return type.
            #[doc(hidden)] pub mod __methods {
                $( $crate::__traitstub_functions! { @call_trait [$contract]
                    [[$($attrs)*] $name [$($gen),*] $call ($($arg:$ty),*) [$out] [$($where)*]] } )*
            }
            $( $($attrs)* pub use self::__methods::$name::__Call as $name; )*
        }
        #[doc(hidden)]
        #[derive(Clone, Copy, Default)]
        pub struct __TraitstubProduction<__Functions>(__Functions);
        impl<__Functions> $contract for __TraitstubProduction<__Functions> {}
        const _: () = {
            $crate::__traitstub_functions! { @storage [$($name)*] }
            $( $crate::__traitstub_functions! { @bound_impl
                [$contract] [[$($attrs)*] $name [$($gen),*] $call ($($arg:$ty),*) [$out] [$($where)*]] } )*
        };
        $crate::__traitstub_functions! { @constructor [$contract] [$($name)*] [];
            $( ([$($attrs)*] $name [$($gen),*] $call ($($arg:$ty),*) [$out] [$($where)*]) )* }
    };
    (@call_trait [$contract:ident] [[$($attrs:tt)*] $name:ident [$($gen:ident),*] $call:tt
        ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        $($attrs)* #[allow(non_snake_case)] pub mod $name {
            #[allow(unused_imports)] use super::super::super::*;
            pub trait __Call<$($gen),*>: $contract where $($where)* {
                fn __call(&self, $($arg:$ty),*) -> $out;
            }
            $crate::__traitstub_functions! { @inference [$($gen),*] ($($arg:$ty),*) [$out] }
        }
    };
    (@inference [] ($($arg:ident:$ty:ty),*) [$out:ty]) => {
        pub trait __Signature: ::core::ops::Fn($($ty),*) -> $out {}
        impl<__Function: ::core::ops::Fn($($ty),*) -> $out> __Signature for __Function {}
        pub fn __infer<__Function>(value: __Function) -> __Function
            where __Function: ::core::ops::Fn($($ty),*) -> $out { value }
    };
    (@inference $gen:tt $args:tt $out:tt) => {};
    (@infer [] [$name:ident] [$value:expr]) => { __function_schema::__methods::$name::__infer($value) };
    (@infer $gen:tt $name:tt [$value:expr]) => { $value };
    // Keep field type parameters away from source signatures (a dependency may
    // itself be named String). Projections here describe storage, not user types.
    (@storage [$($name:ident)*]) => {
        #[allow(dead_code, non_camel_case_types)]
        trait __Storage { $(type $name; fn $name(&self) -> &Self::$name;)* }
        #[allow(non_camel_case_types, clippy::builtin_type_shadow)]
        impl<$($name,)*> __Storage for ($($name,)*) {
            $crate::__traitstub_functions! { @getters [] ; $($name)* }
        }
    };
    (@getters [$($previous:tt)*]; $name:ident $($rest:ident)*) => {
        type $name = $name;
        fn $name(&self) -> &Self::$name {
            let ($($previous,)* value, ..) = self; value
        }
        $crate::__traitstub_functions! { @getters [$($previous)* _]; $($rest)* }
    };
    (@getters $previous:tt;) => {};
    (@bound_impl [$contract:ident] [[$($attrs:tt)*] $name:ident [$($gen:ident),*] $call:tt
        ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        $($attrs)* impl<$($gen,)* __Functions> calls::$name<$($gen),*> for __TraitstubProduction<__Functions>
        where __Functions: __Storage,
            <__Functions as __Storage>::$name: ::core::ops::Fn($($ty),*) -> $out, $($where)* {
            fn __call(&self, $($arg:$ty),*) -> $out { (self.0.$name())($($arg),*) }
        }
    };
    // Concrete functions provide contextual closure typing at construction.
    // Generic functions are checked at calls, after their generic types are known.
    (@constructor $contract:tt $names:tt [$($bounds:tt)*];
     ($attrs:tt $name:ident [] $call:tt ($($arg:ident:$ty:ty),*) [$out:ty] $where:tt) $($rest:tt)*) => {
        $crate::__traitstub_functions! { @constructor $contract $names
            [$($bounds)* $name: calls::__methods::$name::__Signature,]; $($rest)* }
    };
    (@constructor $contract:tt $names:tt $bounds:tt; $signature:tt $($rest:tt)*) => {
        $crate::__traitstub_functions! { @constructor $contract $names $bounds; $($rest)* }
    };
    (@constructor [$contract:ident] [$($name:ident)*] [$($bounds:tt)*];) => {
        #[doc(hidden)]
        #[allow(non_camel_case_types, clippy::builtin_type_shadow)]
        pub fn __traitstub_bind<$($name,)*>($($name:$name),*)
            -> __TraitstubProduction<($($name,)*)> where $($bounds)* {
            __TraitstubProduction(($($name,)*))
        }
        pub use __traitstub_bind as $contract;
    };
}

#[cfg(test)]
mod tests {
    #![allow(dead_code, non_snake_case)]
    // Names that previously collided with constructor exports or signature types.
    crate::deps! {
        bind(input: u64) -> String;
        String(input: usize) -> String;
    }
    crate::stubs! { bind => |_| String::new(); String => |_| String::new(); }
    #[test]
    fn constructor_and_overrides_infer_concrete_inputs_without_name_collisions() {
        let deps = Deps(|input| input.to_string(), |input| input.to_string());
        assert_eq!(deps.bind(7), "7");
        assert_eq!(deps.String(8), "8");
        let prefix = String::from("test");
        let test = crate::defs! { bind => |input| { let rendered = input.to_string(); format!("{prefix}-{rendered}") }; };
        assert_eq!(test.bind(7), "test-7");
        assert_eq!(test.String(8), "");
    }
    mod reference_inputs {
        pub struct View<'a>(pub &'a [u64]);
        crate::deps! {
            len(input: &str) -> usize;
            record(input: &mut Vec<u64>) -> usize;
            view(input: &source::deps::functions::tests::reference_inputs::View<'_>) -> usize;
        }
        crate::stubs! { len => |_| 0; record => |_| 0; view => |_| 0; }
        #[test]
        fn concrete_borrowed_inputs_keep_their_call_lifetimes() {
            let deps = Deps(
                |input| input.len(),
                |input| {
                    input.push(7);
                    input.len()
                },
                |input| input.0.len(),
            );
            let text = String::from("hello");
            let mut values = Vec::new();
            assert_eq!(deps.len(&text), 5);
            assert_eq!(deps.record(&mut values), 1);
            assert_eq!(deps.view(&View(&values)), 1);
            let bonus = 2;
            let test = crate::defs! { len => |input| input.len() + bonus; };
            assert_eq!(test.len(&text), 7);
        }
    }
}
