use std::{borrow::BorrowMut, sync::Arc};

use afs_primitives::bigint::{
    check_carry_to_zero::get_carry_max_abs_and_bits,
    utils::{big_int_to_limbs, big_uint_sub},
    CanonicalUint, DefaultLimbConfig, OverflowInt,
};
use afs_stark_backend::{
    config::{StarkGenericConfig, Val},
    rap::{get_air_name, AnyRap},
    Chip,
};
use num_bigint_dig::{BigInt, BigUint, Sign};
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;

use super::{
    columns::{ModularAddSubAuxCols, ModularAddSubCols, ModularAddSubIoCols},
    ModularAddSubChip,
};
use crate::{
    arch::{
        instructions::{ModularArithmeticOpcode, UsizeOpcode},
        VmChip,
    },
    system::memory::MemoryHeapDataIoCols,
    utils::limbs_to_biguint,
};

impl<F: PrimeField32, const NUM_LIMBS: usize, const LIMB_SIZE: usize> VmChip<F>
    for ModularAddSubChip<F, NUM_LIMBS, LIMB_SIZE>
{
    fn generate_trace(self) -> RowMajorMatrix<F> {
        let aux_cols_factory = self.memory_controller.borrow().aux_cols_factory();

        let height = self.data.len();
        let height = height.next_power_of_two();

        let blank_row = vec![F::zero(); ModularAddSubCols::<F, NUM_LIMBS>::width()];
        let mut rows = vec![blank_row; height];

        for (i, record) in self.data.iter().enumerate() {
            let row = &mut rows[i];
            let cols: &mut ModularAddSubCols<F, NUM_LIMBS> = row[..].borrow_mut();
            cols.io = ModularAddSubIoCols {
                from_state: record.from_state.map(F::from_canonical_usize),
                x: MemoryHeapDataIoCols::<F, NUM_LIMBS>::from(record.x_array_read.clone()),
                y: MemoryHeapDataIoCols::<F, NUM_LIMBS>::from(record.y_array_read.clone()),
                z: MemoryHeapDataIoCols::<F, NUM_LIMBS>::from(record.z_array_write.clone()),
            };
            let x = limbs_to_biguint(
                &record
                    .x_array_read
                    .data_read
                    .data
                    .map(|x| x.as_canonical_u32()),
                LIMB_SIZE,
            );
            let y = limbs_to_biguint(
                &record
                    .y_array_read
                    .data_read
                    .data
                    .map(|x| x.as_canonical_u32()),
                LIMB_SIZE,
            );
            let r = limbs_to_biguint(
                &record
                    .z_array_write
                    .data_write
                    .data
                    .map(|x| x.as_canonical_u32()),
                LIMB_SIZE,
            );
            let is_add = match ModularArithmeticOpcode::from_usize(record.instruction.opcode) {
                ModularArithmeticOpcode::ADD => true,
                ModularArithmeticOpcode::SUB => false,
                _ => unreachable!(),
            };

            if is_add {
                self.generate_aux_cols_add(cols.aux.borrow_mut(), x, y, r);
            } else {
                self.generate_aux_cols_sub(cols.aux.borrow_mut(), x, y, r);
            }

            cols.aux.is_valid = F::one();
            cols.aux.read_x_aux_cols =
                aux_cols_factory.make_heap_read_aux_cols(record.x_array_read.clone());
            cols.aux.read_y_aux_cols =
                aux_cols_factory.make_heap_read_aux_cols(record.y_array_read.clone());
            cols.aux.write_z_aux_cols =
                aux_cols_factory.make_heap_write_aux_cols(record.z_array_write.clone());
            cols.aux.is_add = F::from_bool(is_add);
        }

        RowMajorMatrix::new(rows.concat(), ModularAddSubCols::<F, NUM_LIMBS>::width())
    }

    fn air_name(&self) -> String {
        get_air_name(&self.air)
    }

    fn current_trace_height(&self) -> usize {
        self.data.len()
    }

    fn trace_width(&self) -> usize {
        ModularAddSubCols::<F, NUM_LIMBS>::width()
    }
}

impl<SC: StarkGenericConfig, const NUM_LIMBS: usize, const LIMB_SIZE: usize> Chip<SC>
    for ModularAddSubChip<Val<SC>, NUM_LIMBS, LIMB_SIZE>
where
    Val<SC>: PrimeField32,
{
    fn air(&self) -> Arc<dyn AnyRap<SC>> {
        Arc::new(self.air.clone())
    }
}

impl<F: PrimeField32, const NUM_LIMBS: usize, const LIMB_SIZE: usize>
    ModularAddSubChip<F, NUM_LIMBS, LIMB_SIZE>
{
    fn generate_aux_cols_add(
        &self,
        aux: &mut ModularAddSubAuxCols<F, NUM_LIMBS>,
        x: BigUint,
        y: BigUint,
        r: BigUint,
    ) {
        let raw_sum = x.clone() + y.clone();
        let sign = if raw_sum < self.modulus {
            // x + y - r == 0
            Sign::NoSign
        } else {
            Sign::Plus
        };

        let q = BigInt::from_biguint(sign, (raw_sum - r.clone()) / self.modulus.clone());
        self.generate_aux_cols(aux, x, y, r, q, true);
    }
    fn generate_aux_cols_sub(
        &self,
        aux: &mut ModularAddSubAuxCols<F, NUM_LIMBS>,
        x: BigUint,
        y: BigUint,
        r: BigUint,
    ) {
        let q = big_uint_sub(x.clone(), y.clone() + r.clone());
        let q = q / BigInt::from_biguint(Sign::Plus, self.modulus.clone());
        self.generate_aux_cols(aux, x, y, r, q, false);
    }
    fn generate_aux_cols(
        &self,
        aux: &mut ModularAddSubAuxCols<F, NUM_LIMBS>,
        x: BigUint,
        y: BigUint,
        r: BigUint,
        q: BigInt,
        is_add: bool,
    ) {
        let mut q_limbs: Vec<isize> = big_int_to_limbs(&q, LIMB_SIZE);
        if q_limbs.is_empty() {
            q_limbs.push(0);
        }
        let q = q_limbs[0];

        self.range_checker_chip
            .add_count((q + (1 << LIMB_SIZE)) as u32, LIMB_SIZE + 1);
        let q_f = F::from_canonical_usize(q.unsigned_abs());
        aux.q = if q >= 0 { q_f } else { q_f * F::neg_one() };

        let x: OverflowInt<isize> =
            CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&x, Some(NUM_LIMBS)).into();
        let y: OverflowInt<isize> =
            CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&y, Some(NUM_LIMBS)).into();
        let r: OverflowInt<isize> =
            CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(&r, Some(NUM_LIMBS)).into();
        let p: OverflowInt<isize> = CanonicalUint::<isize, DefaultLimbConfig>::from_big_uint(
            &self.modulus,
            Some(NUM_LIMBS),
        )
        .into();

        let q_overflow = OverflowInt {
            limbs: q_limbs,
            max_overflow_bits: LIMB_SIZE + 1,
            limb_max_abs: (1 << LIMB_SIZE),
        };

        let expr: OverflowInt<isize> = if is_add { x + y } else { x - y } - r - p * q_overflow;
        let carries = expr.calculate_carries(LIMB_SIZE);
        let (carry_min_abs, carry_bits) =
            get_carry_max_abs_and_bits(expr.max_overflow_bits, LIMB_SIZE);

        for (i, &carry) in carries.iter().enumerate() {
            self.range_checker_chip
                .add_count((carry + carry_min_abs as isize) as u32, carry_bits);
            let carry_f = F::from_canonical_usize(carry.unsigned_abs());
            aux.carries[i] = if carry >= 0 {
                carry_f
            } else {
                carry_f * F::neg_one()
            };
        }
    }
}
