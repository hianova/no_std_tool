#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Halt = 0,
    LoadImm = 1,
    LoadImm16 = 2,

    Add = 3,
    Sub = 4,
    Mul = 5,
    Div = 6,
    Mod = 7,
    FAdd = 8,
    FSub = 9,
    FMul = 10,
    FDiv = 11,

    And = 12,
    Or = 13,
    Xor = 14,
    Shl = 15,
    Shr = 16,
    CmpEq = 17,
    CmpLt = 18,

    Jmp = 19,
    JmpIfZero = 20,
    JmpIfEq = 21,
    JmpIfLt = 22,
    JmpIfGt = 23,
    JmpIfFLt = 24,
    JmpIfFGt = 25,

    Call = 26,
    Ret = 27,

    PrintReg = 28,
    SysCall = 29,

    Load = 30,
    Store = 31,

    Exp = 32,
    Rsqrt = 33,
    Silu = 34,

    HardwareCall = 35,
    UiCall = 36,
    NeuralCall = 37,
    Yield = 38,

    VecAdd = 39,
    VecMul = 40,
    VecDot = 41,

    Spawn = 42,
    Await = 43,

    Mmap = 44,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Instruction(pub u32);

impl Instruction {
    pub const fn new(opcode: u8, a: u8, b: u8, c: u8) -> Self {
        Self((opcode as u32) | ((a as u32) << 8) | ((b as u32) << 16) | ((c as u32) << 24))
    }
}

#[macro_export]
macro_rules! opcode {
    ($inst:expr) => {
        ($inst.0 & 0xFF) as u8
    };
}

#[macro_export]
macro_rules! inst_a {
    ($inst:expr) => {
        (($inst.0 >> 8) & 0xFF) as usize
    };
}

#[macro_export]
macro_rules! inst_b {
    ($inst:expr) => {
        (($inst.0 >> 16) & 0xFF) as usize
    };
}

#[macro_export]
macro_rules! inst_c {
    ($inst:expr) => {
        (($inst.0 >> 24) & 0xFF) as usize
    };
}

#[macro_export]
macro_rules! inst_imm16 {
    ($inst:expr) => {
        (($inst.0 >> 16) & 0xFFFF) as u16
    };
}
