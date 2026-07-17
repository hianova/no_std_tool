#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
//! `no_std_tool` is a universal foundation library for `#![no_std]` bare-metal Rust projects.
//!
//! This crate consolidates essential utilities that are frequently required in embedded,
//! OS-development, or other resource-constrained environments where the standard library
//! is unavailable. It isolates the complexity of dependency management, hardware-specific
//! math fallbacks, and low-level memory operations.
//!
//! # Features
//! - **Synchronization (`sync`)**: Spinlocks, exponential backoff, and full atomic support.
//! - **Math (`math`)**: Zero-float approximations for exponentials and inverse square roots.
//! - **Collections (`collections`)**: High-performance, `no_std` compatible `HashMap` and `HashSet` backed by `ahash`.
//! - **Debugging (`debug`)**: Global tracking for memory leaks and thread lifecycles.
//! - **Macros (`macros`)**: Common boilerplate generators for `#![no_std]` projects.

extern crate alloc;

pub mod collections;
pub mod debug;
pub mod scriptgo_vm;
pub mod macros;
pub mod math;
pub mod sync;

