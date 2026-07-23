#![deny(unsafe_op_in_unsafe_fn)]
#![allow(dead_code, unused_imports, unused_variables, unused_assignments, unused_mut, unreachable_code)]
pub mod avx2;
pub mod neon;
pub mod scalar;
