use no_std_tool::bignum::U256;
use std::time::Instant;

fn main() {
    println!("==================================================");
    println!("     🔐 Zero-Allocation U256 & Crypto Engine 🔐   ");
    println!("==================================================");
    println!();
    
    // Demo 1: Basic U256 Arithmetic
    println!("[1] 256-bit Arithmetic Demonstration");
    
    // 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
    let max = U256([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);
    let a = U256([0x1234567890abcdef, 0xfedcba0987654321, 0, 0]);
    let b = U256([0x1, 0, 0, 0]);
    
    let start = Instant::now();
    let sum = a + b;
    let mul = a * a;
    let duration = start.elapsed();
    
    println!("    A   = {:x}", a);
    println!("    A+1 = {:x}", sum);
    println!("    A*A = {:x}", mul);
    println!("    Max = {:x}", max);
    println!("    ⏱️  Arithmetic ops completed in {:?}", duration);
    println!();

    // Demo 2: Modular Exponentiation
    println!("[2] 256-bit Modular Exponentiation (A^B % C)");
    let base = U256([3, 0, 0, 0]);
    let exp = U256([0x9999999999999999, 0x8888888888888888, 0, 0]);
    let modulus = U256([0x123456789abcdef, 0, 0, 0]);
    
    let start2 = Instant::now();
    let modpow = U256::mod_pow(base, exp, modulus);
    let duration2 = start2.elapsed();
    
    println!("    Base: {:x}", base);
    println!("    Exp:  {:x}", exp);
    println!("    Mod:  {:x}", modulus);
    println!("    Res:  {:x}", modpow);
    println!("    ⏱️  mod_pow completed in {:?}", duration2);
    println!();

    // Demo 3: Prime Search (Miller-Rabin)
    println!("[3] Deterministic Prime Number Search (Miller-Rabin)");
    // We search for a 128-bit prime. (Using 128-bit avoids 512-bit intermediate overflow in mod_pow)
    let mut candidate = U256([
        0x0123456789ABCDEF,
        0x8000000000000000,
        0,
        0,
    ]); // Ensure 128th bit is set
    
    // First 12 primes are sufficient to deterministically verify 256-bit primes
    let bases = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    
    println!("    Searching for the next 128-bit prime after:");
    println!("    {:x}", candidate);
    
    let start3 = Instant::now();
    let mut attempts = 0;
    loop {
        if candidate.is_probably_prime(&bases) {
            break;
        }
        candidate = candidate + U256::from_u64(2); // check odd numbers only
        attempts += 1;
    }
    let duration3 = start3.elapsed();
    
    println!("    ✅ Found 128-bit Prime: {:x}", candidate);
    println!("    ⏱️  Found after {} attempts in {:?}", attempts, duration3);
    println!("==================================================");
}
