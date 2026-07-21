#![doc = " Debugging and lifecycle tracking utilities."]
#![doc = ""]
#![doc = " This module provides a global resource tracker to ensure that no memory leaks"]
#![doc = " occur and that all threads and background operations drop correctly in a"]
#![doc = " `#![no_std]` environment where memory sanitizers might not be available."]
#![doc = ""]
#![doc = " **Note for Aerospace-Grade `no_std` Use:**"]
#![doc = " To truly track dynamic memory allocations, your global allocator must be wrapped in `TrackingAllocator`."]
#![doc = " For thread tracking, your custom task scheduler must manually invoke `track_thread_spawn` and `track_thread_exit`."]
#![doc = " Relying solely on `ScopedResource` only provides scoped tracking and does not catch global leaks or untracked task terminations."]
use core::sync::atomic::{AtomicU32, Ordering};
static RESOURCE_COUNT: AtomicU32 = AtomicU32::new(0);
static THREAD_ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);
#[doc = " Tracks a single memory allocation."]
#[doc = " This should be called by your `GlobalAlloc` wrapper (e.g., `TrackingAllocator`)."]
#[inline]
pub fn track_alloc() {
    RESOURCE_COUNT.fetch_add(1, Ordering::SeqCst);
}
#[doc = " Tracks a single memory deallocation."]
#[doc = " This should be called by your `GlobalAlloc` wrapper (e.g., `TrackingAllocator`)."]
#[inline]
pub fn track_dealloc() {
    RESOURCE_COUNT.fetch_sub(1, Ordering::SeqCst);
}
#[doc = " Tracks the spawning of a new task or thread."]
#[doc = " This should be called by your custom RTOS or task scheduler."]
#[inline]
pub fn track_thread_spawn() {
    THREAD_ACTIVE_COUNT.fetch_add(1, Ordering::SeqCst);
}
#[doc = " Tracks the termination or dropping of a task or thread."]
#[doc = " This should be called by your custom RTOS or task scheduler."]
#[inline]
pub fn track_thread_exit() {
    THREAD_ACTIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
}
#[doc = " A scoped resource tracker to ensure everything is dropped correctly."]
#[doc = " This tracks a single resource block and also increments the thread active count."]
#[doc = " **Warning**: Do not rely on this alone for global memory safety! Use `TrackingAllocator`"]
#[doc = " to track true dynamic allocations."]
#[repr(C, align(64))]
pub struct ScopedResource;
impl ScopedResource {
    #[inline]
    pub fn new() -> Self {
        track_alloc();
        track_thread_spawn();
        Self
    }
}
impl Default for ScopedResource {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for ScopedResource {
    #[inline]
    fn drop(&mut self) {
        track_dealloc();
        track_thread_exit();
    }
}
#[doc = " Checks if there are any memory leaks (resources not dropped)."]
#[doc = " Returns true if there are no leaks."]
#[doc = " **Note**: This will only be accurate if you use `TrackingAllocator` as your global allocator."]
#[inline]
pub fn check_memory_leaks() -> bool {
    RESOURCE_COUNT.load(Ordering::SeqCst) == 0
}
#[doc = " Checks if all threads/operations have correctly dropped."]
#[doc = " Returns true if all are dropped."]
#[doc = " **Note**: This will only be accurate if your scheduler explicitly calls `track_thread_spawn` and `track_thread_exit`."]
#[inline]
pub fn check_thread_drops() -> bool {
    THREAD_ACTIVE_COUNT.load(Ordering::SeqCst) == 0
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_thread_tracking() {
        while !check_thread_drops() {
            track_thread_exit();
        }
        assert!(check_thread_drops());
        track_thread_spawn();
        assert!(!check_thread_drops());
        track_thread_exit();
        assert!(check_thread_drops());
    }
}
