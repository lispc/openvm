use std::sync::Arc;

use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;

use crate::{range_gate::RangeCheckerGateChip, sub_chip::LocalTraceInstructions};

use super::{
    columns::{IsLessThanAuxCols, IsLessThanCols, IsLessThanIOCols},
    IsLessThanAir, IsLessThanChip,
};

impl IsLessThanChip {
    pub fn generate_trace<F: PrimeField64>(&self, pairs: Vec<(u32, u32)>) -> RowMajorMatrix<F> {
        let num_cols: usize =
            IsLessThanCols::<F>::get_width(*self.air.limb_bits(), *self.air.decomp());

        let mut rows = vec![];

        // generate a row for each pair of numbers to compare
        for (x, y) in pairs {
            let row: Vec<F> = self
                .air
                .generate_trace_row((x, y, self.range_checker.clone()))
                .flatten();
            rows.extend(row);
        }

        RowMajorMatrix::new(rows, num_cols)
    }
}

impl<F: PrimeField64> LocalTraceInstructions<F> for IsLessThanAir {
    type LocalInput = (u32, u32, Arc<RangeCheckerGateChip>);

    fn generate_trace_row(&self, input: (u32, u32, Arc<RangeCheckerGateChip>)) -> Self::Cols<F> {
        let (x, y, range_checker) = input;
        let less_than = if x < y { 1 } else { 0 };

        // to range check the last limb of the decomposed lower_bits, we need to shift it to make sure it is in
        // the correct range
        let last_limb_shift = (self.decomp() - (self.limb_bits() % self.decomp())) % self.decomp();

        // obtain the lower_bits
        let check_less_than = (1 << self.limb_bits()) + y - x - 1;
        let lower = F::from_canonical_u32(check_less_than & ((1 << self.limb_bits()) - 1));
        let lower_u32 = check_less_than & ((1 << self.limb_bits()) - 1);

        // decompose lower_bits into limbs and range check
        let mut lower_decomp: Vec<F> = vec![];
        for i in 0..*self.num_limbs() {
            let bits = (lower_u32 >> (i * self.decomp())) & ((1 << self.decomp()) - 1);
            lower_decomp.push(F::from_canonical_u32(bits));
            range_checker.add_count(bits);
        }

        // shift the last limb and range check
        let bits =
            (lower_u32 >> ((self.num_limbs() - 1) * self.decomp())) & ((1 << self.decomp()) - 1);
        if (bits << last_limb_shift) < *self.range_max() {
            range_checker.add_count(bits << last_limb_shift);
        }
        lower_decomp.push(F::from_canonical_u32(bits << last_limb_shift));

        let io = IsLessThanIOCols {
            x: F::from_canonical_u32(x),
            y: F::from_canonical_u32(y),
            less_than: F::from_canonical_u32(less_than),
        };
        let aux = IsLessThanAuxCols {
            lower,
            lower_decomp,
        };

        IsLessThanCols { io, aux }
    }
}
