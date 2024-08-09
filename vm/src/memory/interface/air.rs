use afs_stark_backend::interaction::InteractionBuilder;
use p3_air::{Air, BaseAir};
use p3_field::Field;
use p3_matrix::Matrix;

use crate::memory::{expand::MemoryDimensions, interface::columns::MemoryInterfaceCols};

pub struct MemoryInterfaceAir<const NUM_WORDS: usize, const WORD_SIZE: usize> {
    pub memory_dimensions: MemoryDimensions,
}

impl<const NUM_WORDS: usize, const WORD_SIZE: usize, F: Field> BaseAir<F>
    for MemoryInterfaceAir<NUM_WORDS, WORD_SIZE>
{
    fn width(&self) -> usize {
        MemoryInterfaceCols::<NUM_WORDS, WORD_SIZE, F>::get_width()
    }
}

impl<const NUM_WORDS: usize, const WORD_SIZE: usize, AB: InteractionBuilder> Air<AB>
    for MemoryInterfaceAir<NUM_WORDS, WORD_SIZE>
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let local = MemoryInterfaceCols::<NUM_WORDS, WORD_SIZE, AB::Var>::from_slice(&local);

        // `direction` should be -1, 0, 1
        builder.assert_eq(
            local.expand_direction,
            local.expand_direction * local.expand_direction * local.expand_direction,
        );

        self.eval_interactions(builder, local);
    }
}
