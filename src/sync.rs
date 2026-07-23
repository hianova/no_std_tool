#![doc = " Synchronization primitives for `no_std` environments."]
#![doc = ""]
#![doc = " This module provides a complete set of synchronization primitives and atomic types"]
#![doc = " suitable for bare-metal, OS-less, or otherwise constrained environments where the"]
#![doc = " standard library is unavailable."]
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
#[doc = " Error indicating a spinlock timeout."]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(64))]
pub struct TimeoutError;
pub use alloc::sync::{Arc, Weak};
#[doc = " Emits a CPU spin-loop hint to optimize power consumption and thread scheduling"]
#[doc = " during busy-wait loops."]
pub use core::hint::spin_loop;
#[doc = " 64-bit atomics are conditionally compiled based on target architecture support."]
#[cfg(target_has_atomic = "64")]
pub use core::sync::atomic::AtomicU64;
#[doc = " A full suite of atomic types re-exported from `core::sync::atomic`."]
pub use core::sync::atomic::{
    AtomicBool, AtomicI8, AtomicI16, AtomicI32, AtomicIsize, AtomicPtr, AtomicU8, AtomicU16,
    AtomicU32, AtomicUsize, Ordering,
};
#[doc = " A simple, lightweight spinlock mutex for `#![no_std]` environments."]
#[doc = ""]
#[doc = " `SpinMutex` relies purely on an `AtomicBool` and `core::hint::spin_loop()` to"]
#[doc = " achieve mutual exclusion without relying on OS-level thread blocking or context switching."]
#[doc = ""]
#[doc = " # Examples"]
#[doc = " ```"]
#[doc = " use no_std_tool::sync::SpinMutex;"]
#[doc = ""]
#[doc = " let mutex = SpinMutex::new(0);"]
#[doc = " {"]
#[doc = "     let mut guard = mutex.lock().unwrap();"]
#[doc = "     *guard += 1;"]
#[doc = " }"]
#[doc = " assert_eq!(*mutex.lock().unwrap(), 1);"]
#[doc = " ```"]
#[repr(C, align(64))]
pub struct SpinMutex<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}
unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}
#[doc = " An RAII implementation of a \"scoped lock\" of a `SpinMutex`."]
#[doc = " When this structure is dropped (falls out of scope), the lock will be unlocked."]
#[repr(C, align(64))]
pub struct SpinMutexGuard<'a, T: ?Sized> {
    mutex: &'a SpinMutex<T>,
}
impl<T> SpinMutex<T> {
    #[doc = " Creates a new spinlock in an unlocked state ready for use."]
    pub const fn new(val: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(val),
        }
    }
}
impl<T: ?Sized> SpinMutex<T> {
    #[doc = " Acquires a mutex with a bounded spin limit to prevent infinite hangs."]
    #[doc = ""]
    #[doc = " This function will spin the CPU up to a maximum number of cycles. If the lock"]
    #[doc = " cannot be acquired, it returns `Err(TimeoutError)`."]
    pub fn lock(&self) -> Result<SpinMutexGuard<'_, T>, TimeoutError> {
        let mut spins = 0u32;
        loop {
            if spins >= crate::covopt_param!("SPIN_MUTEX_LIMIT", 10_000u32, 100u32..=100_000u32) {
                return Err(TimeoutError);
            }
            spins += 1;
            if self
                .locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(SpinMutexGuard { mutex: self });
            }
            while self.locked.load(Ordering::Relaxed) {
                if spins >= crate::covopt_param!("SPIN_MUTEX_LIMIT", 10_000u32, 100u32..=100_000u32) {
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
#[doc = " A simple spin-then-yield backoff helper for tight polling loops."]
#[doc = ""]
#[doc = " `Backoff` is designed to be used inside loops that must wait for an external condition"]
#[doc = " without burning excessive CPU. It limits the number of spin-loop hints before signaling"]
#[doc = " to the caller that a more aggressive yielding strategy might be needed."]
#[repr(C, align(64))]
pub struct Backoff {
    spins: u32,
}
impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}
impl Backoff {
    #[doc = " Creates a new `Backoff` with the spin counter reset to zero."]
    pub fn new() -> Self {
        Self { spins: 0 }
    }
    #[doc = " Emits a CPU spin-loop hint and increments the internal spin counter."]
    pub fn snooze(&mut self) {
        core::hint::spin_loop();
        self.spins += 1;
    }
    #[doc = " Returns `true` when the spin budget has been exhausted."]
    pub fn is_completed(&self) -> bool {
        self.spins > 100
    }
}
#[doc = " An Interrupt-Safe Spinlock Mutex for x86_64 OS environments."]
#[doc = ""]
#[doc = " This mutex will automatically disable interrupts (`cli`) upon acquiring the lock,"]
#[doc = " and restore the original interrupt flag state (via `RFLAGS`) upon releasing the lock."]
#[doc = " This completely prevents deadlocks that could occur if an interrupt handler attempts"]
#[doc = " to acquire a lock already held by the interrupted execution context."]
#[cfg(target_arch = "x86_64")]
#[repr(C, align(64))]
pub struct IrqSafeMutex<T: ?Sized> {
    inner: SpinMutex<T>,
}
#[cfg(target_arch = "x86_64")]
unsafe impl<T: ?Sized + Send> Send for IrqSafeMutex<T> {}
#[cfg(target_arch = "x86_64")]
unsafe impl<T: ?Sized + Send> Sync for IrqSafeMutex<T> {}
#[cfg(target_arch = "x86_64")]
#[repr(C, align(64))]
pub struct IrqSafeMutexGuard<'a, T: ?Sized> {
    inner_guard: core::mem::ManuallyDrop<SpinMutexGuard<'a, T>>,
    saved_rflags: u64,
}
#[cfg(target_arch = "x86_64")]
impl<T> IrqSafeMutex<T> {
    #[doc = " Creates a new interrupt-safe mutex."]
    pub const fn new(val: T) -> Self {
        Self {
            inner: SpinMutex::new(val),
        }
    }
}
#[cfg(target_arch = "x86_64")]
impl<T: ?Sized> IrqSafeMutex<T> {
    #[doc = " Acquires the lock safely by disabling interrupts and saving the prior state."]
    #[doc = " Returns `Err(TimeoutError)` and restores interrupts if the lock acquisition times out."]
    pub fn lock(&self) -> Result<IrqSafeMutexGuard<'_, T>, TimeoutError> {
        let mut rflags: u64;
        unsafe {
            core :: arch :: asm ! ("pushfq" , "pop {0}" , "cli" , out (reg) rflags , options (nomem , preserves_flags));
        }
        match self.inner.lock() {
            Ok(inner_guard) => Ok(IrqSafeMutexGuard {
                inner_guard: core::mem::ManuallyDrop::new(inner_guard),
                saved_rflags: rflags,
            }),
            Err(e) => {
                if (rflags & 0x200) != 0 {
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
        unsafe {
            core::mem::ManuallyDrop::drop(&mut self.inner_guard);
        }
        if (self.saved_rflags & 0x200) != 0 {
            unsafe {
                core::arch::asm!("sti", options(nomem, nostack));
            }
        }
    }
}
#[doc = " A wait-free, single-element lock-free mailbox."]
#[doc = ""]
#[doc = " This primitive uses an `AtomicU32` under the hood to store state, allowing producers"]
#[doc = " to unconditionally `try_push` and consumers to `try_pop` without any spin-looping."]
#[doc = " It uses `u32::MAX` as the sentinel for \"empty\"."]
#[repr(C, align(64))]
pub struct AtomicMailboxU32 {
    state: core::sync::atomic::AtomicU32,
}
impl Default for AtomicMailboxU32 {
    fn default() -> Self {
        Self::new()
    }
}
impl AtomicMailboxU32 {
    #[doc = " Creates a new empty mailbox."]
    pub const fn new() -> Self {
        Self {
            state: core::sync::atomic::AtomicU32::new(u32::MAX),
        }
    }
    #[doc = " Try to push a value. Fails and returns the value if the mailbox is already full."]
    pub fn try_push(&self, data: u32) -> Result<(), u32> {
        let current = self.state.load(core::sync::atomic::Ordering::Acquire);
        if current != u32::MAX {
            return Err(data);
        }
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
    #[doc = " Try to pop the value. Returns None if empty."]
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
#[doc = " A wrapper that pads the type to the size of a cache line (64 bytes)."]
#[doc = " This prevents false sharing between CPUs."]
#[derive(Default, Debug, Clone, Copy)]
#[repr(align(64))]
pub struct CachePadded<T> {
    pub value: T,
}
