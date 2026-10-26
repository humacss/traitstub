//! Schema imports and method selection/emission shared by stubs! and defs!.
//!
//! A normalized method is `([attributes] name [generics] [forwarding arguments]
//! (named arguments) [result] [where predicates])`. A replacement is a named
//! body, a raw method, or a marker for a captured closure stored by defs!.
//!
//! @selectors declares one helper per entry. Explicit helpers shadow glob-imported
//! defaults, so @implement can repeat qualified calls without searching a list.
//! @emit writes an ordinary Rust method. Missing entries are errors for stubs! and forwarding calls for defs!.
//! These helpers must be exported for downstream macro expansion, but are hidden
//! implementation details rather than additional supported entry points.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_methods {
    (@selectors [$kind:ident]
        [$(($attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt))*]
        [$(($changed:ident [$($replacement:tt)*]))*]) => {
        mod __methodselectors {
            pub mod defaults {
                $( $crate::__traitstub_methods! { @selector [$] [$kind] [$name] [forward] } )*
            }
            pub mod selected {

                #[allow(unused_imports)] pub(crate) use super::defaults::*;
                $( $crate::__traitstub_methods! { @replacement [$kind] [$changed] [$($replacement)*] } )*
            }
        }
    };
    (@replacement [$kind:ident] $name:tt [entry $entry:tt]) => {
        $crate::__traitstub_stubs! { @entry [$kind] $entry }
    };
    (@replacement [$kind:ident] [$name:ident] $replacement:tt) => {
        $crate::__traitstub_methods! { @selector [$] [$kind] [$name] $replacement }
    };
    (@selector [$d:tt] [$kind:ident] [$name:ident] [$($replacement:tt)*]) => {
        #[allow(non_snake_case)] pub mod $name {
            #[allow(unused_macros)] macro_rules! select {
                ($d signature:tt) => { $crate::__traitstub_methods! { @emit [$kind] $d signature [$($replacement)*] } };
            }
            #[allow(unused_imports)] pub(crate) use select;
        }
    };
    (@implement [$kind:ident] [$(($attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt))*] $overrides:tt) => {
        $( __methodselectors::selected::$name::select! { [$attrs $name $gen $call $args $out $where] } )*
    };

    (@emit [field] $signature:tt $replacement:tt) => {
        $crate::__traitstub_entries! { @call $signature }
    };
    (@emit [function_layer] $signature:tt $replacement:tt) => {
        $crate::__traitstub_function_layer! { @method $signature $replacement }
    };
    (@emit [function_stubs] $signature:tt $replacement:tt) => {
        $crate::__traitstub_function_stubs! { @method $signature $replacement }
    };
    // --- @emit: raw method, stateless body, missing stub, forwarding call or capture ---
    (@emit [$kind:ident] [[$($attrs:tt)*] $($signature:tt)*] [method $method:item]) => { $($attrs)* $method };
    (@emit [$kind:ident] [[$($attrs:tt)*] $name:ident [$($gen:tt)*] [$($call:tt)*] ($($arg:ident: $ty:ty),*) [$out:ty] [$($where:tt)*]] [body $body:expr]) => {
        $($attrs)*
        fn $name<$($gen)*>(&self, $($arg: $ty),*) -> $out where $($where)* {
            let call: fn($($ty),*) -> _ = $body;
            call($($arg),*)
        }
    };
    (@emit [named] [[$($attrs:tt)*] $name:ident $($signature:tt)*] [forward]) => {
        $($attrs)*
        compile_error!(concat!("stubs! is missing a stub for ", stringify!($name)));
    };
    (@emit [layer] [[$($attrs:tt)*] $name:ident [$($gen:tt)*] [$($call:tt)*] ($($arg:ident: $ty:ty),*) [$out:ty] [$($where:tt)*]] [forward]) => {
        $($attrs)*
        fn $name<$($gen)*>(&self, $($arg: $ty),*) -> $out where $($where)* {
            <__Base as __schema>::$name::<$($call)*>(&self.base, $($arg),*)
        }
    };
    (@emit [layer] [[$($attrs:tt)*] $name:ident [$($gen:tt)*] [$($call:tt)*] ($($arg:ident: $ty:ty),*) [$out:ty] [$($where:tt)*]] [capture]) => {
        $($attrs)*
        fn $name<$($gen)*>(&self, $($arg: $ty),*) -> $out where $($where)* {
            (self.replacement.$name())($($arg),*)
        }
    };

    // --- @import_schema: bring both trait and metadata macro into scope as __schema ---
    (@import_schema [__local] [] [$contract:ident]) => {
        #[allow(unused_imports)] use crate as source;
        use $contract as __schema;
    };
    (@import_schema [$root:ident] [$($scope:ident)*] [$contract:ident]) => {
        use $root::{self as source};
        use source $(::$scope)* :: $contract as __schema;
    };
}
