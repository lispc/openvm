use std::collections::HashMap;

use p3_field::{PrimeField32, PrimeField64};

use afs_primitives::offline_checker::OfflineChecker;

use crate::cpu::{MEMORY_BUS, RANGE_CHECKER_BUS};
use crate::memory::api::VmMemory;
use crate::memory::offline_checker::air::MemoryOfflineChecker;
use crate::memory::offline_checker::MemoryAccess;

pub mod api;
pub mod expand;
pub mod interface;
pub mod offline_checker;
#[cfg(test)]
pub mod tests;
pub mod tree;

#[derive(PartialEq, Copy, Clone, Debug, Eq)]
pub enum OpType {
    Read = 0,
    Write = 1,
}

// panics if the word is not equal to decompose(elem) for some elem: F
pub fn compose<const WORD_SIZE: usize, F: PrimeField64>(word: [F; WORD_SIZE]) -> F {
    for &cell in word.iter().skip(1) {
        assert_eq!(cell, F::zero());
    }
    word[0]
}

pub fn decompose<const WORD_SIZE: usize, F: PrimeField64>(field_elem: F) -> [F; WORD_SIZE] {
    std::array::from_fn(|i| if i == 0 { field_elem } else { F::zero() })
}

pub struct MemoryCircuit<const WORD_SIZE: usize, F: PrimeField32> {
    pub offline_checker: MemoryOfflineChecker,

    pub accesses: Vec<MemoryAccess<WORD_SIZE, F>>,
    memory: HashMap<(F, F), F>,
    last_timestamp: Option<usize>,
}

impl<const WORD_SIZE: usize, F: PrimeField32> MemoryCircuit<WORD_SIZE, F> {
    pub fn new(
        addr_space_limb_bits: usize,
        pointer_limb_bits: usize,
        clk_limb_bits: usize,
        decomp: usize,
    ) -> Self {
        let idx_clk_limb_bits = vec![addr_space_limb_bits, pointer_limb_bits, clk_limb_bits];

        let offline_checker = OfflineChecker::new(
            idx_clk_limb_bits,
            decomp,
            2,
            WORD_SIZE,
            RANGE_CHECKER_BUS,
            MEMORY_BUS,
        );

        Self {
            offline_checker: MemoryOfflineChecker { offline_checker },
            accesses: vec![],
            memory: HashMap::new(),
            last_timestamp: None,
        }
    }
}

impl<const WORD_SIZE: usize, F: PrimeField32> VmMemory<WORD_SIZE, F>
    for MemoryCircuit<WORD_SIZE, F>
{
    fn read_word(&mut self, timestamp: usize, address_space: F, address: F) -> [F; WORD_SIZE] {
        if address_space == F::zero() {
            return decompose(address);
        }
        if let Some(last_timestamp) = self.last_timestamp {
            assert!(timestamp > last_timestamp);
        }
        self.last_timestamp = Some(timestamp);
        let data = std::array::from_fn(|i| {
            self.memory[&(address_space, address + F::from_canonical_usize(i))]
        });
        self.accesses.push(MemoryAccess {
            timestamp,
            op_type: OpType::Read,
            address_space,
            address,
            data,
        });
        data
    }

    fn write_word(&mut self, timestamp: usize, address_space: F, address: F, data: [F; WORD_SIZE]) {
        assert_ne!(address_space, F::zero());
        if let Some(last_timestamp) = self.last_timestamp {
            assert!(timestamp > last_timestamp);
        }
        self.last_timestamp = Some(timestamp);
        for (i, &datum) in data.iter().enumerate() {
            self.memory
                .insert((address_space, address + F::from_canonical_usize(i)), datum);
        }
        self.accesses.push(MemoryAccess {
            timestamp,
            op_type: OpType::Write,
            address_space,
            address,
            data,
        });
    }

    fn unsafe_read_word(&self, address_space: F, address: F) -> [F; WORD_SIZE] {
        std::array::from_fn(|i| self.memory[&(address_space, address + F::from_canonical_usize(i))])
    }

    fn compose(word: [F; WORD_SIZE]) -> F {
        compose(word)
    }

    fn decompose(field_elem: F) -> [F; WORD_SIZE] {
        decompose(field_elem)
    }
}
