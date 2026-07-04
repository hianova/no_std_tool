//! Collection types and hashing algorithms for `no_std` environments.
//!
//! This module provides dynamic data structures that require memory allocation
//! but do not require the standard library. It re-exports high-performance, 
//! `no_std` compatible containers and hashing algorithms.

/// Fixed-capacity collections backed by `heapless`.
/// 
/// In a strict aerospace `no_std` environment, standard dynamic allocation is unavailable.
/// This implementation provides static, fixed-capacity equivalents.
pub use heapless::{Vec, String, FnvIndexMap, FnvIndexSet, LinearMap};
pub use ahash;
pub use hashbrown::{HashMap, HashSet};

use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::MaybeUninit;
use core::cell::UnsafeCell;

/// A lock-free, zero-allocation Single-Producer Single-Consumer (SPSC) Ring Buffer.
///
/// This queue is wait-free and requires no dynamic allocation. It is highly suitable
/// for cross-privilege (Ring 3 vs Ring 0) IPC or interrupt-driven data channels.
pub struct RingBuffer<T, const N: usize> {
    head: AtomicUsize,
    tail: AtomicUsize,
    data: [UnsafeCell<MaybeUninit<T>>; N],
}

unsafe impl<T: Send, const N: usize> Send for RingBuffer<T, N> {}
unsafe impl<T: Send, const N: usize> Sync for RingBuffer<T, N> {}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Asserts that N is a power of two to prevent modulo discontinuity on overflow.
    pub const fn new() -> Self {
        assert!(N.is_power_of_two(), "RingBuffer capacity must be a power of two");
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            data: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
        }
    }

    /// Attempts to push a value into the ring buffer.
    /// Returns `Err(value)` if the buffer is full.
    pub fn push(&self, value: T) -> Result<(), T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head.wrapping_sub(tail) >= N {
            return Err(value);
        }
        
        unsafe {
            (*self.data[head & (N - 1)].get()).write(value);
        }
        
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }

    /// Attempts to pop a value from the ring buffer.
    /// Returns `None` if the buffer is empty.
    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        
        if head == tail {
            return None;
        }
        
        let value = unsafe {
            (*self.data[tail & (N - 1)].get()).assume_init_read()
        };
        
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(value)
    }
}

pub mod mpsc_queue;
pub use mpsc_queue::BoundedQueue;

pub mod bloom;
pub use bloom::SimpleBloom;
