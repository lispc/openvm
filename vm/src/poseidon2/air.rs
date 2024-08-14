use std::borrow::Borrow;

use afs_primitives::sub_chip::AirConfig;
use afs_stark_backend::interaction::InteractionBuilder;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{AbstractField, Field};
use p3_matrix::Matrix;
use poseidon2_air::poseidon2::columns::Poseidon2Cols;

use super::{columns::Poseidon2VmCols, Poseidon2VmAir, CHUNK};

impl<const WIDTH: usize, F: Clone> AirConfig for Poseidon2VmAir<WIDTH, F> {
    type Cols<T> = Poseidon2VmCols<WIDTH, T>;
}

impl<const WIDTH: usize, F: Field> BaseAir<F> for Poseidon2VmAir<WIDTH, F> {
    fn width(&self) -> usize {
        Poseidon2VmCols::<WIDTH, F>::get_width(self)
    }
}

impl<AB: InteractionBuilder, const WIDTH: usize> Air<AB> for Poseidon2VmAir<WIDTH, AB::F> {
    /// Checks and constrains multiplicity indicators, and does subair evaluation
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let local: &[<AB>::Var] = (*local).borrow();

        let index_map = Poseidon2Cols::index_map(&self.inner);
        let cols = Poseidon2VmCols::<WIDTH, AB::Var>::from_slice(local, &index_map);

        self.eval_interactions(builder, cols.io, &cols.aux);
        self.inner
            .eval_without_interactions(builder, cols.aux.internal.io, cols.aux.internal.aux);

        // boolean constraints for alloc/cmp markers
        // these constraints hold for current trace generation mechanism but are in actuality not necessary
        builder.assert_bool(cols.io.is_opcode);
        builder.assert_bool(cols.io.is_direct);
        builder.assert_bool(cols.io.cmp);
        // can only be comparing if row is allocated
        builder.assert_eq(cols.io.is_opcode * cols.io.cmp, cols.io.cmp);
        // if io.cmp is false, then constrain rhs = lhs + CHUNK
        builder.when(cols.io.is_opcode - cols.io.cmp).assert_eq(
            cols.aux.rhs,
            cols.aux.lhs + AB::F::from_canonical_usize(CHUNK),
        );
    }
}
