//! Parameterized contracts use the ordinary signature parser and condition pass.
//! Metadata preserves collection parameters alongside associated types and methods.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_parameter_schema {
    (@headers $context:tt [$($types:tt)*]; type $name:ident = $ty:ty; $($rest:tt)*) => {
        $crate::__traitstub_parameter_schema! { @headers $context [$($types)* type $name = $ty;]; $($rest)* }
    };
    (@headers $context:tt $types:tt; $($methods:tt)*) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types [$($methods)*] []; $($methods)* }
    };
    // Preserve elided self-borrowed outputs before they become opaque ty fragments.
    // Stateless shorthand can return a static reference; fixture captures instead
    // supply their own named callable lifetime through the explicit signature.
    (@outputs $context:tt $types:tt $methods:tt $outputs:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) -> & $life:lifetime mut $out:ty $(where [$($where:tt)*])?, $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods $outputs; $($rest)* }
    };
    (@outputs $context:tt $types:tt $methods:tt [$($outputs:tt)*];
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) -> & mut $out:ty $(where [$($where:tt)*])?, $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods
            [$($outputs)* ($name [&'static mut $out])]; $($rest)* }
    };
    (@outputs $context:tt $types:tt $methods:tt $outputs:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) -> & $life:lifetime $out:ty $(where [$($where:tt)*])?, $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods $outputs; $($rest)* }
    };
    (@outputs $context:tt $types:tt $methods:tt [$($outputs:tt)*];
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) -> & $out:ty $(where [$($where:tt)*])?, $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods
            [$($outputs)* ($name [&'static $out])]; $($rest)* }
    };
    (@outputs $context:tt $types:tt $methods:tt $outputs:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) $(-> $out:ty)? $(where [$($where:tt)*])?, $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods $outputs; $($rest)* }
    };
    (@outputs [$($context:tt)*] $types:tt [$($methods:tt)*] $outputs:tt;) => {
        $crate::__traitstub_deps! { @entries [[__parameterized] $($context)* $types $outputs] []; $($methods)* }
    };
    (@outputs $context:tt $types:tt $methods:tt $outputs:tt; $($tail:tt)+) => {
        $crate::__traitstub_parameter_schema! { @last_output $context $types $methods $outputs; $($tail)+; }
    };
    (@last_output $context:tt $types:tt $methods:tt $outputs:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) -> & $($output:tt)+) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods $outputs;
            $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])? ($($arg:$ty),*) -> & $($output)+ }
    };
    (@last_output $context:tt $types:tt $methods:tt $outputs:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident:$ty:ty),* $(,)?) $(-> $out:ty)? $(where [$($where:tt)*])?, $body:expr;) => {
        $crate::__traitstub_parameter_schema! { @outputs $context $types $methods $outputs;
            $(#[$($attr)*])* $name $([$($gen)*])? $(call [$($call)+])? ($($arg:$ty),*) $(-> $out)? $(where [$($where)*])?, $body; }
    };
    (@schema [$d:tt] [__local] $contract:tt $export:tt $scope:tt $params:tt $where:tt $types:tt $outputs:tt $($methods:tt)*) => {
        $crate::__traitstub_parameter_schema! { @emit [$d] [__local] $contract $export $scope $params $where $types $outputs
            [] [pub(crate)] [__local]; $($methods)* }
    };
    (@schema [$d:tt] [$origin:ident] $contract:tt $export:tt $scope:tt $params:tt $where:tt $types:tt $outputs:tt $($methods:tt)*) => {
        $crate::__traitstub_parameter_schema! { @emit [$d] [$origin] $contract $export $scope $params $where $types $outputs
            [#[doc(hidden)] #[macro_export]] [pub] [$d $origin]; $($methods)* }
    };
    (@emit [$d:tt] [$origin:ident] [$contract:ident] [$export:ident] [$($scope:ident)*]
     [$($param:ident),+] [$($where:tt)*] [$(type $assoc:ident = $default:ty;)*] [$(($borrowed:ident [$borrowed_out:ty]))*]
     [$($attributes:tt)*] [$visibility:vis] $root:tt;
     $($(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
       ($($arg:ident: $ty:ty),*) -> $out:ty $(where [$($method_where:tt)*])?, $production:expr;)*) => {
        $crate::__traitstub_schema! { @source [$origin] }
        pub trait $contract<$($param),+> where $($where)* {
            $(type $assoc;)*
            $($(#[$($attr)*])* fn $name $(<$($gen)*>)?(&self, $($arg: $ty),*) -> $out $(where $($method_where)*)?;)*
        }
        #[doc(hidden)]
        #[derive(Clone, Copy, Default)]
        pub struct __TraitstubProduction;
        impl<$($param),+> $contract<$($param),+> for __TraitstubProduction where $($where)* {
            $(type $assoc = $default;)*
            $($(#[$($attr)*])* fn $name $(<$($gen)*>)?(&self, $($arg: $ty),*) -> $out $(where $($method_where)*)? {
                let call: fn($($ty),*) -> _ = $production;
                call($($arg),*)
            })*
        }
        #[allow(non_upper_case_globals)]
        pub const $contract: __TraitstubProduction = __TraitstubProduction;

        $($attributes)*
        macro_rules! $export {
            (@defaults [$d($d runtime:tt)*] [$d output:ident] [$d metadata:ident]
             [$d default_origin:ident] [$d($d default_scope:ident)*] [$d($d choices:tt)*] [$d($d overrides:tt)*]) => {
                $d($d runtime)*::__traitstub_stubs! { @parse
                    [[__parameterized] [$d] $root [$contract] [$($scope)*] [$($param),+] [$($where)*] [$($assoc)*]
                     [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($method_where)*)?]) )*]
                     [$d output] [$d metadata] [$d default_origin] [$d($d default_scope)*] [$d($d choices)*]] [];
                    $d($d overrides)* }
            };
            (@instance [$d($d runtime:tt)*] [$d($d request:tt)*]) => {
                $d($d runtime)*::__traitstub_parameter_defs! { @instance
                    [$root [$contract] [$($scope)*] [$($param),+] [$($where)*] [$($assoc)*]
                     [$( ([$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($method_where)*)?]) )*]]
                    [$d($d request)*] }
            };
            $(
                (@type [$d invoke:ident] $assoc [$d fallback:ty]; [type $assoc = $d value:ty; $d($d rest:tt)*]) => { $d value };
                (@type [$d invoke:ident] $assoc [$d fallback:ty]; [type $d other:ident = $d ignored:ty; $d($d rest:tt)*]) => {
                    $d invoke!(@type [$d invoke] $assoc [$d fallback]; [$d($d rest)*])
                };
                (@type [$d invoke:ident] $assoc [$d fallback:ty]; []) => { $d fallback };
                (@production_type $assoc) => { $default };
                (@valid_type $assoc) => {};
            )*
            $(
                (@select [$d($d runtime:tt)*] [$d kind:ident] [$d attrs:tt $name $d($d signature:tt)*]
                 [$name] [$d($d value:tt)*] [$d($d rest:tt)*]) => {
                    $d($d runtime)*::__traitstub_methods! { @emit [$d kind] [$d attrs $name $d($d signature)*] [$d($d value)*] }
                };
                (@capture [$d($d runtime:tt)*] $name [$d($d context:tt)*]) => {
                    __schema! { @capture_output [$d($d runtime)*] $name [$d($d context)*]
                        [[$(#[$($attr)*])*] $name [$($($gen)*)?] [$($($call)+)?] ($($arg: $ty),*) [$out] [$($($method_where)*)?]] }
                };
                (@valid_method $name) => {};
            )*
            $(
                (@capture_output [$d($d runtime:tt)*] $borrowed $d context:tt $d signature:tt) => {
                    $d($d runtime)*::__traitstub_parameter_defs! { @captured_output $d context $d signature [$borrowed_out] }
                };
            )*
            (@capture_output [$d($d runtime:tt)*] $d name:ident $d context:tt
             [$d attrs:tt $d name2:ident $d gen:tt $d call:tt $d args:tt [$d out:ty] $d where:tt]) => {
                $d($d runtime)*::__traitstub_parameter_defs! { @captured_output $d context
                    [$d attrs $d name2 $d gen $d call $d args [$d out] $d where] [$d out] }
            };
            (@select [$d($d runtime:tt)*] [$d kind:ident] [$d($d signature:tt)*]
             [$d other:ident] [$d($d value:tt)*] [$d($d rest:tt)*]) => {
                $d($d runtime)*::__traitstub_methods! { @find [$d kind] [$d($d signature)*] [$d($d rest)*] }
            };
            (@valid_type $d bad:ident) => { compile_error!(concat!("unknown associated type: ", stringify!($d bad))); };
            (@valid_method $d bad:ident) => { compile_error!(concat!("unknown dependency: ", stringify!($d bad))); };
            (@capture [$d($d runtime:tt)*] $d bad:ident [$d($d context:tt)*]) => {
                compile_error!(concat!("unknown dependency: ", stringify!($d bad)));
            };
        }
        #[allow(unused_imports)]
        $visibility use $export as $contract;
    };
}
#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_imports)]

    mod unconstrained {
        crate::deps! {
            for [T];
            identity(value: T) -> T, |value| value;
        }
        crate::stubs! { identity => |value| value; }
        #[rstest::rstest]
        #[case::number(7_u64)]
        #[case::text(String::from("text"))]
        fn production_and_defaults_keep_the_callers_type<T: PartialEq + std::fmt::Debug + Clone>(
            #[case] value: T,
        ) {
            assert_eq!(Deps.identity(value.clone()), value);
            assert_eq!(Defs.identity(value.clone()), value);
        }
        #[test]
        fn overrides_need_no_repeated_signature() {
            let deps = crate::defs! { identity => |value: u64| value + 1; };
            assert_eq!(deps.identity(7), 8);
        }
    }
    mod final_borrowed_output {
        crate::deps! { for [T]; current() -> &str, || "production" }
        crate::stubs! { current => || "shared"; }
        #[test]
        fn elided_output_survives_an_omitted_final_semicolon() {
            let deps = crate::defs! { current => || "override"; };
            assert_eq!(Deps::<u64>::current(&deps), "override");
        }
    }
    mod conditions {
        crate::deps! {
            target = Lookup;
            for [T] where [T: Clone,];
            #[cfg_attr(all(), cfg(any()))]
            disabled(value: Missing) -> Missing, |value| value;
            copy(value: &T) -> T, |value| value.clone();
        }
        crate::stubs! { contract = Lookup; name = Shared; copy => |value| value.clone(); }
        #[test]
        fn filtering_and_custom_names_preserve_generic_bounds() {
            assert_eq!(Lookup.copy(&7), 7);
            let deps = crate::defs! { contract = Lookup; target = Shared; copy => |value: &u64| *value + 1; };
            assert_eq!(deps.copy(&7), 8);
        }
    }
}
