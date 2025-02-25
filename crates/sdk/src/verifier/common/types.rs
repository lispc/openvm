use std::{array, borrow::BorrowMut};

use openvm_circuit::{
    circuit_derive::AlignedBorrow,
    system::{connector::VmConnectorPvs, memory::merkle::MemoryMerklePvs},
};
use openvm_native_compiler::prelude::*;
use openvm_stark_sdk::openvm_stark_backend::p3_field::PrimeField32;

#[derive(Debug, Clone, Copy)]
pub struct SpecialAirIds {
    pub program_air_id: usize,
    pub connector_air_id: usize,
    pub public_values_air_id: usize,
}

#[derive(Debug, Clone, Copy, AlignedBorrow)]
#[repr(C)]
pub struct VmVerifierPvs<T> {
    /// The commitment of the app program.
    pub app_commit: [T; DIGEST_SIZE],
    /// The merged execution state of all the segments this circuit aggregates.
    pub connector: VmConnectorPvs<T>,
    /// The memory state before/after all the segments this circuit aggregates.
    pub memory: MemoryMerklePvs<T, DIGEST_SIZE>,
    /// The merkle root of all public values. This is only meaningful when the last segment is
    /// aggregated by this circuit.
    pub public_values_commit: [T; DIGEST_SIZE],
}

impl<F: PrimeField32> VmVerifierPvs<Felt<F>> {
    pub fn uninit<C: Config<F = F>>(builder: &mut Builder<C>) -> Self {
        Self {
            app_commit: array::from_fn(|_| builder.uninit()),
            connector: VmConnectorPvs {
                initial_pc: builder.uninit(),
                final_pc: builder.uninit(),
                exit_code: builder.uninit(),
                is_terminate: builder.uninit(),
            },
            memory: MemoryMerklePvs {
                initial_root: array::from_fn(|_| builder.uninit()),
                final_root: array::from_fn(|_| builder.uninit()),
            },
            public_values_commit: array::from_fn(|_| builder.uninit()),
        }
    }
}

impl<F: Default + Clone> VmVerifierPvs<Felt<F>> {
    pub fn flatten(self) -> Vec<Felt<F>> {
        let mut v = vec![Felt(0, Default::default()); VmVerifierPvs::<u8>::width()];
        *v.as_mut_slice().borrow_mut() = self;
        v
    }
}
