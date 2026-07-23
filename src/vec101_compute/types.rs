#[doc = " The fundamental compute block for vec101."]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct vec101_block {
    pub w_pos_bits: [u64; 4],
    pub w_neg_bits: [u64; 4],
}

#[doc = " 完美對齊 64-Byte，且維持 256 維度的終極設計！"]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vec101SuperBlock {
    pub scales: [i16; 8],
    pub offsets: [i16; 8],
    pub _padding: [u8; 32],
    pub blocks: [vec101_block; 8],
}

#[doc = " Supported quantization types for Dual Engine"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantType {
    Bit1_58,
}

#[doc = " The runtime context for the vec101 engine."]
#[repr(C)]
pub struct vec101_context {
    #[doc = " Quantization type of the current stream"]
    pub quant_type: QuantType,
    #[doc = " Highly compressed SuperBlocks stream or Q4_0 blocks stream (Zero-Copy Archived)"]
    pub w_stream: *const u8,
    #[doc = " Continuous activation values stream."]
    pub x_stream: *const i8,
    #[doc = " Quantization scaling factor stream per row."]
    pub s_stream: *const i32,
    #[doc = " Output buffer."]
    pub out_buffer: *mut i32,
    #[doc = " Pointers to Paged Attention KV blocks."]
    pub kv_blocks: *const *const i32,
    #[doc = " Number of valid blocks in the kv_blocks array."]
    pub num_blocks: usize,
    #[doc = " Number of tokens per block (e.g. 16 or 64)."]
    pub block_size: usize,
    #[doc = " Number of tokens processed simultaneously (GEMM Batch Dimension)"]
    pub batch_size: usize,
    #[doc = " Number of rows in the weight matrix"]
    pub num_rows: usize,
    #[doc = " Number of SuperBlocks per row"]
    pub blocks_per_row: usize,
    #[doc = " Number of parallel threads to use"]
    pub num_threads: usize,
    #[doc = " Tree mask for speculative decoding (1D array of parent indices)"]
    pub tree_mask: *const u32,
    #[doc = " Number of nodes in the speculative decoding tree"]
    pub tree_size: usize,
    #[doc = " Opaque pointer to the hardware backend (e.g., CudaDevice or Metal Device)"]
    #[doc = " The application layer is responsible for its allocation and lifecycle"]
    pub hardware_handle: *mut core::ffi::c_void,
    #[doc = " Enable Liquid Neural Network ODE integration fusion"]
    pub enable_liquid: bool,
    #[doc = " Liquid Time-Constant integration time delta (dt)"]
    pub dt: f32,
    #[doc = " Pointer to Liquid Neural Network states"]
    pub liquid_state: *mut f32,
    #[doc = " Pointer to Liquid Neural Network tau (time-constant) parameters"]
    pub liquid_tau: *const i32,
    #[doc = " Output buffer for quantized i8 states"]
    pub liquid_out_buffer: *mut i8,
    #[doc = " Pre-allocated scratch buffer for intermediate computations"]
    pub scratch_buffer: *mut u8,
    #[doc = " Size of the scratch buffer in bytes"]
    pub scratch_size: usize,
}
unsafe impl Send for vec101_context {}
unsafe impl Sync for vec101_context {}
