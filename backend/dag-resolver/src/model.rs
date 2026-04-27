use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: String,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    #[serde(default)]
    pub after: Vec<String>,
}

impl Intent {
    pub fn normalized_reads(&self) -> BTreeSet<String> {
        self.reads
            .iter()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .collect()
    }

    pub fn normalized_writes(&self) -> BTreeSet<String> {
        self.writes
            .iter()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .collect()
    }

    pub fn normalized_after(&self) -> BTreeSet<String> {
        self.after
            .iter()
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DependencyReason {
    ReadAfterWrite { resource: String },
    WriteAfterRead { resource: String },
    WriteAfterWrite { resource: String },
    ExplicitAfter { dependency: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub reasons: Vec<DependencyReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelBatch {
    pub index: usize,
    pub intents: Vec<Intent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionReport {
    pub total_intents: usize,
    pub total_edges: usize,
    pub max_parallel_width: usize,
    pub batches: Vec<ParallelBatch>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IntentDocument {
    List(Vec<Intent>),
    Wrapped { intents: Vec<Intent> },
}

impl IntentDocument {
    pub fn into_intents(self) -> Vec<Intent> {
        match self {
            Self::List(intents) => intents,
            Self::Wrapped { intents } => intents,
        }
    }
}
