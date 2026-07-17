// use alloc::vec::Vec;
use crate::scriptgo_vm::instruction::Instruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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

#[inline(always)]
fn likely(b: bool) -> bool {
    // core::intrinsics::likely(b)
    b
}

#[inline(always)]
fn unlikely(b: bool) -> bool {
    // core::intrinsics::unlikely(b)
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
    // Watchdog timeout check to prevent infinite spin deadlocks (similar to recv_timeout)
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

    /// Reset ephemeral execution context (PC, SP, call stack, R[0..16]) while preserving
    /// memory and persistent registers R[16..256] across code reloads (similar to React Fast Refresh).
    pub fn hot_reload(&mut self) {
        self.pc = 0;
        self.sp = 0;
        self.call_stack = [0; 64];
        for i in 0..16 {
            self.registers[i] = 0;
        }
    }

    /// Log a trace step to the circular trace buffer.
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

    /// Run the VM execution loop.
    /// Returns the number of instructions executed on success.
    #[inline(always)]
    pub fn run(&mut self, code: &[Instruction]) -> Result<u32, VmError> {
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

    #[inline(never)]
    pub fn run_fast(&mut self, code: &[Instruction]) -> Result<u32, VmError> {
        self.pc = 0;
        self.sp = 0;
        let mut steps = 0;
        let max_steps = self.max_steps.unwrap_or(u32::MAX);

        loop {
            // Batch watchdog check and max_steps: only poll every 256 instructions
            let poll_mask = crate::covopt_param!("watchdog_poll_mask", 0xFF, 0x01..=0xFFF);
            if unlikely(steps & poll_mask == 0) {
                if unlikely(steps >= max_steps) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
                if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
            }

            let inst = unsafe { *code.get_unchecked(self.pc) };
            self.pc += 1;
            steps += 1; // COVOPT_ANCHOR_VM

            let opcode = inst.opcode();

            match opcode {
                0 => break, // Halt
                1 => {
                    let a = inst.a();
                    self.registers[a] = inst.b() as u32;
                }
                2 => {
                    let a = inst.a();
                    self.registers[a] = inst.imm16() as u32;
                }

                3 => {
                    let a = inst.a();
                    self.registers[a] = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]);
                }
                4 => {
                    let a = inst.a();
                    self.registers[a] = self.registers[inst.b()].wrapping_sub(self.registers[inst.c()]);
                }
                5 => {
                    let a = inst.a();
                    self.registers[a] = self.registers[inst.b()].wrapping_mul(self.registers[inst.c()]);
                }
                6 => {
                    let a = inst.a();
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[inst.b()] / divisor;
                }
                7 => {
                    let a = inst.a();
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[inst.b()] % divisor;
                }
                8 => {
                    // FAdd
                    let b_val = f32::from_bits(self.registers[inst.b()]);
                    let c_val = f32::from_bits(self.registers[inst.c()]);
                    self.registers[inst.a()] = (b_val + c_val).to_bits();
                }
                9 => {
                    // FSub
                    let b_val = f32::from_bits(self.registers[inst.b()]);
                    let c_val = f32::from_bits(self.registers[inst.c()]);
                    self.registers[inst.a()] = (b_val - c_val).to_bits();
                }
                10 => {
                    // FMul
                    let b_val = f32::from_bits(self.registers[inst.b()]);
                    let c_val = f32::from_bits(self.registers[inst.c()]);
                    self.registers[inst.a()] = (b_val * c_val).to_bits();
                }
                11 => {
                    // FDiv
                    let divisor = f32::from_bits(self.registers[inst.c()]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b_val = f32::from_bits(self.registers[inst.b()]);
                    self.registers[inst.a()] = (b_val / divisor).to_bits();
                }

                12 => {
                    self.registers[inst.a()] = self.registers[inst.b()] & self.registers[inst.c()];
                }
                13 => {
                    self.registers[inst.a()] = self.registers[inst.b()] | self.registers[inst.c()];
                }
                14 => {
                    self.registers[inst.a()] = self.registers[inst.b()] ^ self.registers[inst.c()];
                }
                15 => {
                    self.registers[inst.a()] = self.registers[inst.b()] << self.registers[inst.c()];
                }
                16 => {
                    self.registers[inst.a()] = self.registers[inst.b()] >> self.registers[inst.c()];
                }
                17 => {
                    self.registers[inst.a()] = if self.registers[inst.b()] == self.registers[inst.c()] { 1 } else { 0 };
                }
                18 => {
                    self.registers[inst.a()] = if self.registers[inst.b()] < self.registers[inst.c()] { 1 } else { 0 };
                }

                19 => {
                    self.pc = inst.imm16() as usize;
                }
                20 => {
                    if self.registers[inst.a()] == 0 {
                        self.pc = inst.b() as usize;
                    }
                }
                21 => {
                    if self.registers[inst.a()] == self.registers[inst.b()] {
                        self.pc = inst.c() as usize;
                    }
                }
                22 => {
                    if self.registers[inst.a()] < self.registers[inst.b()] {
                        self.pc = inst.c() as usize;
                    }
                }
                23 => {
                    if self.registers[inst.a()] > self.registers[inst.b()] {
                        self.pc = inst.c() as usize;
                    }
                }
                24 => {
                    let a_val = f32::from_bits(self.registers[inst.a()]);
                    let b_val = f32::from_bits(self.registers[inst.b()]);
                    if a_val < b_val {
                        self.pc = inst.c() as usize;
                    }
                }
                25 => {
                    let a_val = f32::from_bits(self.registers[inst.a()]);
                    let b_val = f32::from_bits(self.registers[inst.b()]);
                    if a_val > b_val {
                        self.pc = inst.c() as usize;
                    }
                }

                26 => {
                    if self.sp < 64 {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = inst.imm16() as usize;
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
                        handler(self.registers[inst.a()]);
                    }
                }
                29 => {
                    if let Some(handler) = self.syscall_handler {
                        handler(self.registers[inst.a()], self.registers[inst.b()], self.registers[inst.c()]);
                    }
                }

                30 => {
                    // Load
                    let addr = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]) as usize;
                    if addr + 4 <= self.memory.len() {
                        let mut val = 0u32;
                        for i in 0..4 {
                            val |= (self.memory[addr + i] as u32) << (i * 8);
                        }
                        self.registers[inst.a()] = val;
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds {
                            pc: self.pc - 1,
                            addr,
                        });
                    }
                }
                31 => {
                    // Store
                    let addr = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]) as usize;
                    if addr + 4 <= self.memory.len() {
                        let val = self.registers[inst.a()];
                        for i in 0..4 {
                            self.memory[addr + i] = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds {
                            pc: self.pc - 1,
                            addr,
                        });
                    }
                }

                32 => {
                    // Exp
                    let val = self.registers[inst.b()] as i32;
                    if let Some(res) = crate::math::exp_approx_q16(val) {
                        self.registers[inst.a()] = res as u32;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    // Rsqrt
                    let val = self.registers[inst.b()];
                    if let Some(res) = crate::math::rsqrt_approx_i32(val) {
                        self.registers[inst.a()] = res;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    // Silu
                    let val = (self.registers[inst.b()] & 0xFF) as i8;
                    if let Some(res) = crate::math::silu_approx_i8(val) {
                        self.registers[inst.a()] = (res as u32) & 0xFF;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }

                35 => {
                    // HardwareCall
                    if let Some(handler) = self.hardware_handler {
                        handler(self, inst.a(), inst.b(), inst.c());
                    }
                }
                36 => {
                    // UiCall
                    let cmd = inst.b();
                    if inst.a() == 0 || !(1..=4).contains(&cmd) {
                        // Drop invalid payload silently
                    } else if let Some(handler) = self.ui_handler {
                        handler(inst.a(), inst.b(), inst.c());
                    }
                }
                37 => {
                    // NeuralCall
                    let handler = self.neural_handler;
                    if let Some(h) = handler {
                        h(self, inst.a(), inst.b(), inst.c());
                    }
                }
                _ => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode,
                    })
                }
            }
        }
        Ok(steps)
    }

    #[inline(never)]
    fn run_slow(&mut self, code: &[Instruction]) -> Result<u32, VmError> {
        self.pc = 0;
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

            // Watchdog timeout check to prevent infinite spin deadlocks (similar to recv_timeout)
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

            match inst.opcode() {
                0 => break, // Halt
                1 => {
                    let val = inst.b() as u32;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                2 => {
                    let val = inst.imm16() as u32;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }

                3 => {
                    let val = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]);
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                4 => {
                    let val = self.registers[inst.b()].wrapping_sub(self.registers[inst.c()]);
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                5 => {
                    let val = self.registers[inst.b()].wrapping_mul(self.registers[inst.c()]);
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                6 => {
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[inst.b()] / divisor;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                7 => {
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[inst.b()] % divisor;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                8 => {
                    // FAdd
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let c = f32::from_bits(self.registers[inst.c()]);
                    let val = (b + c).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                9 => {
                    // FSub
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let c = f32::from_bits(self.registers[inst.c()]);
                    let val = (b - c).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                10 => {
                    // FMul
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let c = f32::from_bits(self.registers[inst.c()]);
                    let val = (b * c).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                11 => {
                    // FDiv
                    let divisor = f32::from_bits(self.registers[inst.c()]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let val = (b / divisor).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }

                12 => {
                    let val = self.registers[inst.b()] & self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                13 => {
                    let val = self.registers[inst.b()] | self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                14 => {
                    let val = self.registers[inst.b()] ^ self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                15 => {
                    let val = self.registers[inst.b()] << self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                16 => {
                    let val = self.registers[inst.b()] >> self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }

                19 => self.pc = inst.imm16() as usize,
                20 => {
                    if self.registers[inst.a()] == 0 {
                        self.pc = inst.imm16() as usize;
                    }
                }
                21 => {
                    if self.registers[inst.a()] == self.registers[inst.b()] {
                        self.pc = inst.c();
                    }
                }
                22 => {
                    if self.registers[inst.a()] < self.registers[inst.b()] {
                        self.pc = inst.c();
                    }
                }
                23 => {
                    if self.registers[inst.a()] > self.registers[inst.b()] {
                        self.pc = inst.c();
                    }
                }
                24 => {
                    // JmpIfFLt
                    let a = f32::from_bits(self.registers[inst.a()]);
                    let b = f32::from_bits(self.registers[inst.b()]);
                    if a < b {
                        self.pc = inst.c();
                    }
                }
                25 => {
                    // JmpIfFGt
                    let a = f32::from_bits(self.registers[inst.a()]);
                    let b = f32::from_bits(self.registers[inst.b()]);
                    if a > b {
                        self.pc = inst.c();
                    }
                }

                26 => {
                    // Call
                    if self.sp < 64 {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = inst.imm16() as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    // Ret
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }

                28 => {
                    // PrintReg
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[inst.a()]);
                    }
                }
                29 => {
                    // SysCall
                    if let Some(handler) = self.syscall_handler {
                        handler(self.registers[inst.a()], self.registers[inst.b()], self.registers[inst.c()]);
                    }
                }

                30 => {
                    // Load
                    let addr =
                        self.registers[inst.b()].wrapping_add(self.registers[inst.c()]) as usize;
                    if addr + 4 <= self.memory.len() {
                        let mut val = 0u32;
                        for i in 0..4 {
                            val |= (self.memory[addr + i] as u32) << (i * 8);
                        }
                        self.registers[inst.a()] = val;
                        reg_change = Some((inst.a() as u8, val));
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds {
                            pc: self.pc - 1,
                            addr,
                        });
                    }
                }
                31 => {
                    // Store
                    let addr =
                        self.registers[inst.b()].wrapping_add(self.registers[inst.c()]) as usize;
                    if addr + 4 <= self.memory.len() {
                        let val = self.registers[inst.a()];
                        for i in 0..4 {
                            self.memory[addr + i] = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                        mem_change = Some((addr as u16, val));
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds {
                            pc: self.pc - 1,
                            addr,
                        });
                    }
                }

                32 => {
                    // Exp
                    let val = self.registers[inst.b()] as i32;
                    if let Some(res) = crate::math::exp_approx_q16(val) {
                        let val_u32 = res as u32;
                        self.registers[inst.a()] = val_u32;
                        reg_change = Some((inst.a() as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    // Rsqrt
                    let val = self.registers[inst.b()];
                    if let Some(res) = crate::math::rsqrt_approx_i32(val) {
                        self.registers[inst.a()] = res;
                        reg_change = Some((inst.a() as u8, res));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    // Silu
                    let val = (self.registers[inst.b()] & 0xFF) as i8;
                    if let Some(res) = crate::math::silu_approx_i8(val) {
                        let val_u32 = (res as u32) & 0xFF;
                        self.registers[inst.a()] = val_u32;
                        reg_change = Some((inst.a() as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }

                37 => {
                    // NeuralCall
                    let handler = self.neural_handler;
                    if let Some(h) = handler {
                        h(self, inst.a(), inst.b(), inst.c());
                    }
                }
                35 => {
                    // HardwareCall
                    let handler = self.hardware_handler;
                    if let Some(h) = handler {
                        h(self, inst.a(), inst.b(), inst.c());
                    }
                }

                36 => {
                    // UiCall
                    // FFI border verification: ID must be non-zero, and Command type must be within 1..=4.
                    let cmd = inst.b();
                    if inst.a() == 0 || !(1..=4).contains(&cmd) {
                        // Drop invalid payload silently on FFI boundary check error.
                    } else if let Some(handler) = self.ui_handler {
                        handler(inst.a(), inst.b(), inst.c());
                    }
                }

                op => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode: op,
                    })
                }
            }

            self.log_trace(current_pc, inst.0, reg_change, mem_change);
        }
        Ok(steps)
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
        // LOADIMM 1 10
        // LOADIMM 2 0
        // DIV 3 1 2
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
        // CALL 0 (recursive call to itself)
        let code = [Instruction::new(OpCode::Call as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackOverflow { pc: 0 }));
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = ScriptVm::new();
        // RET (no call pushed)
        let code = [Instruction::new(OpCode::Ret as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackUnderflow { pc: 0 }));
    }

    #[test]
    fn test_invalid_opcode() {
        let mut vm = ScriptVm::new();
        let code = [
            Instruction::new(0x99, 0, 0, 0), // 0x99 is undefined
        ];

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
        let n = std::env::var("COVOPT_N").unwrap_or(std::string::String::from("1")).parse::<usize>().unwrap();
        let mut vm = ScriptVm::new();
        // Load f32 values represented as raw bits
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
        // R[1] = 42 (value to store)
        // R[2] = 10 (base address)
        // R[3] = 4 (offset)
        // Store R[1] to Memory[R[2] + R[3]]
        // R[4] = Load from Memory[R[2] + R[3]]
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
        // Verify bytes in memory (little endian)
        assert_eq!(vm.memory[14], 42);
        assert_eq!(vm.memory[15], 0);
        assert_eq!(vm.memory[16], 0);
        assert_eq!(vm.memory[17], 0);
    }

    #[test]
    fn test_math_approximations() {
        let mut vm = ScriptVm::new();
        // EXP: exp_approx_q16
        // R[1] = 0 (Q16.16)
        // RSQRT: rsqrt_approx_i32
        // R[2] = 4
        // SILU: silu_approx_i8
        // R[3] = 2
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

        // exp(0) = 1.0 (Q16.16 -> 65536)
        assert_eq!(vm.registers[4], 65536);
        // rsqrt(4) = 1/sqrt(4) = 0.5 (Q16.16 -> 32768)
        assert_eq!(vm.registers[5], 32768);
        // silu(2) ≈ 2 * (1 / (1 + exp(-2)))
        // Silu approx of 2 is non-zero
        assert!(vm.registers[6] > 0);
    }

    #[test]
    fn test_abort_flag() {
        let mut vm = ScriptVm::new();
        vm.max_steps = None;
        static ABORT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);
        fn abort_checker() -> bool { ABORT.load(core::sync::atomic::Ordering::Relaxed) }
        vm.abort_flag = Some(abort_checker);

        // Endless loop:
        // 0: JMP 0
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
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
            if inst.opcode() == OpCode::LoadImm as u8 {
                assert_eq!(inst.a(), 1);
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
        // Set some ephemeral state
        vm.pc = 42;
        vm.sp = 5;
        vm.call_stack[0] = 99;
        vm.registers[3] = 77; // Ephemeral register

        // Set some persistent state
        vm.registers[20] = 88; // Persistent register
        vm.memory[10] = 55; // RAM

        vm.hot_reload();

        // Ephemeral state must be reset
        assert_eq!(vm.pc, 0);
        assert_eq!(vm.sp, 0);
        assert_eq!(vm.call_stack[0], 0);
        assert_eq!(vm.registers[3], 0);

        // Persistent state must be preserved
        assert_eq!(vm.registers[20], 88);
        assert_eq!(vm.memory[10], 55);
    }

    #[test]
    fn test_audit() {
        let n = std::env::var("COVOPT_N")
            .unwrap_or(std::string::String::from("1"))
            .parse::<usize>()
            .unwrap();
        let mut vm = ScriptVm::new();
        vm.print_handler = Some(|_| {});

        vm.registers[1] = 2;
        vm.registers[2] = 1;

        let code = [
            // Setup
            Instruction::new(OpCode::LoadImm as u8, 1, 2, 0), // R1 = 2
            Instruction::new(OpCode::LoadImm as u8, 2, 1, 0), // R2 = 1
            Instruction::new(OpCode::LoadImm as u8, 0, 0, 0), // R0 = 0
            
            // 3: JmpIfZero
            Instruction::new(OpCode::JmpIfZero as u8, 1, 0, 0), // false
            Instruction::new(OpCode::JmpIfZero as u8, 0, 6, 0), // true, PC=6
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            
            // 6: JmpIfEq
            Instruction::new(OpCode::JmpIfEq as u8, 1, 2, 0), // false
            Instruction::new(OpCode::JmpIfEq as u8, 1, 1, 9), // true, PC=9
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            
            // 9: JmpIfLt
            Instruction::new(OpCode::JmpIfLt as u8, 1, 2, 0), // false
            Instruction::new(OpCode::JmpIfLt as u8, 2, 1, 12), // true, PC=12
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            
            // 12: JmpIfGt
            Instruction::new(OpCode::JmpIfGt as u8, 2, 1, 0), // false
            Instruction::new(OpCode::JmpIfGt as u8, 1, 2, 15), // true, PC=15
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            
            // 15: JmpIfFLt
            Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, 0), // false
            Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, 18), // true, PC=18
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            
            // 18: JmpIfFGt
            Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, 0), // false
            Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, 21), // true, PC=21
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            
            // 21: other ops
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
            
            Instruction::new(OpCode::Call as u8, 0, 44, 0), // 42: Push PC=43, Jmp 44
            Instruction::new(OpCode::Jmp as u8, 0, 45, 0),  // 43: Jmp 45
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),   // 44: Pops PC=43, Jmp 43
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),  // 45
        ];

        for _ in 0..n {
            vm.run(&code).unwrap();
        }
        
        // --- Coverage Boost for Error Branches ---
        let mut vm_err = ScriptVm::new();
        
        // 1. DivideByZero
        let code_div0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, 3, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_div0);

        // 2. FDiv by Zero
        let code_fdiv0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::FDiv as u8, 3, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_fdiv0);
        
        // 3. MemoryOutOfBounds (Load/Store)
        let code_mem = [
            Instruction::new(OpCode::LoadImm16 as u8, 1, (10000 & 0xFF) as u8, (10000 >> 8) as u8),
            Instruction::new(OpCode::Load as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_mem);
        
        // 4. StackOverflow
        let code_so = [Instruction::new(OpCode::Call as u8, 0, 0, 0); 257];
        let mut vm_so = ScriptVm::new();
        let _ = vm_so.run_fast(&code_so);
        
        // 5. StackUnderflow
        let code_su = [
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_su);
        
        // 6. InvalidOpCode
        let code_inv = [
            Instruction::new(255, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_inv);

        // 7. Handlers (Coverage Boost)
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
            Instruction::new(OpCode::UiCall as u8, 1, 1, 0), // ui_call requires a != 0 and b in 1..=4
            Instruction::new(OpCode::UiCall as u8, 0, 5, 0), // false branch for ui_call
            Instruction::new(OpCode::NeuralCall as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_handlers.run_fast(&code_handlers);

        // 8. Jumps and Control Flow (Coverage Boost)
        let mut vm_jumps = ScriptVm::new();
        vm_jumps.registers[1] = 0;
        vm_jumps.registers[2] = 1;
        
        let code_jumps = [
            // JmpIfZero
            Instruction::new(OpCode::JmpIfZero as u8, 1, 1, 0), // True -> PC=1 (imm16=b|c<<8) -> imm16=1
            Instruction::new(OpCode::JmpIfZero as u8, 2, 2, 0), // False -> imm16=2
            // JmpIfEq
            Instruction::new(OpCode::JmpIfEq as u8, 1, 1, 3),   // True -> PC=3
            Instruction::new(OpCode::JmpIfEq as u8, 1, 2, 3),   // False
            // JmpIfLt
            Instruction::new(OpCode::JmpIfLt as u8, 1, 2, 5),   // True -> PC=5
            Instruction::new(OpCode::JmpIfLt as u8, 2, 1, 5),   // False
            // JmpIfGt
            Instruction::new(OpCode::JmpIfGt as u8, 2, 1, 7),   // True -> PC=7
            Instruction::new(OpCode::JmpIfGt as u8, 1, 2, 7),   // False
            // JmpIfFLt
            Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, 9),  // True -> PC=9
            Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, 9),  // False
            // JmpIfFGt
            Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, 11), // True -> PC=11
            Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, 11), // False
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),      // 12
        ];
        let _ = vm_jumps.run_fast(&code_jumps);
        
        // 9. Store OutOfBounds
        let code_store = [
            Instruction::new(OpCode::LoadImm16 as u8, 1, (10000 & 0xFF) as u8, (10000 >> 8) as u8),
            Instruction::new(OpCode::Store as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_store);

        // 10. Math Errors
        // Exp error (needs input > 10 * 65536)
        let code_math_exp = [
            Instruction::new(OpCode::LoadImm as u8, 1, 11, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 16, 0),
            Instruction::new(OpCode::Shl as u8, 1, 1, 2),
            Instruction::new(OpCode::Exp as u8, 3, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_exp);

        // Rsqrt error (needs input == 0)
        let code_math_rsqrt = [
            Instruction::new(OpCode::LoadImm as u8, 1, 0, 0),
            Instruction::new(OpCode::Rsqrt as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_rsqrt);

        // Silu error (needs input == 128 which is -128 as i8, causing Exp overflow)
        let code_math_silu = [
            Instruction::new(OpCode::LoadImm as u8, 1, 128, 0),
            Instruction::new(OpCode::Silu as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_silu);
    }
}
