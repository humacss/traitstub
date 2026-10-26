//! Generate complete shared defaults for per-function dispatch contracts.
#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_function_stubs {
    (@implementation [$root:ident] [$contract:ident] [$($scope:ident)*] [$name:ident]
     $lookup:tt $methods:tt [$(($changed:ident [$($replacement:tt)*]))*]) => {
        #[derive(Clone, Copy, Default)] pub struct $name;
        const _: () = {
            $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
            __schema! { @functions }
            $crate::__traitstub_methods! { @selectors [function_stubs] $methods [$(($changed [$($replacement)*]))*] }
            impl __schema for $name {}
            #[allow(dead_code)] type __Stub = $name;
            #[allow(dead_code)] struct __UniqueMethods { $($changed: (),)* }
            $(__schema!(@valid_method $changed);)*
            $crate::__traitstub_methods! { @implement [function_stubs] $methods [$(($changed [$($replacement)*]))*] }
        };
    };
    (@method [[$($attrs:tt)*] $name:ident [$($gen:ident),*] $call:tt
        ($($arg:ident:$ty:ty),*) [$out:ty] [$($where:tt)*]] [body $body:expr]) => {
        $($attrs)* impl<$($gen),*> __function_schema::$name<$($gen),*> for __Stub
        where $($where)* {
            fn __call(&self, $($arg:$ty),*) -> $out {
                let call: fn($($ty),*) -> _ = $body;
                call($($arg),*)
            }
        }
    };
    (@method [[$($attrs:tt)*] $name:ident $($signature:tt)*] [forward]) => {
        compile_error!(concat!("stubs! is missing a stub for ", stringify!($name)));
    };
}

#[cfg(test)]
mod tests {
    mod local_contract {
        mod api {
            crate::deps! { query[H](host: &H) -> usize where [H: ?Sized]; }
        }
        use api::{calls, Deps};
        crate::stubs! { query => |_| 0; }
        #[test]
        fn imported_local_contracts_keep_their_function_capabilities() {
            let text = String::from("hello");
            let deps = crate::defs! { query => |host: &str| host.len(); };
            assert_eq!(deps.query(text.as_str()), 5);
            assert_eq!(Defs.query(&7_u64), 0);
        }
    }
}
