use p3_field::PrimeField64;

use afs_primitives::offline_checker::OfflineCheckerOperation;

use crate::memory::OpType;

pub mod air;
pub mod trace;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryAccess<const WORD_SIZE: usize, F> {
    pub timestamp: usize,
    pub op_type: OpType,
    pub address_space: F,
    pub address: F,
    pub data: [F; WORD_SIZE],
}

impl<const WORD_SIZE: usize, F: PrimeField64> OfflineCheckerOperation<F>
    for MemoryAccess<WORD_SIZE, F>
{
    fn get_timestamp(&self) -> usize {
        self.timestamp
    }

    fn get_idx(&self) -> Vec<F> {
        vec![self.address_space, self.address]
    }

    fn get_data(&self) -> Vec<F> {
        self.data.to_vec()
    }
    fn get_op_type(&self) -> u8 {
        self.op_type as u8
    }
}
