#![doc = " Zero-Float Mathematical Engine for `no_std` environments."]
#![doc = ""]
#![doc = " This module provides pure-integer approximations of floating-point operations"]
#![doc = " (such as exponentials, inverse square roots, and activation functions) using"]
#![doc = " fixed-point arithmetic. It avoids linking to `libm` and prevents hardware FPU"]
#![doc = " traps in bare-metal targets."]
pub const FIXED_POINT_SHIFT: i32 = 16;
pub const FIXED_POINT_ONE: i32 = 1 << FIXED_POINT_SHIFT;
#[doc = " Approximates exp(x) where x is in Q16.16 fixed-point format."]
#[doc = " Returns exp(x) in Q16.16 format. Returns `None` on overflow."]
#[doc = " Uses the property: exp(x) = 2^(x * log2(e))"]
macro_rules! unlikely {
    ($ b : expr) => {
        $b
    };
}
pub fn exp_approx_q16(x: i32) -> Option<i32> {
    if unlikely!(x < -10 * FIXED_POINT_ONE) {
        return Some(0);
    }
    if unlikely!(x > 10 * FIXED_POINT_ONE) {
        return None;
    }
    let x_scaled = ((x as i64 * 94548) >> FIXED_POINT_SHIFT) as i32;
    let int_part = x_scaled >> FIXED_POINT_SHIFT;
    let frac_part = x_scaled & (FIXED_POINT_ONE - 1);
    let approx_frac = FIXED_POINT_ONE + frac_part;
    if int_part >= 0 {
        Some(approx_frac << int_part)
    } else {
        Some(approx_frac >> (-int_part))
    }
}
#[doc = " Approximates 1 / sqrt(x) where x is a standard u32 integer."]
#[doc = " Returns the result in Q16.16 fixed-point format. Returns `None` if x is 0."]
pub fn rsqrt_approx_i32(x: u32) -> Option<u32> {
    if x == 0 {
        return None;
    }
    let msb = 31 - x.leading_zeros();
    let sqrt_approx = 1 << (msb / 2);
    let one_q16 = 1u32 << 16;
    Some(one_q16 / sqrt_approx)
}
#[doc = " Approximates SiLU (Swish) activation: x * sigmoid(x) = x / (1 + exp(-x))"]
#[doc = " Expects standard i8 input and returns standard i8. Returns `None` on overflow."]
pub fn silu_approx_i8(x: i8) -> Option<i8> {
    
    let x_q16 = (x as i32) << FIXED_POINT_SHIFT;
    let exp_neg_x = exp_approx_q16(-x_q16)?;
    let denom = FIXED_POINT_ONE + exp_neg_x;
    let result = (x_q16 as i64 * FIXED_POINT_ONE as i64) / denom as i64;
    let res_i32 = (result >> FIXED_POINT_SHIFT) as i32;
    Some(res_i32.clamp(-128, 127) as i8)
}
