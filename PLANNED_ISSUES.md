# Planned Issues for pifp-stellar

These are high-priority tasks designed for contributors. Please create these as GitHub issues following the templates.

---

## Issue 1: Implement Core Project Registry (Smart Contract)
**Complexity:** High (200 points)
**Type:** Feature Request

### Context & Goal
The current `PifpProtocol` contract is a skeleton. We need a robust registry to store project details on-chain. This allows the frontend to query and display available projects.

### Requirements
- [ ] Update `Project` struct to include:
    - `deadline: u64` (timestamp)
    - `target_amount: i128`
    - `current_amount: i128`
    - `status: ProjectStatus` (enum: Funding, Active, Completed, Expired)
- [ ] Implement `register_project` function that validates inputs (deadline in future, target > 0).
- [ ] Implement `get_project(id)` to retrieve details.

### Implementation Guidelines
- Modify `contracts/pifp_protocol/src/lib.rs`.
- Use `soroban_sdk::Map` or `Vec` for storage efficiency if needed, or keyed storage.
- Add unit tests for successful registration and failure cases (invalid deadline).

### ETA
Required.

---

## Issue 2: Implement Donation with Commitments (Smart Contract)
**Complexity:** High (200 points)
**Type:** Feature Request

### Context & Goal
We need to enable users to fund projects anonymously. This involves a commit-reveal scheme or simple hash-based commitments initially.

### Requirements
- [ ] Implement `donate(project_id, amount, commitment_hash)`.
- [ ] Store the commitment on-chain mapped to the `project_id`.
- [ ] Update `Project.current_amount`.
- [ ] Ensure `donate` fails if project is expired or completed.

### Implementation Guidelines
- Extend `PifpProtocol` in `contracts/pifp_protocol/src/lib.rs`.
- Verify `amount > 0`.
- Emit an event `DonationReceived(project_id, amount)`.

### ETA
Required.

---

## Issue 3: Backend Oracle Skeleton (Rust)
**Complexity:** Medium (150 points)
**Type:** Feature Request

### Context & Goal
The smart contract relies on an oracle to submit verified proof hashes. We need a Rust-based backend service that can listen for events or be triggered to verify off-chain data.

### Requirements
- [ ] Set up a new Rust crate `pifp-oracle`.
- [ ] Implement a basic structure:
    - `main.rs` entry point.
    - `config` struct (stellar node url, admin key).
    - `verify_proof(proof_data) -> hash` function.
- [ ] A mock function `submit_to_contract(hash)` that just logs for now.

### Implementation Guidelines
- Create a new folder `backend/oracle`.
- use `soroban-client` (or appropriate SDK) for future chain interaction.

### ETA
Required.
