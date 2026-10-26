//! Parse instance-owned callable replacements for constructor-bound functions.
#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_function_overrides {
    (@instance [[$root:ident] [$contract:ident] [$($scope:ident)*] [] $methods:tt] [$($request:tt)*]) => {{
        $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
        __schema! { @functions }
        $crate::__traitstub_function_overrides! { @headers $methods [Defs]; $($request)* }
    }};
    (@headers $methods:tt $base:tt; target = $base_value:expr; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @headers $methods [$base_value]; $($rest)* }
    };
    (@headers $methods:tt $base:tt; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base []; $($rest)* }
    };
    (@parse $methods:tt $base:tt [$($done:tt)*]; $name:ident => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base [$($done)* ($name [$value])]; $($rest)* }
    };
    (@parse $methods:tt $base:tt $done:tt; $name:ident => $value:expr) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base $done; $name => $value; }
    };
    (@parse $methods:tt $base:tt $done:tt; $name:ident($($arg:ident),*) => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base $done; $name => |$($arg),*| $value; $($rest)* }
    };
    (@parse $methods:tt $base:tt $done:tt; $name:ident($($arg:ident),*) => $value:expr) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base $done; $name => |$($arg),*| $value; }
    };
    (@parse $methods:tt $base:tt $done:tt;
     $name:ident($($arg:ident:$ty:ty),*) -> $out:ty => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base $done;
            $name => {
                fn __typed<__Function>(value: __Function) -> __Function
                    where __Function: ::core::ops::Fn($($ty),*) -> $out { value }
                __typed($value)
            }; $($rest)* }
    };
    (@parse $methods:tt $base:tt $done:tt;
     $name:ident($($arg:ident:$ty:ty),*) -> $out:ty => $value:expr) => {
        $crate::__traitstub_function_overrides! { @parse $methods $base $done;
            $name($($arg:$ty),*) -> $out => $value; }
    };
    (@parse $methods:tt [$base:expr] [$(($name:ident [$value:expr]))+];) => {{
        #[allow(dead_code)] struct __UniqueMethods { $($name: (),)+ }
        $(__schema!(@valid_method $name);)+
        $crate::__traitstub_captures! { @storage [$($name)*] }
        $crate::__traitstub_function_layer! { @emit $methods [$base] [$(($name [capture]))+] }
        let __target = $base;
        __Layer { base: __target, replacement: __Captures { $($name: __schema!(@infer [$crate] $name [$value]),)+ } }
    }};
    (@parse $($bad:tt)*) => { compile_error!("defs! expects one or more callable overrides"); };
}
