use std::{array::from_fn, collections::BTreeMap};

use itertools::Itertools;
use p3_field::{Field, PrimeField64};

use afs_primitives::{
    is_equal_vec::{columns::IsEqualVecAuxCols, IsEqualVecAir},
    sub_chip::LocalTraceInstructions,
};

use crate::arch::instructions::CORE_INSTRUCTIONS;

use super::{CPU_MAX_ACCESSES_PER_CYCLE, CpuOptions, OpCode, trace::disabled_memory_cols};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuIoCols<T> {
    pub timestamp: T,
    pub pc: T,

    pub opcode: T,
    pub op_a: T,
    pub op_b: T,
    pub op_c: T,
    pub d: T,
    pub e: T,
    pub op_f: T,
    pub op_g: T,
}

impl<T: Clone> CpuIoCols<T> {
    pub fn from_slice(slc: &[T]) -> Self {
        Self {
            timestamp: slc[0].clone(),
            pc: slc[1].clone(),
            opcode: slc[2].clone(),
            op_a: slc[3].clone(),
            op_b: slc[4].clone(),
            op_c: slc[5].clone(),
            d: slc[6].clone(),
            e: slc[7].clone(),
            op_f: slc[8].clone(),
            op_g: slc[9].clone(),
        }
    }

    pub fn flatten(&self) -> Vec<T> {
        vec![
            self.timestamp.clone(),
            self.pc.clone(),
            self.opcode.clone(),
            self.op_a.clone(),
            self.op_b.clone(),
            self.op_c.clone(),
            self.d.clone(),
            self.e.clone(),
            self.op_f.clone(),
            self.op_g.clone(),
        ]
    }

    pub fn get_width() -> usize {
        10
    }
}

impl<T: Field> CpuIoCols<T> {
    pub fn nop_row(pc: T, timestamp: T) -> Self {
        Self {
            timestamp,
            pc,
            opcode: T::from_canonical_usize(OpCode::NOP as usize),
            op_a: T::default(),
            op_b: T::default(),
            op_c: T::default(),
            d: T::default(),
            e: T::default(),
            op_f: T::default(),
            op_g: T::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryAccessCols<const WORD_SIZE: usize, T> {
    pub enabled: T,

    pub address_space: T,
    pub is_immediate: T,
    pub is_zero_aux: T,

    pub address: T,

    pub data: [T; WORD_SIZE],
}

impl<const WORD_SIZE: usize, T: Clone> MemoryAccessCols<WORD_SIZE, T> {
    pub fn from_slice(slc: &[T]) -> Self {
        Self {
            enabled: slc[0].clone(),
            address_space: slc[1].clone(),
            is_immediate: slc[2].clone(),
            is_zero_aux: slc[3].clone(),
            address: slc[4].clone(),
            data: from_fn(|i| slc[5 + i].clone()),
        }
    }
    pub fn flatten(&self) -> Vec<T> {
        let mut flattened = vec![
            self.enabled.clone(),
            self.address_space.clone(),
            self.is_immediate.clone(),
            self.is_zero_aux.clone(),
            self.address.clone(),
        ];
        flattened.extend(self.data.to_vec());
        flattened
    }

    pub fn get_width() -> usize {
        5 + WORD_SIZE
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuAuxCols<const WORD_SIZE: usize, T> {
    pub operation_flags: BTreeMap<OpCode, T>,
    pub public_value_flags: Vec<T>,
    pub accesses: [MemoryAccessCols<WORD_SIZE, T>; CPU_MAX_ACCESSES_PER_CYCLE],
    pub read0_equals_read1: T,
    pub is_equal_vec_aux: IsEqualVecAuxCols<T>,
}

impl<const WORD_SIZE: usize, T: Clone> CpuAuxCols<WORD_SIZE, T> {
    pub fn from_slice(slc: &[T], options: CpuOptions) -> Self {
        let mut start = 0;
        let mut end = CORE_INSTRUCTIONS.len();
        let operation_flags_vec = slc[start..end].to_vec();
        let mut operation_flags = BTreeMap::new();
        for (opcode, operation_flag) in CORE_INSTRUCTIONS.iter().zip_eq(operation_flags_vec) {
            operation_flags.insert(*opcode, operation_flag);
        }

        start = end;
        end += options.num_public_values;
        let public_value_flags = slc[start..end].to_vec();

        let accesses = from_fn(|_| {
            start = end;
            end += MemoryAccessCols::<WORD_SIZE, T>::get_width();
            MemoryAccessCols::from_slice(&slc[start..end])
        });

        let beq_check = slc[end].clone();
        let is_equal_vec_aux = IsEqualVecAuxCols::from_slice(&slc[end + 1..], WORD_SIZE);

        Self {
            operation_flags,
            public_value_flags,
            accesses,
            read0_equals_read1: beq_check,
            is_equal_vec_aux,
        }
    }

    pub fn flatten(&self, options: CpuOptions) -> Vec<T> {
        let mut flattened = vec![];
        for opcode in CORE_INSTRUCTIONS {
            flattened.push(self.operation_flags.get(&opcode).unwrap().clone());
        }
        flattened.extend(self.public_value_flags.clone());
        flattened.extend(self.accesses.iter().flat_map(MemoryAccessCols::flatten));
        flattened.push(self.read0_equals_read1.clone());
        flattened.extend(self.is_equal_vec_aux.flatten());
        flattened
    }

    pub fn get_width(options: CpuOptions) -> usize {
        CORE_INSTRUCTIONS.len()
            + options.num_public_values
            + (CPU_MAX_ACCESSES_PER_CYCLE * MemoryAccessCols::<WORD_SIZE, T>::get_width())
            + 1
            + IsEqualVecAuxCols::<T>::width(WORD_SIZE)
    }
}

impl<const WORD_SIZE: usize, T: PrimeField64> CpuAuxCols<WORD_SIZE, T> {
    pub fn nop_row(options: CpuOptions) -> Self {
        let mut operation_flags = BTreeMap::new();
        for opcode in CORE_INSTRUCTIONS {
            operation_flags.insert(opcode, T::from_bool(opcode == OpCode::NOP));
        }
        let accesses = [disabled_memory_cols(); CPU_MAX_ACCESSES_PER_CYCLE];
        let is_equal_vec_cols = LocalTraceInstructions::generate_trace_row(
            &IsEqualVecAir::new(WORD_SIZE),
            (accesses[0].data.to_vec(), accesses[1].data.to_vec()),
        );
        Self {
            operation_flags,
            public_value_flags: vec![T::zero(); options.num_public_values],
            accesses,
            read0_equals_read1: T::one(),
            is_equal_vec_aux: is_equal_vec_cols.aux,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuCols<const WORD_SIZE: usize, T> {
    pub io: CpuIoCols<T>,
    pub aux: CpuAuxCols<WORD_SIZE, T>,
}

impl<const WORD_SIZE: usize, T: Clone> CpuCols<WORD_SIZE, T> {
    pub fn from_slice(slc: &[T], options: CpuOptions) -> Self {
        let io = CpuIoCols::<T>::from_slice(&slc[..CpuIoCols::<T>::get_width()]);
        let aux =
            CpuAuxCols::<WORD_SIZE, T>::from_slice(&slc[CpuIoCols::<T>::get_width()..], options);

        Self { io, aux }
    }

    pub fn flatten(&self, options: CpuOptions) -> Vec<T> {
        let mut flattened = self.io.flatten();
        flattened.extend(self.aux.flatten(options));
        flattened
    }

    pub fn get_width(options: CpuOptions) -> usize {
        CpuIoCols::<T>::get_width() + CpuAuxCols::<WORD_SIZE, T>::get_width(options)
    }
}

impl<const WORD_SIZE: usize, T: PrimeField64> CpuCols<WORD_SIZE, T> {
    pub fn nop_row(options: CpuOptions, pc: T, timestamp: T) -> Self {
        Self {
            io: CpuIoCols::<T>::nop_row(pc, timestamp),
            aux: CpuAuxCols::<WORD_SIZE, T>::nop_row(options),
        }
    }
}
