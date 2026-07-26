#![deny(unsafe_op_in_unsafe_fn)]

use crate::covopt_param;
#[cfg(target_arch = "x86_64")]
use crate::vec101_compute::types::Vector101__Computation__Context;
extern crate alloc;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn expand_bits_to_mask(w_32: u32) -> __m256i {
    let mut mask_arr = [0u8; 32];
    for b in 0..32 {
        if (w_32 & (1 << b)) != 0 {
            mask_arr[b] = 255;
        } else {
            mask_arr[b] = 0x00;
        }
    }
    unsafe { _mm256_loadu_si256(mask_arr.as_ptr() as *const __m256i) }
}
#[cfg(target_arch = "x86_64")]
pub unsafe fn process_row_avx2_gemv(row: usize, context: &Vector101__Computation__Context, x_mask: &[u64]) {
    if context.blocks_per_row == 0 {
        return;
    }
    match context.quant_type {
        crate::vec101_compute::types::QuantType::Bit1_58 => unsafe { process_row_avx2_gemv_bit1_58(row, context, x_mask) },
    }
}
#[cfg(target_arch = "x86_64")]
unsafe fn process_row_avx2_gemv_bit1_58(row: usize, context: &Vector101__Computation__Context, x_mask: &[u64]) {
    let scale = unsafe { *context.s_stream.add(row) };
    let mut final_sum = 0i32;
    let ones_u8 = unsafe { _mm256_set1_epi8(1) };
    let ones_i16 = unsafe { _mm256_set1_epi16(1) };
    for col in 0..context.blocks_per_row {
        let block_idx = row * context.blocks_per_row + col;
        let w_super = unsafe { &(*(context.w_stream as *const crate::vec101_compute::types::Vec101SuperBlock).add(block_idx)) };
        for sub_blk in 0..8 {
            let micro_scale = w_super.scales[sub_blk] as i32;
            let w_block = &w_super.blocks[sub_blk];
            let mut acc_pos = unsafe { _mm256_setzero_si256() };
            let mut acc_neg = unsafe { _mm256_setzero_si256() };
            for sub in 0..8 {
                let u64_idx = sub / 2;
                let shift_amt = (sub % 2) * 32;
                let w_pos_32 = (w_block.w_pos_bits[u64_idx] >> shift_amt) as u32;
                let w_neg_32 = (w_block.w_neg_bits[u64_idx] >> shift_amt) as u32;
                let x_ptr = unsafe { context.x_stream.add(col * 2048 + sub_blk * 256 + sub * 32) };
                let x_val = unsafe { _mm256_loadu_si256(x_ptr as *const __m256i) };
                let mask_pos = unsafe { expand_bits_to_mask(w_pos_32) };
                let mask_neg = unsafe { expand_bits_to_mask(w_neg_32) };
                let x_pos = unsafe { _mm256_and_si256(x_val, mask_pos) };
                let x_neg = unsafe { _mm256_and_si256(x_val, mask_neg) };
                let sum16_pos = unsafe { _mm256_maddubs_epi16(ones_u8, x_pos) };
                let sum32_pos = unsafe { _mm256_madd_epi16(sum16_pos, ones_i16) };
                acc_pos = unsafe { _mm256_add_epi32(acc_pos, sum32_pos) };
                let sum16_neg = unsafe { _mm256_maddubs_epi16(ones_u8, x_neg) };
                let sum32_neg = unsafe { _mm256_madd_epi16(sum16_neg, ones_i16) };
                acc_neg = unsafe { _mm256_add_epi32(acc_neg, sum32_neg) };
            }
            let mut sum_arr_pos = [0i32; 8];
            unsafe { _mm256_storeu_si256(sum_arr_pos.as_mut_ptr() as *mut __m256i, acc_pos); }
            let mut block_sum_pos = 0i32;
            for val in sum_arr_pos.iter() {
                block_sum_pos += val;
            }
            let mut sum_arr_neg = [0i32; 8];
            unsafe { _mm256_storeu_si256(sum_arr_neg.as_mut_ptr() as *mut __m256i, acc_neg); }
            let mut block_sum_neg = 0i32;
            for val in sum_arr_neg.iter() {
                block_sum_neg += val;
            }
            final_sum += ((block_sum_pos - block_sum_neg) * micro_scale) >> 8;
        }
    }
    let out_val = ((final_sum as i64 * scale as i64) >> 16) as i32;
    if context.enable_liquid {
        let tau = unsafe { *context.liquid_tau.add(row) };
        let state_ptr = unsafe { context.liquid_state.add(row) };
        let quantized = crate::vec101_compute::liquid_step_i8(out_val, context.dt, unsafe { &mut *state_ptr }, tau);
        unsafe { *context.liquid_out_buffer.add(row) = quantized; }
    } else {
        let out_ptr = unsafe { context.out_buffer.add(row) };
        unsafe { *out_ptr += out_val; }
    }
}
#[cfg(target_arch = "x86_64")]
pub unsafe fn process_row_avx2_gemm(
    row: usize,
    context: &Vector101__Computation__Context,
    x_t: &[i8],
    x_mask: &[u64],
    padded_batch: usize,
    row_sums: &mut [i32],
) {
    if context.blocks_per_row == 0 {
        return;
    }
    match context.quant_type {
        crate::vec101_compute::types::QuantType::Bit1_58 => unsafe {
            process_row_avx2_gemm_bit1_58(row, context, x_t, x_mask, padded_batch, row_sums)
        },
    }
}
#[cfg(target_arch = "x86_64")]
unsafe fn process_row_avx2_gemm_bit1_58(
    row: usize,
    context: &Vector101__Computation__Context,
    x_t: &[i8],
    x_mask: &[u64],
    padded_batch: usize,
    row_sums: &mut [i32],
) {
    let scale = unsafe { *context.s_stream.add(row) };
    let mut row_sums_int = alloc::vec![0i32; context.batch_size];
    for col in 0..context.blocks_per_row {
        let block_idx = row * context.blocks_per_row + col;
        let w_super = unsafe { &(*(context.w_stream as *const crate::vec101_compute::types::Vec101SuperBlock).add(block_idx)) };
        for sub_blk in 0..8 {
            let micro_scale = w_super.scales[sub_blk] as i32;
            let w_block = &w_super.blocks[sub_blk];
            row_sums.fill(0);
            let mask_base = col * 32 + sub_blk * 4;
            for sub in 0..4 {
                let mask = x_mask[mask_base + sub];
                let mut pos_bits = w_block.w_pos_bits[sub] & mask;
                while pos_bits != 0 {
                    let tz = pos_bits.trailing_zeros();
                    pos_bits &= pos_bits - 1;
                    let f = col * 2048 + sub_blk * 256 + sub * 64 + tz as usize;
                    for b in 0..context.batch_size {
                        row_sums[b] += x_t[f * padded_batch + b] as i32;
                    }
                }
                let mut neg_bits = w_block.w_neg_bits[sub] & mask;
                while neg_bits != 0 {
                    let tz = neg_bits.trailing_zeros();
                    neg_bits &= neg_bits - 1;
                    let f = col * 2048 + sub_blk * 256 + sub * 64 + tz as usize;
                    for b in 0..context.batch_size {
                        row_sums[b] -= x_t[f * padded_batch + b] as i32;
                    }
                }
            }
            for b in 0..context.batch_size {
                row_sums_int[b] += (row_sums[b] * micro_scale) >> 8;
            }
        }
    }
    for b in 0..context.batch_size {
        let out_val = ((row_sums_int[b] as i64 * scale as i64) >> 16) as i32;
        if context.enable_liquid {
            let tau = unsafe { *context.liquid_tau.add(row) };
            let state_ptr = unsafe { context.liquid_state.add(b * context.num_rows + row) };
            let quantized = crate::vec101_compute::liquid_step_i8(out_val, context.dt, unsafe { &mut *state_ptr }, tau);
            unsafe { *context.liquid_out_buffer.add(b * context.num_rows + row) = quantized; }
        } else {
            unsafe { *context.out_buffer.add(b * context.num_rows + row) += out_val; }
        }
    }
}

