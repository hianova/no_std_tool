#![deny(unsafe_op_in_unsafe_fn)]
#![allow(dead_code, unused_imports, unused_variables, unused_assignments, unused_mut, unreachable_code)]
use crate::vec101_compute::types::vec101_context;
#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn expand_bits_to_mask_neon(w_16: u16, bit_mask: uint8x16_t) -> int8x16_t {
    let lo = unsafe { vdup_n_u8(w_16 as u8) };
    let hi = unsafe { vdup_n_u8((w_16 >> 8) as u8) };
    let combined = unsafe { vcombine_u8(lo, hi) };
    unsafe { vreinterpretq_s8_u8(vtstq_u8(combined, bit_mask)) }
}
#[cfg(target_arch = "aarch64")]
#[doc = " # Safety"]
#[doc = " The caller must ensure that `row` is within bounds and `ctx` pointers are valid."]
pub unsafe fn process_row_neon_gemv(row: usize, ctx: &vec101_context, x_mask: &[u64]) {
    if ctx.blocks_per_row == 0 {
        return;
    }
    match ctx.quant_type {
        crate::vec101_compute::types::QuantType::Bit1_58 => unsafe { process_row_neon_gemv_bit1_58(row, ctx, x_mask) },
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn process_row_neon_gemv_bit1_58(row: usize, ctx: &vec101_context, x_mask: &[u64]) {
    let scale = unsafe { *ctx.s_stream.add(row) };
    let mut final_sum = 0i32;
    let bit_mask_arr: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];
    let bit_mask = unsafe { vld1q_u8(bit_mask_arr.as_ptr()) };
    for col in 0..ctx.blocks_per_row {
        let block_idx = row * ctx.blocks_per_row + col;
        let w_super = unsafe { &(*(ctx.w_stream as *const crate::vec101_compute::types::Vec101SuperBlock).add(block_idx)) };
        for sub_blk in 0..8 {
            let micro_scale = w_super.scales[sub_blk] as i32;
            let w_block = &w_super.blocks[sub_blk];
            let mut acc = unsafe { vdupq_n_s32(0) };
            for sub in 0..8 {
                let u64_idx = sub / 2;
                let shift_amt = (sub % 2) * 32;
                let w_pos_32 = (w_block.w_pos_bits[u64_idx] >> shift_amt) as u32;
                let w_neg_32 = (w_block.w_neg_bits[u64_idx] >> shift_amt) as u32;
                let mask_pos_lo = unsafe { expand_bits_to_mask_neon((w_pos_32 & 0xFFFF) as u16, bit_mask) };
                let mask_pos_hi = unsafe { expand_bits_to_mask_neon((w_pos_32 >> 16) as u16, bit_mask) };
                let mask_neg_lo = unsafe { expand_bits_to_mask_neon((w_neg_32 & 0xFFFF) as u16, bit_mask) };
                let mask_neg_hi = unsafe { expand_bits_to_mask_neon((w_neg_32 >> 16) as u16, bit_mask) };
                let w_vec_lo = unsafe { vsubq_s8(mask_neg_lo, mask_pos_lo) };
                let w_vec_hi = unsafe { vsubq_s8(mask_neg_hi, mask_pos_hi) };
                let x_ptr = unsafe { ctx.x_stream.add(col * 2048 + sub_blk * 256 + sub * 32) };
                let x_val_lo = unsafe { vld1q_s8(x_ptr) };
                let x_val_hi = unsafe { vld1q_s8(x_ptr.add(16)) };
                unsafe { core :: arch :: asm ! ("sdot {acc:v}.4s, {x:v}.16b, {w:v}.16b" , acc = inout (vreg) acc , x = in (vreg) x_val_lo , w = in (vreg) w_vec_lo ,) };
                unsafe { core :: arch :: asm ! ("sdot {acc:v}.4s, {x:v}.16b, {w:v}.16b" , acc = inout (vreg) acc , x = in (vreg) x_val_hi , w = in (vreg) w_vec_hi ,) };
            }
            let sum = unsafe { vaddvq_s32(acc) };
            final_sum += (sum * micro_scale) >> 8;
        }
    }
    let out_val = ((final_sum as i64 * scale as i64) >> 16) as i32;
    if ctx.enable_liquid {
        let tau = unsafe { *ctx.liquid_tau.add(row) };
        let state_ptr = unsafe { ctx.liquid_state.add(row) };
        let quantized = crate::vec101_compute::liquid_step_i8(out_val, ctx.dt, &mut unsafe { *state_ptr }, tau);
        unsafe { *ctx.liquid_out_buffer.add(row) = quantized; }
    } else {
        let out_ptr = unsafe { ctx.out_buffer.add(row) };
        unsafe { *out_ptr = (*out_ptr).saturating_add(out_val); }
    }
}
#[cold]
fn branch_unlikely() {}
#[cfg(target_arch = "aarch64")]
#[doc = " # Safety"]
#[doc = " The caller must ensure that `row` is within bounds and `ctx` pointers are valid."]
pub unsafe fn process_row_neon_gemm(row: usize, ctx: &vec101_context, x_t: &[i8], x_mask: &[u64], padded_batch: usize, row_sums: &mut [i32]) {
    if ctx.blocks_per_row == 0 {
        branch_unlikely();
        return;
    }
    match ctx.quant_type {
        crate::vec101_compute::types::QuantType::Bit1_58 => unsafe { process_row_neon_gemm_bit1_58(row, ctx, x_t, x_mask, padded_batch, row_sums) },
    }
}
#[cfg(target_arch = "aarch64")]
unsafe fn process_row_neon_gemm_bit1_58(row: usize, ctx: &vec101_context, x_t: &[i8], x_mask: &[u64], padded_batch: usize, row_sums: &mut [i32]) {
    let scale = unsafe { *ctx.s_stream.add(row) };
    let in_features = ctx.blocks_per_row * 2048;
    let bit_mask_arr: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];
    let bit_mask = unsafe { vld1q_u8(bit_mask_arr.as_ptr()) };
    #[repr(align(64))]
    struct CachePaddedArray([i32; 8]);
    row_sums[..ctx.batch_size].fill(0);
    for col in 0..ctx.blocks_per_row {
        let block_idx = row * ctx.blocks_per_row + col;
        let w_super = unsafe { &(*(ctx.w_stream as *const crate::vec101_compute::types::Vec101SuperBlock).add(block_idx)) };
        for sub_blk in 0..8 {
            let micro_scale = w_super.scales[sub_blk] as i32;
            let w_block = &w_super.blocks[sub_blk];
            let mut w_micro = [0i8; 256];
            for sub in 0..8 {
                let u64_idx = sub / 2;
                let shift_amt = (sub % 2) * 32;
                let w_pos_32 = (w_block.w_pos_bits[u64_idx] >> shift_amt) as u32;
                let w_neg_32 = (w_block.w_neg_bits[u64_idx] >> shift_amt) as u32;
                let mask_pos_lo = unsafe { expand_bits_to_mask_neon((w_pos_32 & 0xFFFF) as u16, bit_mask) };
                let mask_pos_hi = unsafe { expand_bits_to_mask_neon((w_pos_32 >> 16) as u16, bit_mask) };
                let mask_neg_lo = unsafe { expand_bits_to_mask_neon((w_neg_32 & 0xFFFF) as u16, bit_mask) };
                let mask_neg_hi = unsafe { expand_bits_to_mask_neon((w_neg_32 >> 16) as u16, bit_mask) };
                let w_vec_lo = unsafe { vsubq_s8(mask_neg_lo, mask_pos_lo) };
                let w_vec_hi = unsafe { vsubq_s8(mask_neg_hi, mask_pos_hi) };
                let offset = sub * 32;
                unsafe { vst1q_s8(w_micro.as_mut_ptr().add(offset), w_vec_lo) };
                unsafe { vst1q_s8(w_micro.as_mut_ptr().add(offset + 16), w_vec_hi) };
            }
            let mut b_idx = 0;
            while b_idx + 3 < ctx.batch_size {
                let ptr0 = unsafe { ctx.x_stream.add(b_idx * in_features) };
                let ptr1 = unsafe { ctx.x_stream.add((b_idx + 1) * in_features) };
                let ptr2 = unsafe { ctx.x_stream.add((b_idx + 2) * in_features) };
                let ptr3 = unsafe { ctx.x_stream.add((b_idx + 3) * in_features) };
                let mut acc0 = unsafe { vdupq_n_s32(0) };
                let mut acc1 = unsafe { vdupq_n_s32(0) };
                let mut acc2 = unsafe { vdupq_n_s32(0) };
                let mut acc3 = unsafe { vdupq_n_s32(0) };
                for chunk in 0..16 {
                    let offset = col * 2048 + sub_blk * 256 + chunk * 16;
                    let w_val = unsafe { vld1q_s8(w_micro.as_ptr().add(chunk * 16)) };
                    let x0 = unsafe { vld1q_s8(ptr0.add(offset)) };
                    let x1 = unsafe { vld1q_s8(ptr1.add(offset)) };
                    let x2 = unsafe { vld1q_s8(ptr2.add(offset)) };
                    let x3 = unsafe { vld1q_s8(ptr3.add(offset)) };
                    unsafe { core :: arch :: asm ! ("sdot {acc0:v}.4s, {x0:v}.16b, {w:v}.16b" , "sdot {acc1:v}.4s, {x1:v}.16b, {w:v}.16b" , "sdot {acc2:v}.4s, {x2:v}.16b, {w:v}.16b" , "sdot {acc3:v}.4s, {x3:v}.16b, {w:v}.16b" , acc0 = inout (vreg) acc0 , acc1 = inout (vreg) acc1 , acc2 = inout (vreg) acc2 , acc3 = inout (vreg) acc3 , x0 = in (vreg) x0 , x1 = in (vreg) x1 , x2 = in (vreg) x2 , x3 = in (vreg) x3 , w = in (vreg) w_val ,) };
                }
                row_sums[b_idx] += (unsafe { vaddvq_s32(acc0) } * micro_scale) >> 8;
                row_sums[b_idx + 1] += (unsafe { vaddvq_s32(acc1) } * micro_scale) >> 8;
                row_sums[b_idx + 2] += (unsafe { vaddvq_s32(acc2) } * micro_scale) >> 8;
                row_sums[b_idx + 3] += (unsafe { vaddvq_s32(acc3) } * micro_scale) >> 8;
                b_idx += 4;
            }
            while b_idx < ctx.batch_size {
                let x_batch_ptr = unsafe { ctx.x_stream.add(b_idx * in_features) };
                let mut acc = unsafe { vdupq_n_s32(0) };
                for chunk in 0..16 {
                    let offset = col * 2048 + sub_blk * 256 + chunk * 16;
                    let x_val = unsafe { vld1q_s8(x_batch_ptr.add(offset)) };
                    let w_val = unsafe { vld1q_s8(w_micro.as_ptr().add(chunk * 16)) };
                    unsafe { core :: arch :: asm ! ("sdot {acc:v}.4s, {x:v}.16b, {w:v}.16b" , acc = inout (vreg) acc , x = in (vreg) x_val , w = in (vreg) w_val ,) };
                }
                let sum = unsafe { vaddvq_s32(acc) };
                row_sums[b_idx] += (sum * micro_scale) >> 8;
                b_idx += 1;
            }
        }
    }
    for (b, &sum) in row_sums.iter().enumerate().take(ctx.batch_size) {
        unsafe { *ctx.out_buffer.add(b * ctx.num_rows + row) += ((sum as i64 * scale as i64) >> 16) as i32; }
    }
}
