use std::sync::Arc;

use afs_stark_backend::{
    config::{Com, PcsProof, PcsProverData, StarkGenericConfig, Val},
    keygen::types::MultiStarkProvingKey,
};
use afs_test_utils::engine::StarkEngine;
use datafusion::{
    arrow::datatypes::Schema, error::Result, execution::context::SessionContext,
    logical_expr::TableSource,
};
use futures::lock::Mutex;
use p3_field::PrimeField64;
use serde::{de::DeserializeOwned, Serialize};

use super::{AfsNode, AfsNodeExecutable};
use crate::{afs_expr::AfsExpr, committed_page::CommittedPage};

pub struct Projection<SC: StarkGenericConfig, E: StarkEngine<SC>> {
    pub schema: Schema,
    pub pk: Option<MultiStarkProvingKey<SC>>,
    pub input: Arc<Mutex<AfsNode<SC, E>>>,
    pub output: Option<CommittedPage<SC>>,
}

impl<SC: StarkGenericConfig, E: StarkEngine<SC>> AfsNodeExecutable<SC, E> for Projection<SC, E>
where
    Val<SC>: PrimeField64,
    PcsProverData<SC>: Serialize + DeserializeOwned + Send + Sync,
    PcsProof<SC>: Send + Sync,
    Com<SC>: Send + Sync,
    SC::Pcs: Send + Sync,
    SC::Challenge: Send + Sync,
{
    async fn execute(&mut self, ctx: &SessionContext) -> Result<()> {
        let input_page = self.input.lock().await.output().as_ref().unwrap();
        Ok(())
    }

    async fn keygen(&mut self, ctx: &SessionContext, engine: &E) -> Result<()> {
        Ok(())
    }

    async fn prove(&mut self, ctx: &SessionContext) -> Result<()> {
        Ok(())
    }

    async fn verify(&self, ctx: &SessionContext) -> Result<()> {
        Ok(())
    }

    fn output(&self) -> &Option<CommittedPage<SC>> {
        &self.output
    }
}
