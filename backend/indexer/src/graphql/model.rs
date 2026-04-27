use async_graphql::*;
use serde::{Deserialize, Serialize};
use crate::db::ProjectRecord;
use crate::events::EventRecord;

#[derive(SimpleObject, Clone)]
pub struct Project {
    pub project_id: String,
    pub creator: String,
    pub status: String,
    pub goal: String,
    pub primary_token: String,
    pub created_ledger: i64,
    pub created_at: i64,
    pub title: String,
    pub description: String,
}

impl From<ProjectRecord> for Project {
    fn from(record: ProjectRecord) -> Self {
        Self {
            project_id: record.project_id,
            creator: record.creator,
            status: record.status,
            goal: record.goal,
            primary_token: record.primary_token,
            created_ledger: record.created_ledger,
            created_at: record.created_at,
            title: record.title,
            description: record.description,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Event {
    pub id: ID,
    pub event_type: String,
    pub project_id: Option<String>,
    pub actor: Option<String>,
    pub amount: Option<String>,
    pub ledger: i64,
    pub timestamp: i64,
    pub contract_id: String,
    pub tx_hash: Option<String>,
    pub extra_data: Option<String>,
    pub created_at: i64,
}

impl From<EventRecord> for Event {
    fn from(record: EventRecord) -> Self {
        Self {
            id: ID::from(record.id),
            event_type: record.event_type,
            project_id: record.project_id,
            actor: record.actor,
            amount: record.amount,
            ledger: record.ledger,
            timestamp: record.timestamp,
            contract_id: record.contract_id,
            tx_hash: record.tx_hash,
            extra_data: record.extra_data,
            created_at: record.created_at,
        }
    }
}
