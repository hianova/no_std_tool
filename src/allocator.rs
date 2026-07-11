use core::cell::UnsafeCell;

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
