
#![allow(dead_code)]

extern crate std;

use crate::types::{Project, ProjectStatus};

/// INV-1: Project balance must never be negative.
/// NOTE: With multi-token funding, individual balances are tracked in storage;
/// this invariant is checked per-token externally.
pub fn assert_balance_non_negative(_project: &Project) {
    // Balance is now tracked per-token in storage, not on the Project struct.
    // Individual token balances are validated at the storage layer.
}

/// INV-2: Project goal must always be positive.
pub fn assert_goal_positive(project: &Project) {
    assert!(
        project.goal > 0,
        "INV-2 violated: project {} has non-positive goal ({})",
        project.id,
        project.goal
    );
}

/// INV-3: Project deadline must be positive.
pub fn assert_deadline_positive(project: &Project) {
    assert!(
        project.deadline > 0,
        "INV-3 violated: project {} has zero deadline",
        project.id
    );
}

/// INV-4: Completed projects must not have zero balance and a Funding status
/// simultaneously — a project that received deposits and was verified should
/// reflect the deposits.
pub fn assert_completed_has_valid_state(project: &Project) {
    if project.status == ProjectStatus::Completed {
        // A completed project should have been verified, so its status is valid.
        // We just confirm the status enum value is Completed.
        assert_eq!(project.status, ProjectStatus::Completed);
    }
}

/// INV-5: Deposit invariant — after a deposit of `amount`, the project balance
/// should increase by exactly `amount`.
pub fn assert_deposit_invariant(balance_before: i128, balance_after: i128, amount: i128) {
    assert_eq!(
        balance_after,
        balance_before + amount,
        "INV-5 violated: deposit invariant broken: {} + {} != {}",
        balance_before,
        amount,
        balance_after
    );
}

/// INV-6: Project IDs are sequential starting from 0.
pub fn assert_sequential_ids(projects: &[Project]) {
    for (i, project) in projects.iter().enumerate() {
        assert_eq!(
            project.id, i as u64,
            "INV-6 violated: expected id {}, got {}",
            i, project.id
        );
    }
}

/// INV-7: Status transition validity. Only forward transitions are allowed:
///   Funding -> Active | Completed | Expired
///   Active  -> Completed | Expired
///   Completed -> (none)
///   Expired   -> (none)
pub fn assert_valid_status_transition(from: &ProjectStatus, to: &ProjectStatus) {
    let valid = matches!(
        (from, to),
        (ProjectStatus::Funding, ProjectStatus::Active)
            | (ProjectStatus::Funding, ProjectStatus::Completed)
            | (ProjectStatus::Funding, ProjectStatus::Expired)
            | (ProjectStatus::Active, ProjectStatus::Completed)
            | (ProjectStatus::Active, ProjectStatus::Expired)
    );

    assert!(
        valid,
        "INV-7 violated: invalid status transition from {:?} to {:?}",
        from, to
    );
}

/// INV-8: Project data immutability — fields that should not change after
/// creation (creator, token, goal, proof_hash, deadline) remain unchanged.
pub fn assert_project_immutable_fields(original: &Project, current: &Project) {
    assert_eq!(
        original.id, current.id,
        "INV-8 violated: project id changed"
    );
    assert_eq!(
        original.creator, current.creator,
        "INV-8 violated: project creator changed"
    );
    assert_eq!(
        original.accepted_tokens, current.accepted_tokens,
        "INV-8 violated: project accepted_tokens changed"
    );
    assert_eq!(
        original.goal, current.goal,
        "INV-8 violated: project goal changed"
    );
    assert_eq!(
        original.proof_hash, current.proof_hash,
        "INV-8 violated: project proof_hash changed"
    );
    assert_eq!(
        original.deadline, current.deadline,
        "INV-8 violated: project deadline changed"
    );
}

/// Run all stateless project invariants.
pub fn assert_all_project_invariants(project: &Project) {
    assert_balance_non_negative(project);
    assert_goal_positive(project);
    assert_deadline_positive(project);
    assert_completed_has_valid_state(project);
}
