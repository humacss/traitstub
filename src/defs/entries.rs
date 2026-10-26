//! Each override defines a selector for one method. Schema defaults fill the rest.
//! Repetition invokes these helpers independently; no list is consumed recursively.
//! Binding evaluates closures in caller scope and replaces one storage field.
//! Defining selectors emits only method tokens, keeping captured values out of
//! generated modules and preserving support for block-local declarations.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_entries {
    (@request [define] $lifetimes:tt [for $binder:tt $name:ident($($args:tt)*) -> $out:ty => $value:expr;]) => {
        $crate::__traitstub_methods! { @selector [$] [field] [$name] [] }
    };
    (@request [define] $lifetimes:tt [$name:ident($($args:tt)*) -> $out:ty => $value:expr;]) => {
        $crate::__traitstub_methods! { @selector [$] [field] [$name] [] }
    };
    (@request [define] $lifetimes:tt [$name:ident => $value:expr;]) => {
        $crate::__traitstub_methods! { @selector [$] [field] [$name] [] }
    };
    (@request [define] $lifetimes:tt [$name:ident($($arg:ident),*) => $value:expr;]) => {
        $crate::__traitstub_methods! { @selector [$] [layer] [$name] [body |$($arg),*| $value] }
    };
    (@request [define] $lifetimes:tt [$name:ident {$method:item}]) => {
        $crate::__traitstub_methods! { @selector [$] [layer] [$name] [method $method] }
    };
    (@request $mode:tt $lifetimes:tt [for [$($binder:lifetime),+] $name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr;]) => {
        __schema! { @entry [$crate] $mode $lifetimes [$name] [capture [for<$($binder),+>] [$($ty),*] [$out] [$value]] }
    };
    (@request $mode:tt $lifetimes:tt [$name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr;]) => {
        __schema! { @entry [$crate] $mode $lifetimes [$name] [capture [] [$($ty),*] [$out] [$value]] }
    };
    (@request $mode:tt $lifetimes:tt [$name:ident($($arg:ident),*) => $value:expr;]) => {
        __schema! { @entry [$crate] $mode $lifetimes [$name] [body |$($arg),*| $value] }
    };
    (@request $mode:tt $lifetimes:tt [$name:ident {$method:item}]) => {
        __schema! { @entry [$crate] $mode $lifetimes [$name] [method $method] }
    };
    (@request $mode:tt $lifetimes:tt [$name:ident => $value:expr;]) => {
        __schema! { @entry [$crate] $mode $lifetimes [$name] [inferred $value] }
    };
    (@request $($bad:tt)*) => { compile_error!("defs! expects a dependency override"); };
    (@unique [for $binder:tt $name:ident $($rest:tt)*]) => { #[allow(non_camel_case_types)] struct $name; };
    (@unique [$name:ident $($rest:tt)*]) => { #[allow(non_camel_case_types)] struct $name; };

    (@select $mode:tt [$d:tt] $lifetimes:tt [$attrs:tt $name:ident $($signature:tt)*] [inferred $value:expr]) => {
        __schema! { @capture [$crate] [$name] [[select $mode [$d] $lifetimes [$attrs $name $($signature)*]]] [$value] }
    };
    (@select [bind $slot:ident] [$d:tt] $lifetimes:tt [$attrs:tt $name:ident $($signature:tt)*] [capture $binder:tt $args:tt $out:tt [$value:expr]]) => {
        let $slot = __schema! { @field [$crate] $lifetimes [$attrs $name $($signature)*] $binder $args $out [$value] [$slot] };
    };
    (@select [bind $slot:ident] $d:tt $lifetimes:tt [$attrs:tt $name:ident $($signature:tt)*] [body $value:expr]) => {
        let $slot = __schema! { @stateless [$crate] [$attrs $name $($signature)*] [$value] [$slot] };
    };
    (@select [bind $slot:ident] $d:tt $lifetimes:tt $signature:tt $replacement:tt) => {};
    (@call [[$($attrs:tt)*] $name:ident [$($gen:tt)*] [$($call:tt)*] ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        $($attrs)* fn $name<$($gen)*>(&self, $($arg:$ty),*) -> $out where $($where)* {
            __callers::$name::__Call::$name::<$($call)*>(self.replacement.$name(), $($arg),*)
        }
    };
    (@closure [$d:tt] [$($bounds:tt)*] [$($lifetime:lifetime),*]
        [$attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt]
        [$($binder:tt)*] [$($input:ty),*] [$result:ty] [$value:expr] [$slot:ident]
        [$($method:ident)*]) => {{
        #[derive(Clone, Copy, Default)]
        struct __Override<$($lifetime,)* __Function> {
            call: __Function,
            marker: ::core::marker::PhantomData<($(&$lifetime (),)*)>,
        }
        impl<$($lifetime,)* __Function> __callers::$name::__Call for __Override<$($lifetime,)* __Function>
        where __Function: $($binder)* ::core::ops::Fn($($input),*) -> $result + $($bounds)* {
            $crate::__traitstub_entries! { @field_call [$attrs $name $gen $call $args $out $where] }
        }
        fn typed<$($lifetime,)* __Function>(value: __Function) -> __Function
            where __Function: $($binder)* ::core::ops::Fn($($input),*) -> $result + $($bounds)* { value }
        let __replacement = __Override { call: typed($value), marker: ::core::marker::PhantomData };
        macro_rules! __take_field {
            ([$name] [$d old:ident] [$d new:ident]) => { $d new };
            ([$d other:ident] [$d old:ident] [$d new:ident]) => { $d old.$d other };
        }
        __Captures { $($method: __take_field! { [$method] [$slot] [__replacement] },)* }
    }};
    (@stateless [$d:tt] [$attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt]
        [$value:expr] [$slot:ident]
        [$($method:ident)*]) => {{
        #[derive(Clone, Copy, Default)] struct __Override;
        impl __callers::$name::__Call for __Override {
            $crate::__traitstub_methods! { @emit [layer] [$attrs $name $gen $call $args $out $where] [body $value] }
        }
        macro_rules! __take_field {
            ([$name] [$d old:ident]) => { __Override };
            ([$d other:ident] [$d old:ident]) => { $d old.$d other };
        }
        __Captures { $($method: __take_field! { [$method] [$slot] },)* }
    }};
    (@field_call [[$($attrs:tt)*] $name:ident [$($gen:tt)*] $call:tt ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        $($attrs)* fn $name<$($gen)*>(&self, $($arg:$ty),*) -> $out where $($where)* { (self.call)($($arg),*) }
    };
    (@call_trait $root:tt [[$($attrs:tt)*] $name:ident [$($gen:tt)*] $call:tt ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        #[allow(non_snake_case)] pub mod $name {
            #[allow(unused_imports)] use super::super::*;
            $crate::__traitstub_schema! { @source $root }
            pub trait __Call {
                $($attrs)* fn $name<$($gen)*>(&self, $($arg:$ty),*) -> $out where $($where)*;
            }
            $crate::__traitstub_results! { @result_alias [$($gen)*] [$out] [$($where)*] }
        }
    };
    (@base_call [[$($attrs:tt)*] $name:ident [$($gen:tt)*] [$($call:tt)*] ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        impl<__Base: __schema> __callers::$name::__Call for __Base {
            $($attrs)* fn $name<$($gen)*>(&self, $($arg:$ty),*) -> $out where $($where)* {
                <__Base as __schema>::$name::<$($call)*>(self, $($arg),*)
            }
        }
    };
}
