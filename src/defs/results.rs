//! Let Rust resolve output elision in a receiver-like function signature.
//! Explicit input lifetimes stay generic; an implicit receiver borrow becomes
//! static for shorthand. Test-owned result lifetimes still use explicit syntax.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_results {
    (@result_alias [] [$out:ty] $where:tt) => {
        pub type __Result<'__receiver> = <fn(&'__receiver ()) -> $out as $crate::__TraitstubSignatureResult>::Output;
        pub type __StaticResult = __Result<'static>;
    };
    (@result_alias [$($lifetime:lifetime),+ $(,)?] [$out:ty] []) => {
        pub type __Result<'__receiver, $($lifetime),+> = <fn(&'__receiver ()) -> $out as $crate::__TraitstubSignatureResult>::Output;
        $crate::__traitstub_results! { @static_alias []; $($lifetime),+ }
    };
    (@result_alias $gen:tt $out:tt $where:tt) => {};
    (@static_alias [$($done:tt)*]; $next:lifetime $(,$rest:lifetime)*) => {
        $crate::__traitstub_results! { @static_alias [$($done)* 'static,]; $($rest),* }
    };
    (@static_alias [$($done:tt)*];) => { pub type __StaticResult = __Result<'static, $($done)*>; };
}

/// Extract the result of a signature whose reference supplies output elision.
#[doc(hidden)]
pub trait SignatureResult {
    type Output;
}
impl<'a, R> SignatureResult for fn(&'a ()) -> R {
    type Output = R;
}

#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_imports)]
    // The unused lifetime is intentional: it reproduces a valid old declaration.
    #[allow(clippy::extra_unused_lifetimes)]
    mod lifetime_generic_self_borrow {
        crate::deps! {
            value['unused]() -> &str, || "production";
            explicit['unused]() -> &'_ str, || "production";
            named['output]() -> &'output str, || "production";
            owned['unused]() -> usize, || 0;
            fixed['unused]() -> &'static str, || "production";
            shorten['long, 'short](value: &'long str) -> &'short str
                where ['long: 'short], |value| value;
        }
        crate::stubs! {
            value => || "stub";
            explicit => || "stub";
            named => || "stub";
            owned => || 0;
            fixed => || "stub";
            shorten => |value| value;
        }
        #[test]
        fn an_unrelated_lifetime_does_not_change_an_output_borrow() {
            let count = 7;
            let fixed = "fixed";
            let deps = crate::defs! {
                owned => || count;
                fixed => || fixed;
                value => || "test";
                explicit => || "explicit";
                named => || "named";
                shorten => |value| value;
            };
            assert_eq!(deps.value(), "test");
            assert_eq!(deps.explicit(), "explicit");
            assert_eq!(deps.named(), "named");
            assert_eq!(deps.owned(), 7);
            assert_eq!(deps.fixed(), "fixed");
            let input = String::from("borrowed");
            assert_eq!(deps.shorten(&input), "borrowed");
        }
    }

    mod default_body {
        crate::deps! { bounds = [Default]; current() -> &str, || "production"; }
        crate::stubs! { current => || "stub"; }
        #[test]
        fn borrowed_result_body_overrides_support_default() {
            let deps = crate::defs! { current() => "test"; };
            fn fresh<D: Deps>(_: &D) -> D {
                D::default()
            }
            assert_eq!(fresh(&deps).current(), "test");
        }
    }

    mod output_signatures {
        crate::deps! {
            nested() -> Result<Option<Vec<u64>>, &'static str>, || Ok(Some(vec![1]));
            borrowed() -> Option<(&str, usize)>, || Some(("production", 1));
            matrix() -> [[u8; 2]; 2], || [[1, 2], [3, 4]];
            predicate() -> fn(u64) -> bool, || |value| value > 0
        }
        crate::stubs! {
            nested => || Ok(None);
            borrowed => || None;
            matrix => || [[0; 2]; 2];
            predicate => || |_| false;
        }
        #[test]
        fn nested_types_borrowed_outputs_and_omitted_semicolons_keep_working() {
            let limit = 7;
            let deps = crate::defs! {
                nested => || Ok(Some(vec![limit]));
                borrowed => || Some(("test", 2));
                matrix => || [[1; 2]; 2];
                predicate => || |value| value > 0;
            };
            assert_eq!(deps.nested(), Ok(Some(vec![7])));
            assert_eq!(deps.borrowed(), Some(("test", 2)));
            assert_eq!(deps.matrix(), [[1; 2]; 2]);
            assert!((deps.predicate())(1));
        }
    }
}
