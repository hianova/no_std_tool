pub mod types;
pub mod simd;

pub use types::{QuantType, Vec101SuperBlock, vec101_block, vec101_context};

#[doc = " Safe abstraction for processing a single row in GEMV mode"]
#[inline(always)]
pub fn process_row_gemv_safe(row: usize, ctx: &vec101_context) {
    #[cfg(target_arch = "x86_64")]
    unsafe { simd::avx2::process_row_avx2_gemv(row, ctx) };
    #[cfg(target_arch = "aarch64")]
    unsafe { simd::neon::process_row_neon_gemv(row, ctx) };
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    unsafe { simd::scalar::process_row_scalar_gemv(row, ctx) };
}

#[doc = " Safe abstraction for processing a single row in GEMM mode"]
#[inline(always)]
pub fn process_row_gemm_safe(
    row: usize, 
    ctx: &vec101_context, 
    x_t_ref: &[i8], 
    padded_batch: usize, 
    row_sums: &mut [i32]
) {
    #[cfg(target_arch = "x86_64")]
    unsafe { simd::avx2::process_row_avx2_gemm(row, ctx, x_t_ref, padded_batch, row_sums) };
    #[cfg(target_arch = "aarch64")]
    {
        // NEON implementation doesn't use x_t_ref and padded_batch directly in signature, it reads from ctx
        let _ = (x_t_ref, padded_batch); 
        unsafe { simd::neon::process_row_neon_gemm(row, ctx, row_sums) };
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    unsafe { simd::scalar::process_row_scalar_gemm(row, ctx, x_t_ref, padded_batch, row_sums) };
}
