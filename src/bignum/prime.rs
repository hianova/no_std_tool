use crate::bignum::u256::U256;

impl U256 {
/// Modular Exponentiation: (base ^ exp) % modulus
    pub fn mod_pow(base: U256, mut exp: U256, modulus: U256) -> U256 {
        if modulus.is_zero() {
            panic!("mod_pow: Modulo by zero");
        }
        if modulus == U256::one() {
            return U256::zero();
        }

        // Fallback for even modulus
        if modulus.is_even() {
            let mut res = U256::one();
            let mut b = base % modulus;
            while !exp.is_zero() {
                if !exp.is_even() {
                    res = (res * b) % modulus;
                }
                b = (b * b) % modulus;
                exp = exp.shr1();
            }
            return res;
        }

        // Montgomery Reduction for odd modulus
        let n0_inv = modulus.inv_word();
        
        let mut r = U256::one();
        for _ in 0..256 {
            r = r.shl1();
            if r >= modulus {
                r = r - modulus;
            }
        }
        
        let mut r2 = r;
        for _ in 0..256 {
            r2 = r2.shl1();
            if r2 >= modulus {
                r2 = r2 - modulus;
            }
        }

        let mut b_mont = U256::mont_reduce(base.mul_wide(r2), &modulus, n0_inv);
        let mut res_mont = r;

        while !exp.is_zero() {
            if !exp.is_even() {
                res_mont = U256::mont_reduce(res_mont.mul_wide(b_mont), &modulus, n0_inv);
            }
            b_mont = U256::mont_reduce(b_mont.mul_wide(b_mont), &modulus, n0_inv);
            exp = exp.shr1();
        }

        U256::mont_reduce(res_mont.mul_wide(U256::one()), &modulus, n0_inv)
    }

    /// Miller-Rabin Primality Test
    /// Returns true if the number is probably prime.
    pub fn is_probably_prime(&self, bases: &[u64]) -> bool {
        if self.is_zero() || *self == U256::one() {
            return false;
        }
        if *self == U256::from_u64(2) || *self == U256::from_u64(3) {
            return true;
        }
        if self.is_even() {
            return false;
        }

        // Write n-1 as d * 2^s
        let n_minus_1 = *self - U256::one();
        let mut s = 0;
        let mut d = n_minus_1;
        while d.is_even() {
            d = d.shr1();
            s += 1;
        }

        for &a_val in bases {
            let a = U256::from_u64(a_val);
            if a >= *self {
                continue;
            }
            
            let mut x = U256::mod_pow(a, d, *self);
            if x == U256::one() || x == n_minus_1 {
                continue;
            }

            let mut composite = true;
            for _ in 1..s {
                x = (x * x) % *self;
                if x == n_minus_1 {
                    composite = false;
                    break;
                }
            }

            if composite {
                return false;
            }
        }
        
        true
    }
}
