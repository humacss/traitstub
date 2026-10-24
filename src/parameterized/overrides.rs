//! Normalize overrides and use schema signatures to infer captured closure types.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_parameter_defs {
    (@instance [[$root:ident] [$contract:ident] [$($scope:ident)*] $params:tt $where:tt $assoc:tt $methods:tt] [$($request:tt)*]) => {{
        $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
        $crate::__traitstub_parameter_defs! { @headers [$params $where $assoc $methods [__base __witness]] [Defs] []; $($request)* }
    }};
    (@headers $context:tt $base:tt $life:tt; target = $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @headers $context [$value] $life; $($rest)* }
    };
    (@headers $context:tt $base:tt []; capture [$($life:lifetime),+]; $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @headers $context $base [$($life),+]; $($rest)* }
    };
    (@headers $context:tt $base:tt $life:tt; $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life []; $($rest)* }
    };
    (@parse $context:tt $base:tt $life:tt [$($done:tt)*]; $name:ident => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life [$($done)* ($name [infer $value])]; $($rest)* }
    };
    (@parse $context:tt $base:tt $life:tt $done:tt; $name:ident => $value:expr) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life $done; $name => $value; }
    };
    (@parse $context:tt $base:tt $life:tt $done:tt; $name:ident($($arg:ident),*) => $body:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life $done; $name => |$($arg),*| $body; $($rest)* }
    };
    (@parse $context:tt $base:tt $life:tt $done:tt; $name:ident($($arg:ident),*) => $body:expr) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life $done; $name => |$($arg),*| $body; }
    };
    (@parse $context:tt $base:tt $life:tt [$($done:tt)*]; $name:ident { $method:item } $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life [$($done)* ($name [method $method])]; $($rest)* }
    };
    (@parse $context:tt $base:tt $life:tt [$($done:tt)*]; $(for [$($binder:lifetime),+])?
     $name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr; $($rest:tt)*) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life
            [$($done)* ($name [typed [$($($binder),+)?] [$($ty),*] [$out] $value])]; $($rest)* }
    };
    (@parse $context:tt $base:tt $life:tt $done:tt; $(for [$($binder:lifetime),+])?
     $name:ident($($arg:ident: $ty:ty),*) -> $out:ty => $value:expr) => {
        $crate::__traitstub_parameter_defs! { @parse $context $base $life $done;
            $(for [$($binder),+])? $name($($arg:$ty),*) -> $out => $value; }
    };
    (@parse $context:tt $base:tt $life:tt [$(($name:ident [$($value:tt)*]))+];) => {{
        #[allow(dead_code)] struct __UniqueMethods { $($name: (),)+ }
        $crate::__traitstub_parameter_defs! { @collect $context $base $life [] [];
            $(($name [$($value)*]))+ }
    }};
    (@parse $($bad:tt)*) => { compile_error!("defs! expects one or more dependency overrides"); };

    // Generic type/const methods remain stateless; ordinary and lifetime-only
    // methods infer Fn captures from the contract and the chosen base.
    (@collect $context:tt $base:tt $life:tt $captures:tt $overrides:tt;
     ($name:ident [$($value:tt)*]) $($rest:tt)*) => {
        __schema! { @capture [$crate] $name [$context $base $life $captures $overrides [$($value)*] [$($rest)*]] }
    };
    (@captured_output [$context:tt $base:tt $life:tt $captures:tt [$($overrides:tt)*] [method $method:item] [$($rest:tt)*]]
     [$attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt] $hint:tt) => {
        $crate::__traitstub_parameter_defs! { @collect $context $base $life $captures
            [$($overrides)* ($name [method $method])]; $($rest)* }
    };
    (@captured_output [$context:tt $base:tt $life:tt [$($captures:tt)*] [$($overrides:tt)*]
      [typed $binder:tt $args:tt $out:tt $value:expr] [$($rest:tt)*]] $signature:tt $hint:tt) => {
        $crate::__traitstub_parameter_defs! { @capture $context $base $life [$($captures)*] [$($overrides)*]
            $signature $binder $args $out [$value] [$($rest)*] }
    };
    (@captured_output [$context:tt $base:tt $life:tt $captures:tt $overrides:tt [infer $value:expr] $rest:tt]
     [$attrs:tt $name:ident [$($binder:lifetime),*] $call:tt ($($arg:ident:$ty:ty),*) [$out:ty] $where:tt] $hint:tt) => {
        $crate::__traitstub_parameter_defs! { @capture $context $base $life $captures $overrides
            [$attrs $name [$($binder),*] $call ($($arg:$ty),*) [$out] $where]
            [$($binder),*] [$($ty),*] $hint [$value] $rest }
    };
    (@captured_output [$context:tt $base:tt $life:tt $captures:tt [$($overrides:tt)*] [infer $value:expr] [$($rest:tt)*]]
     [$attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt] $hint:tt) => {
        $crate::__traitstub_parameter_defs! { @collect $context $base $life $captures
            [$($overrides)* ($name [body $value])]; $($rest)* }
    };
    (@capture $context:tt $base:tt $life:tt [$($captures:tt)*] [$($overrides:tt)*]
     [$attrs:tt $name:ident $gen:tt $call:tt $args:tt $out:tt $where:tt]
     $binder:tt $capture_args:tt $capture_out:tt [$value:expr] [$($rest:tt)*]) => {
        $crate::__traitstub_parameter_defs! { @collect $context $base $life
            [$($captures)* ($attrs $name $gen $args $out $where $binder $capture_args $capture_out [$value])]
            [$($overrides)* ($name [capture])]; $($rest)* }
    };
    (@collect $context:tt $base:tt $life:tt $captures:tt $overrides:tt;) => {
        $crate::__traitstub_parameter_layer! { @emit $context $base $life $captures $overrides }
    };
}
#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_imports)]
    use std::cell::RefCell;

    pub trait Host {
        type Value;
        type Error;
        fn query(&self, id: u64) -> Result<Option<Self::Value>, Self::Error>;
    }
    pub struct Memory;
    impl Host for Memory {
        type Value = String;
        type Error = &'static str;
        fn query(&self, id: u64) -> Result<Option<String>, &'static str> {
            Ok(Some(format!("real-{id}")))
        }
    }
    crate::deps! {
        for [H] where [H: source::parameterized::overrides::tests::Host + ?Sized];
        query(host: &H, id: u64) -> Result<Option<H::Value>, H::Error>, |host, id| host.query(id);
        label() -> &'static str, || "production";
    }
    crate::stubs! {
        query => |_, _| Ok(None);
        label => || "shared";
    }
    fn read<H: Host + ?Sized>(
        deps: &impl Deps<H>,
        host: &H,
        id: u64,
    ) -> Result<Option<H::Value>, H::Error> {
        deps.query(host, id)
    }

    #[test]
    fn infers_host_and_result_from_an_annotated_capture() {
        let seen = RefCell::new(Vec::new());
        let value = String::from("captured");
        let deps = crate::defs! {
            query => |_: &Memory, id| {
                seen.borrow_mut().push(id);
                Ok(Some(value.clone()))
            };
        };
        assert_eq!(read(&deps, &Memory, 7), Ok(Some("captured".into())));
        assert_eq!(*seen.borrow(), [7]);
        assert_eq!(<_ as Deps<Memory>>::label(&deps), "shared");
        assert_eq!(read(&Defs, &Memory, 7), Ok(None));
        assert_eq!(read(&Deps, &Memory, 7), Ok(Some("real-7".into())));
    }

    #[test]
    fn layered_overrides_preserve_captures_and_evaluate_the_base_once() {
        let value = String::from("captured");
        let base = crate::defs! { query => move |_: &Memory, _| Ok(Some(value.clone())); };
        let mut evaluations = 0;
        let deps = crate::defs! {
            target = { evaluations += 1; base };
            label => || "layer";
        };
        assert_eq!(evaluations, 1);
        assert_eq!(read(&deps, &Memory, 1), Ok(Some("captured".into())));
        assert_eq!(<_ as Deps<Memory>>::label(&deps), "layer");
    }

    #[test]
    fn later_layers_infer_unannotated_reference_arguments_from_the_base() {
        let base = crate::defs! { query => |_: &Memory, _| Ok(Some("base".into())); };
        let deps = crate::defs! { target = base; query => |_, _| Ok(None); };
        assert_eq!(read(&deps, &Memory, 1), Ok(None));
    }

    #[test]
    fn clone_and_copy_depend_on_storage_not_the_host_type() {
        let value = String::from("borrowed");
        let deps = crate::defs! { query => |_: &Memory, _| Ok(Some(value.clone())); };
        fn copied<D: Copy + Clone>(deps: &D) -> D {
            *deps
        }
        let other = copied(&deps);
        assert_eq!(read(&other, &Memory, 1), Ok(Some("borrowed".into())));
    }

    #[test]
    fn typed_captures_can_be_shared_between_workers() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = AtomicUsize::new(0);
        let deps = crate::defs! { query => |_: &Memory, _| {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        }; };
        std::thread::scope(|scope| {
            let first = scope.spawn(|| read(&deps, &Memory, 1));
            let second = scope.spawn(|| read(&deps, &Memory, 2));
            assert_eq!(
                (first.join().unwrap(), second.join().unwrap()),
                (Ok(None), Ok(None))
            );
        });
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn defaults_work_with_unsized_hosts_and_concrete_errors() {
        let host: &dyn Host<Value = String, Error = &'static str> = &Memory;
        assert_eq!(read(&Deps, host, 9), Ok(Some("real-9".into())));
        let deps = crate::defs! { query => |_: &Memory, _| Err("unavailable"); };
        assert_eq!(read(&deps, &Memory, 9), Err("unavailable"));
    }
}
