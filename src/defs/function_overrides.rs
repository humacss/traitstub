//! Parse instance-owned callable replacements for constructor-bound functions.
#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_function_overrides {
    (@instance [[$root:ident] [$contract:ident] [$($scope:ident)*] [] $methods:tt] [$($request:tt)*]) => {{
        $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
        __schema! { @functions }
        $crate::__traitstub_function_overrides! { @headers $methods [Stubs]; $($request)* }
    }};
    (@headers $methods:tt $base:tt; lookup = $($path:ident)::+; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @headers $methods $base; $($rest)* }
    };
    (@headers $methods:tt $base:tt; target = $base_value:expr; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @headers $methods [$base_value]; $($rest)* }
    };
    (@headers $methods:tt $base:tt; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base []; $($rest)* }
    };
    (@parse $methods:tt [$base:expr] [];
     $( $name:ident $(($($args:tt)*))? $(-> $out:ty)? => $value:expr; )+) => {{
        #[allow(dead_code)] struct __UniqueMethods { $($name: (),)+ }
        $(__schema!(@valid_method $name);)+
        $crate::__traitstub_captures! { @storage [$($name)*] }
        $crate::__traitstub_function_layer! { @emit $methods [$base] [$(($name [capture]))+] }
        let __target = $base;
        __Layer { base: __target, replacement: __Captures {
            $($name: __schema!(@infer [$crate] $name [
                $crate::__traitstub_function_overrides! { @value [$(($($args)*))? $(-> $out)?] [$value] }
            ]),)+
        } }
    }};
    (@parse $methods:tt $base:tt []; $($entries:tt)+) => {
        $crate::__traitstub_function_overrides! { @terminated $methods $base; $($entries)+ ; }
    };
    (@terminated $methods:tt $base:tt;
     $( $name:ident $(($($args:tt)*))? $(-> $out:ty)? => $value:expr; )+) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base [];
            $( $name $(($($args)*))? $(-> $out)? => $value; )+ }
    };
    (@terminated $($bad:tt)*) => { compile_error!("defs! expects one or more callable overrides"); };
    (@value [] [$value:expr]) => { $value };
    (@value [($($arg:ident),*)] [$value:expr]) => { |$($arg),*| $value };
    (@value [($($arg:ident:$ty:ty),*) -> $out:ty] [$value:expr]) => {{
        fn typed<F: ::core::ops::Fn($($ty),*) -> $out>(value:F) -> F { value }
        typed($value)
    }};
    (@parse $($bad:tt)*) => { compile_error!("defs! expects one or more callable overrides"); };
}
