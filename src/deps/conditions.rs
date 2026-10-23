//! Resolve cfg/cfg_attr in the declaring crate before exporting signatures.
//! Other attributes stay attached to the surviving methods.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_conditions {
    // --- @active: resolve cfg here, before metadata crosses crate boundaries ---
    // Raw attribute tokens are retained until this pass so cfg/cfg_attr can be
    // recognized. Every surviving non-conditional attribute stays on the method.
    (@active [$($context:tt)*] [$($done:tt)*];) => {
        $crate::__traitstub_schema! { @schema $($context)* $($done)* }
    };
    (@active $context:tt $done:tt;
     $(#[$($attr:tt)*])* $name:ident $([$($gen:tt)*])? $(call [$($call:tt)+])?
     ($($arg:ident: $ty:ty),*) -> $out:ty $(where [$($where:tt)*])?, $production:expr; $($rest:tt)*) => {
        $crate::__traitstub_conditions! { @attributes $context $done
            [$name $([$($gen)*])? $(call [$($call)+])? ($($arg: $ty),*) -> $out
             $(where [$($where)*])?, $production;]
            [] [$(#[$($attr)*])*] [$($rest)*]
        }
    };
    (@attributes $context:tt $done:tt $method:tt $kept:tt
     [#[cfg($condition:meta $(,)?)] $($attrs:tt)*] [$($rest:tt)*]) => {
        #[cfg($condition)]
        $crate::__traitstub_conditions! { @attributes $context $done $method $kept [$($attrs)*] [$($rest)*] }
        #[cfg(not($condition))]
        $crate::__traitstub_conditions! { @active $context $done; $($rest)* }
    };
    (@attributes $context:tt $done:tt $method:tt $kept:tt
     [#[cfg_attr($condition:meta, $($nested:tt)*)] $($attrs:tt)*] $rest:tt) => {
        #[cfg($condition)]
        $crate::__traitstub_conditions! { @expand_attributes $context $done $method $kept
            [] [$($nested)* ,] [$($attrs)*] $rest }
        #[cfg(not($condition))]
        $crate::__traitstub_conditions! { @attributes $context $done $method $kept [$($attrs)*] $rest }
    };
    (@attributes $context:tt $done:tt $method:tt [$($kept:tt)*]
     [#[$($attr:tt)*] $($attrs:tt)*] $rest:tt) => {
        $crate::__traitstub_conditions! { @attributes $context $done $method
            [$($kept)* #[$($attr)*]] [$($attrs)*] $rest }
    };
    (@attributes $context:tt [$($done:tt)*] [$($method:tt)*] [$($kept:tt)*] [] [$($rest:tt)*]) => {
        $crate::__traitstub_conditions! { @active $context [$($done)* $($kept)* $($method)*]; $($rest)* }
    };

    // Expand cfg_attr's comma-separated meta items without forwarding opaque
    // `meta` fragments: nested cfg/cfg_attr must still be recognized above.
    (@expand_attributes $context:tt $done:tt $method:tt $kept:tt $expanded:tt [, $($tail:tt)*] $attrs:tt $rest:tt) => {
        $crate::__traitstub_conditions! { @expand_attributes $context $done $method $kept $expanded [$($tail)*] $attrs $rest }
    };
    (@expand_attributes $context:tt $done:tt $method:tt $kept:tt [$($expanded:tt)*]
     [$head:ident $(::$path:ident)* ($($args:tt)*) , $($tail:tt)*] $attrs:tt $rest:tt) => {
        $crate::__traitstub_conditions! { @expand_attributes $context $done $method $kept
            [$($expanded)* #[$head $(::$path)* ($($args)*)]] [$($tail)*] $attrs $rest }
    };
    (@expand_attributes $context:tt $done:tt $method:tt $kept:tt [$($expanded:tt)*]
     [$head:ident $(::$path:ident)* = $value:expr , $($tail:tt)*] $attrs:tt $rest:tt) => {
        $crate::__traitstub_conditions! { @expand_attributes $context $done $method $kept
            [$($expanded)* #[$head $(::$path)* = $value]] [$($tail)*] $attrs $rest }
    };
    (@expand_attributes $context:tt $done:tt $method:tt $kept:tt [$($expanded:tt)*]
     [$head:ident $(::$path:ident)* , $($tail:tt)*] $attrs:tt $rest:tt) => {
        $crate::__traitstub_conditions! { @expand_attributes $context $done $method $kept
            [$($expanded)* #[$head $(::$path)*]] [$($tail)*] $attrs $rest }
    };
    (@expand_attributes $context:tt $done:tt $method:tt $kept:tt [$($expanded:tt)*] [] [$($attrs:tt)*] $rest:tt) => {
        $crate::__traitstub_conditions! { @attributes $context $done $method $kept [$($expanded)* $($attrs)*] $rest }
    };
}

#[cfg(test)]
mod tests {
    // Each scenario intentionally uses only part of its generated contract.
    #![allow(dead_code, unused_imports)]

    mod conditional_methods {
        mod disabled {
            crate::deps! {
                #[cfg(any())]
                unavailable(value: DoesNotExist) -> DoesNotExist, |value| value;
            }
            crate::stubs! {}
            #[test]
            fn a_collection_can_have_no_active_methods() {
                fn accepts(_: &impl Deps) {}
                accepts(&Deps);
                accepts(&Defs);
            }
        }

        mod attributes {
            crate::deps! {
                #[cfg_attr(all(), cfg_attr(all(), allow(dead_code), doc = "A retained method"), cfg(all()))]
                value() -> u64, || 1;
                #[cfg_attr(any(), cfg(any()))]
                other() -> u64, || 2;
                #[cfg_attr(all(), cfg(any()))]
                unavailable() -> MissingType, || unreachable!();
            }
            crate::stubs! { value => || 3; other => || 4; }
            #[test]
            fn nested_conditions_are_resolved_without_losing_other_attributes() {
                let deps = crate::defs! { value() => 5; };
                assert_eq!((Deps.value(), Defs.other(), deps.value()), (1, 4, 5));
            }
        }
    }
}
