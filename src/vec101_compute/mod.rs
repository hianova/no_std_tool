pub mod types;
pub mod simd;

pub use types::{QuantType, Vec101SuperBlock, vec101_block, vec101_context};

#[doc = " Safe abstraction for processing a single row in GEMV mode"]
#[inline(always)]
pub fn process_row_gemv_safe(row: usize, ctx: &vec101_context, x_mask: &[u64]) {
    #[cfg(target_arch = "x86_64")]
    unsafe { simd::avx2::process_row_avx2_gemv(row, ctx, x_mask) };
    #[cfg(target_arch = "aarch64")]
    unsafe { simd::neon::process_row_neon_gemv(row, ctx, x_mask) };
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    unsafe { simd::scalar::process_row_scalar_gemv(row, ctx, x_mask) };
}

#[doc = " Safe abstraction for processing a single row in GEMM mode"]
#[inline(always)]
pub fn process_row_gemm_safe(
    row: usize, 
    ctx: &vec101_context, 
    x_t_ref: &[i8], 
    x_mask: &[u64],
    padded_batch: usize, 
    row_sums: &mut [i32]
) {
    #[cfg(target_arch = "x86_64")]
    unsafe { simd::avx2::process_row_avx2_gemm(row, ctx, x_t_ref, x_mask, padded_batch, row_sums) };
    #[cfg(target_arch = "aarch64")]
    unsafe { simd::neon::process_row_neon_gemm(row, ctx, x_t_ref, x_mask, padded_batch, row_sums) };
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    unsafe { simd::scalar::process_row_scalar_gemm(row, ctx, x_t_ref, x_mask, padded_batch, row_sums) };
}

#[doc = " Liquid Time-Constant (LTC) ODE integration step for Liquid Neural Networks."]
#[doc = " Integrates the `dot_product` with the current `state` using `tau_scaled` time constant."]
#[doc = " Returns the quantized INT8 activation."]
#[inline(always)]
pub fn liquid_step_i8(dot_product: i32, dt: f32, state: &mut f32, tau_scaled: i32) -> i8 {
    let input_f32 = dot_product as f32 / 128.0;
    let f_input = input_f32.abs();
    let dx_dt = -(1.0 / (tau_scaled as f32) + f_input) * (*state) + input_f32;
    *state += dx_dt * dt;
    let mut out = (*state * 127.0) as i32;
    out = out.clamp(-128, 127);
    out as i8
}
