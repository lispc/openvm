use p3_baby_bear::BabyBear;
use p3_field::PrimeField32;

use crate::{
    arch::instructions::{
        CORE_INSTRUCTIONS, FIELD_ARITHMETIC_INSTRUCTIONS, FIELD_EXTENSION_INSTRUCTIONS, OpCode,
        OpCode::*,
    },
    field_extension::FieldExtensionArithmeticAir,
    memory::offline_checker::air::NewMemoryOfflineChecker,
    poseidon2::Poseidon2Chip,
};

#[cfg(test)]
pub mod tests;

pub mod air;
pub mod bridge;
pub mod columns;
pub mod trace;

pub const INST_WIDTH: usize = 1;

pub const READ_INSTRUCTION_BUS: usize = 0;
pub const MEMORY_BUS: usize = 1;
pub const ARITHMETIC_BUS: usize = 2;
pub const FIELD_EXTENSION_BUS: usize = 3;
pub const RANGE_CHECKER_BUS: usize = 4;
pub const POSEIDON2_BUS: usize = 5;
pub const POSEIDON2_DIRECT_BUS: usize = 6;
// TODO[osama]: to be renamed to MEMORY_BUS
pub const NEW_MEMORY_BUS: usize = 7;
pub const EXPAND_BUS: usize = 8;
pub const POSEIDON2_DIRECT_REQUEST_BUS: usize = 9;
pub const MEMORY_INTERFACE_BUS: usize = 10;

pub const CPU_MAX_READS_PER_CYCLE: usize = 2;
pub const CPU_MAX_WRITES_PER_CYCLE: usize = 1;
pub const CPU_MAX_ACCESSES_PER_CYCLE: usize = CPU_MAX_READS_PER_CYCLE + CPU_MAX_WRITES_PER_CYCLE;

pub const WORD_SIZE: usize = 1;

fn max_accesses_per_instruction(opcode: OpCode) -> usize {
    match opcode {
        LOADW | STOREW => 3,
        // JAL only does WRITE, but it is done as timestamp + 2
        JAL => 3,
        BEQ | BNE => 2,
        TERMINATE => 0,
        PUBLISH => 2,
        opcode if FIELD_ARITHMETIC_INSTRUCTIONS.contains(&opcode) => 3,
        opcode if FIELD_EXTENSION_INSTRUCTIONS.contains(&opcode) => {
            FieldExtensionArithmeticAir::max_accesses_per_instruction(opcode)
        }
        FAIL => 0,
        PRINTF => 1,
        COMP_POS2 | PERM_POS2 => {
            Poseidon2Chip::<16, BabyBear>::max_accesses_per_instruction(opcode)
        }
        SHINTW => 3,
        HINT_INPUT | HINT_BITS => 0,
        CT_START | CT_END => 0,
        NOP => 0,
        _ => panic!(),
    }
}

#[derive(Default, Clone, Copy)]
pub struct CpuOptions {
    pub field_arithmetic_enabled: bool,
    pub field_extension_enabled: bool,
    pub compress_poseidon2_enabled: bool,
    pub perm_poseidon2_enabled: bool,
    pub num_public_values: usize,
}

#[derive(Default, Clone, Copy)]
/// State of the CPU.
pub struct ExecutionState {
    pub clock_cycle: usize,
    pub timestamp: usize,
    pub pc: usize,
    pub is_done: bool,
}

impl CpuOptions {
    pub fn poseidon2_enabled(&self) -> bool {
        self.compress_poseidon2_enabled || self.perm_poseidon2_enabled
    }

    pub fn enabled_instructions(&self) -> Vec<OpCode> {
        let mut result = CORE_INSTRUCTIONS.to_vec();
        if self.field_extension_enabled {
            result.extend(FIELD_EXTENSION_INSTRUCTIONS);
        }
        if self.field_arithmetic_enabled {
            result.extend(FIELD_ARITHMETIC_INSTRUCTIONS);
        }
        if self.compress_poseidon2_enabled {
            result.push(COMP_POS2);
        }
        if self.perm_poseidon2_enabled {
            result.push(PERM_POS2);
        }
        result
    }

    pub fn num_enabled_instructions(&self) -> usize {
        self.enabled_instructions().len()
    }
}

// #[derive(Clone)]
/// Air for the CPU. Carries no state and does not own execution.
pub struct CpuAir<const WORD_SIZE: usize> {
    pub options: CpuOptions,
    pub memory_offline_checker: NewMemoryOfflineChecker<WORD_SIZE>,
}

impl<const WORD_SIZE: usize> CpuAir<WORD_SIZE> {
    pub fn new(options: CpuOptions, clk_max_bits: usize, decomp: usize) -> Self {
        Self {
            options,
            memory_offline_checker: NewMemoryOfflineChecker::new(clk_max_bits, decomp),
        }
    }
}

/// Chip for the CPU. Carries all state and owns execution.
pub struct CpuChip<const WORD_SIZE: usize, F: Clone> {
    pub air: CpuAir<WORD_SIZE>,
    pub rows: Vec<Vec<F>>,
    pub state: ExecutionState,
    /// Program counter at the start of the current segment.
    pub start_state: ExecutionState,
    /// Public inputs for the current segment.
    pub pis: Vec<F>,
}

impl<const WORD_SIZE: usize, F: Clone> CpuChip<WORD_SIZE, F> {
    pub fn new(options: CpuOptions, clk_max_bits: usize, decomp: usize) -> Self {
        Self {
            air: CpuAir::new(options, clk_max_bits, decomp),
            rows: vec![],
            state: ExecutionState::default(),
            start_state: ExecutionState::default(),
            pis: vec![],
        }
    }

    pub fn current_height(&self) -> usize {
        self.rows.len()
    }

    /// Sets the current state of the CPU.
    pub fn set_state(&mut self, state: ExecutionState) {
        self.state = state;
    }

    /// Sets the current state of the CPU.
    pub fn from_state(
        options: CpuOptions,
        state: ExecutionState,
        clk_max_bits: usize,
        decomp: usize,
    ) -> Self {
        let mut chip = Self::new(options, clk_max_bits, decomp);
        chip.state = state;
        chip.start_state = state;
        chip
    }
}

impl<const WORD_SIZE: usize, F: PrimeField32> CpuChip<WORD_SIZE, F> {
    /// Writes the public inputs for the current segment (beginning and end program counters).
    ///
    /// Should only be called after segment end.
    fn generate_pvs(&mut self) {
        let first_row_pc = self.start_state.pc;
        let last_row_pc = self.state.pc;
        self.pis = vec![
            F::from_canonical_usize(first_row_pc),
            F::from_canonical_usize(last_row_pc),
        ];
    }
}
