use std::sync::Arc;

use crate::{is_less_than::IsLessThanAir, range_gate::RangeCheckerGateChip};

#[cfg(test)]
pub mod tests;

pub mod air;
pub mod bridge;
pub mod columns;
pub mod trace;

pub use air::IsLessThanTupleAir;

/// This chip computes whether one tuple is lexicographically less than another. Each element of the
/// tuple has its own max number of bits, given by the limb_bits array. The chip assumes that each limb
/// is within its given max limb_bits.
///
/// The IsLessThanTupleChip uses the IsLessThanChip as a subchip to check whether individual tuple elements
/// are less than each other.
#[derive(Clone, Debug)]
pub struct IsLessThanTupleChip {
    pub air: IsLessThanTupleAir,

    pub range_checker: Arc<RangeCheckerGateChip>,
}

impl IsLessThanTupleChip {
    pub fn new(
        bus_index: usize,
        limb_bits: Vec<usize>,
        decomp: usize,
        range_checker: Arc<RangeCheckerGateChip>,
    ) -> Self {
        let is_less_than_airs = limb_bits
            .iter()
            .map(|&limb_bit| IsLessThanAir::new(bus_index, limb_bit, decomp))
            .collect::<Vec<_>>();

        let air = IsLessThanTupleAir {
            bus_index,
            decomp,
            is_less_than_airs,
        };

        Self { air, range_checker }
    }
}
