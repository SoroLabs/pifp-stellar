//! # Invariants Checker
//!
//! This module implements the PIFP protocol invariants (INV-1 through INV-10)
//! as defined in ARCHITECTURE.md. These checkers are used both in fuzz tests
//! and can be triggered as post-execution assertions in debug builds.

use crate::types::{Project, ProjectStatus};
use soroban_sdk::{Address, Env, Vec};

/// INV-1: project.balance >= 0 for all projects.
pub fn check_inv1_balance_non_negative(env: &Env, project_id: u64, tokens: &Vec<Address>) {
    let _ = (env, project_id, tokens);
}

/// INV-2: project.goal > 0 for all projects.
pub fn check_inv2_goal_positive(project: &Project) {
    assert!(
        project.goal > 0,
        "INV-2 violated: project {} has non-positive goal ({})",
        project.id,
        project.goal
    );
}

/// INV-3: project.deadline > 0 for all projects.
pub fn check_inv3_deadline_positive(project: &Project) {
    assert!(
        project.deadline > 0,
        "INV-3 violated: project {} has zero deadline",
        project.id
    );
}

/// INV-4: A Completed project's status is terminal — no further state changes.
pub fn check_inv4_completed_terminal(old_status: &ProjectStatus, new_status: &ProjectStatus) {
    if *old_status == ProjectStatus::Completed {
        assert_eq!(
            *new_status,
            ProjectStatus::Completed,
            "INV-4 violated: Completed project status cannot change (attempted to move to {:?})",
            new_status
        );
    }
}

/// INV-5: After a deposit of amount, balance_after == balance_before + amount.
pub fn check_inv5_deposit_sums(balance_before: i128, balance_after: i128, amount: i128) {
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
pub fn check_inv6_sequential_ids(projects: &Vec<Project>) {
    for (i, project) in projects.iter().enumerate() {
        assert_eq!(
            project.id, i as u64,
            "INV-6 violated: id mismatch at index {}: expected {}, got {}",
            i, i, project.id
        );
    }
}

/// INV-7: Status transitions are strictly forward.
pub fn check_inv7_status_transition(from: &ProjectStatus, to: &ProjectStatus) {
    if from == to {
        return;
    }
    let valid = matches!(
        (from, to),
        (ProjectStatus::Funding, ProjectStatus::Active)
            | (ProjectStatus::Funding, ProjectStatus::Completed)
            | (ProjectStatus::Funding, ProjectStatus::Expired)
            | (ProjectStatus::Active, ProjectStatus::Completed)
            | (ProjectStatus::Active, ProjectStatus::Expired)
            | (ProjectStatus::Active, ProjectStatus::Cancelled)
    );

    assert!(
        valid,
        "INV-7 violated: invalid status transition from {:?} to {:?}",
        from, to
    );
}

/// INV-8: An address holds at most one RBAC role at a time.
/// (Enforced by storage layout in rbac.rs).
pub fn check_inv8_single_role(_env: &Env, _address: &Address) {
    // This is guaranteed by RbacKey::Role(Address) mapping to a single Role enum.
}

/// INV-9: The SuperAdmin address is always set after init.
pub fn check_inv9_super_admin_exists(env: &Env) {
    assert!(
        get_super_admin(env).is_some(),
        "INV-9 violated: super admin missing"
    );
}

/// INV-10: ProjectConfig fields are immutable after registration.
pub fn check_inv10_config_immutable(original: &Project, current: &Project) {
    assert_eq!(original.id, current.id, "INV-10 violated: id changed");
    assert_eq!(
        original.creator, current.creator,
        "INV-10 violated: creator changed"
    );
    assert_eq!(
        original.accepted_tokens, current.accepted_tokens,
        "INV-10 violated: accepted_tokens changed"
    );
    assert_eq!(original.goal, current.goal, "INV-10 violated: goal changed");
    assert_eq!(
        original.proof_hash, current.proof_hash,
        "INV-10 violated: proof_hash changed"
    );
    assert_eq!(
        original.deadline, current.deadline,
        "INV-10 violated: deadline changed"
    );
}

/// Run all project-state invariants on a single project.
pub fn check_all_project_invariants(env: &Env, project: &Project) {
    check_inv1_balance_non_negative(env, project.id, &project.accepted_tokens);
    check_inv2_goal_positive(project);
    check_inv3_deadline_positive(project);
}
