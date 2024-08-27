use std::vec::IntoIter;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;

use super::{
    columns::{LongArithmeticAuxCols, LongArithmeticCols, LongArithmeticIoCols},
    num_limbs, CalculationResult, LongArithmeticChip, LongArithmeticInstruction,
    LongArithmeticOperation,
};
use crate::{
    arch::{columns::ExecutionState, instructions::Opcode},
    memory::{
        manager::trace_builder::MemoryTraceBuilder,
        offline_checker::columns::MemoryOfflineCheckerAuxCols, OpType,
    },
};

pub fn create_row_from_operation<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32>(
    operation: &LongArithmeticOperation<F>,
    is_valid: bool,
    oc_aux_iter: &mut IntoIter<MemoryOfflineCheckerAuxCols<1, F>>,
) -> Vec<F> {
    LongArithmeticCols::<ARG_SIZE, LIMB_SIZE, F> {
        io: LongArithmeticIoCols {
            instruction: operation.instruction.clone(),
            x_limbs: operation
                .operand1
                .iter()
                .map(|x| F::from_canonical_u32(*x))
                .collect(),
            y_limbs: operation
                .operand2
                .iter()
                .map(|x| F::from_canonical_u32(*x))
                .collect(),
            z_limbs: operation
                .result
                .result_limbs
                .iter()
                .map(|x| F::from_canonical_u32(*x))
                .collect(),
            cmp_result: F::from_canonical_u8(operation.result.cmp_result as u8),
        },
        aux: LongArithmeticAuxCols {
            is_valid: F::from_bool(is_valid),
            opcode_add_flag: F::from_bool(
                operation.instruction.opcode.as_canonical_u32() == Opcode::ADD256 as u32,
            ),
            opcode_sub_flag: F::from_bool(
                operation.instruction.opcode.as_canonical_u32() == Opcode::SUB256 as u32,
            ),
            opcode_lt_flag: F::from_bool(
                operation.instruction.opcode.as_canonical_u32() == Opcode::LT256 as u32,
            ),
            opcode_eq_flag: F::from_bool(
                operation.instruction.opcode.as_canonical_u32() == Opcode::EQ256 as u32,
            ),
            buffer: operation
                .result
                .buffer_limbs
                .iter()
                .map(|x| F::from_canonical_u32(*x))
                .collect(),
            mem_oc_aux_cols: (0..num_limbs::<ARG_SIZE, LIMB_SIZE>())
                .map(|_| oc_aux_iter.next().unwrap())
                .collect(),
        },
    }
    .flatten()
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32>
    LongArithmeticChip<ARG_SIZE, LIMB_SIZE, F>
{
    fn make_blank_row(&self) -> Vec<F> {
        let mut trace_builder = MemoryTraceBuilder::new(self.memory_manager.clone());

        let timestamp = self
            .memory_manager
            .borrow_mut()
            .timestamp()
            .as_canonical_u32() as usize;

        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();

        trace_builder.disabled_op(F::from_canonical_usize(num_limbs), OpType::Read);
        trace_builder.disabled_op(F::from_canonical_usize(num_limbs), OpType::Read);
        trace_builder.disabled_op(F::from_canonical_usize(num_limbs), OpType::Write);
        let mut mem_oc_aux_iter = trace_builder.take_accesses_buffer().into_iter();

        create_row_from_operation::<ARG_SIZE, LIMB_SIZE, F>(
            &LongArithmeticOperation {
                instruction: LongArithmeticInstruction {
                    opcode: F::from_canonical_u8(self.air.base_op as u8),
                    from_state: ExecutionState {
                        pc: F::zero(),
                        timestamp: F::from_canonical_usize(timestamp),
                    },
                    x_address: Default::default(),
                    y_address: Default::default(),
                    z_address: Default::default(),
                },
                operand1: vec![0; num_limbs],
                operand2: vec![0; num_limbs],
                result: CalculationResult {
                    result_limbs: vec![0; num_limbs],
                    buffer_limbs: vec![0; num_limbs],
                    cmp_result: false,
                },
            },
            false,
            &mut mem_oc_aux_iter,
        )
    }
}

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize, F: PrimeField32>
    LongArithmeticChip<ARG_SIZE, LIMB_SIZE, F>
{
    pub fn generate_trace(&mut self) -> RowMajorMatrix<F> {
        let accesses = self.memory.take_accesses_buffer();
        let mut accesses_iter = accesses.into_iter();

        let rows = self
            .operations
            .iter()
            .map(|operation| {
                create_row_from_operation::<ARG_SIZE, LIMB_SIZE, F>(
                    operation,
                    true,
                    &mut accesses_iter,
                )
            })
            .collect::<Vec<_>>();

        let height = rows.len();
        let padded_height = height.next_power_of_two();

        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();

        let blank_row = create_row_from_operation::<ARG_SIZE, LIMB_SIZE, F>(
            &LongArithmeticOperation {
                instruction: LongArithmeticInstruction {
                    opcode: F::from_canonical_u8(self.air.base_op as u8),
                    from_state: ExecutionState {
                        pc: F::zero(),
                        timestamp: F::zero(),
                    },
                    x_address: Default::default(),
                    y_address: Default::default(),
                    z_address: Default::default(),
                },
                operand1: vec![0; num_limbs],
                operand2: vec![0; num_limbs],
                result: CalculationResult {
                    result_limbs: vec![0; num_limbs],
                    buffer_limbs: vec![0; num_limbs],
                    cmp_result: false,
                },
            },
            false,
            &mut accesses_iter,
        );
        // set rcv_count to 0
        let blank_row = [vec![F::zero()], blank_row[1..].to_vec()].concat();
        let width = blank_row.len();

        let mut padded_rows = rows;
        padded_rows.extend(std::iter::repeat(blank_row).take(padded_height - height));

        RowMajorMatrix::new(padded_rows.concat(), width)
    }
}
