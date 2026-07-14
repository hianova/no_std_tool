use core::sync::atomic::{AtomicU64, Ordering};

/// An O(1) complexity task scheduler ready-queue using a 64-bit bitmap.
/// Leverages hardware `tzcnt` (trailing_zeros) for single-cycle task lookup.
pub struct BitmapScheduler {
    ready_queue: AtomicU64,
}

impl Default for BitmapScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl BitmapScheduler {
    pub const fn new() -> Self {
        BitmapScheduler {
            ready_queue: AtomicU64::new(0),
        }
    }

    /// Marks a task as ready to execute.
    pub fn set_ready(&self, task_id: usize) {
        if task_id < 64 {
            self.ready_queue.fetch_or(1 << task_id, Ordering::Release);
        }
    }

    /// Marks a task as parked (not ready).
    pub fn clear_ready(&self, task_id: usize) {
        if task_id < 64 {
            self.ready_queue
                .fetch_and(!(1 << task_id), Ordering::Release);
        }
    }

    /// Checks if a task is currently in the ready queue.
    pub fn is_ready(&self, task_id: usize) -> bool {
        if task_id < 64 {
            (self.ready_queue.load(Ordering::Acquire) & (1 << task_id)) != 0
        } else {
            false
        }
    }

    /// Finds the next ready task in O(1) time using hardware trailing_zeros.
    /// Implements Round-Robin by masking bits before the `current_task_id`.
    #[inline(always)]
    pub fn get_next_ready(&self, current_task_id: usize) -> Option<usize> {
        let bitmap = self.ready_queue.load(Ordering::Acquire);
        if bitmap == 0 {
            return None;
        }

        // We want to find the next task *after* current_task_id.
        // If current_task_id is 63, shl will overflow, so we handle it safely.
        let mask = if current_task_id >= 63 {
            0
        } else {
            !((1u64 << (current_task_id + 1)) - 1)
        };

        let masked = bitmap & mask;

        if masked != 0 {
            Some(masked.trailing_zeros() as usize)
        } else {
            // Wraparound: pick the lowest bit from the original bitmap
            Some(bitmap.trailing_zeros() as usize)
        }
    }

    /// Returns the number of currently ready tasks in O(1) using hardware popcnt.
    pub fn count_ready(&self) -> u32 {
        self.ready_queue.load(Ordering::Relaxed).count_ones()
    }
}
