use std::sync::Arc;

use afs_primitives::{
    is_less_than_tuple::columns::IsLessThanTupleCols, range_gate::RangeCheckerGateChip,
    sub_chip::LocalTraceInstructions,
};
use p3_field::{AbstractField, PrimeField};
use p3_matrix::dense::RowMajorMatrix;
use p3_uni_stark::{StarkGenericConfig, Val};

use crate::common::page::Page;

use super::{columns::IndexedOutputPageAuxCols, IndexedOutputPageAir};

impl IndexedOutputPageAir {
    /// The trace is the whole page (including the is_alloc column)
    pub fn gen_page_trace<SC: StarkGenericConfig>(&self, page: &Page) -> RowMajorMatrix<Val<SC>>
    where
        Val<SC>: PrimeField,
    {
        page.gen_trace()
    }

    /// This generates the auxiliary trace required to ensure proper formating
    /// of the page
    pub fn gen_aux_trace<SC: StarkGenericConfig>(
        &self,
        page: &Page,
        range_checker: Arc<RangeCheckerGateChip>,
    ) -> RowMajorMatrix<Val<SC>>
    where
        Val<SC>: PrimeField,
    {
        let mut rows: Vec<Vec<Val<SC>>> = vec![];

        for i in 0..page.height() {
            let prv_idx = if i == 0 {
                vec![0; self.idx_len]
            } else {
                page[i - 1].idx.clone()
            };

            let cur_idx = page[i].idx.clone();

            let lt_cols: IsLessThanTupleCols<Val<SC>> = LocalTraceInstructions::generate_trace_row(
                &self.lt_air,
                (prv_idx, cur_idx, range_checker.clone()),
            );
            let mut row = vec![Val::<SC>::zero(); self.aux_width()];
            let page_aux_cols = IndexedOutputPageAuxCols {
                lt_cols: lt_cols.aux,
                lt_out: lt_cols.io.tuple_less_than,
            };
            let _ = page_aux_cols.flatten(&mut row, 0);
            rows.push(row);
        }

        RowMajorMatrix::new(rows.concat(), self.aux_width())
    }
}
