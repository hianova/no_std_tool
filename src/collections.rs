//! Collection types and hashing algorithms for `no_std` environments.
//!
//! This module provides dynamic data structures that require memory allocation
//! but do not require the standard library. It re-exports high-performance,
//! `no_std` compatible containers and hashing algorithms.

pub use ahash;
pub use alloc::boxed::Box;
pub use alloc::vec::Vec as AllocVec;
pub use hashbrown::{HashMap, HashSet};
/// Fixed-capacity collections backed by `heapless`.
///
/// In a strict aerospace `no_std` environment, standard dynamic allocation is unavailable.
/// This implementation provides static, fixed-capacity equivalents.
pub use heapless::{FnvIndexMap, FnvIndexSet, LinearMap, String, Vec};

pub mod mpsc_queue;
pub use mpsc_queue::BoundedQueue;

pub mod bloom;
pub use bloom::SimpleBloom;
