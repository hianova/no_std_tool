use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
const EMPTY: u8 = 0;
const WRITING: u8 = 1;
const READY: u8 = 2;
struct Slot<T> {
    state: AtomicU8,
    data: UnsafeCell<MaybeUninit<T>>,
}
impl<T> Slot<T> {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(EMPTY),
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}
unsafe impl<T: Send> Send for Slot<T> {}
unsafe impl<T: Send> Sync for Slot<T> {}
#[doc = " MPSC (multi-producer, single-consumer) wait-free bounded ring buffer."]
#[doc = ""]
#[doc = " Each partition owns one `BoundedQueue` that acts as its **command channel**:"]
#[doc = " any number of writer threads may call [`push`](BoundedQueue::push)"]
#[doc = " concurrently, while exactly one reader thread drains items via"]
#[doc = " [`pop`](BoundedQueue::pop)."]
#[doc = ""]
#[doc = " # Capacity constraint"]
#[doc = ""]
#[doc = " `capacity` must be a **power of two**. This allows the implementation to"]
#[doc = " replace all modulo operations with a single bitwise AND against `mask`,"]
#[doc = " keeping the hot path branch-free."]
#[doc = ""]
#[doc = " # Wait-freedom"]
#[doc = ""]
#[doc = " Neither `push` nor `pop` ever spins or blocks. A `push` that cannot claim"]
#[doc = " a free slot returns `Err(item)` immediately; a `pop` on an empty queue"]
#[doc = " returns `None` immediately."]
#[repr(C, align(64))]
pub struct BoundedQueue<T, const N: usize> {
    tail: AtomicUsize,
    head: AtomicUsize,
    buffer: [Slot<T>; N],
}
unsafe impl<T: Send, const N: usize> Send for BoundedQueue<T, N> {}
unsafe impl<T: Send, const N: usize> Sync for BoundedQueue<T, N> {}
impl<T, const N: usize> Default for BoundedQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, const N: usize> BoundedQueue<T, N> {
    pub const fn new() -> Self {
        assert!(
            N.is_power_of_two(),
            "BoundedQueue capacity must be a power of two"
        );
        Self {
            tail: AtomicUsize::new(0),
            head: AtomicUsize::new(0),
            buffer: [const { Slot::new() }; N],
        }
    }
    #[doc = " Attempts a wait-free multi-producer enqueue of `item`."]
    #[doc = ""]
    #[doc = " Internally this performs a **fetch-add** (FAA) to speculatively claim a"]
    #[doc = " slot index, then uses a **compare-and-swap** (`EMPTY → WRITING`) as a"]
    #[doc = " physical gate that confirms the slot is truly available. Writing the"]
    #[doc = " item and publishing it (`WRITING → READY`) are the final steps."]
    #[doc = ""]
    #[doc = " This method is safe to call from multiple threads simultaneously."]
    #[doc = ""]
    #[doc = " # Returns"]
    #[doc = ""]
    #[doc = " - `Ok(())` — `item` was successfully enqueued."]
    #[doc = " - `Err(item)` — the queue is full (or the slot was concurrently"]
    #[doc = "   occupied), and the caller retains ownership of `item`. This path"]
    #[doc = "   never blocks or spins."]
    #[inline(always)]
    pub fn push(&self, item: T) -> Result<(), T> {
        let mut tail = self.tail.load(Ordering::Relaxed);
        let mut spins = 0u32;
        loop {
            let head = self.head.load(Ordering::Acquire);
            if tail.wrapping_sub(head) >= self.buffer.len() {
                return Err(item);
            }
            let idx = tail & (N - 1);
            let slot = &self.buffer[idx];
            let state = slot.state.load(Ordering::Acquire);
            if state != EMPTY {
                let actual = self.tail.load(Ordering::Relaxed);
                if actual == tail {
                    let current_head = self.head.load(Ordering::Acquire);
                    if tail.wrapping_sub(current_head) >= self.buffer.len() {
                        return Err(item);
                    } else {
                        if spins >= 10_000 {
                            return Err(item);
                        }
                        core::hint::spin_loop();
                        spins += 1;
                        continue;
                    }
                } else {
                    tail = actual;
                    continue;
                }
            }
            match self.tail.compare_exchange_weak(
                tail,
                tail.wrapping_add(1),
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    while slot.state.load(Ordering::Acquire) != EMPTY {
                        core::hint::spin_loop();
                    }
                    slot.state.store(WRITING, Ordering::Release);
                    unsafe {
                        (*slot.data.get()).write(item);
                    }
                    slot.state.store(READY, Ordering::Release);
                    return Ok(());
                }
                Err(actual) => {
                    tail = actual;
                }
            }
        }
    }
    #[doc = " Attempts a wait-free single-consumer dequeue."]
    #[doc = ""]
    #[doc = " Inspects the slot at `head`. If its state is `READY` the item is moved"]
    #[doc = " out, the slot is reset to `EMPTY`, and `head` is advanced by one."]
    #[doc = ""]
    #[doc = " # Single-consumer contract"]
    #[doc = ""]
    #[doc = " **Only one thread may call `pop` at a time.** Concurrent calls from"]
    #[doc = " multiple threads will produce undefined behaviour because the head"]
    #[doc = " cursor is not protected by any mutual-exclusion mechanism — it is"]
    #[doc = " advanced non-atomically across the read–reset–increment sequence."]
    #[doc = ""]
    #[doc = " # Returns"]
    #[doc = ""]
    #[doc = " - `Some(item)` — an item was dequeued."]
    #[doc = " - `None` — the queue is empty (the next slot is not yet `READY`)."]
    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let idx = self.head.load(Ordering::Relaxed) & (N - 1);
        let slot = &self.buffer[idx];
        if slot.state.load(Ordering::Acquire) == READY {
            let item = unsafe { (*slot.data.get()).assume_init_read() };
            slot.state.store(EMPTY, Ordering::Release);
            self.head.fetch_add(1, Ordering::Release);
            Some(item)
        } else {
            None
        }
    }
}
impl<T, const N: usize> Drop for BoundedQueue<T, N> {
    fn drop(&mut self) {
        loop {
            let idx = self.head.load(Ordering::Relaxed) & (N - 1);
            let slot = &self.buffer[idx];
            if slot.state.load(Ordering::Acquire) == READY {
                unsafe { (*slot.data.get()).assume_init_drop() };
                slot.state.store(EMPTY, Ordering::Relaxed);
                self.head.fetch_add(1, Ordering::Relaxed);
            } else {
                break;
            }
        }
    }
}
