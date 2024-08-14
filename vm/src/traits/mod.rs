use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::StarkGenericConfig;

use afs_stark_backend::rap::AnyRap;

use crate::cpu::trace::Instruction;

mod bridge;
mod simple;

#[derive(Clone, Copy)]
pub struct ExecutionState<T> {
    pub pc: T,
    pub timestamp: T,
}

#[derive(Clone)]
pub struct InstructionCols<T> {
    pub opcode: T,
    pub a: T,
    pub b: T,
    pub c: T,
    pub d: T,
    pub e: T,
}

pub trait OpCodeExecutor<F> {
    fn execute(
        &mut self,
        instruction: &Instruction<F>,
        prev_state: ExecutionState<usize>,
    ) -> ExecutionState<usize>;
}

pub trait MachineChip<F> {
    fn generate_trace(&mut self) -> RowMajorMatrix<F>;
    fn air<SC: StarkGenericConfig>(&self) -> &dyn AnyRap<SC>;
}
