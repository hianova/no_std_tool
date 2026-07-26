#![deny(unsafe_op_in_unsafe_fn)]

use crate::covopt_param;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
use crate::vec101_compute::types::Vector101__Computation__Context;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub unsafe fn process_row_scalar_gemv(row: usize, context: &Vector101__Computation__Context, x_mask: &[u64]) {
    if context.blocks_per_row == 0 {
        return;
    }
    match context.quant_type {
        crate::vec101_compute::types::QuantType::Bit1_58 => unsafe { process_row_scalar_gemv_bit1_58(row, context, x_mask) },
    }
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
unsafe fn process_row_scalar_gemv_bit1_58(row: usize, context: &Vector101__Computation__Context, x_mask: &[u64]) {
    let scale = unsafe { *context.s_stream.add(row) };
    let mut final_sum = 0i32;
    for col in 0..context.blocks_per_row {
        let block_idx = row * context.blocks_per_row + col;
        let w_super = unsafe { &(*(context.w_stream as *const crate::vec101_compute::types::Vec101SuperBlock).add(block_idx)) };
        for sub_blk in 0..8 {
            let micro_scale = w_super.scales[sub_blk] as i32;
            let w_block = &w_super.blocks[sub_blk];
            let mut micro_sum = 0i32;
            let mask_base = col * 32 + sub_blk * 4;
            for sub in 0..8 {
                let mask = x_mask[mask_base + sub / 2];
                let u64_idx = sub / 2;
                let shift_amt = (sub % 2) * 32;
                let w_pos_32 = ((w_block.w_pos_bits[u64_idx] & mask) >> shift_amt) as u32;
                let w_neg_32 = ((w_block.w_neg_bits[u64_idx] & mask) >> shift_amt) as u32;
                let x_ptr = unsafe { context.x_stream.add(col * 2048 + sub_blk * 256 + sub * 32) };
                for k in 0..32 {
                    let x_val = unsafe { *x_ptr.add(k) as i32 };
                    if (w_pos_32 & (1 << k)) != 0 {
                        micro_sum += x_val;
                    } else if (w_neg_32 & (1 << k)) != 0 {
                        micro_sum -= x_val;
                    }
                }
            }
            final_sum += (micro_sum * micro_scale) >> 8;
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
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub unsafe fn process_row_scalar_gemm(
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
            process_row_scalar_gemm_bit1_58(row, context, x_t, x_mask, padded_batch, row_sums)
        },
    }
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
unsafe fn process_row_scalar_gemm_bit1_58(
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
            let mask_base = col * 32 + sub_blk * 4;
            row_sums.fill(0);
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

