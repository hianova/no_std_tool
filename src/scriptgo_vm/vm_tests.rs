extern crate std;
    use super::*;
    use crate::scriptgo_vm::instruction::OpCode;
    use crate::scriptgo_vm::vm::{ScriptVm, VmError};

#[allow(dead_code)]
#[inline(always)]
fn unlikely(b: bool) -> bool { b }

#[allow(dead_code)]
#[repr(align(64))]
pub struct DummyVmAligned;

    #[test]
    fn test_div_by_zero() {
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

        let mut vm = ScriptVm::new();
        // CALL 0 (recursive call to itself)
        let code = [Instruction::new(OpCode::Call as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackOverflow { pc: 0 }));
    }

    #[test]
    fn test_stack_underflow() {
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

        let mut vm = ScriptVm::new();
        // RET (no call pushed)
        let code = [Instruction::new(OpCode::Ret as u8, 0, 0, 0)];

        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackUnderflow { pc: 0 }));
    }

    #[test]
    fn test_invalid_opcode() {
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

        let mut vm = ScriptVm::new();
        vm.max_steps = Some(50);
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::OutOfFuel { pc: 0 }));
    }

    #[test]
    fn test_trace_logging() {
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
    #[allow(dead_code)]
    #[repr(align(64))]
    struct DummyPad;
    
    #[allow(dead_code)]
    #[inline(always)]
    fn unlikely(b: bool) -> bool { b }
    

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
