use core::cell::UnsafeCell;
use core::alloc::{GlobalAlloc, Layout};
use crate::debug::{track_alloc, track_dealloc};

/// A simple Bump Allocator (Arena) for the Application Layer.
/// Allows applications to dynamically allocate memory from a wholesale physical frame
/// handed out by the OS's PMM.
pub struct AppArena {
    start: usize,
    end: usize,
    next: UnsafeCell<usize>,
}

unsafe impl Send for AppArena {}
unsafe impl Sync for AppArena {}

impl AppArena {
    pub const fn new(start: usize, size: usize) -> Self {
        AppArena {
            start,
            end: start + size,
            next: UnsafeCell::new(start),
        }
    }

    /// Allocates memory from the arena. Returns a raw pointer.
    pub fn alloc<T>(&self) -> Option<*mut T> {
        let size = core::mem::size_of::<T>();
        let align = core::mem::align_of::<T>();
        
        unsafe {
            let next_ptr = self.next.get();
            let mut current = *next_ptr;
            
            // Align the pointer
            let align_offset = current % align;
            if align_offset != 0 {
                current += align - align_offset;
            }
            
            if current + size > self.end {
                return None; // OOM in Arena
            }
            
            *next_ptr = current + size;
            Some(current as *mut T)
        }
    }

    /// Resets the arena, discarding all allocations.
    pub unsafe fn reset(&self) {
        unsafe {
            *self.next.get() = self.start;
        }
    }
}

/// A wrapper around a global allocator that automatically tracks memory allocations
/// for leak detection using `no_std_tool::debug`.
/// In an aerospace-grade `no_std` environment, use this as your `#[global_allocator]`.
pub struct TrackingAllocator<A> {
    inner: A,
}

impl<A> TrackingAllocator<A> {
    /// Creates a new `TrackingAllocator` wrapping the provided allocator.
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        track_alloc();
        unsafe { self.inner.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        track_dealloc();
        unsafe { self.inner.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        track_alloc();
        unsafe { self.inner.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // realloc does not change the number of active allocations
        unsafe { self.inner.realloc(ptr, layout, new_size) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::alloc::Layout;

    struct MockAllocator;

    unsafe impl GlobalAlloc for MockAllocator {
        unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
            1 as *mut u8
        }
        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
    }

    #[test]
    fn test_tracking_allocator() {
        use crate::debug::{check_memory_leaks, track_dealloc};

        // Reset state for isolation just in case
        while !check_memory_leaks() {
            track_dealloc();
        }

        let allocator = TrackingAllocator::new(MockAllocator);
        let layout = Layout::new::<u32>();

        assert!(check_memory_leaks());

        unsafe {
            let ptr = allocator.alloc(layout);
            assert!(!check_memory_leaks());
            allocator.dealloc(ptr, layout);
        }

        assert!(check_memory_leaks());
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlabHandle(u32);

impl SlabHandle {
    pub fn index(&self) -> usize {
        (self.0 & 0xFFFF) as usize
    }
    
    pub fn generation(&self) -> u16 {
        (self.0 >> 16) as u16
    }
    
    pub fn new(index: usize, generation: u16) -> Self {
        SlabHandle(((generation as u32) << 16) | (index as u32))
    }
}

pub struct StaticSlab<T, const N: usize> {
    mask: AtomicU32,
    generations: [AtomicU16; N],
    slots: [crate::sync::CachePadded<UnsafeCell<MaybeUninit<T>>>; N],
}

unsafe impl<T: Send, const N: usize> Sync for StaticSlab<T, N> {}

use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use core::mem::MaybeUninit;

impl<T, const N: usize> StaticSlab<T, N> {
    pub const fn new() -> Self {
        assert!(N <= 32, "StaticSlab max capacity is 32");
        StaticSlab {
            mask: AtomicU32::new(0),
            generations: [const { AtomicU16::new(0) }; N],
            slots: [const { crate::sync::CachePadded { value: UnsafeCell::new(MaybeUninit::uninit()) } }; N],
        }
    }
    
    pub fn allocate(&self, value: T) -> Result<SlabHandle, ()> {
        let mut current_mask = self.mask.load(Ordering::Relaxed);
        let max_mask = if N == 32 { u32::MAX } else { (1 << N) - 1 };
        
        loop {
            let effective_mask = current_mask | !max_mask;
            if effective_mask == u32::MAX {
                return Err(());
            }
            
            let index = effective_mask.trailing_ones() as usize;
            let new_mask = current_mask | (1 << index);
            
            match self.mask.compare_exchange_weak(
                current_mask,
                new_mask,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    unsafe {
                        (*self.slots[index].value.get()).write(value);
                    }
                    let gen_val = self.generations[index].load(Ordering::Relaxed);
                    return Ok(SlabHandle::new(index, gen_val));
                }
                Err(updated) => current_mask = updated,
            }
        }
    }
    
    pub fn free(&self, handle: SlabHandle) -> Result<(), ()> {
        let index = handle.index();
        if index >= N { return Err(()); }
        
        let gen_val = self.generations[index].load(Ordering::Acquire);
        if gen_val != handle.generation() {
            return Err(());
        }
        
        let prev = self.mask.fetch_and(!(1 << index), Ordering::Release);
        if (prev & (1 << index)) != 0 {
            unsafe {
                (*self.slots[index].value.get()).assume_init_drop();
            }
            self.generations[index].fetch_add(1, Ordering::Release);
            Ok(())
        } else {
            Err(())
        }
    }
    
    pub fn get(&self, handle: SlabHandle) -> Option<&T> {
        let index = handle.index();
        if index >= N { return None; }
        
        let gen_val = self.generations[index].load(Ordering::Acquire);
        if gen_val != handle.generation() {
            return None;
        }
        
        if self.mask.load(Ordering::Acquire) & (1 << index) != 0 {
            unsafe {
                Some((*self.slots[index].value.get()).assume_init_ref())
            }
        } else {
            None
        }
    }
    
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut_unchecked(&self, handle: SlabHandle) -> Option<&mut T> {
        let index = handle.index();
        if index >= N { return None; }
        
        let gen_val = self.generations[index].load(Ordering::Acquire);
        if gen_val != handle.generation() {
            return None;
        }
        
        if self.mask.load(Ordering::Acquire) & (1 << index) != 0 {
            unsafe {
                Some((*self.slots[index].value.get()).assume_init_mut())
            }
        } else {
            None
        }
    }
}

