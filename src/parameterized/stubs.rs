//! Complete zero-sized defaults, including associated-type selections.

#[doc(hidden)]
#[macro_export]
macro_rules! __traitstub_parameter_stubs {
    (@emit [$d:tt] $root:tt $contract:tt $scope:tt $params:tt $where:tt $assoc:tt $methods:tt
     $name:tt $export:tt [__local] $default_scope:tt $choices:tt $overrides:tt) => {
        $crate::__traitstub_parameter_stubs! { @implementation $root $contract $scope $params $where $assoc $methods $name $choices $overrides }
        $crate::__traitstub_parameter_stubs! { @metadata [$d] $root $contract $scope $params $where $assoc $methods $name $export [] [pub(crate)] }
    };
    (@emit [$d:tt] $root:tt $contract:tt $scope:tt $params:tt $where:tt $assoc:tt $methods:tt
     $name:tt $export:tt [$origin:ident] $default_scope:tt $choices:tt $overrides:tt) => {
        $crate::__traitstub_parameter_stubs! { @implementation $root $contract $scope $params $where $assoc $methods $name $choices $overrides }
        $crate::__traitstub_parameter_stubs! { @metadata [$d] $root $contract $scope $params $where $assoc $methods $name $export
            [#[doc(hidden)] #[macro_export]] [pub] }
    };
    (@metadata [$d:tt] $root:tt $contract:tt $scope:tt $params:tt $where:tt $assoc:tt $methods:tt
     [$name:ident] [$export:ident] [$($attributes:tt)*] [$visibility:vis]) => {
        $($attributes)*
        macro_rules! $export {
            ([$d($d runtime:tt)*] $d($d request:tt)*) => {
                $d($d runtime)*::__traitstub_parameter_defs! { @instance
                    [$root $contract $scope $params $where $assoc $methods] [$d($d request)*] }
            };
        }
        #[allow(unused_imports)]
        $visibility use $export as $name;
    };
    (@implementation [$root:ident] [$contract:ident] [$($scope:ident)*]
     [$($param:ident),+] [$($where:tt)*] [$($assoc:ident)*] $methods:tt [$name:ident]
     [$(type $choice:ident = $selected:ty;)*] [$(($changed:ident [$($replacement:tt)*]))*]) => {
        #[derive(Clone, Copy, Default)]
        pub struct $name;
        const _: () = {
            $crate::__traitstub_methods! { @import_schema [$root] [$($scope)*] [$contract] }
            #[allow(dead_code)] struct __UniqueMethods { $($changed: (),)* }
            #[allow(dead_code, non_snake_case)] struct __UniqueTypes { $($choice: (),)* }
            $(__schema!(@valid_method $changed);)*
            $(__schema!(@valid_type $choice);)*
            $crate::__traitstub_parameter_stubs! { @trait_impl [$($param),+] [$($where)*] [$($assoc)*]
                [$name] [$(type $choice = $selected;)*] $methods [$(($changed [$($replacement)*]))*] }
        };
    };
    (@trait_impl [$($param:ident),+] [$($where:tt)*] [$($assoc:ident)*] [$name:ident] $choices:tt $methods:tt $overrides:tt) => {
        impl<$($param),+> __schema<$($param),+> for $name where $($where)* {
            $(type $assoc = __schema!(@type [__schema] $assoc [__schema!(@production_type $assoc)]; $choices);)*
            $crate::__traitstub_methods! { @implement [named] $methods $overrides }
        }
    };
}
#[cfg(test)]
mod tests {
    #![allow(dead_code, unused_imports)]
    crate::deps! {
        for [Input, Output] where [Input: AsRef<str>, Output: From<usize>];
        length(input: &Input) -> Output, |input| input.as_ref().len().into();
        marker(output: Output) -> Output, |output| output;
    }
    crate::stubs! { length => |input| input.as_ref().len().into(); marker => |output| output; }

    #[test]
    fn multiple_type_parameters_are_inferred_from_the_closure_and_call() {
        let adjustment = 3_usize;
        let deps = crate::defs! {
            length => |input: &String| input.len() + adjustment;
        };
        assert_eq!(deps.length(&"four".to_owned()), 7_usize);
        assert_eq!(deps.marker(11_usize), 11);
        let shared: usize = Defs.length(&"four".to_owned());
        assert_eq!(shared, 4);
    }
}
