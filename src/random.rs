use core::sync::atomic::{AtomicU64, Ordering};

/// A fast, zero-allocation Pseudo-Random Number Generator (PRNG) based on Xoshiro256**
/// It requires no external crates (`rand` or `ahash`) and operates purely on bitwise math.
pub struct Xoshiro256StarStar {
    s: [u64; 4],
}

impl Xoshiro256StarStar {
    /// Create a new PRNG with a specific seed. Seed must not be all zeros.
    pub fn new(seed: u64) -> Self {
        let mut rng = Self {
            s: [
                SplitMix64::next_seed(seed),
                SplitMix64::next_seed(seed.wrapping_add(1)),
                SplitMix64::next_seed(seed.wrapping_add(2)),
                SplitMix64::next_seed(seed.wrapping_add(3)),
            ],
        };
        // Ensure state is not entirely zero
        if rng.s == [0, 0, 0, 0] {
            rng.s[0] = 1;
        }
        rng
    }

    /// Automatically seed from environment entropy (No `rand` crate needed)
    /// Gathers entropy from Memory Layout (ASLR) and CPU counters if available.
    pub fn from_entropy() -> Self {
        let seed = gather_entropy();
        Self::new(seed)
    }

    /// Generate the next 64-bit random number
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    /// Generate a random boolean
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// A lightweight SplitMix64 generator used strictly for seeding Xoshiro
struct SplitMix64;
impl SplitMix64 {
    fn next_seed(mut state: u64) -> u64 {
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

/// Gathers entropy without any OS standard library
#[inline(always)]
fn gather_entropy() -> u64 {
    let mut entropy = 0u64;

    // 1. Stack ASLR Entropy (Pointer Address)
    // The OS randomizes the stack memory address at program launch
    let local_var = 0u8;
    entropy ^= (&local_var as *const u8 as u64).rotate_left(11);

    // 2. Static Memory ASLR Entropy
    static STATIC_VAR: AtomicU64 = AtomicU64::new(0);
    entropy ^= (&STATIC_VAR as *const AtomicU64 as u64).rotate_left(23);

    // 3. CPU Timestamp Counter (TSC) / Instruction Pointer Entropy
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            // Read CPU timestamp counter (extremely high frequency timer)
            let tsc = core::arch::x86_64::_rdtsc();
            entropy ^= tsc;
        }
    }

    // 4. Global Counter (To ensure multiple RNGs created in the same tick don't share seeds)
    entropy ^= STATIC_VAR.fetch_add(1, Ordering::Relaxed).wrapping_mul(0x123456789ABCDEF);

    entropy
}
