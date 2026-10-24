//! Store inferred Fn closures and forward the remaining methods to their base.
//!
//! Callable bounds live on the base's helper traits, so Self::Value projections
//! keep their meaning without inspecting or rewriting Rust types in signatures.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_parameter_layer {
    (@emit $context:tt $base:tt $life:tt $captures:tt $overrides:tt) => {
        $crate::__traitstub_parameter_layer! { @parameters $context $base $life $captures $overrides }
    };
    (@parameters [[$($param:ident),+] $($context:tt)*] $base:tt $life:tt $captures:tt $overrides:tt) => {
        $crate::__traitstub_parameter_layer! { @implementation [[$($param),+] $($context)*] [$($param),+] $base $life $life $captures $overrides }
    };
    (@call_trait [$($life:lifetime),*] [[$($attrs:tt)*] $name:ident [$($gen:tt)*] ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        #[allow(mismatched_lifetime_syntaxes)]
        $($attrs)* fn $name<'__traitstub_call, $($gen)*>(&'__traitstub_call self,
            functions: &'__traitstub_call __Functions, $($arg:$ty),*) -> $out
            where $($life: '__traitstub_call,)* $($where)*;
    };
    (@call_impl [$($life:lifetime),*] [[$($attrs:tt)*] $name:ident [$($gen:tt)*] ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]]) => {
        #[allow(mismatched_lifetime_syntaxes)]
        $($attrs)* fn $name<'__traitstub_call, $($gen)*>(&'__traitstub_call self,
            functions: &'__traitstub_call __Functions, $($arg:$ty),*) -> $out
            where $($life: '__traitstub_call,)* $($where)* {
                (functions.$name())($($arg),*)
            }
    };
    (@assoc [$($param:ident),+] $assoc:ident) => { type $assoc = <__Base as __schema<$($param),+>>::$assoc; };
    (@implementation [[$($param:ident),+] [$($where:tt)*] [$($assoc:ident)*] $methods:tt [$base_ident:ident $witness:ident]]
      $parameters:tt [$base:expr] [$($life:lifetime),*] $lifetimes:tt
     [$( ([$($attrs:tt)*] $name:ident [$($gen:tt)*] ($($arg:ident:$ty:ty),*) [$out:ty] [$($method_where:tt)*]
          [$($binder:lifetime),*] [$($capture_ty:ty),*] [$capture_out:ty] [$value:expr]) )*] $overrides:tt) => {{
        $crate::__traitstub_captures! { @storage [$($name)*] }
        trait __Inference<$($life,)* $($param),+>: __schema<$($param),+> where $($where)* {
            type __Witness;
            $( $($attrs)* fn $name<__Function>(&self,
                _: Self::__Witness, value: __Function) -> __Function
                where __Function: for<$($binder),*> ::core::ops::Fn($($capture_ty),*) -> $capture_out { value } )*
        }
        impl<$($life,)* $($param,)+ __Base> __Inference<$($life,)* $($param),+> for __Base
            where __Base: __schema<$($param),+>, $($where)* {
                type __Witness = ::core::marker::PhantomData<($(&$life (),)* fn($(&$param),+),)>;
            }

        trait __Calls<$($life,)* $($param,)+ __Functions>: __schema<$($param),+> where $($where)* {
            $( $crate::__traitstub_parameter_layer! { @call_trait $lifetimes
                [[$($attrs)*] $name [$($gen)*] ($($arg:$ty),*) [$out] [$($method_where)*]] } )*
        }
        impl<$($life,)* $($param,)+ __Base, __Functions> __Calls<$($life,)* $($param,)+ __Functions> for __Base
        where __Base: __schema<$($param),+>, __Functions: __CaptureCalls,
            $( <__Functions as __CaptureCalls>::$name: for<$($binder),*> ::core::ops::Fn($($capture_ty),*) -> $capture_out, )*
            $($where)* {
            $( $crate::__traitstub_parameter_layer! { @call_impl $lifetimes
                [[$($attrs)*] $name [$($gen)*] ($($arg:$ty),*) [$out] [$($method_where)*]] } )*
        }
        struct __Layer<$($life,)* $($param,)+ __Base, __Functions> where $($where)* {
            base: __Base,
            replacement: __Functions,
            marker: ::core::marker::PhantomData<($(&$life (),)* fn($(&$param),+),)>,
        }
        impl<$($life,)* $($param,)+ __Base, __Functions> __schema<$($param),+> for __Layer<$($life,)* $($param,)+ __Base, __Functions>
        where __Base: __Calls<$($life,)* $($param,)+ __Functions>, $($where)* {
            $( $crate::__traitstub_parameter_layer! { @assoc $parameters $assoc } )*
            $crate::__traitstub_methods! { @implement [parameter_layer] $methods $overrides }
        }
        impl<$($life,)* $($param,)+ __Base: Clone, __Functions: Clone> Clone for __Layer<$($life,)* $($param,)+ __Base, __Functions> where $($where)* {
            fn clone(&self) -> Self {
                Self { base: self.base.clone(), replacement: self.replacement.clone(), marker: ::core::marker::PhantomData }
            }
        }
        impl<$($life,)* $($param,)+ __Base: Copy, __Functions: Copy> Copy for __Layer<$($life,)* $($param,)+ __Base, __Functions> where $($where)* {}
        impl<$($life,)* $($param,)+ __Base: Default, __Functions: Default> Default for __Layer<$($life,)* $($param,)+ __Base, __Functions> where $($where)* {
            fn default() -> Self {
                Self { base: __Base::default(), replacement: __Functions::default(), marker: ::core::marker::PhantomData }
            }
        }
        fn __make<$($life,)* $($param,)+ __Base, __Functions, __Build>(base: __Base, build: __Build)
            -> __Layer<$($life,)* $($param,)+ __Base, __Functions>
        where __Base: __Calls<$($life,)* $($param,)+ __Functions>,
            __Build: ::core::ops::FnOnce(&__Base, ::core::marker::PhantomData<($(&$life (),)* fn($(&$param),+),)>) -> __Functions,
            $($where)* {
            let replacement = build(&base, ::core::marker::PhantomData);
            __Layer { base, replacement, marker: ::core::marker::PhantomData }
        }
        let __target = $base;
        __make(__target, |$base_ident, $witness| __Captures {
            $( $name: __Inference::$name($base_ident, $witness, $value), )*
        })
    }};
}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_imports)]
    crate::deps! {
        for [H] where [H: Clone];
        type Value = String;
        value() -> Self::Value, String::new;
        query(host: &H) -> H, |host| host.clone();
        copy[T: Clone](value: T) -> (T, T), |value| (value.clone(), value);
        trim['a](value: &'a str) -> &'a str, |value| value.trim();
        current() -> &str, || "production";
        optional() -> Option<&str>, || Some("production");
    }
    crate::stubs! {
        type Value = Vec<u8>;
        value => || vec![1];
        query => |host| host.clone();
        copy => |value| (value.clone(), value);
        trim => |value| value;
        current => || "shared";
        optional => || None;
    }

    #[test]
    fn combines_inferred_captures_associated_types_and_generic_methods() {
        let value = vec![2, 3];
        let deps = crate::defs! {
            query => |host: &u64| *host + 1;
            value => || value.clone();
            copy(input) => (input.clone(), input);
            trim(value) => value.trim_start();
        };
        assert_eq!(deps.query(&7), 8);
        assert_eq!(deps.value(), vec![2, 3]);
        assert_eq!(deps.copy("text"), ("text", "text"));
        assert_eq!(deps.copy(9), (9, 9));
        assert_eq!(deps.trim(" text "), "text ");
    }

    #[test]
    fn shorthand_can_override_an_elided_borrowed_result() {
        let deps = crate::defs! { current => || "override"; };
        assert_eq!(deps.query(&7_u64), 7);
        assert_eq!(deps.current(), "override");
    }

    #[test]
    fn nested_borrowed_outputs_use_an_explicit_callable_lifetime() {
        let static_deps = crate::defs! {
            optional() -> Option<&'static str> => || Some("static");
        };
        assert_eq!(static_deps.query(&7_u64), 7);
        assert_eq!(static_deps.optional(), Some("static"));
        let name = String::from("fixture");
        let borrowed = crate::defs! {
            target = static_deps;
            capture ['fixture];
            optional() -> Option<&'fixture str> => || Some(name.as_str());
        };
        assert_eq!(borrowed.optional(), Some("fixture"));
    }

    #[test]
    fn borrowed_results_still_use_test_owned_lifetimes() {
        let name = String::from("fixture");
        let deps = crate::defs! {
            capture ['fixture];
            current() -> &'fixture str => || name.as_str();
        };
        assert_eq!(deps.query(&7_u64), 7);
        assert_eq!(deps.current(), "fixture");
    }
}
