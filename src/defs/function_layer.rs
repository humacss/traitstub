//! Dispatch each replaced function independently of the base's capabilities.
#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_function_layer {
    (@emit $methods:tt $base:tt $overrides:tt) => {
        $crate::__traitstub_methods! { @selectors [function_layer] $methods $overrides }
        #[derive(Clone, Copy, Default)]
        struct __Layer<__Base, __Functions> { #[allow(dead_code)] base: __Base, replacement: __Functions }
        impl<__Base: __schema, __Functions> __schema for __Layer<__Base, __Functions> {}
        $crate::__traitstub_methods! { @implement [function_layer] $methods $overrides }
    };
    (@method [[$($attrs:tt)*] $name:ident [$($gen:ident),*] $call:tt
       ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]] [capture]) => {
        impl<$($gen,)* __Base: __schema, __Functions> __function_schema::$name<$($gen),*>
            for __Layer<__Base, __Functions>
        where __Functions: __CaptureCalls,
              <__Functions as __CaptureCalls>::$name: ::core::ops::Fn($($ty),*) -> $out, $($where)* {
            #[allow(unused_attributes)]
            $($attrs)* fn __call(&self, $($arg:$ty),*) -> $out { (self.replacement.$name())($($arg),*) }
        }
    };
    (@method [[$($attrs:tt)*] $name:ident [$($gen:ident),*] $call:tt
       ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]] [forward]) => {
        impl<$($gen,)* __Base, __Functions> __function_schema::$name<$($gen),*>
            for __Layer<__Base, __Functions>
        where __Base: __schema + __function_schema::$name<$($gen),*>, $($where)* {
            #[allow(unused_attributes)]
            $($attrs)* fn __call(&self, $($arg:$ty),*) -> $out {
                <__Base as __function_schema::$name<$($gen),*>>::__call(&self.base, $($arg),*)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    crate::deps! {
        first[H](host: &H) -> usize;
        second[H](host: &H) -> usize;
    }
    crate::stubs! { first => |_| 0; second => |_| 1; }
    #[test]
    fn different_functions_can_support_different_hosts_and_share_captures() {
        let seen = AtomicUsize::new(0);
        let deps = crate::defs! {
            first => |host: &u64| { seen.fetch_add(1, Ordering::Relaxed); *host as usize };
            second => |host: &String| host.len();
        };
        let text = String::from("hello");
        std::thread::scope(|scope| {
            let first = scope.spawn(|| deps.first(&7_u64));
            let second = scope.spawn(|| deps.second(&text));
            assert_eq!(first.join().unwrap(), 7);
            assert_eq!(second.join().unwrap(), 5);
        });
        assert_eq!(seen.load(Ordering::Relaxed), 1);
        assert_eq!(Defs.first(&text), 0);
    }
    #[test]
    fn typed_overrides_preserve_the_base_and_evaluate_the_target_once() {
        let mut evaluations = 0;
        let answer = 42;
        let deps = crate::defs! {
            target = { evaluations += 1; Defs };
            first(host: &u64) -> usize => |_| answer;
        };
        assert_eq!(evaluations, 1);
        assert_eq!(deps.first(&7_u64), 42);
        assert_eq!(deps.second(&"another host"), 1);
    }
}

#[cfg(test)]
mod method_attribute_tests {
    #![deny(unused_attributes)]
    crate::deps! {
        #[inline]
        #[track_caller]
        #[must_use]
        first(input: usize) -> usize;
        #[inline]
        #[track_caller]
        #[must_use]
        second(input: usize) -> usize;
    }
    crate::stubs! { first => |input| input + 1; second => |input| input * 2; }
    #[test]
    fn function_attributes_apply_to_replaced_and_inherited_methods() {
        let offset = 2;
        let deps = crate::defs! { first => |input| input + offset; };
        assert_eq!(deps.first(40), 42);
        assert_eq!(deps.second(21), 42);
    }
}
