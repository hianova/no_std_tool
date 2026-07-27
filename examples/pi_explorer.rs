use std::io::{self, Write};
use std::time::Instant;

/// Modular exponentiation: (base^exp) % mod_val
fn mod_pow(mut base: f64, mut exp: i64, mod_val: f64) -> f64 {
    let mut res = 1.0;
    base = base % mod_val;
    while exp > 0 {
        if exp % 2 == 1 {
            res = (res * base) % mod_val;
        }
        base = (base * base) % mod_val;
        exp /= 2;
    }
    res
}

/// Calculates the fractional part of one term in the BBP formula
fn bbp_term(n: i64, j: i64) -> f64 {
    let mut sum = 0.0;
    
    // First sum: k = 0 to n
    for k in 0..=n {
        let r = 8 * k + j;
        let num = mod_pow(16.0, n - k, r as f64);
        sum = (sum + num / (r as f64)) % 1.0;
    }
    
    // Second sum: k = n+1 to infinity (converges quickly)
    let mut k = n + 1;
    loop {
        let r = 8 * k + j;
        let term = 16_f64.powi((n - k) as i32) / (r as f64);
        if term < 1e-15 {
            break;
        }
        sum = (sum + term) % 1.0;
        k += 1;
    }
    
    sum
}

/// BBP Algorithm: Returns the n-th hexadecimal digit of Pi (0-indexed after the decimal)
fn bbp_pi_hex_digit(n: i64) -> String {
    let p1 = bbp_term(n, 1);
    let p4 = bbp_term(n, 4);
    let p5 = bbp_term(n, 5);
    let p6 = bbp_term(n, 6);
    
    let mut pi_frac = (4.0 * p1 - 2.0 * p4 - p5 - p6) % 1.0;
    if pi_frac < 0.0 {
        pi_frac += 1.0;
    }
    
    let hex_val = (pi_frac * 16.0) as u32;
    format!("{:X}", hex_val)
}

fn explore_pi() {
    println!("==================================================");
    println!("              🥧 Pi (π) Explorer 🥧              ");
    println!("==================================================");
    
    println!("🏆 Current World Record (as of 2026): 314 Trillion Digits (StorageReview)\n");

    println!("[1] BBP Algorithm (Bailey–Borwein–Plouffe)");
    println!("    Dynamically calculating the N-th Hexadecimal digits of Pi without the preceding ones:");
    
    print!("    Hex digits from position 0 to 15: 3.");
    let start = Instant::now();
    for i in 0..16 {
        print!("{}", bbp_pi_hex_digit(i));
        io::stdout().flush().unwrap();
    }
    let duration = start.elapsed();
    println!("\n    ⏱️  Calculated in {:?}", duration);
    
    let deep_pos = 1_000_000;
    let start = Instant::now();
    let deep_digit = bbp_pi_hex_digit(deep_pos);
    let duration = start.elapsed();
    println!("    🔍 The 1,000,000th Hex digit of Pi is: {} (Calculated in {:?})", deep_digit, duration);
    println!();
}

fn explore_e() {
    println!("==================================================");
    println!("             🌿 Euler's Number (e) 🌿            ");
    println!("==================================================");
    
    println!("🏆 Current World Record: 31.4 Trillion Digits (y-cruncher, 2020)\n");
    
    println!("[1] Taylor Series Approximation: e = Σ (1 / n!)");
    let start = Instant::now();
    
    let mut e_approx = 1.0;
    let mut factorial = 1.0;
    
    for i in 1..=20 {
        factorial *= i as f64;
        e_approx += 1.0 / factorial;
    }
    let duration = start.elapsed();
    
    println!("    Approx (20 iterations): {:.15}", e_approx);
    println!("    Std Library f64::e:     {:.15}", std::f64::consts::E);
    println!("    ⏱️  Calculated in {:?}", duration);
    println!();
    
    println!("[2] Zero-Float Approximation (no_std_tool::math)");
    println!("    Using pure Q16.16 fixed-point arithmetic without FPU for bare-metal:");
    let start2 = Instant::now();
    
    // In Q16.16 format, 1.0 is represented as 1 << 16
    let fixed_point_one = 1 << 16;
    let e_fixed = no_std_tool::math::exp_approx_q16(fixed_point_one).unwrap();
    
    let duration2 = start2.elapsed();
    
    // Convert back to f64 just for display purposes
    let e_fixed_f64 = e_fixed as f64 / fixed_point_one as f64;
    
    println!("    Q16.16 Raw Binary Value: 0x{:08X}", e_fixed);
    println!("    Converted to Decimal:    {:.15}", e_fixed_f64);
    println!("    ⏱️  Calculated in {:?}", duration2);
    println!();
}

fn explore_phi() {
    println!("==================================================");
    println!("             ✨ Golden Ratio (φ) ✨             ");
    println!("==================================================");
    
    println!("[1] Continued Fraction & Fibonacci Approximation");
    let start = Instant::now();
    
    // Using Fibonacci sequence ratio
    let mut a: f64 = 0.0;
    let mut b: f64 = 1.0;
    for _ in 0..50 {
        let temp = a;
        a = b;
        b = temp + b;
    }
    let phi_approx = b / a;
    let duration = start.elapsed();
    
    let phi_actual = (1.0 + 5_f64.sqrt()) / 2.0;
    
    println!("    Approx (50th Fib ratio): {:.15}", phi_approx);
    println!("    Actual (1 + √5)/2:       {:.15}", phi_actual);
    println!("    ⏱️  Calculated in {:?}", duration);
    println!();
}


fn main() {
    explore_pi();
    explore_e();
    explore_phi();
}
