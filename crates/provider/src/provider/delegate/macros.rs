// Defines macros for implementing a provider layer by delegating to an inner Provider field.
//
// When updating this file or the Provider trait, you should also run `scripts/provider_methods.rs`.
// See the script for usage.

#[macro_export]
#[doc(hidden)]
macro_rules! _pd_await_if_async {
    (async $($t:tt)*) => { $($t)*.await };
    ($($t:tt)*) => { $($t)* };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _pd_call_if_all_ne {
    ($a:ident, [], $($t:tt)*) => { $($t)* };
    ($a:ident, [$b:ident $(, $rest:ident)*], $($t:tt)*) => {
        $crate::pd_call_if_ne!($a, $b, $crate::pd_call_if_all_ne!($a, [$($rest),*], $($t)*););
    };
}

macro_rules! mk_delegate {
    ([$d:tt] $( [$($async:ident)?] fn $name:ident[$($generics:tt)*](&self $(, $arg:ident: $arg_ty:ty)* $(,)?) -> $ret:ty [$($where_:tt)*]; )*) => {
        #[macro_export]
        #[doc(hidden)]
        macro_rules! _pd_delegate {
            $(
                ($name, $d field:ident) => {
                    $($async)? fn $name $($generics)* (&self $(, $arg: $arg_ty )*) -> $ret $($where_)* {
                        $crate::pd_await_if_async!($($async)? self.$d field.$name($( $arg ),*))
                    }
                };
            )*
            ($d name2:ident, $d field:ident) => {
                compile_error!(concat!("method \"", stringify!($d name2), "\" is not a Provider method that can be overridden"));
            };
        }

        #[macro_export]
        #[doc(hidden)]
        macro_rules! _pd_call_if_ne {
            $(
                ($name, $name, $d($d t:tt)*) => {};
            )*
            ($d a:ident, $d b:tt, $d($d t:tt)*) => { $d($d t)* };
        }

        /// Delegates all provider methods to `self.$field`, except for the methods listed in `except`.
        ///
        /// Usage: `provider_delegate_except!(field, [except1, except2, ...]);`
        #[macro_export]
        macro_rules! _provider_delegate_except {
            ($d field:ident, [$d($d except:ident),* $d(,)?]) => {
                $(
                    $crate::pd_call_if_all_ne!($name, [$d($d except),*], $crate::pd_delegate!($name, $d field););
                )*
            };
        }
    };
}
crate::_pd_all_methods!([$] mk_delegate);

#[doc(hidden)]
pub use _pd_await_if_async as pd_await_if_async;
#[doc(hidden)]
pub use _pd_call_if_all_ne as pd_call_if_all_ne;
#[doc(hidden)]
pub use _pd_call_if_ne as pd_call_if_ne;
#[doc(hidden)]
pub use _pd_delegate as pd_delegate;
#[doc(hidden)]
pub use _provider_delegate_except as provider_delegate_except;
