use core::ops::{Add, BitAnd, BitOr, Div, Mul, Rem, Shl, Shr, Sub};
use core::cmp::Ordering;
use core::fmt;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct U256(pub [u64; 4]);

impl U256 {
    pub const fn zero() -> Self {
        U256([0; 4])
    }

    pub const fn one() -> Self {
        U256([1, 0, 0, 0])
    }

    pub const fn from_u64(v: u64) -> Self {
        Self([v, 0, 0, 0])
    }

    /// Parse a hex string into a U256
    pub fn from_hex_str(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches("0x");
        if hex.is_empty() || hex.len() > 64 {
            return None;
        }
        let mut words = [0u64; 4];
        let mut chars = hex.chars().rev();
        for i in 0..4 {
            let mut word = 0u64;
            for j in 0..16 {
                if let Some(c) = chars.next() {
                    let val = c.to_digit(16)? as u64;
                    word |= val << (j * 4);
                } else {
                    break;
                }
            }
            words[i] = word;
        }
        Some(Self(words))
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0; 4]
    }

    pub fn is_even(&self) -> bool {
        self.0[0] & 1 == 0
    }
    
    pub fn leading_zeros(&self) -> u32 {
        let mut zeros = 0;
        for i in (0..4).rev() {
            if self.0[i] == 0 {
                zeros += 64;
            } else {
                zeros += self.0[i].leading_zeros();
                break;
            }
        }
        zeros
    }

    /// Add with carry out
    pub fn overflowing_add(self, other: U256) -> (U256, bool) {
        let mut res = [0u64; 4];
        let mut carry = 0u64;
        for i in 0..4 {
            let (sum1, overflow1) = self.0[i].overflowing_add(other.0[i]);
            let (sum2, overflow2) = sum1.overflowing_add(carry);
            res[i] = sum2;
            carry = (overflow1 as u64) | (overflow2 as u64);
        }
        (U256(res), carry > 0)
    }

    /// Sub with borrow out
    pub fn overflowing_sub(self, other: U256) -> (U256, bool) {
        let mut res = [0u64; 4];
        let mut borrow = 0u64;
        for i in 0..4 {
            let (sub1, overflow1) = self.0[i].overflowing_sub(other.0[i]);
            let (sub2, overflow2) = sub1.overflowing_sub(borrow);
            res[i] = sub2;
            borrow = (overflow1 as u64) | (overflow2 as u64);
        }
        (U256(res), borrow > 0)
    }

    /// Shift left by 1
    pub fn shl1(&self) -> U256 {
        let mut res = [0u64; 4];
        let mut carry = 0;
        for i in 0..4 {
            res[i] = (self.0[i] << 1) | carry;
            carry = self.0[i] >> 63;
        }
        U256(res)
    }

    /// Shift right by 1
    pub fn shr1(&self) -> U256 {
        let mut res = [0u64; 4];
        let mut carry = 0;
        for i in (0..4).rev() {
            res[i] = (self.0[i] >> 1) | carry;
            carry = self.0[i] << 63;
        }
        U256(res)
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Equal => continue,
                other_order => return other_order,
            }
        }
        Ordering::Equal
    }
}

impl Add for U256 {
    type Output = U256;
    fn add(self, other: U256) -> U256 {
        let (res, _) = self.overflowing_add(other);
        res
    }
}

impl Sub for U256 {
    type Output = U256;
    fn sub(self, other: U256) -> U256 {
        let (res, _) = self.overflowing_sub(other);
        res
    }
}

impl Mul for U256 {
    type Output = U256;
    fn mul(self, other: U256) -> U256 {
        let mut res = [0u64; 4];
        for i in 0..4 {
            let mut carry = 0u64;
            for j in 0..4 {
                if i + j >= 4 { break; }
                let (prod, overflow) = mul_u64(self.0[i], other.0[j]);
                let (sum1, overflow1) = res[i + j].overflowing_add(prod);
                let (sum2, overflow2) = sum1.overflowing_add(carry);
                res[i + j] = sum2;
                carry = overflow + (overflow1 as u64) + (overflow2 as u64);
            }
        }
        U256(res)
    }
}

pub const fn mul_u64(a: u64, b: u64) -> (u64, u64) {
    let res = (a as u128) * (b as u128);
    (res as u64, (res >> 64) as u64)
}

impl Div for U256 {
    type Output = U256;
    fn div(self, other: U256) -> U256 {
        let (q, _) = self.div_rem(other);
        q
    }
}

impl Rem for U256 {
    type Output = U256;
    fn rem(self, other: U256) -> U256 {
        let (_, r) = self.div_rem(other);
        r
    }
}

impl U256 {

    pub const fn mul_wide(self, other: U256) -> [u64; 8] {
        let mut res = [0u64; 8];
        let mut i = 0;
        while i < 4 {
            let mut carry = 0u64;
            let mut j = 0;
            while j < 4 {
                let (prod, overflow) = mul_u64(self.0[i], other.0[j]);
                let (sum1, overflow1) = res[i + j].overflowing_add(prod);
                let (sum2, overflow2) = sum1.overflowing_add(carry);
                res[i + j] = sum2;
                carry = overflow + (overflow1 as u64) + (overflow2 as u64);
                j += 1;
            }
            res[i + 4] = carry;
            i += 1;
        }
        res
    }

    pub const fn inv_word(&self) -> u64 {
        let n = self.0[0];
        let mut inv = n;
        inv = inv.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inv)));
        inv = inv.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inv)));
        inv = inv.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inv)));
        inv = inv.wrapping_mul(2u64.wrapping_sub(n.wrapping_mul(inv)));
        inv.wrapping_neg()
    }

    pub fn mont_reduce(mut t: [u64; 8], modulus: &U256, n0_inv: u64) -> U256 {
        for i in 0..4 {
            let m = t[i].wrapping_mul(n0_inv);
            let mut carry = 0u64;
            for j in 0..4 {
                let (prod, overflow) = mul_u64(m, modulus.0[j]);
                let (sum1, overflow1) = t[i + j].overflowing_add(prod);
                let (sum2, overflow2) = sum1.overflowing_add(carry);
                t[i + j] = sum2;
                carry = overflow + (overflow1 as u64) + (overflow2 as u64);
            }
            let mut j = i + 4;
            while carry > 0 && j < 8 {
                let (sum, overflow) = t[j].overflowing_add(carry);
                t[j] = sum;
                carry = overflow as u64;
                j += 1;
            }
        }
        let mut res = U256([t[4], t[5], t[6], t[7]]);
        if res >= *modulus {
            res = res - *modulus;
        }
        res
    }

    /// Zero-allocation Div/Rem using shift-and-subtract
    pub fn div_rem(self, other: U256) -> (U256, U256) {
        if other.is_zero() {
            panic!("Divide by zero in U256");
        }
        if self < other {
            return (U256::zero(), self);
        }
        
        let mut q = U256::zero();
        let mut r = U256::zero();
        
        let bits = 256 - self.leading_zeros();
        for i in (0..bits).rev() {
            r = r.shl1();
            
            // extract i-th bit from self and set it to LSB of r
            let bit = (self.0[(i / 64) as usize] >> (i % 64)) & 1;
            r.0[0] |= bit;
            
            if r >= other {
                r = r - other;
                q.0[(i / 64) as usize] |= 1 << (i % 64);
            }
        }
        (q, r)
    }
}

impl fmt::LowerHex for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut started = false;
        for i in (0..4).rev() {
            if !started {
                if self.0[i] != 0 || i == 0 {
                    write!(f, "{:x}", self.0[i])?;
                    started = true;
                }
            } else {
                write!(f, "{:016x}", self.0[i])?;
            }
        }
        Ok(())
    }
}
