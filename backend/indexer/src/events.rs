//! Canonical event types emitted by the PIFP protocol contract.
//!
//! These mirror the Soroban contract events defined in `contracts/pifp_protocol/src/events.rs`
//! and `contracts/pifp_protocol/src/rbac.rs`.

use serde::{Deserialize, Serialize};

/// All recognised event kinds from the PIFP contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A new project was registered (`created` topic).
    ProjectCreated,
    /// A donation was made to a project (`funded` topic).
    ProjectFunded,
    /// An oracle verified a project's proof (`verified` topic).
    ProjectVerified,
    /// Verified funds were released to the creator (`released` topic).
    FundsReleased,
    /// Donator funds were refunded from an expired project (`refunded` topic).
    DonatorRefunded,
    /// A role was granted or replaced (`role_set` topic).
    RoleSet,
    /// A role was revoked (`role_del` topic).
    RoleDel,
    /// Protocol was paused (`paused` topic).
    ProtocolPaused,
    /// Protocol was unpaused (`unpaused` topic).
    ProtocolUnpaused,
    /// An event from this contract that we don't recognise yet.
    Unknown,
}

impl EventKind {
    /// Parse the leading topic symbol string produced by Soroban into an [`EventKind`].
    pub fn from_topic(topic: &str) -> Self {
        match topic {
            "created" => Self::ProjectCreated,
            "funded" => Self::ProjectFunded,
            "verified" => Self::ProjectVerified,
            "released" => Self::FundsReleased,
            "refunded" => Self::DonatorRefunded,
            "role_set" => Self::RoleSet,
            "role_del" => Self::RoleDel,
            "paused" => Self::ProtocolPaused,
            "unpaused" => Self::ProtocolUnpaused,
            _ => Self::Unknown,
        }
    }

    /// Return a short identifier string suitable for storage in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProjectCreated => "project_created",
            Self::ProjectFunded => "project_funded",
            Self::ProjectVerified => "project_verified",
            Self::FundsReleased => "funds_released",
            Self::DonatorRefunded => "donator_refunded",
            Self::RoleSet => "role_set",
            Self::RoleDel => "role_del",
            Self::ProtocolPaused => "protocol_paused",
            Self::ProtocolUnpaused => "protocol_unpaused",
            Self::Unknown => "unknown",
        }
    }
}

/// A fully decoded PIFP event, ready to be stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PifpEvent {
    pub event_type: String,
    pub project_id: Option<String>,
    pub actor: Option<String>,
    pub amount: Option<String>,
    pub ledger: i64,
    pub timestamp: i64,
    pub contract_id: String,
    pub tx_hash: Option<String>,
}

/// A raw event record as stored in / read from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRecord {
    pub id: i64,
    pub event_type: String,
    pub project_id: Option<String>,
    pub actor: Option<String>,
    pub amount: Option<String>,
    pub ledger: i64,
    pub timestamp: i64,
    pub contract_id: String,
    pub tx_hash: Option<String>,
    pub created_at: i64,
}
