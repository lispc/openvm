use crate::{
    is_less_than_tuple::columns::{IsLessThanTupleCols, IsLessThanTupleIOCols},
    sub_chip::SubAirWithInteractions,
};

use super::columns::AssertSortedCols;
use afs_stark_backend::interaction::{Chip, Interaction};
use p3_field::PrimeField64;

use super::AssertSortedAir;

impl<F: PrimeField64> Chip<F> for AssertSortedAir {
    fn sends(&self) -> Vec<Interaction<F>> {
        let num_cols = AssertSortedCols::<F>::get_width(
            self.is_less_than_tuple_air().limb_bits().clone(),
            *self.is_less_than_tuple_air().decomp(),
            self.is_less_than_tuple_air().tuple_len(),
        );
        let all_cols = (0..num_cols).collect::<Vec<usize>>();

        let cols_numbered = AssertSortedCols::<usize>::from_slice(
            &all_cols,
            self.is_less_than_tuple_air().limb_bits().clone(),
            *self.is_less_than_tuple_air().decomp(),
            self.is_less_than_tuple_air().tuple_len(),
        );

<<<<<<< HEAD
        // here, y doesn't matter since we are only range checking the decompositions of x
=======
        // range check the decompositions of x within aux columns; here the io doesn't matter
>>>>>>> bd362118ce67d69c20c52cdf88fc66e47000305a
        let is_less_than_tuple_cols = IsLessThanTupleCols {
            io: IsLessThanTupleIOCols {
                x: cols_numbered.key.clone(),
                y: cols_numbered.key.clone(),
                tuple_less_than: cols_numbered.less_than_next_key,
            },
            aux: cols_numbered.is_less_than_tuple_aux,
        };

        let subchip_interactions = SubAirWithInteractions::<F>::sends(
            self.is_less_than_tuple_air(),
            is_less_than_tuple_cols,
        );

        subchip_interactions
    }
}
