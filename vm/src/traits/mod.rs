use enum_dispatch::enum_dispatch;
use p3_air::Air;
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::StarkGenericConfig;

use afs_stark_backend::rap::AnyRap;

use crate::cpu::trace::Instruction;

pub mod bridge;
pub mod simple;

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

#[enum_dispatch]
pub trait OpCodeExecutor<F> {
    fn execute(
        &mut self,
        instruction: &Instruction<F>,
        prev_state: ExecutionState<usize>,
    ) -> ExecutionState<usize>;
}

#[enum_dispatch(OpCodeExecutor<F>)]
pub enum OpCodeExecutorVariant<F> {
    A(A),
}

#[enum_dispatch]
pub trait MachineChip<F> {
    fn generate_trace(&mut self) -> RowMajorMatrix<F>;
    fn air<SC: StarkGenericConfig>(&self) -> Box<dyn AnyRap<SC>>;
    fn get_public_values(&mut self) -> Vec<F> {
        vec![]
    }
}

#[enum_dispatch(MachineChip<F>)]
pub enum MachineChipVariant<F> {
    A(A),
}
