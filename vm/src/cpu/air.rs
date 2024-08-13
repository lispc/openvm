use std::borrow::Borrow;

use afs_primitives::{
    is_equal_vec::{columns::IsEqualVecIoCols, IsEqualVecAir},
    sub_chip::SubAir,
};
use afs_stark_backend::interaction::InteractionBuilder;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir};
use p3_field::{AbstractField, Field};
use p3_matrix::Matrix;

use super::{
    columns::{CpuAuxCols, CpuCols, CpuIoCols},
    max_accesses_per_instruction, CpuAir,
    OpCode::*,
    CPU_MAX_READS_PER_CYCLE, FIELD_ARITHMETIC_INSTRUCTIONS, INST_WIDTH,
};

impl<const WORD_SIZE: usize, F: Field> BaseAir<F> for CpuAir<WORD_SIZE> {
    fn width(&self) -> usize {
        CpuCols::<WORD_SIZE, F>::get_width(self)
    }
}

impl<const WORD_SIZE: usize> CpuAir<WORD_SIZE> {
    fn assert_compose<AB: AirBuilder>(
        &self,
        builder: &mut AB,
        word: [AB::Var; WORD_SIZE],
        field_elem: AB::Expr,
    ) {
        builder.assert_eq(word[0], field_elem);
        for &cell in word.iter().take(WORD_SIZE).skip(1) {
            builder.assert_zero(cell);
        }
    }
}

// TODO[osama]: here, there should be some relation enforced between the timestamp for the cpu and the memory timestamp
// TODO[osama]: also, rename to clk
impl<const WORD_SIZE: usize, AB: AirBuilderWithPublicValues + InteractionBuilder> Air<AB>
    for CpuAir<WORD_SIZE>
{
    // TODO: continuation verification checks program counters match up [INT-1732]
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let pis = builder.public_values();

        let start_pc = pis[0];
        let end_pc = pis[1];

        let inst_width = AB::F::from_canonical_usize(INST_WIDTH);

        let local = main.row_slice(0);
        let local: &[AB::Var] = (*local).borrow();
        let local_cols = CpuCols::<WORD_SIZE, AB::Var>::from_slice(local, self);

        let next = main.row_slice(1);
        let next: &[AB::Var] = (*next).borrow();
        let next_cols = CpuCols::<WORD_SIZE, AB::Var>::from_slice(next, self);
        let CpuCols { io, aux } = local_cols;
        let CpuCols { io: next_io, .. } = next_cols;

        let CpuIoCols {
            timestamp,
            pc,
            opcode,
            op_a: a,
            op_b: b,
            op_c: c,
            d,
            e,
        } = io;
        let CpuIoCols {
            timestamp: next_timestamp,
            pc: next_pc,
            ..
        } = next_io;

        let CpuAuxCols {
            operation_flags,
            public_value_flags,
            accesses,
            read0_equals_read1,
            is_equal_vec_aux,
        } = aux;

        let read1 = &accesses[0];
        let read2 = &accesses[1];
        let write = &accesses[CPU_MAX_READS_PER_CYCLE];

        // assert that the start pc is correct
        builder.when_first_row().assert_eq(pc, start_pc);
        builder.when_last_row().assert_eq(pc, end_pc);

        // set correct operation flag
        for &flag in operation_flags.values() {
            builder.assert_bool(flag);
        }

        let mut sum_flags = AB::Expr::zero();
        let mut match_opcode = AB::Expr::zero();
        for (&opcode, &flag) in operation_flags.iter() {
            sum_flags = sum_flags + flag;
            match_opcode += flag * AB::F::from_canonical_usize(opcode as usize);
        }
        builder.assert_one(sum_flags);
        builder.assert_eq(opcode, match_opcode);

        // keep track of when memory accesses should be enabled
        let mut read1_enabled_check = AB::Expr::zero();
        let mut read2_enabled_check = AB::Expr::zero();
        let mut write_enabled_check = AB::Expr::zero();

        // LOADW: d[a] <- e[d[c] + b]
        let loadw_flag = operation_flags[&LOADW];
        read1_enabled_check = read1_enabled_check + loadw_flag;
        read2_enabled_check = read2_enabled_check + loadw_flag;
        write_enabled_check = write_enabled_check + loadw_flag;

        let mut when_loadw = builder.when(loadw_flag);

        when_loadw.assert_eq(read1.addr_space(), d);
        when_loadw.assert_eq(read1.pointer(), c);

        when_loadw.assert_eq(read2.addr_space(), e);
        self.assert_compose(&mut when_loadw, read1.data(), read2.pointer() - b);

        when_loadw.assert_eq(write.addr_space(), d);
        when_loadw.assert_eq(write.pointer(), a);
        for i in 0..WORD_SIZE {
            when_loadw.assert_eq(write.data()[i], read2.data()[i]);
        }

        when_loadw
            .when_transition()
            .assert_eq(next_pc, pc + inst_width);

        // STOREW: e[d[c] + b] <- d[a]
        let storew_flag = operation_flags[&STOREW];
        read1_enabled_check = read1_enabled_check + storew_flag;
        read2_enabled_check = read2_enabled_check + storew_flag;
        write_enabled_check = write_enabled_check + storew_flag;

        let mut when_storew = builder.when(storew_flag);
        when_storew.assert_eq(read1.addr_space(), d);
        when_storew.assert_eq(read1.pointer(), c);

        when_storew.assert_eq(read2.addr_space(), d);
        when_storew.assert_eq(read2.pointer(), a);

        when_storew.assert_eq(write.addr_space(), e);
        self.assert_compose(&mut when_storew, read1.data(), write.pointer() - b);
        for i in 0..WORD_SIZE {
            when_storew.assert_eq(write.data()[i], read2.data()[i]);
        }

        when_storew
            .when_transition()
            .assert_eq(next_pc, pc + inst_width);

        // SHINTW: e[d[a] + b] <- ?
        let shintw_flag = operation_flags[&SHINTW];
        read1_enabled_check = read1_enabled_check + shintw_flag;
        write_enabled_check = write_enabled_check + shintw_flag;

        let mut when_shintw = builder.when(shintw_flag);
        when_shintw.assert_eq(read1.addr_space(), d);
        when_shintw.assert_eq(read1.pointer(), a);

        when_shintw.assert_eq(write.addr_space(), e);
        self.assert_compose(&mut when_shintw, read1.data(), write.pointer() - b);

        when_shintw
            .when_transition()
            .assert_eq(next_pc, pc + inst_width);

        // JAL: d[a] <- pc + INST_WIDTH, pc <- pc + b
        let jal_flag = operation_flags[&JAL];
        write_enabled_check = write_enabled_check + jal_flag;

        let mut when_jal = builder.when(jal_flag);

        when_jal.assert_eq(write.addr_space(), d);
        when_jal.assert_eq(write.pointer(), a);
        self.assert_compose(&mut when_jal, write.data(), pc + inst_width);

        when_jal.when_transition().assert_eq(next_pc, pc + b);

        // BEQ: If d[a] = e[b], pc <- pc + c
        let beq_flag = operation_flags[&BEQ];
        read1_enabled_check = read1_enabled_check + beq_flag;
        read2_enabled_check = read2_enabled_check + beq_flag;

        let mut when_beq = builder.when(beq_flag);

        when_beq.assert_eq(read1.addr_space(), d);
        when_beq.assert_eq(read1.pointer(), a);

        when_beq.assert_eq(read2.addr_space(), e);
        when_beq.assert_eq(read2.pointer(), b);

        when_beq
            .when_transition()
            .when(read0_equals_read1)
            .assert_eq(next_pc, pc + c);
        when_beq
            .when_transition()
            .when(AB::Expr::one() - read0_equals_read1)
            .assert_eq(next_pc, pc + inst_width);

        // BNE: If d[a] != e[b], pc <- pc + c
        let bne_flag = operation_flags[&BNE];
        read1_enabled_check = read1_enabled_check + bne_flag;
        read2_enabled_check = read2_enabled_check + bne_flag;

        let mut when_bne = builder.when(bne_flag);

        when_bne.assert_eq(read1.addr_space(), d);
        when_bne.assert_eq(read1.pointer(), a);

        when_bne.assert_eq(read2.addr_space(), e);
        when_bne.assert_eq(read2.pointer(), b);

        when_bne
            .when_transition()
            .when(read0_equals_read1)
            .assert_eq(next_pc, pc + inst_width);
        when_bne
            .when_transition()
            .when(AB::Expr::one() - read0_equals_read1)
            .assert_eq(next_pc, pc + c);

        // NOP constraints same pc and timestamp as next row
        let nop_flag = operation_flags[&NOP];
        let mut when_nop = builder.when(nop_flag);
        when_nop.when_transition().assert_eq(next_pc, pc);
        when_nop
            .when_transition()
            .assert_eq(next_timestamp, timestamp);

        // TERMINATE
        let terminate_flag = operation_flags[&TERMINATE];
        let mut when_terminate = builder.when(terminate_flag);
        when_terminate.when_transition().assert_eq(next_pc, pc);

        // PUBLISH

        let publish_flag = operation_flags[&PUBLISH];
        read1_enabled_check = read1_enabled_check + publish_flag;
        read2_enabled_check = read2_enabled_check + publish_flag;

        let mut sum_flags = AB::Expr::zero();
        let mut match_public_value_index = AB::Expr::zero();
        let mut match_public_value = AB::Expr::zero();
        for (i, &flag) in public_value_flags.iter().enumerate() {
            builder.assert_bool(flag);
            sum_flags = sum_flags + flag;
            match_public_value_index += flag * AB::F::from_canonical_usize(i);
            match_public_value += flag * builder.public_values()[i + 2].into();
        }

        let mut when_publish = builder.when(publish_flag);

        when_publish.assert_one(sum_flags);
        self.assert_compose(&mut when_publish, read1.data(), match_public_value_index);
        self.assert_compose(&mut when_publish, read2.data(), match_public_value);

        when_publish.assert_eq(read1.addr_space(), d);
        when_publish.assert_eq(read1.pointer(), a);

        when_publish.assert_eq(read2.addr_space(), e);
        when_publish.assert_eq(read2.pointer(), b);

        // arithmetic operations
        if self.options.field_arithmetic_enabled {
            let mut arithmetic_flags = AB::Expr::zero();
            for opcode in FIELD_ARITHMETIC_INSTRUCTIONS {
                arithmetic_flags += operation_flags[&opcode].into();
            }
            read1_enabled_check += arithmetic_flags.clone();
            read2_enabled_check += arithmetic_flags.clone();
            write_enabled_check += arithmetic_flags.clone();
            let mut when_arithmetic = builder.when(arithmetic_flags);

            // read from d[b] and e[c]
            when_arithmetic.assert_eq(read1.addr_space(), d);
            when_arithmetic.assert_eq(read1.pointer(), b);

            when_arithmetic.assert_eq(read2.addr_space(), e);
            when_arithmetic.assert_eq(read2.pointer(), c);

            // write to d[a]
            when_arithmetic.assert_eq(write.addr_space(), d);
            when_arithmetic.assert_eq(write.pointer(), a);

            when_arithmetic
                .when_transition()
                .assert_eq(next_pc, pc + inst_width);
        }

        // immediate calculation

        for oc_cols in [read1, read2, write] {
            SubAir::eval(&self.memory_offline_checker, builder, oc_cols.clone(), ());
        }
        // maybe writes to immediate address space are ignored instead of disallowed?
        //builder.assert_zero(write.is_immediate);

        // evaluate equality between read1 and read2

        let is_equal_vec_io_cols = IsEqualVecIoCols {
            x: read1.data().to_vec(),
            y: read2.data().to_vec(),
            is_equal: read0_equals_read1,
        };
        SubAir::eval(
            &IsEqualVecAir::new(WORD_SIZE),
            builder,
            is_equal_vec_io_cols,
            is_equal_vec_aux,
        );

        // update the timestamp correctly
        for (&opcode, &flag) in operation_flags.iter() {
            if opcode != TERMINATE && opcode != NOP {
                builder.when(flag).assert_eq(
                    next_timestamp,
                    timestamp + AB::F::from_canonical_usize(max_accesses_per_instruction(opcode)),
                )
            }
        }

        // make sure program terminates or shards with NOP
        builder.when_last_row().assert_zero(
            (opcode - AB::Expr::from_canonical_usize(TERMINATE as usize))
                * (opcode - AB::Expr::from_canonical_usize(NOP as usize)),
        );

        // check accesses enabled
        builder.assert_eq(read1.enabled, read1_enabled_check);
        builder.assert_eq(read2.enabled, read2_enabled_check);
        builder.assert_eq(write.enabled, write_enabled_check);

        // Turn on all interactions
        self.eval_interactions(builder, io, accesses, &operation_flags);
    }
}
