use afs_stark_backend::interaction::InteractionBuilder;

use crate::traits::{ExecutionState, InstructionCols};

pub struct ExecutionBus {
    bus_index: usize,
}

impl ExecutionBus {
    pub fn new(bus_index: usize) -> Self {
        ExecutionBus { bus_index }
    }
    pub fn interact_execute<AB: InteractionBuilder>(
        &self,
        builder: &mut AB,
        prev_state: ExecutionState<AB::Expr>,
        next_state: ExecutionState<AB::Expr>,
        instruction: InstructionCols<AB::Expr>,
    ) {
    }
    pub fn interact_initial_final<AB: InteractionBuilder>(
        &self,
        builder: &mut AB,
        prev_state: ExecutionState<AB::Expr>,
        next_state: ExecutionState<AB::Expr>,
    ) {
    }
}
