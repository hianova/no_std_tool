use crate::scriptgo_vm::instruction::{Instruction, OpCode};
use alloc::vec::Vec;
#[derive(Default)]
#[repr(C, align(64))]
pub struct ScriptAssembler {
    instructions: Vec<Instruction>,
}
impl ScriptAssembler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
        }
    }
    pub fn current_address(&self) -> usize {
        self.instructions.len()
    }
    pub fn set_instruction(&mut self, index: usize, inst: Instruction) {
        if index < self.instructions.len() {
            self.instructions[index] = inst;
        }
    }
    pub fn get_instruction(&self, index: usize) -> Option<Instruction> {
        self.instructions.get(index).copied()
    }
    pub fn build(self) -> Vec<Instruction> {
        self.instructions
    }
    #[doc = " Emits a raw instruction."]
    pub fn emit(&mut self, inst: Instruction) -> &mut Self {
        self.instructions.push(inst);
        self
    }
    #[doc = " R[A] = B"]
    pub fn load_imm(&mut self, a: u8, b: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::LoadImm as u8, a, b, 0))
    }
    #[doc = " R[A] = imm16(B, C)"]
    pub fn load_imm16(&mut self, a: u8, val: u16) -> &mut Self {
        let b = (val & 0xFF) as u8;
        let c = ((val >> 8) & 0xFF) as u8;
        self.emit(Instruction::new(OpCode::LoadImm16 as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] + R[C]"]
    pub fn add(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::Add as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] - R[C]"]
    pub fn sub(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::Sub as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] * R[C]"]
    pub fn mul(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::Mul as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] / R[C]"]
    pub fn div(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::Div as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] + R[C] as f32"]
    pub fn fadd(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::FAdd as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] - R[C] as f32"]
    pub fn fsub(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::FSub as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] * R[C] as f32"]
    pub fn fmul(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::FMul as u8, a, b, c))
    }
    #[doc = " R[A] = R[B] / R[C] as f32"]
    pub fn fdiv(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::FDiv as u8, a, b, c))
    }
    #[doc = " PC = imm16"]
    pub fn jmp(&mut self, target: u16) -> &mut Self {
        let b = (target & 0xFF) as u8;
        let c = ((target >> 8) & 0xFF) as u8;
        self.emit(Instruction::new(OpCode::Jmp as u8, 0, b, c))
    }
    #[doc = " If R[A] == 0, PC = target"]
    pub fn jmp_if_zero(&mut self, a: u8, target: u16) -> &mut Self {
        let b = (target & 0xFF) as u8;
        let c = ((target >> 8) & 0xFF) as u8;
        self.emit(Instruction::new(OpCode::JmpIfZero as u8, a, b, c))
    }
    #[doc = " If R[A] > R[B] as f32, PC = target"]
    pub fn jmp_if_fgt(&mut self, a: u8, b: u8, target: u16) -> &mut Self {
        let target_8 = target as u8;
        self.emit(Instruction::new(OpCode::JmpIfFGt as u8, a, b, target_8))
    }
    pub fn cmp_eq(&mut self, a: u8, b: u8, c: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::CmpEq as u8, a, b, c))
    }
    pub fn jmp_if_lt(&mut self, a: u8, b: u8, target: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::JmpIfLt as u8, a, b, target))
    }
    pub fn jmp_if_gt(&mut self, a: u8, b: u8, target: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::JmpIfGt as u8, a, b, target))
    }
    pub fn jmp_if_eq(&mut self, a: u8, b: u8, target: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::JmpIfEq as u8, a, b, target))
    }
    #[doc = " System call: Print R[A]"]
    pub fn print_reg(&mut self, a: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::PrintReg as u8, a, 0, 0))
    }
    pub fn halt(&mut self) -> &mut Self {
        self.emit(Instruction::new(OpCode::Halt as u8, 0, 0, 0))
    }
    pub fn yield_op(&mut self) -> &mut Self {
        self.emit(Instruction::new(OpCode::Yield as u8, 0, 0, 0))
    }
    pub fn vec_add(&mut self, dest: u8, src1: u8, src2: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::VecAdd as u8, dest, src1, src2))
    }
    pub fn vec_mul(&mut self, dest: u8, src1: u8, src2: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::VecMul as u8, dest, src1, src2))
    }
    pub fn vec_dot(&mut self, dest_reg: u8, src1: u8, src2: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::VecDot as u8, dest_reg, src1, src2))
    }
    pub fn spawn(&mut self, dest: u8, target_pc: u16) -> &mut Self {
        let b = (target_pc & 0xFF) as u8;
        let c = ((target_pc >> 8) & 0xFF) as u8;
        self.emit(Instruction::new(OpCode::Spawn as u8, dest, b, c))
    }
    pub fn await_op(&mut self, dest: u8, resource_reg: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::Await as u8, dest, resource_reg, 0))
    }
    pub fn mmap_op(&mut self, resource_reg: u8) -> &mut Self {
        self.emit(Instruction::new(OpCode::Mmap as u8, 0, resource_reg, 0))
    }
}
