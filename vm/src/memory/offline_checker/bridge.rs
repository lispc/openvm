use std::collections::VecDeque;

use afs_primitives::{
    is_less_than::{columns::IsLessThanIoCols, IsLessThanAir},
    is_zero::{
        columns::{IsZeroCols, IsZeroIoCols},
        IsZeroAir,
    },
    offline_checker::columns::OfflineCheckerCols,
    sub_chip::{AirConfig, SubAir},
    utils::{and, implies},
};
use afs_stark_backend::{air_builders::PartitionedAirBuilder, interaction::InteractionBuilder};
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{AbstractField, Field, PrimeField32};
use p3_matrix::Matrix;

use super::{
    bus::MemoryBus, columns::MemoryOfflineCheckerAuxCols, MemoryChip, MemoryOfflineChecker,
};
use crate::{
    cpu::{NEW_MEMORY_BUS, RANGE_CHECKER_BUS},
    memory::{
        manager::{access_cell::AccessCell, operation::MemoryOperation},
        MemoryAddress,
    },
};

/// The [MemoryBridge] can be created within any AIR evaluation function to be used as the
/// interface for constraining logical memory read or write operations. The bridge will add
/// all necessary constraints and interactions.
///
/// ## Usage
/// [MemoryBridge] must be initialized with the correct number of auxiliary columns to match the
/// max number of memory operations to be constrained.
#[derive(Clone, Debug)]
// TODO: WORD_SIZE should not be here, refactor
pub struct MemoryBridge<V, const WORD_SIZE: usize> {
    offline_checker: NewMemoryOfflineChecker,
    // TODO[jpw]:
    // Need separate VecDeque for writes to keep track of data_prev (since reads don't need)
    // TODO[jpw]: MemoryOfflineCheckerAuxCols needs to be refactored to deal with variable word size
    pub aux: VecDeque<MemoryOfflineCheckerAuxCols<WORD_SIZE, V>>,
    // @dev: do not let MemoryBridge own &mut builder. The mutable borrow will not allow builder to be
    // used again elsewhere while MemoryBridge is in scope.
}

impl<V, const WORD_SIZE: usize> MemoryBridge<V, WORD_SIZE> {
    /// Create a new [MemoryBridge] with the given number of auxiliary columns.
    pub fn new(
        offline_checker: NewMemoryOfflineChecker,
        aux: impl IntoIterator<Item = MemoryOfflineCheckerAuxCols<WORD_SIZE, V>>,
    ) -> Self {
        Self {
            offline_checker,
            aux: VecDeque::from_iter(aux),
        }
    }

    /// Prepare a logical memory read operation.
    pub fn read<T>(
        // , const WORD_SIZE: usize>(
        &mut self,
        address: MemoryAddress<impl Into<T>, impl Into<T>>,
        data: [impl Into<T>; WORD_SIZE],
        timestamp: impl Into<T>,
    ) -> MemoryReadOperation<T, V, WORD_SIZE> {
        let aux = self
            .aux
            .pop_front()
            .expect("Exceeded max capacity of memory accesses");
        MemoryReadOperation {
            offline_checker: self.offline_checker,
            address: MemoryAddress::from(address),
            data: data.map(Into::into),
            timestamp: timestamp.into(),
            aux,
        }
    }

    /// Prepare a logical memory write operation.
    pub fn write<T>(
        // , const WORD_SIZE: usize>(
        &mut self,
        address: MemoryAddress<impl Into<T>, impl Into<T>>,
        data: [impl Into<T>; WORD_SIZE],
        timestamp: impl Into<T>,
    ) -> MemoryWriteOperation<T, V, WORD_SIZE> {
        let aux = self
            .aux
            .pop_front()
            .expect("Exceeded max capacity of memory accesses");
        MemoryWriteOperation {
            offline_checker: self.offline_checker,
            address: MemoryAddress::from(address),
            data: data.map(Into::into),
            timestamp: timestamp.into(),
            aux,
        }
    }
}

// **TODO[jpw]**: Read does not need duplicate trace cells for old_data and data since they are the same.
// **Move old_cell out of AuxCols**
/// Constraints and interactions for a logical memory read of `(address, data)` at time `timestamp`.
/// This reads `(address, data, timestamp_prev)` from the memory bus and writes
/// `(address, data, timestamp)` to the memory bus.
/// Includes constraints for `timestamp_prev < timestamp`.
///
/// The generic `T` type is intended to be `AB::Expr` where `AB` is the [AirBuilder].
/// The auxiliary columns are not expected to be expressions, so the generic `V` type is intended
/// to be `AB::Var`.
pub struct MemoryReadOperation<T, V, const WORD_SIZE: usize> {
    offline_checker: NewMemoryOfflineChecker,
    address: MemoryAddress<T, T>,
    data: [T; WORD_SIZE],
    /// The timestamp of the last write to this address
    // timestamp_prev: T,
    /// The timestamp of the current read
    timestamp: T,
    aux: MemoryOfflineCheckerAuxCols<WORD_SIZE, V>,
}

impl<T: AbstractField, V, const WORD_SIZE: usize> MemoryReadOperation<T, V, WORD_SIZE> {
    /// Evaluate constraints and send/receive interactions.
    pub fn eval<AB>(self, builder: &mut AB, count: impl Into<AB::Expr>)
    where
        AB: InteractionBuilder<Var = V, Expr = T>,
    {
        let op = MemoryOperation {
            addr_space: self.address.address_space,
            pointer: self.address.pointer,
            op_type: AB::Expr::from_bool(false),
            cell: AccessCell::new(self.data, self.timestamp),
            enabled: count.into(),
        };
        self.offline_checker.subair_eval(builder, op, self.aux);
    }
}

/// Constraints and interactions for a logical memory write of `(address, data)` at time `timestamp`.
/// This reads `(address, data_prev, timestamp_prev)` from the memory bus and writes
/// `(address, data, timestamp)` to the memory bus.
/// Includes constraints for `timestamp_prev < timestamp`.
///
/// **Note:** This can be used as a logical read operation by setting `data_prev = data`.
pub struct MemoryWriteOperation<T, V, const WORD_SIZE: usize> {
    offline_checker: NewMemoryOfflineChecker,
    address: MemoryAddress<T, T>,
    data: [T; WORD_SIZE],
    /// The timestamp of the current read
    timestamp: T,
    aux: MemoryOfflineCheckerAuxCols<WORD_SIZE, V>,
}

impl<T: AbstractField, V, const WORD_SIZE: usize> MemoryWriteOperation<T, V, WORD_SIZE> {
    /// Evaluate constraints and send/receive interactions.
    pub fn eval<AB>(self, builder: &mut AB, count: impl Into<AB::Expr>)
    where
        AB: InteractionBuilder<Var = V, Expr = T>,
    {
        let op = MemoryOperation {
            addr_space: self.address.address_space,
            pointer: self.address.pointer,
            op_type: AB::Expr::from_bool(false),
            cell: AccessCell::new(self.data, self.timestamp),
            enabled: count.into(),
        };
        self.offline_checker.subair_eval(builder, op, self.aux);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NewMemoryOfflineChecker {
    pub memory_bus: MemoryBus,
    pub timestamp_lt_air: IsLessThanAir,
    pub is_zero_air: IsZeroAir,
}

impl NewMemoryOfflineChecker {
    pub fn new(clk_max_bits: usize, decomp: usize) -> Self {
        Self {
            memory_bus: NEW_MEMORY_BUS,
            timestamp_lt_air: IsLessThanAir::new(RANGE_CHECKER_BUS, clk_max_bits, decomp),
            is_zero_air: IsZeroAir,
        }
    }
}

impl NewMemoryOfflineChecker {
    pub fn subair_eval<AB: InteractionBuilder, const WORD_SIZE: usize>(
        &self,
        builder: &mut AB,
        op: MemoryOperation<WORD_SIZE, AB::Expr>,
        aux: MemoryOfflineCheckerAuxCols<WORD_SIZE, AB::Var>,
    ) {
        builder.assert_bool(op.op_type.clone());
        builder.assert_bool(op.enabled.clone());

        // Ensuring is_immediate is correct
        let addr_space_is_zero_cols = IsZeroCols::<AB::Expr>::new(
            IsZeroIoCols::<AB::Expr>::new(op.addr_space.clone(), aux.is_immediate.into()),
            aux.is_zero_aux.into(),
        );

        self.is_zero_air.subair_eval(
            builder,
            addr_space_is_zero_cols.io,
            addr_space_is_zero_cols.inv,
        );

        // // immediate => enabled
        // builder.assert_one(implies::<AB>(aux.is_immediate.into(), op.enabled.clone()));

        // TODO[osama]: make this degree 2
        // is_immediate => read
        builder.assert_one(implies::<AB>(
            and::<AB>(op.enabled.clone(), aux.is_immediate.into()),
            AB::Expr::one() - op.op_type.clone(),
        ));

        let clk_lt_io_cols = IsLessThanIoCols::<AB::Expr>::new(
            aux.old_cell.clk.into(),
            op.cell.clk.clone(),
            aux.clk_lt.into(),
        );

        self.timestamp_lt_air
            .subair_eval(builder, clk_lt_io_cols, aux.clk_lt_aux);

        // TODO[osama]: this should be reduced to degree 2
        builder.assert_one(implies::<AB>(
            and::<AB>(
                op.enabled.clone(),
                AB::Expr::one() - aux.is_immediate.into(),
            ),
            aux.clk_lt.into(),
        ));

        // Ensuring that if op_type is Read, data_read is the same as data_write
        for i in 0..WORD_SIZE {
            builder.assert_zero(
                op.enabled.clone()
                    * (AB::Expr::one() - op.op_type.clone())
                    * (op.cell.data[i].clone() - aux.old_cell.data[i]),
            );
        }

        // TODO[osama]: resolve is_immediate stuff
        let count = op.enabled * (AB::Expr::one() - aux.is_immediate.into());
        let address = MemoryAddress::new(op.addr_space, op.pointer);
        self.memory_bus
            .read(address.clone(), aux.old_cell.data, aux.old_cell.clk)
            .eval(builder, count.clone());
        self.memory_bus
            .write(address, op.cell.data, op.cell.clk)
            .eval(builder, count);
    }
}

// TODO: delete all below
impl<const WORD_SIZE: usize, F: PrimeField32> AirConfig for MemoryChip<WORD_SIZE, F> {
    type Cols<T> = OfflineCheckerCols<T>;
}

impl<F: Field> BaseAir<F> for MemoryOfflineChecker {
    fn width(&self) -> usize {
        self.air_width()
    }
}

impl<AB: PartitionedAirBuilder + InteractionBuilder> Air<AB> for MemoryOfflineChecker {
    /// This constrains extra rows to be at the bottom and the following on non-extra rows:
    /// same_addr_space, same_pointer, same_data, lt_bit is correct (see definition in columns.rs)
    /// A read must be preceded by a write with the same address space, pointer, and data
    fn eval(&self, builder: &mut AB) {
        let main = &builder.main();

        let local_cols = OfflineCheckerCols::from_slice(&main.row_slice(0), &self.offline_checker);
        let next_cols = OfflineCheckerCols::from_slice(&main.row_slice(1), &self.offline_checker);

        builder.assert_bool(local_cols.op_type);

        // loop over data_len
        // is_valid * (1 - op_type) * same_idx * (x[i] - y[i])
        for i in 0..self.offline_checker.data_len {
            // NOTE: constraint degree is 4
            builder.when_transition().assert_zero(
                next_cols.is_valid.into()
                    * (AB::Expr::one() - next_cols.op_type.into())
                    * next_cols.same_idx.into()
                    * (local_cols.data[i] - next_cols.data[i]),
            );
        }

        SubAir::eval(&self.offline_checker, builder, (local_cols, next_cols), ());
    }
}
