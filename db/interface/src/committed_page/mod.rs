use afs_page::common::page::Page;
use afs_stark_backend::{
    config::{Com, PcsProof, PcsProverData, StarkGenericConfig, Val},
    prover::trace::ProverTraceData,
};
use datafusion::arrow::{
    array::{Int64Array, RecordBatch, UInt32Array},
    datatypes::{DataType, Schema},
};
use derivative::Derivative;
use p3_field::PrimeField64;
use p3_uni_stark::Domain;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use self::utils::{page_to_record_batch, record_batch_to_page};
use crate::{utils::data_types::num_fe, NUM_IDX_COLS};

pub mod column;
pub mod execution_plan;
pub mod table_provider;
pub mod utils;

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Clone(bound = "ProverTraceData<SC>: Clone"))]
#[serde(bound(
    serialize = "ProverTraceData<SC>: Serialize",
    deserialize = "ProverTraceData<SC>: Deserialize<'de>"
))]
pub struct CommittedPage<SC: StarkGenericConfig> {
    pub page_id: String,
    pub schema: Schema,
    pub page: Page,
    pub cached_trace: Option<ProverTraceData<SC>>,
}

impl<SC: StarkGenericConfig> CommittedPage<SC>
where
    Val<SC>: PrimeField64,
    PcsProverData<SC>: Serialize + DeserializeOwned + Send + Sync,
    PcsProof<SC>: Send + Sync,
    Domain<SC>: Send + Sync,
    Com<SC>: Send + Sync,
    SC::Pcs: Send + Sync,
    SC::Challenge: Send + Sync,
{
    pub fn new(
        page_id: String,
        schema: Schema,
        page: Page,
        cached_trace: Option<ProverTraceData<SC>>,
    ) -> Self {
        Self {
            page_id,
            schema,
            page,
            cached_trace,
        }
    }

    pub fn from_file(path: &str) -> Self {
        let bytes = std::fs::read(path).unwrap();
        let committed_page: CommittedPage<SC> = bincode::deserialize(&bytes).unwrap();
        committed_page
    }

    pub fn from_record_batch(rb: RecordBatch, height: usize) -> Self {
        let schema = (*rb.schema()).clone();
        let page = record_batch_to_page(&rb, height);
        Self {
            // TODO: generate a page_id based on the hash of the Page
            page_id: "".to_string(),
            schema,
            page,
            cached_trace: None,
        }
    }

    pub fn to_record_batch(&self) -> RecordBatch {
        page_to_record_batch(self.page.clone(), self.schema.clone())
    }

    pub fn write_cached_trace(&mut self, trace: ProverTraceData<SC>) {
        self.cached_trace = Some(trace);
    }
}

impl<SC: StarkGenericConfig> std::fmt::Debug for CommittedPage<SC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CommittedPage {{ page_id: {}, schema: {:?}, page: {:?} }}",
            self.page_id, self.schema, self.page
        )
    }
}

#[macro_export]
macro_rules! committed_page {
    ($name:expr, $page_path:expr, $schema_path:expr, $config:tt) => {{
        let page_path = std::fs::read($page_path).unwrap();
        let page: Page = bincode::deserialize(&page_path).unwrap();
        let schema_path = std::fs::read($schema_path).unwrap();
        let schema: Schema = bincode::deserialize(&schema_path).unwrap();
        $crate::committed_page::CommittedPage::<$config>::new($name.to_string(), schema, page, None)
    }};
}
