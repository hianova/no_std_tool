//! Collection types and hashing algorithms for `no_std` environments.
//!
//! This module provides dynamic data structures that require memory allocation
//! but do not require the standard library. It re-exports high-performance, 
//! `no_std` compatible containers and hashing algorithms.

/// A hash map implementation backed by `hashbrown`.
/// 
/// In a `no_std` environment, standard `std::collections::HashMap` is unavailable.
/// This implementation provides an equivalent API but relies only on the `alloc` crate.
pub use hashbrown::{HashMap, HashSet};

/// High-performance, non-cryptographic hashing algorithms.
///
/// Contains `AHasher` and `RandomState` for seeding HashMaps and Bloom filters.
/// This hashing algorithm is much faster than the default SipHash and is resistant
/// to hash-flooding DOS attacks.
pub mod ahash {
    pub use ahash::*;
}
