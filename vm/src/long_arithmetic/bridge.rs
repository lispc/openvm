use afs_stark_backend::interaction::InteractionBuilder;
use p3_field::AbstractField;

use super::{air::LongArithmeticAir, columns::LongArithmeticCols, num_limbs};
use crate::{
    arch::columns::InstructionCols,
    memory::{offline_checker::bridge::MemoryBridge, MemoryAddress},
};

impl<const ARG_SIZE: usize, const LIMB_SIZE: usize> LongArithmeticAir<ARG_SIZE, LIMB_SIZE> {
    pub fn eval_interactions<AB: InteractionBuilder>(
        &self,
        builder: &mut AB,
        local: LongArithmeticCols<ARG_SIZE, LIMB_SIZE, AB::Var>,
    ) {
        let LongArithmeticCols { io, aux } = local;
        let instruction = &io.instruction;
        let num_limbs = num_limbs::<ARG_SIZE, LIMB_SIZE>();
        self.execution_bus.execute_increment_pc(
            builder,
            aux.is_valid,
            instruction.from_state.map(Into::into),
            aux.is_valid.into() * AB::F::from_canonical_usize(2 * num_limbs + 1)
                + (aux.opcode_add_flag.into() + aux.opcode_sub_flag.into())
                    * AB::F::from_canonical_usize(num_limbs - 1),
            InstructionCols::new(
                instruction.opcode,
                [
                    instruction.z_address.address,
                    instruction.x_address.address,
                    instruction.y_address.address,
                    instruction.z_address.address_space,
                    instruction.x_address.address_space,
                    instruction.y_address.address_space,
                ],
            ),
        );

        let mut memory_bridge = MemoryBridge::new(self.mem_oc, aux.mem_oc_aux_cols);
        let mut timestamp: AB::Expr = instruction.from_state.timestamp.into();
        for i in 0..num_limbs {
            memory_bridge
                .read(
                    MemoryAddress::new(
                        instruction.x_address.address_space,
                        instruction.x_address.address.into() + AB::Expr::from_canonical_usize(i),
                    ),
                    [io.x_limbs[i]],
                    timestamp.clone(),
                )
                .eval(builder, aux.is_valid);
            timestamp += aux.is_valid.into();
        }
        for i in 0..num_limbs {
            memory_bridge
                .read(
                    MemoryAddress::new(
                        instruction.y_address.address_space,
                        instruction.y_address.address.into() + AB::Expr::from_canonical_usize(i),
                    ),
                    [io.y_limbs[i]],
                    timestamp.clone(),
                )
                .eval(builder, aux.is_valid);
            timestamp += aux.is_valid.into();
        }
        for i in 0..num_limbs {
            memory_bridge
                .write(
                    MemoryAddress::new(
                        instruction.z_address.address_space,
                        instruction.z_address.address.into() + AB::Expr::from_canonical_usize(i),
                    ),
                    [io.z_limbs[i]],
                    timestamp.clone(),
                )
                .eval(
                    builder,
                    aux.opcode_add_flag.into() + aux.opcode_sub_flag.into(),
                );
            timestamp += aux.is_valid.into();
        }
        memory_bridge
            .write(
                MemoryAddress::new(
                    instruction.z_address.address_space,
                    instruction.z_address.address,
                ),
                [io.cmp_result],
                timestamp,
            )
            .eval(
                builder,
                aux.opcode_lt_flag.into() + aux.opcode_eq_flag.into(),
            );

        for z in io.z_limbs {
            builder.push_send(
                self.bus_index,
                vec![z],
                aux.opcode_add_flag + aux.opcode_sub_flag + aux.opcode_lt_flag,
            );
        }
    }
}
