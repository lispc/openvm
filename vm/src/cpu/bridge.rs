use std::collections::BTreeMap;

use afs_stark_backend::interaction::InteractionBuilder;
use p3_field::AbstractField;

use super::{
    columns::{CpuIoCols, MemoryAccessCols},
    CpuAir, OpCode, ARITHMETIC_BUS, CPU_MAX_ACCESSES_PER_CYCLE, CPU_MAX_READS_PER_CYCLE,
    FIELD_ARITHMETIC_INSTRUCTIONS, FIELD_EXTENSION_BUS, FIELD_EXTENSION_INSTRUCTIONS, MEMORY_BUS,
    POSEIDON2_BUS, READ_INSTRUCTION_BUS,
};
use crate::{
    cpu::{
        OpCode::{COMP_POS2, F_LESS_THAN, PERM_POS2},
        IS_LESS_THAN_BUS,
    },
    memory::{MemoryAccess, OpType},
};

impl<const WORD_SIZE: usize> CpuAir<WORD_SIZE> {
    pub fn eval_interactions<AB: InteractionBuilder>(
        &self,
        builder: &mut AB,
        io: CpuIoCols<AB::Var>,
        accesses: [MemoryAccessCols<WORD_SIZE, AB::Var>; CPU_MAX_ACCESSES_PER_CYCLE],
        operation_flags: &BTreeMap<OpCode, AB::Var>,
    ) {
        // Interaction with program (bus 0)
        builder.push_send(
            READ_INSTRUCTION_BUS,
            [io.pc, io.opcode, io.op_a, io.op_b, io.op_c, io.d, io.e],
            AB::Expr::one() - operation_flags[&OpCode::NOP],
        );

        for (i, access_cols) in accesses.into_iter().enumerate() {
            let memory_cycle = io.timestamp + AB::F::from_canonical_usize(i);
            let is_write = i >= CPU_MAX_READS_PER_CYCLE;

            let access = MemoryAccess {
                timestamp: memory_cycle,
                op_type: if is_write {
                    OpType::Write
                } else {
                    OpType::Read
                },
                address_space: access_cols.address_space.into(),
                address: access_cols.address.into(),
                data: access_cols.data.map(|x| x.into()),
            };

            let count = access_cols.enabled - access_cols.is_immediate;
            MEMORY_BUS.send_interaction(builder, access, count);
        }

        // Interaction with arithmetic (bus 2)
        if self.options.field_arithmetic_enabled {
            let fields = [
                io.opcode,
                accesses[0].data[0],
                accesses[1].data[0],
                accesses[CPU_MAX_READS_PER_CYCLE].data[0],
            ];
            let count = FIELD_ARITHMETIC_INSTRUCTIONS
                .iter()
                .fold(AB::Expr::zero(), |acc, opcode| {
                    acc + operation_flags[opcode]
                });
            builder.push_send(ARITHMETIC_BUS, fields, count);
        }

        // Interaction with field extension arithmetic (bus 3)
        if self.options.field_extension_enabled {
            let fields = [io.opcode, io.op_a, io.op_b, io.op_c, io.d, io.e];
            let count = FIELD_EXTENSION_INSTRUCTIONS
                .iter()
                .fold(AB::Expr::zero(), |acc, opcode| {
                    acc + operation_flags[opcode]
                });
            builder.push_send(FIELD_EXTENSION_BUS, fields, count);
        }

        // Interaction with poseidon2 (bus 5)
        if self.options.poseidon2_enabled() {
            let compression = io.opcode - AB::F::from_canonical_usize(PERM_POS2 as usize);
            let fields = [io.timestamp, io.op_a, io.op_b, io.op_c, io.d, io.e]
                .into_iter()
                .map(Into::into)
                .chain([compression]);

            let mut count = AB::Expr::zero();
            if self.options.compress_poseidon2_enabled {
                count = count + operation_flags[&COMP_POS2];
            }
            if self.options.perm_poseidon2_enabled {
                count = count + operation_flags[&PERM_POS2];
            }
            builder.push_send(POSEIDON2_BUS, fields, count);
        }

        if self.options.is_less_than_enabled {
            let fields = [
                accesses[0].data[0],
                accesses[1].data[0],
                accesses[CPU_MAX_READS_PER_CYCLE].data[0],
            ];
            let count = operation_flags[&F_LESS_THAN];
            builder.push_send(IS_LESS_THAN_BUS, fields, count);
        }
    }
}
