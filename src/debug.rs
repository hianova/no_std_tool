//! Debugging and lifecycle tracking utilities.
//!
//! This module provides a global resource tracker to ensure that no memory leaks
//! occur and that all threads and background operations drop correctly in a
//! `#![no_std]` environment where memory sanitizers might not be available.
//! 
//! **Note for Aerospace-Grade `no_std` Use:** 
//! To truly track dynamic memory allocations, your global allocator must be wrapped in `TrackingAllocator`.
//! For thread tracking, your custom task scheduler must manually invoke `track_thread_spawn` and `track_thread_exit`.
//! Relying solely on `ScopedResource` only provides scoped tracking and does not catch global leaks or untracked task terminations.

use core::sync::atomic::{AtomicU32, Ordering};

// Global counters for memory leaks and thread drop checks using AtomicU32
static RESOURCE_COUNT: AtomicU32 = AtomicU32::new(0);
static THREAD_ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Tracks a single memory allocation.
/// This should be called by your `GlobalAlloc` wrapper (e.g., `TrackingAllocator`).
#[inline]
pub fn track_alloc() {
    RESOURCE_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Tracks a single memory deallocation.
/// This should be called by your `GlobalAlloc` wrapper (e.g., `TrackingAllocator`).
#[inline]
pub fn track_dealloc() {
    RESOURCE_COUNT.fetch_sub(1, Ordering::SeqCst);
}

/// Tracks the spawning of a new task or thread.
/// This should be called by your custom RTOS or task scheduler.
#[inline]
pub fn track_thread_spawn() {
    THREAD_ACTIVE_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Tracks the termination or dropping of a task or thread.
/// This should be called by your custom RTOS or task scheduler.
#[inline]
pub fn track_thread_exit() {
    THREAD_ACTIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
}

/// A scoped resource tracker to ensure everything is dropped correctly.
/// This tracks a single resource block and also increments the thread active count.
/// **Warning**: Do not rely on this alone for global memory safety! Use `TrackingAllocator`
/// to track true dynamic allocations.
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

/// Checks if there are any memory leaks (resources not dropped).
/// Returns true if there are no leaks.
/// **Note**: This will only be accurate if you use `TrackingAllocator` as your global allocator.
#[inline]
pub fn check_memory_leaks() -> bool {
    RESOURCE_COUNT.load(Ordering::SeqCst) == 0
}

/// Checks if all threads/operations have correctly dropped.
/// Returns true if all are dropped.
/// **Note**: This will only be accurate if your scheduler explicitly calls `track_thread_spawn` and `track_thread_exit`.
#[inline]
pub fn check_thread_drops() -> bool {
    THREAD_ACTIVE_COUNT.load(Ordering::SeqCst) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_tracking() {
        // Reset state for isolation
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
