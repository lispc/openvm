use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    sync::{Arc, OnceLock},
};

use ax_circuit_derive::AlignedBorrow;
use ax_stark_backend::{
    config::{StarkGenericConfig, Val},
    interaction::InteractionBuilder,
    prover::types::AirProofInput,
    rap::{get_air_name, AnyRap, BaseAirWithPublicValues, PartitionedBaseAir},
    Chip, ChipUsageGetter,
};
use axvm_instructions::{
    instruction::Instruction, program::DEFAULT_PC_STEP, PhantomDiscriminant, SysPhantom,
    SystemOpcode, UsizeOpcode,
};
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{AbstractField, Field, PrimeField32};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_maybe_rayon::prelude::*;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::{
    arch::{
        ExecutionBridge, ExecutionBus, ExecutionError, ExecutionState, InstructionExecutor,
        PcIncOrSet, PhantomSubExecutor, Streams,
    },
    system::{memory::MemoryControllerRef, program::ProgramBus},
};

#[cfg(test)]
mod tests;

/// PhantomAir still needs columns for each nonzero operand in a phantom instruction.
/// We currently allow `a,b,c` where the lower 16 bits of `c` are used as the [PhantomInstruction] discriminant.
const NUM_PHANTOM_OPERANDS: usize = 3;

#[derive(Clone, Debug)]
pub struct PhantomAir {
    pub execution_bridge: ExecutionBridge,
    /// Global opcode for PhantomOpcode
    pub phantom_opcode: usize,
}

#[derive(AlignedBorrow, Copy, Clone)]
pub struct PhantomCols<T> {
    pub pc: T,
    pub operands: [T; NUM_PHANTOM_OPERANDS],
    pub timestamp: T,
    pub is_valid: T,
}

impl<F: Field> BaseAir<F> for PhantomAir {
    fn width(&self) -> usize {
        PhantomCols::<F>::width()
    }
}
impl<F: Field> PartitionedBaseAir<F> for PhantomAir {}
impl<F: Field> BaseAirWithPublicValues<F> for PhantomAir {}

impl<AB: AirBuilder + InteractionBuilder> Air<AB> for PhantomAir {
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let &PhantomCols {
            pc,
            operands,
            timestamp,
            is_valid,
        } = (*local).borrow();

        self.execution_bridge
            .execute_and_increment_or_set_pc(
                AB::Expr::from_canonical_usize(self.phantom_opcode),
                operands,
                ExecutionState::<AB::Expr>::new(pc, timestamp),
                AB::Expr::ONE,
                PcIncOrSet::Inc(AB::Expr::from_canonical_u32(DEFAULT_PC_STEP)),
            )
            .eval(builder, is_valid);
    }
}

pub struct PhantomChip<F> {
    pub air: PhantomAir,
    pub rows: Vec<PhantomCols<F>>,
    memory: MemoryControllerRef<F>,
    streams: OnceLock<Arc<Mutex<Streams<F>>>>,
    phantom_executors: FxHashMap<PhantomDiscriminant, Box<dyn PhantomSubExecutor<F>>>,
}

impl<F> PhantomChip<F> {
    pub fn new(
        execution_bus: ExecutionBus,
        program_bus: ProgramBus,
        memory_controller: MemoryControllerRef<F>,
        offset: usize,
    ) -> Self {
        Self {
            air: PhantomAir {
                execution_bridge: ExecutionBridge::new(execution_bus, program_bus),
                phantom_opcode: offset + SystemOpcode::PHANTOM.as_usize(),
            },
            rows: vec![],
            memory: memory_controller,
            streams: OnceLock::new(),
            phantom_executors: FxHashMap::default(),
        }
    }

    pub fn set_streams(&mut self, streams: Arc<Mutex<Streams<F>>>) {
        if self.streams.set(streams).is_err() {
            panic!("Streams should only be set once");
        }
    }

    pub(crate) fn add_sub_executor<P: PhantomSubExecutor<F> + 'static>(
        &mut self,
        sub_executor: P,
        discriminant: PhantomDiscriminant,
    ) -> Option<Box<dyn PhantomSubExecutor<F>>> {
        self.phantom_executors
            .insert(discriminant, Box::new(sub_executor))
    }
}

impl<F: PrimeField32> InstructionExecutor<F> for PhantomChip<F> {
    fn execute(
        &mut self,
        instruction: Instruction<F>,
        from_state: ExecutionState<u32>,
    ) -> Result<ExecutionState<u32>, ExecutionError> {
        let Instruction {
            opcode, a, b, c, ..
        } = instruction;
        assert_eq!(opcode, self.air.phantom_opcode);

        let c_u32 = c.as_canonical_u32();
        let discriminant = PhantomDiscriminant(c_u32 as u16);
        // If not a system phantom sub-instruction (which is handled in
        // ExecutionSegment), look for a phantom sub-executor to handle it.
        if SysPhantom::from_repr(discriminant.0).is_none() {
            let sub_executor = self
                .phantom_executors
                .get_mut(&discriminant)
                .ok_or_else(|| ExecutionError::PhantomNotFound {
                    pc: from_state.pc,
                    discriminant,
                })?;
            let memory = RefCell::borrow(&self.memory);
            let mut streams = self.streams.get().unwrap().lock();
            sub_executor
                .as_mut()
                .phantom_execute(
                    &memory,
                    &mut streams,
                    discriminant,
                    a,
                    b,
                    (c_u32 >> 16) as u16,
                )
                .map_err(|e| ExecutionError::Phantom {
                    pc: from_state.pc,
                    discriminant,
                    inner: e,
                })?;
        }

        self.rows.push(PhantomCols {
            pc: F::from_canonical_u32(from_state.pc),
            operands: [a, b, c],
            timestamp: F::from_canonical_u32(from_state.timestamp),
            is_valid: F::ONE,
        });
        RefCell::borrow_mut(&self.memory).increment_timestamp();
        Ok(ExecutionState::new(
            from_state.pc + DEFAULT_PC_STEP,
            from_state.timestamp + 1,
        ))
    }

    fn get_opcode_name(&self, _: usize) -> String {
        format!("{:?}", SystemOpcode::PHANTOM)
    }
}

impl<F: PrimeField32> ChipUsageGetter for PhantomChip<F> {
    fn air_name(&self) -> String {
        get_air_name(&self.air)
    }
    fn current_trace_height(&self) -> usize {
        self.rows.len()
    }
    fn trace_width(&self) -> usize {
        PhantomCols::<F>::width()
    }
    fn current_trace_cells(&self) -> usize {
        self.trace_width() * self.current_trace_height()
    }
}

impl<SC: StarkGenericConfig> Chip<SC> for PhantomChip<Val<SC>>
where
    Val<SC>: PrimeField32,
{
    fn air(&self) -> Arc<dyn AnyRap<SC>> {
        Arc::new(self.air.clone())
    }

    fn generate_air_proof_input(self) -> AirProofInput<SC> {
        let correct_height = self.rows.len().next_power_of_two();
        let width = PhantomCols::<Val<SC>>::width();
        let mut rows = Val::<SC>::zero_vec(width * correct_height);
        rows.par_chunks_mut(width)
            .zip(&self.rows)
            .for_each(|(row, row_record)| {
                let row: &mut PhantomCols<_> = row.borrow_mut();
                *row = *row_record;
            });
        let trace = RowMajorMatrix::new(rows, width);

        AirProofInput::simple(self.air(), trace, vec![])
    }
}
