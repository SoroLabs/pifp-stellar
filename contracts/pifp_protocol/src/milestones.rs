use soroban_sdk::{BytesN, Env, Vec};
use crate::errors::Error;
use crate::types::Milestone;

/// Validates that a milestone can be released and updates the completion tracker.
pub fn verify_milestone(
    env: &Env,
    milestones: &Vec<Milestone>,
    completed_tracker: &mut Vec<bool>,
    milestone_index: u32,
    submitted_hash: BytesN<32>,
) -> Result<u32, Error> {
    if milestone_index >= milestones.len() {
        return Err(Error::InvalidTransition); // Out of bounds
    }

    let milestone = milestones.get(milestone_index).unwrap();
    let is_already_done = completed_tracker.get(milestone_index).unwrap();

    if is_already_done {
        return Err(Error::MilestoneAlreadyReleased);
    }

    if submitted_hash != milestone.proof_hash {
        return Err(Error::VerificationFailed);
    }

    // Mark as completed
    completed_tracker.set(milestone_index, true);

    Ok(milestone.amount_bps)
}

/// Helper to ensure a project registration has a valid milestone set.
pub fn validate_milestone_set(env: &Env, milestones: &Vec<Milestone>) {
    let mut total_bps: u32 = 0;
    for m in milestones.iter() {
        total_bps += m.amount_bps;
    }
    if total_bps != 10000 {
        soroban_sdk::panic_with_error!(env, Error::InvalidGoal); // Or custom Error::InvalidMilestoneTotal
    }
}