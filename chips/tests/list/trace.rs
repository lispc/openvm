use p3_field::PrimeField64;
use p3_matrix::dense::RowMajorMatrix;

use super::{columns::NUM_LIST_COLS, ListChip};

impl<const MAX: u32> ListChip<MAX> {
    pub fn generate_trace<F: PrimeField64>(&self) -> RowMajorMatrix<F> {
        let mut rows = vec![];
        for val in self.vals.iter() {
            rows.push(vec![F::from_canonical_u32(*val)]);
            self.range_checker.add_count(*val);
        }

        RowMajorMatrix::new(rows.concat(), NUM_LIST_COLS)
    }
}
