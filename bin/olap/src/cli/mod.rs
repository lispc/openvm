use afs_stark_backend::config::{Com, PcsProof, PcsProverData};
use afs_test_utils::{
    config::{
        baby_bear_blake3::BabyBearBlake3Engine,
        baby_bear_bytehash::engine_from_byte_hash,
        baby_bear_keccak::BabyBearKeccakEngine,
        baby_bear_poseidon2::{self, BabyBearPoseidon2Engine},
        baby_bear_sha256_compress::{self, BabyBearSha256CompressionEngine},
        goldilocks_poseidon::{self, GoldilocksPoseidonEngine},
        m31_sha256_compress::{self, Mersenne31Sha256CompressionEngine},
        EngineType,
    },
    engine::StarkEngine,
    page_config::PageConfig,
};
use clap::{Parser, Subcommand};
use p3_blake3::Blake3;
use p3_field::PrimeField64;
use p3_keccak::Keccak256Hash;
use p3_uni_stark::{Domain, StarkGenericConfig, Val};
use serde::{de::DeserializeOwned, Serialize};

use crate::commands::{
    cache::CacheCommand, keygen::KeygenCommand, prove::ProveCommand, verify::VerifyCommand,
};

#[derive(Debug, Parser)]
#[command(author, version, about = "OLAP CLI")]
#[command(propagate_version = true)]
pub struct Cli<SC: StarkGenericConfig, E: StarkEngine<SC>> {
    #[command(subcommand)]
    pub command: CliCommand<SC, E>,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand<SC: StarkGenericConfig, E: StarkEngine<SC>> {
    #[command(name = "keygen", about = "Generate proving and verifying keys")]
    /// Run key generation
    Keygen(KeygenCommand<SC, E>),

    #[command(name = "cache", about = "Cache trace data")]
    /// Run cache command
    Cache(CacheCommand<SC, E>),

    #[command(name = "prove", about = "Run proof generation")]
    /// Run proof generation
    Prove(ProveCommand<SC, E>),

    #[command(name = "verify", about = "Verify the proof")]
    /// Run proof verification
    Verify(VerifyCommand<SC, E>),
}

impl<SC: StarkGenericConfig, E: StarkEngine<SC>> Cli<SC, E>
where
    Val<SC>: PrimeField64,
    PcsProverData<SC>: Serialize + DeserializeOwned + Send + Sync,
    PcsProof<SC>: Send + Sync,
    Domain<SC>: Send + Sync,
    Com<SC>: Send + Sync,
    SC::Pcs: Sync,
    SC::Challenge: Send + Sync,
{
    pub fn run_with_engine(config: &PageConfig, engine: &E)
    where
        E: StarkEngine<SC>,
    {
        let cli = Self::parse();
        match &cli.command {
            CliCommand::Keygen(keygen) => {
                KeygenCommand::execute(config, engine, &keygen.common, keygen.keys_folder.clone())
                    .unwrap();
            }
            CliCommand::Cache(cache) => {
                CacheCommand::execute(config, engine, &cache.common, cache.cache_folder.clone())
                    .unwrap();
            }
            CliCommand::Prove(prove) => {
                ProveCommand::execute(
                    config,
                    engine,
                    &prove.common,
                    prove.keys_folder.clone(),
                    prove.cache_folder.clone(),
                )
                .unwrap();
            }
            CliCommand::Verify(verify) => {
                VerifyCommand::execute(
                    config,
                    engine,
                    &verify.common,
                    verify.keys_folder.clone(),
                    verify.cache_folder.clone(),
                    verify.proof_path.clone(),
                )
                .unwrap();
            }
        }
    }
}

pub fn run(config: &PageConfig) {
    let fri_params = config.fri_params;
    let engine_type = config.stark_engine.engine;
    match engine_type {
        EngineType::BabyBearBlake3 => {
            let engine: BabyBearBlake3Engine = engine_from_byte_hash(Blake3, fri_params);
            Cli::run_with_engine(config, &engine)
        }
        EngineType::BabyBearKeccak => {
            let engine: BabyBearKeccakEngine = engine_from_byte_hash(Keccak256Hash, fri_params);
            Cli::run_with_engine(config, &engine)
        }
        EngineType::BabyBearSha256Compress => {
            let engine: BabyBearSha256CompressionEngine =
                baby_bear_sha256_compress::engine_from_fri_params(fri_params);
            Cli::run_with_engine(config, &engine)
        }
        EngineType::Mersenne31Sha256Compress => {
            let engine: Mersenne31Sha256CompressionEngine =
                m31_sha256_compress::engine_from_fri_params(fri_params);
            Cli::run_with_engine(config, &engine)
        }
        EngineType::BabyBearPoseidon2 => {
            let perm = baby_bear_poseidon2::default_perm();
            let engine: BabyBearPoseidon2Engine =
                baby_bear_poseidon2::engine_from_perm(perm, fri_params);
            Cli::run_with_engine(config, &engine)
        }
        EngineType::GoldilocksPoseidon => {
            let perm = goldilocks_poseidon::random_perm();
            let engine: GoldilocksPoseidonEngine =
                goldilocks_poseidon::engine_from_perm(perm, fri_params);
            Cli::run_with_engine(config, &engine)
        }
    }
}
