use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use afs_stark_backend::config::{Com, PcsProof, PcsProverData, StarkGenericConfig, Val};
use datafusion::{error::Result, execution::context::SessionContext, logical_expr::LogicalPlan};
use p3_field::PrimeField64;
use serde::{de::DeserializeOwned, Serialize};

use self::{filter::Filter, page_scan::PageScan};
use crate::{afs_exec::ChildrenContainer, afs_expr::AfsExpr, committed_page::CommittedPage};

pub mod filter;
pub mod page_scan;

macro_rules! delegate_to_node {
    ($self:ident, $method:ident, $ctx:expr) => {
        match $self {
            AfsNode::PageScan(ref mut page_scan) => page_scan.$method($ctx).await,
            AfsNode::Filter(ref mut filter) => filter.$method($ctx).await,
        }
    };
    ($self:ident, $method:ident) => {
        match $self {
            AfsNode::PageScan(page_scan) => page_scan.$method(),
            AfsNode::Filter(filter) => filter.$method(),
        }
    };
}

pub trait AfsNodeExecutable<SC: StarkGenericConfig> {
    /// Runs the node's execution logic without any cryptographic operations
    async fn execute(&mut self, ctx: &SessionContext) -> Result<()>;
    /// Generate the proving key for the node
    async fn keygen(&mut self, ctx: &SessionContext) -> Result<()>;
    /// Geenrate the STARK proof for the node
    async fn prove(&mut self, ctx: &SessionContext) -> Result<()>;
    /// Verify the STARK proof for the node
    async fn verify(&self, ctx: &SessionContext) -> Result<()>;
    /// Get the output of the node
    fn output(&self) -> Option<Arc<CommittedPage<SC>>>;
}

pub enum AfsNode<SC: StarkGenericConfig> {
    PageScan(PageScan<SC>),
    Filter(Filter<SC>),
}

impl<SC: StarkGenericConfig> AfsNode<SC>
where
    Val<SC>: PrimeField64,
    PcsProverData<SC>: Serialize + DeserializeOwned + Send + Sync,
    PcsProof<SC>: Send + Sync,
    Com<SC>: Send + Sync,
    SC::Pcs: Send + Sync,
    SC::Challenge: Send + Sync,
{
    pub fn from(logical_plan: &LogicalPlan, children: ChildrenContainer<SC>) -> Self {
        match logical_plan {
            LogicalPlan::TableScan(table_scan) => {
                let page_id = table_scan.table_name.to_string();
                let source = table_scan.source.clone();
                AfsNode::PageScan(PageScan {
                    page_id,
                    pk: None,
                    input: source,
                    output: None,
                })
            }
            LogicalPlan::Filter(filter) => {
                let afs_expr = AfsExpr::from(&filter.predicate);
                let input = match children {
                    ChildrenContainer::One(child) => child,
                    _ => panic!("Filter node expects exactly one child"),
                };
                AfsNode::Filter(Filter {
                    predicate: afs_expr,
                    pk: None,
                    input,
                    output: None,
                })
            }
            _ => panic!("Invalid node type: {:?}", logical_plan),
        }
    }

    pub fn inputs(&self) -> Vec<&Arc<AfsNode<SC>>> {
        match self {
            AfsNode::PageScan(_) => vec![],
            AfsNode::Filter(filter) => vec![&filter.input],
        }
    }

    pub async fn execute(&mut self, ctx: &SessionContext) -> Result<()> {
        delegate_to_node!(self, execute, ctx)
    }

    pub async fn keygen(&mut self, ctx: &SessionContext) -> Result<()> {
        delegate_to_node!(self, keygen, ctx)
    }

    pub async fn prove(&mut self, ctx: &SessionContext) -> Result<()> {
        delegate_to_node!(self, prove, ctx)
    }

    pub async fn verify(&mut self, ctx: &SessionContext) -> Result<()> {
        delegate_to_node!(self, verify, ctx)
    }

    pub fn output(&self) -> Option<Arc<CommittedPage<SC>>> {
        delegate_to_node!(self, output)
    }
}

impl<SC: StarkGenericConfig> Debug for AfsNode<SC> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AfsNode::PageScan(page_scan) => write!(f, "PageScan {:?}", page_scan.page_id),
            AfsNode::Filter(filter) => write!(f, "Filter {:?}", filter.predicate),
        }
    }
}
