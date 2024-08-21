use std::array;
use afs_stark_backend::interaction::InteractionBuilder;
use p3_field::AbstractField;

use super::{columns::FieldExtensionArithmeticCols, FieldExtensionArithmeticAir, EXTENSION_DEGREE};
use crate::{
    cpu::FIELD_EXTENSION_BUS,
    field_extension::columns::FieldExtensionArithmeticAuxCols,
    memory::{offline_checker::bridge::MemoryBridge, MemoryAddress},
};

#[allow(clippy::too_many_arguments)]
fn eval_rw_interactions<AB: InteractionBuilder, const WORD_SIZE: usize>(
    builder: &mut AB,
    memory_bridge: &mut MemoryBridge<AB::Var, WORD_SIZE>,
    clk_offset: &mut usize,
    is_write: bool,
    start_timestamp: AB::Var,
    addr_space: AB::Var,
    address: AB::Var,
    ext: [AB::Var; EXTENSION_DEGREE],
) {
    for (i, element) in ext.into_iter().enumerate() {
        let pointer = address + AB::F::from_canonical_usize(i * WORD_SIZE);

        let clk = start_timestamp + AB::Expr::from_canonical_usize(*clk_offset);
        *clk_offset += 1;

        if is_write {
            memory_bridge
                .write(
                    MemoryAddress::new(addr_space, pointer),
                    emb(element.into()),
                    clk,
                )
                .eval(builder, AB::F::one());
        } else {
            memory_bridge
                .read(
                    MemoryAddress::new(addr_space, pointer),
                    emb(element.into()),
                    clk,
                )
                .eval(builder, AB::F::one());
        }
    }
}

fn emb<F: AbstractField, const WORD_SIZE: usize>(element: F) -> [F; WORD_SIZE] {
    array::from_fn(|j| if j == 0 { element.clone() } else { F::zero() })
}

impl<const WORD_SIZE: usize> FieldExtensionArithmeticAir<WORD_SIZE> {
    pub fn eval_interactions<AB: InteractionBuilder>(
        &self,
        builder: &mut AB,
        local: FieldExtensionArithmeticCols<WORD_SIZE, AB::Var>,
    ) {
        let mut clk_offset = 0;

        let FieldExtensionArithmeticCols { io, aux } = local;

        let FieldExtensionArithmeticAuxCols {
            op_a,
            op_b,
            op_c,
            d,
            e,
            mem_oc_aux_cols,
            is_valid,
            ..
        } = aux;

        let mut memory_bridge = MemoryBridge::new(self.mem_oc, mem_oc_aux_cols);

        // Reads for x
        eval_rw_interactions::<AB, WORD_SIZE>(
            builder,
            &mut memory_bridge,
            &mut clk_offset,
            false,
            io.clk,
            d,
            op_b,
            io.x,
        );

        // Reads for y
        eval_rw_interactions::<AB, WORD_SIZE>(
            builder,
            &mut memory_bridge,
            &mut clk_offset,
            false,
            io.clk,
            e,
            op_c,
            io.y,
        );

        // Writes for z
        eval_rw_interactions::<AB, WORD_SIZE>(
            builder,
            &mut memory_bridge,
            &mut clk_offset,
            true,
            io.clk,
            d,
            op_a,
            io.z,
        );

        // Receives all IO columns from another chip on bus 3 (FIELD_EXTENSION_BUS)
        builder.push_receive(
            FIELD_EXTENSION_BUS,
            [io.opcode, io.clk, op_a, op_b, op_c, d, e],
            is_valid,
        );
    }
}
