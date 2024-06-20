use std::{iter, sync::Arc};

use afs_chips::range_gate::RangeCheckerGateChip;
use afs_stark_backend::{prover::USE_DEBUG_BUILDER, verifier::VerificationError};
use afs_test_utils::{
    config::baby_bear_poseidon2::run_simple_test_no_pis,
    interaction::dummy_interaction_air::DummyInteractionAir,
};
use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use p3_matrix::dense::RowMajorMatrix;

use crate::cpu::{MEMORY_BUS, RANGE_CHECKER_BUS};

use super::{offline_checker::OfflineChecker, MemoryAccess, OpType};

const DATA_LEN: usize = 3;
const ADDR_SPACE_LIMB_BITS: usize = 8;
const POINTER_LIMB_BITS: usize = 8;
const CLK_LIMB_BITS: usize = 8;
const DECOMP: usize = 4;
const RANGE_MAX: u32 = 1 << DECOMP;

const TRACE_DEGREE: usize = 16;

#[test]
fn test_offline_checker() {
    let range_checker = Arc::new(RangeCheckerGateChip::new(RANGE_CHECKER_BUS, RANGE_MAX));
    let offline_checker = OfflineChecker::new(
        DATA_LEN,
        ADDR_SPACE_LIMB_BITS,
        POINTER_LIMB_BITS,
        CLK_LIMB_BITS,
        DECOMP,
    );
    let requester = DummyInteractionAir::new(2 + offline_checker.mem_width(), true, MEMORY_BUS);

    let ops: Vec<MemoryAccess<BabyBear>> = vec![
        MemoryAccess {
            clock: 1,
            op_type: OpType::Write,
            address_space: BabyBear::zero(),
            address: BabyBear::one(),
            data: vec![
                BabyBear::from_canonical_usize(232),
                BabyBear::from_canonical_usize(888),
                BabyBear::from_canonical_usize(5954),
            ],
        },
        MemoryAccess {
            clock: 0,
            op_type: OpType::Write,
            address_space: BabyBear::zero(),
            address: BabyBear::zero(),
            data: vec![
                BabyBear::from_canonical_usize(2324),
                BabyBear::from_canonical_usize(433),
                BabyBear::from_canonical_usize(1778),
            ],
        },
        MemoryAccess {
            clock: 4,
            op_type: OpType::Write,
            address_space: BabyBear::one(),
            address: BabyBear::zero(),
            data: vec![
                BabyBear::from_canonical_usize(231),
                BabyBear::from_canonical_usize(3883),
                BabyBear::from_canonical_usize(17),
            ],
        },
        MemoryAccess {
            clock: 2,
            op_type: OpType::Read,
            address_space: BabyBear::zero(),
            address: BabyBear::one(),
            data: vec![
                BabyBear::from_canonical_usize(232),
                BabyBear::from_canonical_usize(888),
                BabyBear::from_canonical_usize(5954),
            ],
        },
        MemoryAccess {
            clock: 6,
            op_type: OpType::Read,
            address_space: BabyBear::two(),
            address: BabyBear::zero(),
            data: vec![
                BabyBear::from_canonical_usize(4382),
                BabyBear::from_canonical_usize(8837),
                BabyBear::from_canonical_usize(192),
            ],
        },
        MemoryAccess {
            clock: 5,
            op_type: OpType::Write,
            address_space: BabyBear::two(),
            address: BabyBear::zero(),
            data: vec![
                BabyBear::from_canonical_usize(4382),
                BabyBear::from_canonical_usize(8837),
                BabyBear::from_canonical_usize(192),
            ],
        },
        MemoryAccess {
            clock: 3,
            op_type: OpType::Write,
            address_space: BabyBear::zero(),
            address: BabyBear::one(),
            data: vec![
                BabyBear::from_canonical_usize(3243),
                BabyBear::from_canonical_usize(3214),
                BabyBear::from_canonical_usize(6639),
            ],
        },
    ];

    let offline_checker_trace =
        offline_checker.generate_trace(ops.clone(), range_checker.clone(), TRACE_DEGREE);
    let range_checker_trace = range_checker.generate_trace();
    let requester_trace = RowMajorMatrix::new(
        ops.iter()
            .flat_map(|op: &MemoryAccess<BabyBear>| {
                iter::once(BabyBear::one())
                    .chain(iter::once(BabyBear::from_canonical_usize(op.clock)))
                    .chain(iter::once(BabyBear::from_canonical_u8(op.op_type as u8)))
                    .chain(iter::once(op.address_space))
                    .chain(iter::once(op.address))
                    .chain(op.data.iter().cloned())
            })
            .chain(
                iter::repeat_with(|| {
                    iter::repeat(BabyBear::zero()).take(1 + requester.field_width())
                })
                .take(TRACE_DEGREE - ops.len())
                .flatten(),
            )
            .collect(),
        1 + requester.field_width(),
    );

    run_simple_test_no_pis(
        vec![&offline_checker, &range_checker.air, &requester],
        vec![offline_checker_trace, range_checker_trace, requester_trace],
    )
    .expect("Verification failed");
}

#[test]
fn test_offline_checker_negative_invalid_read() {
    let range_checker = Arc::new(RangeCheckerGateChip::new(RANGE_CHECKER_BUS, RANGE_MAX));
    let offline_checker = OfflineChecker::new(
        DATA_LEN,
        ADDR_SPACE_LIMB_BITS,
        POINTER_LIMB_BITS,
        CLK_LIMB_BITS,
        DECOMP,
    );
    let requester = DummyInteractionAir::new(2 + offline_checker.mem_width(), true, MEMORY_BUS);

    // should fail because we can't read before writing
    let ops: Vec<MemoryAccess<BabyBear>> = vec![MemoryAccess {
        clock: 0,
        op_type: OpType::Read,
        address_space: BabyBear::zero(),
        address: BabyBear::zero(),
        data: vec![
            BabyBear::from_canonical_usize(0),
            BabyBear::from_canonical_usize(0),
            BabyBear::from_canonical_usize(0),
        ],
    }];

    let offline_checker_trace =
        offline_checker.generate_trace(ops.clone(), range_checker.clone(), TRACE_DEGREE);
    let range_checker_trace = range_checker.generate_trace();
    let requester_trace = RowMajorMatrix::new(
        ops.iter()
            .flat_map(|op: &MemoryAccess<BabyBear>| {
                iter::once(BabyBear::one())
                    .chain(iter::once(BabyBear::from_canonical_usize(op.clock)))
                    .chain(iter::once(BabyBear::from_canonical_u8(op.op_type as u8)))
                    .chain(iter::once(op.address_space))
                    .chain(iter::once(op.address))
                    .chain(op.data.iter().cloned())
            })
            .chain(
                iter::repeat_with(|| {
                    iter::repeat(BabyBear::zero()).take(1 + requester.field_width())
                })
                .take(TRACE_DEGREE - ops.len())
                .flatten(),
            )
            .collect(),
        1 + requester.field_width(),
    );

    USE_DEBUG_BUILDER.with(|debug| {
        *debug.lock().unwrap() = false;
    });
    assert_eq!(
        run_simple_test_no_pis(
            vec![&offline_checker, &range_checker.air, &requester],
            vec![offline_checker_trace, range_checker_trace, requester_trace],
        ),
        Err(VerificationError::OodEvaluationMismatch),
        "Expected verification to fail, but it passed"
    );
}

#[test]
fn test_offline_checker_negative_data_mismatch() {
    let range_checker = Arc::new(RangeCheckerGateChip::new(RANGE_CHECKER_BUS, RANGE_MAX));
    let offline_checker = OfflineChecker::new(
        DATA_LEN,
        ADDR_SPACE_LIMB_BITS,
        POINTER_LIMB_BITS,
        CLK_LIMB_BITS,
        DECOMP,
    );
    let requester = DummyInteractionAir::new(2 + offline_checker.mem_width(), true, MEMORY_BUS);

    let ops: Vec<MemoryAccess<BabyBear>> = vec![
        MemoryAccess {
            clock: 0,
            op_type: OpType::Write,
            address_space: BabyBear::zero(),
            address: BabyBear::zero(),
            data: vec![
                BabyBear::from_canonical_usize(2324),
                BabyBear::from_canonical_usize(433),
                BabyBear::from_canonical_usize(1778),
            ],
        },
        MemoryAccess {
            clock: 1,
            op_type: OpType::Write,
            address_space: BabyBear::zero(),
            address: BabyBear::one(),
            data: vec![
                BabyBear::from_canonical_usize(232),
                BabyBear::from_canonical_usize(888),
                BabyBear::from_canonical_usize(5954),
            ],
        },
        // data read does not match write from previous operation
        MemoryAccess {
            clock: 2,
            op_type: OpType::Read,
            address_space: BabyBear::zero(),
            address: BabyBear::one(),
            data: vec![
                BabyBear::from_canonical_usize(233),
                BabyBear::from_canonical_usize(888),
                BabyBear::from_canonical_usize(5954),
            ],
        },
    ];

    let offline_checker_trace =
        offline_checker.generate_trace(ops.clone(), range_checker.clone(), TRACE_DEGREE);
    let range_checker_trace = range_checker.generate_trace();
    let requester_trace = RowMajorMatrix::new(
        ops.iter()
            .flat_map(|op: &MemoryAccess<BabyBear>| {
                iter::once(BabyBear::one())
                    .chain(iter::once(BabyBear::from_canonical_usize(op.clock)))
                    .chain(iter::once(BabyBear::from_canonical_u8(op.op_type as u8)))
                    .chain(iter::once(op.address_space))
                    .chain(iter::once(op.address))
                    .chain(op.data.iter().cloned())
            })
            .chain(
                iter::repeat_with(|| {
                    iter::repeat(BabyBear::zero()).take(1 + requester.field_width())
                })
                .take(TRACE_DEGREE - ops.len())
                .flatten(),
            )
            .collect(),
        1 + requester.field_width(),
    );

    USE_DEBUG_BUILDER.with(|debug| {
        *debug.lock().unwrap() = false;
    });
    assert_eq!(
        run_simple_test_no_pis(
            vec![&offline_checker, &range_checker.air, &requester],
            vec![offline_checker_trace, range_checker_trace, requester_trace],
        ),
        Err(VerificationError::OodEvaluationMismatch),
        "Expected verification to fail, but it passed"
    );
}
