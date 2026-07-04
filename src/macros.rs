//! Declarative macros for `#![no_std]` environment configuration.
//!
//! This module provides macros to inject common boilerplate code required by
//! bare-metal environments, as well as wrappers to suppress standard lints that
//! are often tripped during `no_std` module integration.

/// A macro to inject the necessary boilerplate for `no_std` environments
/// like `extern crate alloc;` and `#[cfg(test)] extern crate std;`.
#[macro_export]
macro_rules! base {
    () => {
        #[cfg(test)]
        extern crate std;
        
        extern crate alloc;
    };
}

/// A macro to wrap a module and apply standard `no_std` lint ignores.
#[macro_export]
macro_rules! module {
    (
        $(#[$meta:meta])*
        $vis:vis mod $name:ident {
            $($item:item)*
        }
    ) => {
        $(#[$meta])*
        #[allow(non_camel_case_types)]
        #[allow(unsafe_op_in_unsafe_fn)]
        #[allow(unused_variables)]
        #[allow(unused_assignments)]
        #[allow(unused_mut)]
        #[allow(dead_code)]
        #[allow(unreachable_code)]
        #[allow(unexpected_cfgs)]
        #[allow(unused_imports)]
        $vis mod $name {
            $($item)*
        }
    };
}
