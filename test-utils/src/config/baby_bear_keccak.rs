use afs_stark_backend::{rap::AnyRap, verifier::VerificationError};
use p3_baby_bear::BabyBear;
use p3_keccak::Keccak256Hash;
use p3_matrix::dense::DenseMatrix;

use super::{
    baby_bear_bytehash::{
        self, config_from_byte_hash, BabyBearByteHashConfig, BabyBearByteHashEngine,
    },
    fri_params::default_fri_params,
};
use crate::engine::StarkEngine;

pub type BabyBearKeccakConfig = BabyBearByteHashConfig<Keccak256Hash>;
pub type BabyBearKeccakEngine = BabyBearByteHashEngine<Keccak256Hash>;

/// `pcs_log_degree` is the upper bound on the log_2(PCS polynomial degree).
pub fn default_engine() -> BabyBearKeccakEngine {
    baby_bear_bytehash::default_engine(Keccak256Hash)
}

/// `pcs_log_degree` is the upper bound on the log_2(PCS polynomial degree).
pub fn default_config() -> BabyBearKeccakConfig {
    let fri_params = default_fri_params();
    config_from_byte_hash(Keccak256Hash, fri_params)
}

/// Runs a single end-to-end test for a given set of chips and traces.
/// This includes proving/verifying key generation, creating a proof, and verifying the proof.
/// This function should only be used on chips where the main trace is **not** partitioned.
///
/// Do not use this if you want to generate proofs for different traces with the same proving key.
///
/// - `chips`, `traces`, `public_values` should be zipped.
pub fn run_simple_test(
    chips: Vec<&dyn AnyRap<BabyBearKeccakConfig>>,
    traces: Vec<DenseMatrix<BabyBear>>,
    public_values: Vec<Vec<BabyBear>>,
) -> Result<(), VerificationError> {
    let engine = default_engine();
    engine.run_simple_test(chips, traces, public_values)
}

/// [run_simple_test] without public values
pub fn run_simple_test_no_pis(
    chips: Vec<&dyn AnyRap<BabyBearKeccakConfig>>,
    traces: Vec<DenseMatrix<BabyBear>>,
) -> Result<(), VerificationError> {
    let num_chips = chips.len();
    run_simple_test(chips, traces, vec![vec![]; num_chips])
}
