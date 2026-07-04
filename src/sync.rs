//! Synchronization primitives for `no_std` environments.
//!
//! This module provides a complete set of synchronization primitives and atomic types
//! suitable for bare-metal, OS-less, or otherwise constrained environments where the
//! standard library is unavailable.

extern crate alloc;

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// A thread-safe reference-counting pointer.
pub use alloc::sync::Arc;

/// A full suite of atomic types re-exported from `core::sync::atomic`.
pub use core::sync::atomic::{
    AtomicBool, AtomicPtr, AtomicU8, AtomicU16, AtomicU32, AtomicUsize, 
    AtomicI8, AtomicI16, AtomicI32, AtomicIsize, Ordering
};

/// 64-bit atomics are conditionally compiled based on target architecture support.
#[cfg(target_has_atomic = "64")]
pub use core::sync::atomic::AtomicU64;

/// Emits a CPU spin-loop hint to optimize power consumption and thread scheduling
/// during busy-wait loops.
pub use core::hint::spin_loop;

/// A simple, lightweight spinlock mutex for `#![no_std]` environments.
///
/// `SpinMutex` relies purely on an `AtomicBool` and `core::hint::spin_loop()` to
/// achieve mutual exclusion without relying on OS-level thread blocking or context switching.
///
/// # Examples
/// ```
/// use no_std_tool::sync::SpinMutex;
/// 
/// let mutex = SpinMutex::new(0);
/// {
///     let mut guard = mutex.lock();
///     *guard += 1;
/// }
/// assert_eq!(*mutex.lock(), 1);
/// ```
pub struct SpinMutex<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}

/// An RAII implementation of a "scoped lock" of a `SpinMutex`.
/// When this structure is dropped (falls out of scope), the lock will be unlocked.
pub struct SpinMutexGuard<'a, T: ?Sized> {
    mutex: &'a SpinMutex<T>,
}

impl<T> SpinMutex<T> {
    /// Creates a new spinlock in an unlocked state ready for use.
    pub const fn new(val: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(val),
        }
    }
}

impl<T: ?Sized> SpinMutex<T> {
    /// Acquires a mutex, blocking the current thread until it is able to do so.
    ///
    /// This function will spin the CPU using `core::hint::spin_loop()` until the lock
    /// becomes available.
    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinMutexGuard { mutex: self }
    }
}

impl<T: ?Sized> Deref for SpinMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

/// A simple spin-then-yield backoff helper for tight polling loops.
///
/// `Backoff` is designed to be used inside loops that must wait for an external condition
/// without burning excessive CPU. It limits the number of spin-loop hints before signaling
/// to the caller that a more aggressive yielding strategy might be needed.
pub struct Backoff {
    spins: u32,
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

impl Backoff {
    /// Creates a new `Backoff` with the spin counter reset to zero.
    pub fn new() -> Self {
        Self { spins: 0 }
    }

    /// Emits a CPU spin-loop hint and increments the internal spin counter.
    pub fn snooze(&mut self) {
        core::hint::spin_loop();
        self.spins += 1;
    }

    /// Returns `true` when the spin budget has been exhausted.
    pub fn is_completed(&self) -> bool {
        self.spins > 100
    }
}
