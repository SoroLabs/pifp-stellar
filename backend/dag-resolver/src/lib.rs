pub mod error;
pub mod graph;
pub mod model;
pub mod planner;

pub use error::{DagResolutionError, Result};
pub use model::{
    DependencyEdge, DependencyReason, Intent, IntentDocument, ParallelBatch, ResolutionReport,
};
pub use planner::resolve_intents;
