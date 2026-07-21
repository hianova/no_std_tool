use crate::scriptgo_vm::instruction::Instruction;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde :: Serialize, serde :: Deserialize)]
#[repr(C, align(64))]
pub struct TraceStep {
    pub pc: u32,
    pub inst: u32,
    pub reg_change: Option<(u8, u32)>,
    pub mem_change: Option<(u16, u32)>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    DivideByZero { pc: usize },
    StackOverflow { pc: usize },
    StackUnderflow { pc: usize },
    InvalidOpcode { pc: usize, opcode: u8 },
    MemoryAccessOutOfBounds { pc: usize, addr: usize },
    MathError { pc: usize },
    OutOfFuel { pc: usize },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmResult {
    Halted(u32),
    Yielded(u32),
    Spawn(u32, u16, u8),
    Awaiting(u32, u32, u8),
    MmapRequest(u32, u32),
}
#[inline(always)]
fn likely(b: bool) -> bool {
    b
}
#[inline(always)]
fn unlikely(b: bool) -> bool {
    b
}
pub type UiHandler = fn(usize, usize, usize);
#[repr(align(64))]
pub struct ScriptVm {
    pub registers: [u32; 256],
    pub pc: usize,
    pub call_stack: [usize; 64],
    pub sp: usize,
    pub print_handler: Option<fn(u32)>,
    pub neural_handler: Option<fn(&mut ScriptVm, usize, usize, usize)>,
    pub syscall_handler: Option<fn(u32, u32, u32)>,
    pub ui_handler: Option<UiHandler>,
    pub hardware_handler: Option<fn(&mut ScriptVm, usize, usize, usize)>,
    pub abort_flag: Option<fn() -> bool>,
    pub debug_hook: Option<fn(&ScriptVm, Instruction)>,
    pub memory: [u8; 1024],
    pub mmap_ptr: usize,
    pub mmap_len: usize,
    pub max_steps: Option<u32>,
    pub tracing_enabled: bool,
    pub trace_buffer: [TraceStep; 1024],
    pub trace_head: usize,
    pub trace_count: usize,
    _tracker: crate::debug::ScopedResource,
}
impl Default for ScriptVm {
    fn default() -> Self {
        Self::new()
    }
}
impl ScriptVm {
    #[inline(always)]
    pub fn get_ptr(&self, addr: usize, len: usize) -> Result<*const u8, VmError> {
        if addr < 1024 {
            if addr + len <= 1024 {
                Ok(self.memory.as_ptr().wrapping_add(addr))
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else if addr >= 0x8000_0000 {
            let offset = addr - 0x8000_0000;
            if offset + len <= self.mmap_len {
                Ok((self.mmap_ptr + offset) as *const u8)
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else {
            Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
        }
    }

    #[inline(always)]
    pub fn get_mut_ptr(&mut self, addr: usize, len: usize) -> Result<*mut u8, VmError> {
        if addr < 1024 {
            if addr + len <= 1024 {
                Ok(self.memory.as_mut_ptr().wrapping_add(addr))
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else if addr >= 0x8000_0000 {
            let offset = addr - 0x8000_0000;
            if offset + len <= self.mmap_len {
                Ok((self.mmap_ptr + offset) as *mut u8)
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else {
            Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
        }
    }
    
    #[inline(always)]
    pub fn check_watchdog_timeout(&self, steps: u32) -> Result<(), VmError> {
        if let Some(limit) = self.max_steps
            && steps >= limit
        {
            return Err(VmError::OutOfFuel { pc: self.pc });
        }
        Ok(())
    }
    pub fn new() -> Self {
        Self {
            registers: [0; 256],
            pc: 0,
            call_stack: [0; 64],
            sp: 0,
            print_handler: None,
            neural_handler: None,
            syscall_handler: None,
            ui_handler: None,
            hardware_handler: None,
            abort_flag: None,
            debug_hook: None,
            memory: [0; 1024],
            mmap_ptr: 0,
            mmap_len: 0,
            max_steps: Some(10000),
            tracing_enabled: false,
            trace_buffer: [TraceStep {
                pc: 0,
                inst: 0,
                reg_change: None,
                mem_change: None,
            }; 1024],
            trace_head: 0,
            trace_count: 0,
            _tracker: crate::debug::ScopedResource::new(),
        }
    }
    #[doc = " Reset ephemeral execution context (PC, SP, call stack, R[0..16]) while preserving"]
    #[doc = " memory and persistent registers R[16..256] across code reloads (similar to React Fast Refresh)."]
    pub fn hot_reload(&mut self) {
        self.pc = 0;
        self.sp = 0;
        self.call_stack = [0; 64];
        for i in 0..16 {
            self.registers[i] = 0;
        }
    }
    #[doc = " Log a trace step to the circular trace buffer."]
    #[inline(always)]
    fn log_trace(
        &mut self,
        pc: u32,
        inst: u32,
        reg_change: Option<(u8, u32)>,
        mem_change: Option<(u16, u32)>,
    ) {
        if self.tracing_enabled {
            let step = TraceStep {
                pc,
                inst,
                reg_change,
                mem_change,
            };
            self.trace_buffer[self.trace_head] = step;
            self.trace_head = (self.trace_head + 1) % 1024;
            if self.trace_count < 1024 {
                self.trace_count += 1;
            }
        }
    }
    #[doc = " Run the VM execution loop."]
    #[doc = " Returns the number of instructions executed on success."]
    #[inline(always)]
    pub fn run(&mut self, code: &[Instruction]) -> Result<VmResult, VmError> {
        if self.debug_hook.is_none() && self.abort_flag.is_none() && !self.tracing_enabled {
            self.run_fast(code)
        } else {
            self.run_slow(code)
        }
    }
    #[inline(never)]
    pub fn step_count_helper(&self) {
        let _ = self.pc;
    }
    #[inline(always)]
    pub fn run_fast(&mut self, code: &[Instruction]) -> Result<VmResult, VmError> {
        // self.pc is intentionally NOT reset to 0 here to support resuming from Yield
        self.sp = 0;
        let mut steps = 0;
        let max_steps = self.max_steps.unwrap_or(u32::MAX);
        let poll_mask = crate::covopt_param!("watchdog_poll_mask", 0xFFu32, 0x01u32..=0xFFFu32);
        loop {
            if (steps & poll_mask) == 0 {
                if unlikely(steps >= max_steps) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
                if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
            }
            let inst = unsafe { *code.get_unchecked(self.pc) };
            self.pc += 1;
            steps += 1;
            let opcode = crate::opcode!(inst);
            match opcode {
                0 => break,
                1 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_b!(inst) as u32; }
                }
                2 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_imm16!(inst) as u32; }
                }
                3 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)]);
                }
                4 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_sub(self.registers[crate::inst_c!(inst)]);
                }
                5 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_mul(self.registers[crate::inst_c!(inst)]);
                }
                6 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] / divisor;
                }
                7 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] % divisor;
                }
                8 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val + c_val).to_bits();
                }
                9 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val - c_val).to_bits();
                }
                10 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val * c_val).to_bits();
                }
                11 => {
                    let divisor = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val / divisor).to_bits();
                }
                12 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] & self.registers[crate::inst_c!(inst)];
                }
                13 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] | self.registers[crate::inst_c!(inst)];
                }
                14 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] ^ self.registers[crate::inst_c!(inst)];
                }
                15 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        << self.registers[crate::inst_c!(inst)];
                }
                16 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        >> self.registers[crate::inst_c!(inst)];
                }
                17 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        == self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                18 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        < self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                19 => {
                    self.pc = crate::inst_imm16!(inst) as usize;
                }
                20 => {
                    if self.registers[crate::inst_a!(inst)] == 0 {
                        self.pc = crate::inst_b!(inst);
                    }
                }
                21 => {
                    if self.registers[crate::inst_a!(inst)] == self.registers[crate::inst_b!(inst)]
                    {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                22 => {
                    if self.registers[crate::inst_a!(inst)] < self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                23 => {
                    if self.registers[crate::inst_a!(inst)] > self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                24 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val < b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                25 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val > b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                26 => {
                    if self.sp < 64 {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = crate::inst_imm16!(inst) as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                28 => {
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[crate::inst_a!(inst)]);
                    }
                }
                29 => {
                    if let Some(handler) = self.syscall_handler {
                        handler(
                            self.registers[crate::inst_a!(inst)],
                            self.registers[crate::inst_b!(inst)],
                            self.registers[crate::inst_c!(inst)],
                        );
                    }
                }
                30 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_ptr(addr, 4)?;
                    let mut val = 0u32;
                    unsafe {
                        for i in 0..4 {
                            val |= (*ptr.add(i) as u32) << (i * 8);
                        }
                    }
                    self.registers[crate::inst_a!(inst)] = val;
                }
                31 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_mut_ptr(addr, 4)?;
                    let val = self.registers[crate::inst_a!(inst)];
                    unsafe {
                        for i in 0..4 {
                            *ptr.add(i) = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                    }
                }
                32 => {
                    let val = self.registers[crate::inst_b!(inst)] as i32;
                    if let Some(res) = crate::math::exp_approx_q16(val) {
                        self.registers[crate::inst_a!(inst)] = res as u32;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    let val = self.registers[crate::inst_b!(inst)];
                    if let Some(res) = crate::math::rsqrt_approx_i32(val) {
                        self.registers[crate::inst_a!(inst)] = res;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    let val = (self.registers[crate::inst_b!(inst)] & 0xFF) as i8;
                    if let Some(res) = crate::math::silu_approx_i8(val) {
                        self.registers[crate::inst_a!(inst)] = (res as u32) & 0xFF;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                35 => {
                    if let Some(handler) = self.hardware_handler {
                        handler(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                36 => {
                    let cmd = crate::inst_b!(inst);
                    if crate::inst_a!(inst) == 0 || !(1..=4).contains(&cmd) {
                    } else if let Some(handler) = self.ui_handler {
                        handler(
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                37 => {
                    let handler = self.neural_handler;
                    if let Some(h) = handler {
                        h(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                38 => {
                    return Ok(VmResult::Yielded(steps));
                }
                39 => { // VecAdd
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let dest_ptr = self.get_mut_ptr(dest, len * 4)?;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            let res = val1 + val2;
                            core::ptr::write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], res.to_le_bytes());
                        }
                    }
                }
                40 => { // VecMul
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let dest_ptr = self.get_mut_ptr(dest, len * 4)?;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            let res = val1 * val2;
                            core::ptr::write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], res.to_le_bytes());
                        }
                    }
                }
                41 => { // VecDot
                    let len = self.registers[0] as usize;
                    let dest_reg = crate::inst_a!(inst) as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        let mut sum = 0.0f32;
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            sum += val1 * val2;
                        }
                        self.registers[dest_reg] = sum.to_bits();
                    }
                }
                42 => { // Spawn
                    let target_pc = crate::inst_imm16!(inst) as usize;
                    return Ok(VmResult::Spawn(steps, target_pc as u16, crate::inst_a!(inst) as u8));
                }
                43 => { // Await
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::Awaiting(steps, resource_id, crate::inst_a!(inst) as u8));
                }
                44 => { // Mmap
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::MmapRequest(steps, resource_id));
                }
                _ => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode,
                    });
                }
            }
        }
        Ok(VmResult::Halted(steps))
    }
    #[inline(always)]
    pub fn run_fast_with<N>(&mut self, code: &[Instruction], mut neural_handler: N) -> Result<VmResult, VmError>
    where
        N: FnMut(&mut ScriptVm, usize, usize, usize),
    {
        // self.pc is NOT reset to 0 to support Yield
        self.sp = 0;
        let mut steps = 0;
        let max_steps = self.max_steps.unwrap_or(u32::MAX);
        let poll_mask = crate::covopt_param!("watchdog_poll_mask", 0xFFu32, 0x01u32..=0xFFFu32);
        loop {
            if (steps & poll_mask) == 0 {
                if unlikely(steps >= max_steps) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
                if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
            }
            let inst = unsafe { *code.get_unchecked(self.pc) };
            self.pc += 1;
            steps += 1;
            let opcode = crate::opcode!(inst);
            match opcode {
                0 => break,
                1 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_b!(inst) as u32; }
                }
                2 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_imm16!(inst) as u32; }
                }
                3 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)]);
                }
                4 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_sub(self.registers[crate::inst_c!(inst)]);
                }
                5 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_mul(self.registers[crate::inst_c!(inst)]);
                }
                6 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] / divisor;
                }
                7 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] % divisor;
                }
                8 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val + c_val).to_bits();
                }
                9 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val - c_val).to_bits();
                }
                10 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val * c_val).to_bits();
                }
                11 => {
                    let divisor = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val / divisor).to_bits();
                }
                12 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] & self.registers[crate::inst_c!(inst)];
                }
                13 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] | self.registers[crate::inst_c!(inst)];
                }
                14 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] ^ self.registers[crate::inst_c!(inst)];
                }
                15 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        << self.registers[crate::inst_c!(inst)];
                }
                16 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        >> self.registers[crate::inst_c!(inst)];
                }
                17 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        == self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                18 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        < self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                19 => {
                    self.pc = crate::inst_imm16!(inst) as usize;
                }
                20 => {
                    if self.registers[crate::inst_a!(inst)] == 0 {
                        self.pc = crate::inst_b!(inst);
                    }
                }
                21 => {
                    if self.registers[crate::inst_a!(inst)] == self.registers[crate::inst_b!(inst)]
                    {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                22 => {
                    if self.registers[crate::inst_a!(inst)] < self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                23 => {
                    if self.registers[crate::inst_a!(inst)] > self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                24 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val < b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                25 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val > b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                26 => {
                    if self.sp < 64 {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = crate::inst_imm16!(inst) as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                28 => {
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[crate::inst_a!(inst)]);
                    }
                }
                29 => {
                    if let Some(handler) = self.syscall_handler {
                        handler(
                            self.registers[crate::inst_a!(inst)],
                            self.registers[crate::inst_b!(inst)],
                            self.registers[crate::inst_c!(inst)],
                        );
                    }
                }
                30 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_ptr(addr, 4)?;
                    let mut val = 0u32;
                    unsafe {
                        for i in 0..4 {
                            val |= (*ptr.add(i) as u32) << (i * 8);
                        }
                    }
                    self.registers[crate::inst_a!(inst)] = val;
                }
                31 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_mut_ptr(addr, 4)?;
                    let val = self.registers[crate::inst_a!(inst)];
                    unsafe {
                        for i in 0..4 {
                            *ptr.add(i) = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                    }
                }
                32 => {
                    let val = self.registers[crate::inst_b!(inst)] as i32;
                    if let Some(res) = crate::math::exp_approx_q16(val) {
                        self.registers[crate::inst_a!(inst)] = res as u32;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    let val = self.registers[crate::inst_b!(inst)];
                    if let Some(res) = crate::math::rsqrt_approx_i32(val) {
                        self.registers[crate::inst_a!(inst)] = res;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    let val = (self.registers[crate::inst_b!(inst)] & 0xFF) as i8;
                    if let Some(res) = crate::math::silu_approx_i8(val) {
                        self.registers[crate::inst_a!(inst)] = (res as u32) & 0xFF;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                35 => {
                    if let Some(handler) = self.hardware_handler {
                        handler(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                36 => {
                    let cmd = crate::inst_b!(inst);
                    if crate::inst_a!(inst) == 0 || !(1..=4).contains(&cmd) {
                    } else if let Some(handler) = self.ui_handler {
                        handler(
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                37 => {
                    neural_handler(
                        self,
                        crate::inst_a!(inst),
                        crate::inst_b!(inst),
                        crate::inst_c!(inst),
                    );
                }
                38 => {
                    return Ok(VmResult::Yielded(steps));
                }
                39 => { // VecAdd
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let dest_ptr = self.get_mut_ptr(dest, len * 4)?;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            let res = val1 + val2;
                            core::ptr::write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], res.to_le_bytes());
                        }
                    }
                }
                40 => { // VecMul
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let dest_ptr = self.get_mut_ptr(dest, len * 4)?;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            let res = val1 * val2;
                            core::ptr::write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], res.to_le_bytes());
                        }
                    }
                }
                41 => { // VecDot
                    let len = self.registers[0] as usize;
                    let dest_reg = crate::inst_a!(inst) as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        let mut sum = 0.0f32;
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            sum += val1 * val2;
                        }
                        self.registers[dest_reg] = sum.to_bits();
                    }
                }
                42 => { // Spawn
                    let target_pc = crate::inst_imm16!(inst) as usize;
                    return Ok(VmResult::Spawn(steps, target_pc as u16, crate::inst_a!(inst) as u8));
                }
                43 => { // Await
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::Awaiting(steps, resource_id, crate::inst_a!(inst) as u8));
                }
                44 => { // Mmap
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::MmapRequest(steps, resource_id));
                }
                _ => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode,
                    });
                }
            }
        }
        Ok(VmResult::Halted(steps))
    }

    #[inline(never)]
    fn run_slow(&mut self, code: &[Instruction]) -> Result<VmResult, VmError> {
        // self.pc is NOT reset to 0 to support Yield
        self.sp = 0;
        let mut steps = 0;
        while likely(self.pc < code.len()) {
            if let Some(abort) = self.abort_flag
                && unlikely(abort())
            {
                break;
            }
            if unlikely(self.max_steps.is_some() && steps >= self.max_steps.unwrap_or(u32::MAX)) {
                return Err(VmError::OutOfFuel { pc: self.pc });
            }
            if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                return Err(VmError::OutOfFuel { pc: self.pc });
            }
            let current_pc = self.pc as u32;
            let inst = code[self.pc];
            if let Some(hook) = self.debug_hook {
                hook(self, inst);
            }
            self.pc += 1;
            steps += 1;
            let mut reg_change = None;
            let mut mem_change = None;
            match crate::opcode!(inst) {
                0 => break,
                1 => {
                    let val = crate::inst_b!(inst) as u32;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                2 => {
                    let val = crate::inst_imm16!(inst) as u32;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                3 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                4 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        .wrapping_sub(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                5 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        .wrapping_mul(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                6 => {
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[crate::inst_b!(inst)] / divisor;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                7 => {
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[crate::inst_b!(inst)] % divisor;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                8 => {
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    let val = (b + c).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                9 => {
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    let val = (b - c).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                10 => {
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    let val = (b * c).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                11 => {
                    let divisor = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let val = (b / divisor).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                12 => {
                    let val =
                        self.registers[crate::inst_b!(inst)] & self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                13 => {
                    let val =
                        self.registers[crate::inst_b!(inst)] | self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                14 => {
                    let val =
                        self.registers[crate::inst_b!(inst)] ^ self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                15 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        << self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                16 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        >> self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                19 => self.pc = crate::inst_imm16!(inst) as usize,
                20 => {
                    if self.registers[crate::inst_a!(inst)] == 0 {
                        self.pc = crate::inst_imm16!(inst) as usize;
                    }
                }
                21 => {
                    if self.registers[crate::inst_a!(inst)] == self.registers[crate::inst_b!(inst)]
                    {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                22 => {
                    if self.registers[crate::inst_a!(inst)] < self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                23 => {
                    if self.registers[crate::inst_a!(inst)] > self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                24 => {
                    let a = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a < b {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                25 => {
                    let a = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a > b {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                26 => {
                    if self.sp < 64 {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = crate::inst_imm16!(inst) as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                28 => {
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[crate::inst_a!(inst)]);
                    }
                }
                29 => {
                    if let Some(handler) = self.syscall_handler {
                        handler(
                            self.registers[crate::inst_a!(inst)],
                            self.registers[crate::inst_b!(inst)],
                            self.registers[crate::inst_c!(inst)],
                        );
                    }
                }
                30 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_ptr(addr, 4)?;
                    let mut val = 0u32;
                    unsafe {
                        for i in 0..4 {
                            val |= (*ptr.add(i) as u32) << (i * 8);
                        }
                    }
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                31 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_mut_ptr(addr, 4)?;
                    let val = self.registers[crate::inst_a!(inst)];
                    unsafe {
                        for i in 0..4 {
                            *ptr.add(i) = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                    }
                    mem_change = Some((addr as u16, val));
                }
                32 => {
                    let val = self.registers[crate::inst_b!(inst)] as i32;
                    if let Some(res) = crate::math::exp_approx_q16(val) {
                        let val_u32 = res as u32;
                        self.registers[crate::inst_a!(inst)] = val_u32;
                        reg_change = Some((crate::inst_a!(inst) as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    let val = self.registers[crate::inst_b!(inst)];
                    if let Some(res) = crate::math::rsqrt_approx_i32(val) {
                        self.registers[crate::inst_a!(inst)] = res;
                        reg_change = Some((crate::inst_a!(inst) as u8, res));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    let val = (self.registers[crate::inst_b!(inst)] & 0xFF) as i8;
                    if let Some(res) = crate::math::silu_approx_i8(val) {
                        let val_u32 = (res as u32) & 0xFF;
                        self.registers[crate::inst_a!(inst)] = val_u32;
                        reg_change = Some((crate::inst_a!(inst) as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                37 => {
                    let handler = self.neural_handler;
                    if let Some(h) = handler {
                        h(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                35 => {
                    let handler = self.hardware_handler;
                    if let Some(h) = handler {
                        h(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                36 => {
                    let cmd = crate::inst_b!(inst);
                    if crate::inst_a!(inst) == 0 || !(1..=4).contains(&cmd) {
                    } else if let Some(handler) = self.ui_handler {
                        handler(
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                38 => {
                    self.log_trace(current_pc, inst.0, reg_change, mem_change);
                    return Ok(VmResult::Yielded(steps));
                }
                39 => { // VecAdd
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let dest_ptr = self.get_mut_ptr(dest, len * 4)?;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            let res = val1 + val2;
                            core::ptr::write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], res.to_le_bytes());
                        }
                    }
                    mem_change = Some((dest as u16, len as u32));
                }
                40 => { // VecMul
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let dest_ptr = self.get_mut_ptr(dest, len * 4)?;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            let res = val1 * val2;
                            core::ptr::write_unaligned(dest_ptr.add(i * 4) as *mut [u8; 4], res.to_le_bytes());
                        }
                    }
                    mem_change = Some((dest as u16, len as u32));
                }
                41 => { // VecDot
                    let len = self.registers[0] as usize;
                    let dest_reg = crate::inst_a!(inst) as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let src1_ptr = self.get_ptr(src1, len * 4)?;
                    let src2_ptr = self.get_ptr(src2, len * 4)?;
                    unsafe {
                        let mut sum = 0.0f32;
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * 4) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * 4) as *const [u8; 4]));
                            sum += val1 * val2;
                        }
                        self.registers[dest_reg] = sum.to_bits();
                        reg_change = Some((dest_reg as u8, sum.to_bits()));
                    }
                }
                42 => { // Spawn
                    let target_pc = crate::inst_imm16!(inst);
                    return Ok(VmResult::Spawn(steps, target_pc, crate::inst_a!(inst) as u8));
                }
                43 => { // Await
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::Awaiting(steps, resource_id, crate::inst_a!(inst) as u8));
                }
                44 => { // Mmap
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::MmapRequest(steps, resource_id));
                }
                op => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode: op,
                    });
                }
            }
            self.log_trace(current_pc, inst.0, reg_change, mem_change);
        }
        Ok(VmResult::Halted(steps))
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::scriptgo_vm::instruction::OpCode;
    #[test]
    fn test_div_by_zero() {
        let mut vm = ScriptVm::new();
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, 3, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::DivideByZero { pc: 2 }));
    }
    #[test]
    fn test_stack_overflow() {
        let mut vm = ScriptVm::new();
        let code = [Instruction::new(OpCode::Call as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackOverflow { pc: 0 }));
    }
    #[test]
    fn test_stack_underflow() {
        let mut vm = ScriptVm::new();
        let code = [Instruction::new(OpCode::Ret as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackUnderflow { pc: 0 }));
    }
    #[test]
    fn test_invalid_opcode() {
        let mut vm = ScriptVm::new();
        let code = [Instruction::new(0x99, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(
            result,
            Err(VmError::InvalidOpcode {
                pc: 0,
                opcode: 0x99
            })
        );
    }
    #[test]
    fn test_floats() {
        let n = std::env::var("COVOPT_N")
            .unwrap_or(std::string::String::from("1"))
            .parse::<usize>()
            .unwrap();
        let mut vm = ScriptVm::new();
        let val1 = 3.5f32.to_bits();
        let val2 = 1.5f32.to_bits();
        vm.registers[1] = val1;
        vm.registers[2] = val2;
        let code = [
            Instruction::new(OpCode::FAdd as u8, 3, 1, 2),
            Instruction::new(OpCode::FSub as u8, 4, 1, 2),
            Instruction::new(OpCode::FMul as u8, 5, 1, 2),
            Instruction::new(OpCode::FDiv as u8, 6, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        for _ in 0..n {
            vm.run(&code).unwrap();
        }
        assert_eq!(f32::from_bits(vm.registers[3]), 5.0f32);
        assert_eq!(f32::from_bits(vm.registers[4]), 2.0f32);
        assert_eq!(f32::from_bits(vm.registers[5]), 5.25f32);
        assert_eq!(f32::from_bits(vm.registers[6]), 3.5 / 1.5);
    }
    #[test]
    fn test_memory_load_store() {
        let mut vm = ScriptVm::new();
        vm.registers[1] = 42;
        vm.registers[2] = 10;
        vm.registers[3] = 4;
        let code = [
            Instruction::new(OpCode::Store as u8, 1, 2, 3),
            Instruction::new(OpCode::Load as u8, 4, 2, 3),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(vm.registers[4], 42);
        assert_eq!(vm.memory[14], 42);
        assert_eq!(vm.memory[15], 0);
        assert_eq!(vm.memory[16], 0);
        assert_eq!(vm.memory[17], 0);
    }
    #[test]
    fn test_math_approximations() {
        let mut vm = ScriptVm::new();
        vm.registers[1] = 0;
        vm.registers[2] = 4;
        vm.registers[3] = 2;
        let code = [
            Instruction::new(OpCode::Exp as u8, 4, 1, 0),
            Instruction::new(OpCode::Rsqrt as u8, 5, 2, 0),
            Instruction::new(OpCode::Silu as u8, 6, 3, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(vm.registers[4], 65536);
        assert_eq!(vm.registers[5], 32768);
        assert!(vm.registers[6] > 0);
    }
    #[test]
    fn test_abort_flag() {
        let mut vm = ScriptVm::new();
        vm.max_steps = None;
        static ABORT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);
        fn abort_checker() -> bool {
            ABORT.load(core::sync::atomic::Ordering::Relaxed)
        }
        vm.abort_flag = Some(abort_checker);
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result.unwrap(), crate::scriptgo_vm::vm::VmResult::Halted(1));
    }
    #[test]
    fn test_out_of_fuel() {
        let mut vm = ScriptVm::new();
        vm.max_steps = Some(50);
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::OutOfFuel { pc: 0 }));
    }
    #[test]
    fn test_trace_logging() {
        let mut vm = ScriptVm::new();
        vm.tracing_enabled = true;
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, 42, 0),
            Instruction::new(OpCode::Store as u8, 1, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(vm.trace_count, 2);
        let trace1 = vm.trace_buffer[0];
        assert_eq!(trace1.pc, 0);
        assert_eq!(trace1.reg_change, Some((1, 42)));
        assert_eq!(trace1.mem_change, None);
        let trace2 = vm.trace_buffer[1];
        assert_eq!(trace2.pc, 1);
        assert_eq!(trace2.reg_change, None);
        assert_eq!(trace2.mem_change, Some((0, 42)));
    }
    #[test]
    fn test_debug_hook() {
        let mut vm = ScriptVm::new();
        use core::sync::atomic::{AtomicUsize, Ordering};
        static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);
        EXEC_COUNT.store(0, Ordering::Relaxed);
        vm.debug_hook = Some(|_vm, inst| {
            EXEC_COUNT.fetch_add(1, Ordering::Relaxed);
            if crate::opcode!(inst) == OpCode::LoadImm as u8 {
                assert_eq!(crate::inst_a!(inst), 1);
            }
        });
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(EXEC_COUNT.load(Ordering::Relaxed), 2);
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_panic_recovery() {
        let mut vm = ScriptVm::new();
        vm.print_handler = Some(|_| {
            panic!("Mock handler panic!");
        });
        let code = [Instruction::new(OpCode::PrintReg as u8, 0, 0, 0)];
        let vm_ref = &mut vm;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _ = vm_ref.run(&code);
        }));
        assert!(result.is_err());
    }
    #[test]
    fn test_hot_reload_state_preservation() {
        let mut vm = ScriptVm::new();
        vm.pc = 42;
        vm.sp = 5;
        vm.call_stack[0] = 99;
        vm.registers[3] = 77;
        vm.registers[20] = 88;
        vm.memory[10] = 55;
        vm.hot_reload();
        assert_eq!(vm.pc, 0);
        assert_eq!(vm.sp, 0);
        assert_eq!(vm.call_stack[0], 0);
        assert_eq!(vm.registers[3], 0);
        assert_eq!(vm.registers[20], 88);
        assert_eq!(vm.memory[10], 55);
    }
    #[test]
    fn test_audit() {
        let n = std::env::var("COVOPT_N")
            .unwrap_or(std::string::String::from("1000"))
            .parse::<usize>()
            .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut handles = std::vec::Vec::new();
        for _ in 0..4 {
            let tx_clone = tx.clone();
            let handle = std::thread::spawn(move || {
                let n = n;
                let mut vm = ScriptVm::new();
                vm.print_handler = Some(|_| {});
                vm.registers[1] = 2;
                vm.registers[2] = 1;
                let code = [
                    Instruction::new(OpCode::LoadImm as u8, 1, 2, 0),
                    Instruction::new(OpCode::LoadImm as u8, 2, 1, 0),
                    Instruction::new(OpCode::LoadImm as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfZero as u8, 1, 0, 0),
                    Instruction::new(OpCode::JmpIfZero as u8, 0, 6, 0),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfEq as u8, 1, 2, 0),
                    Instruction::new(OpCode::JmpIfEq as u8, 1, 1, 9),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfLt as u8, 1, 2, 0),
                    Instruction::new(OpCode::JmpIfLt as u8, 2, 1, 12),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfGt as u8, 2, 1, 0),
                    Instruction::new(OpCode::JmpIfGt as u8, 1, 2, 15),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, 0),
                    Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, 18),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, 0),
                    Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, 21),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::LoadImm16 as u8, 4, 0, 5),
                    Instruction::new(OpCode::Add as u8, 5, 1, 2),
                    Instruction::new(OpCode::Sub as u8, 5, 1, 2),
                    Instruction::new(OpCode::Mul as u8, 5, 1, 2),
                    Instruction::new(OpCode::Div as u8, 5, 1, 2),
                    Instruction::new(OpCode::Mod as u8, 5, 1, 2),
                    Instruction::new(OpCode::And as u8, 5, 1, 2),
                    Instruction::new(OpCode::Or as u8, 5, 1, 2),
                    Instruction::new(OpCode::Xor as u8, 5, 1, 2),
                    Instruction::new(OpCode::Shl as u8, 5, 1, 2),
                    Instruction::new(OpCode::Shr as u8, 5, 1, 2),
                    Instruction::new(OpCode::CmpEq as u8, 5, 1, 2),
                    Instruction::new(OpCode::CmpLt as u8, 5, 1, 2),
                    Instruction::new(OpCode::FAdd as u8, 5, 1, 2),
                    Instruction::new(OpCode::FSub as u8, 5, 1, 2),
                    Instruction::new(OpCode::FMul as u8, 5, 1, 2),
                    Instruction::new(OpCode::FDiv as u8, 5, 1, 2),
                    Instruction::new(OpCode::Store as u8, 1, 0, 2),
                    Instruction::new(OpCode::Load as u8, 5, 0, 2),
                    Instruction::new(OpCode::PrintReg as u8, 5, 0, 0),
                    Instruction::new(OpCode::SysCall as u8, 5, 0, 0),
                    Instruction::new(OpCode::Call as u8, 0, 44, 0),
                    Instruction::new(OpCode::Jmp as u8, 0, 45, 0),
                    Instruction::new(OpCode::Ret as u8, 0, 0, 0),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                ];
                for _ in 0..n {
                    std::hint::black_box(vm.run(&code).unwrap());
                }
                tx_clone.send(()).unwrap();
            });
            handles.push(handle);
        }
        for _ in 0..4 {
            rx.recv_timeout(std::time::Duration::from_secs(5))
                .expect("Watchdog timeout");
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let mut vm_err = ScriptVm::new();
        let code_div0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, 3, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_div0);
        let code_fdiv0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::FDiv as u8, 3, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_fdiv0);
        let code_mem = [
            Instruction::new(
                OpCode::LoadImm16 as u8,
                1,
                (10000 & 0xFF) as u8,
                (10000 >> 8) as u8,
            ),
            Instruction::new(OpCode::Load as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_mem);
        let code_so = [Instruction::new(OpCode::Call as u8, 0, 0, 0); 257];
        let mut vm_so = ScriptVm::new();
        let _ = vm_so.run_fast(&code_so);
        let code_su = [
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_su);
        let code_inv = [
            Instruction::new(255, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_inv);
        let mut vm_handlers = ScriptVm::new();
        vm_handlers.print_handler = Some(|_| {});
        vm_handlers.syscall_handler = Some(|_, _, _| {});
        vm_handlers.hardware_handler = Some(|_, _, _, _| {});
        vm_handlers.ui_handler = Some(|_, _, _| {});
        vm_handlers.neural_handler = Some(|_, _, _, _| {});
        let code_handlers = [
            Instruction::new(OpCode::PrintReg as u8, 0, 0, 0),
            Instruction::new(OpCode::SysCall as u8, 0, 0, 0),
            Instruction::new(OpCode::HardwareCall as u8, 0, 0, 0),
            Instruction::new(OpCode::UiCall as u8, 1, 1, 0),
            Instruction::new(OpCode::UiCall as u8, 0, 5, 0),
            Instruction::new(OpCode::NeuralCall as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_handlers.run_fast(&code_handlers);
        let mut vm_jumps = ScriptVm::new();
        vm_jumps.registers[1] = 0;
        vm_jumps.registers[2] = 1;
        let code_jumps = [
            Instruction::new(OpCode::JmpIfZero as u8, 1, 1, 0),
            Instruction::new(OpCode::JmpIfZero as u8, 2, 2, 0),
            Instruction::new(OpCode::JmpIfEq as u8, 1, 1, 3),
            Instruction::new(OpCode::JmpIfEq as u8, 1, 2, 3),
            Instruction::new(OpCode::JmpIfLt as u8, 1, 2, 5),
            Instruction::new(OpCode::JmpIfLt as u8, 2, 1, 5),
            Instruction::new(OpCode::JmpIfGt as u8, 2, 1, 7),
            Instruction::new(OpCode::JmpIfGt as u8, 1, 2, 7),
            Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, 9),
            Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, 9),
            Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, 11),
            Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, 11),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_jumps.run_fast(&code_jumps);
        let code_store = [
            Instruction::new(
                OpCode::LoadImm16 as u8,
                1,
                (10000 & 0xFF) as u8,
                (10000 >> 8) as u8,
            ),
            Instruction::new(OpCode::Store as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_store);
        let code_math_exp = [
            Instruction::new(OpCode::LoadImm as u8, 1, 11, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 16, 0),
            Instruction::new(OpCode::Shl as u8, 1, 1, 2),
            Instruction::new(OpCode::Exp as u8, 3, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_exp);
        let code_math_rsqrt = [
            Instruction::new(OpCode::LoadImm as u8, 1, 0, 0),
            Instruction::new(OpCode::Rsqrt as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_rsqrt);
        let code_math_silu = [
            Instruction::new(OpCode::LoadImm as u8, 1, 128, 0),
            Instruction::new(OpCode::Silu as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_silu);
    }
}
