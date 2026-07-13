//! Synchronization primitives for `no_std` environments.
//!
//! This module provides a complete set of synchronization primitives and atomic types
//! suitable for bare-metal, OS-less, or otherwise constrained environments where the
//! standard library is unavailable.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// Error indicating a spinlock timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutError;

/// A full suite of atomic types re-exported from `core::sync::atomic`.
pub use core::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI8, AtomicIsize, AtomicPtr, AtomicU16, AtomicU32,
    AtomicU8, AtomicUsize, Ordering,
};
pub use alloc::sync::{Arc, Weak};

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
///     let mut guard = mutex.lock().unwrap();
///     *guard += 1;
/// }
/// assert_eq!(*mutex.lock().unwrap(), 1);
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
    /// Acquires a mutex with a bounded spin limit to prevent infinite hangs.
    ///
    /// This function will spin the CPU up to a maximum number of cycles. If the lock
    /// cannot be acquired, it returns `Err(TimeoutError)`.
    pub fn lock(&self) -> Result<SpinMutexGuard<'_, T>, TimeoutError> {
        let mut spins = 0u32;
        loop {
            if spins >= 10_000 {
                return Err(TimeoutError);
            }
            spins += 1;

            // First Test (Test-and-Set)
            if self
                .locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(SpinMutexGuard { mutex: self });
            }

            // Second Test (Spin on relaxed load to prevent Cache Line Bouncing)
            while self.locked.load(Ordering::Relaxed) {
                if spins >= 10_000 {
                    return Err(TimeoutError);
                }
                core::hint::spin_loop();
                spins += 1;
            }
        }
    }
}

impl<T: ?Sized> Deref for SpinMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: The lock is held, ensuring exclusive access to the underlying data.
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The lock is held, ensuring exclusive access to the underlying data.
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

/// An Interrupt-Safe Spinlock Mutex for x86_64 OS environments.
///
/// This mutex will automatically disable interrupts (`cli`) upon acquiring the lock,
/// and restore the original interrupt flag state (via `RFLAGS`) upon releasing the lock.
/// This completely prevents deadlocks that could occur if an interrupt handler attempts
/// to acquire a lock already held by the interrupted execution context.
#[cfg(target_arch = "x86_64")]
pub struct IrqSafeMutex<T: ?Sized> {
    inner: SpinMutex<T>,
}

#[cfg(target_arch = "x86_64")]
unsafe impl<T: ?Sized + Send> Send for IrqSafeMutex<T> {}
#[cfg(target_arch = "x86_64")]
unsafe impl<T: ?Sized + Send> Sync for IrqSafeMutex<T> {}

#[cfg(target_arch = "x86_64")]
pub struct IrqSafeMutexGuard<'a, T: ?Sized> {
    inner_guard: core::mem::ManuallyDrop<SpinMutexGuard<'a, T>>,
    saved_rflags: u64,
}

#[cfg(target_arch = "x86_64")]
impl<T> IrqSafeMutex<T> {
    /// Creates a new interrupt-safe mutex.
    pub const fn new(val: T) -> Self {
        Self {
            inner: SpinMutex::new(val),
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl<T: ?Sized> IrqSafeMutex<T> {
    /// Acquires the lock safely by disabling interrupts and saving the prior state.
    /// Returns `Err(TimeoutError)` and restores interrupts if the lock acquisition times out.
    pub fn lock(&self) -> Result<IrqSafeMutexGuard<'_, T>, TimeoutError> {
        let mut rflags: u64;
        // SAFETY: Reading RFLAGS and disabling interrupts (cli) is safe in ring 0.
        unsafe {
            core::arch::asm!(
                "pushfq",
                "pop {0}",
                "cli",
                out(reg) rflags,
                options(nomem, preserves_flags)
            );
        }

        match self.inner.lock() {
            Ok(inner_guard) => Ok(IrqSafeMutexGuard {
                inner_guard: core::mem::ManuallyDrop::new(inner_guard),
                saved_rflags: rflags,
            }),
            Err(e) => {
                // Restore interrupts if they were previously enabled before returning error
                if (rflags & 0x200) != 0 {
                    // SAFETY: Restoring interrupts (sti) because they were originally enabled.
                    unsafe {
                        core::arch::asm!("sti", options(nomem, nostack));
                    }
                }
                Err(e)
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl<T: ?Sized> Deref for IrqSafeMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner_guard.deref()
    }
}

#[cfg(target_arch = "x86_64")]
impl<T: ?Sized> DerefMut for IrqSafeMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_guard.deref_mut()
    }
}

#[cfg(target_arch = "x86_64")]
impl<T: ?Sized> Drop for IrqSafeMutexGuard<'_, T> {
    fn drop(&mut self) {
        // Manually drop the inner spinlock guard first to release the lock.
        // This ensures the lock is released BEFORE we re-enable interrupts.
        // SAFETY: inner_guard is initialized and won't be accessed again after this.
        unsafe {
            core::mem::ManuallyDrop::drop(&mut self.inner_guard);
        }

        // Then restore interrupts if they were enabled (IF flag is bit 9, 0x200)
        if (self.saved_rflags & 0x200) != 0 {
            // SAFETY: Restoring original interrupt state safely.
            unsafe {
                core::arch::asm!("sti", options(nomem, nostack));
            }
        }
    }
}

/// A wait-free, single-element lock-free mailbox.
///
/// This primitive uses an `AtomicU32` under the hood to store state, allowing producers
/// to unconditionally `try_push` and consumers to `try_pop` without any spin-looping.
/// It uses `u32::MAX` as the sentinel for "empty".
pub struct AtomicMailboxU32 {
    state: core::sync::atomic::AtomicU32,
}

impl Default for AtomicMailboxU32 {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicMailboxU32 {
    /// Creates a new empty mailbox.
    pub const fn new() -> Self {
        Self {
            state: core::sync::atomic::AtomicU32::new(u32::MAX),
        }
    }

    /// Try to push a value. Fails and returns the value if the mailbox is already full.
    pub fn try_push(&self, data: u32) -> Result<(), u32> {
        let current = self.state.load(core::sync::atomic::Ordering::Acquire);
        if current != u32::MAX {
            return Err(data);
        }
        // Use compare_exchange in case of a race condition with another producer.
        self.state
            .compare_exchange(
                u32::MAX,
                data,
                core::sync::atomic::Ordering::Release,
                core::sync::atomic::Ordering::Relaxed,
            )
            .map(|_| ())
            .map_err(|_| data)
    }

    /// Try to pop the value. Returns None if empty.
    pub fn try_pop(&self) -> Option<u32> {
        let current = self
            .state
            .swap(u32::MAX, core::sync::atomic::Ordering::Acquire);
        if current == u32::MAX {
            None
        } else {
            Some(current)
        }
    }
}

/// A wrapper that pads the type to the size of a cache line (64 bytes).
/// This prevents false sharing between CPUs.
#[derive(Default, Debug, Clone, Copy)]
#[repr(align(64))]
pub struct CachePadded<T> {
    pub value: T,
}
