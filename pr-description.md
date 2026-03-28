## Summary

This PR improves indexer sync performance and operational reliability by:

1. Implementing high-performance event topic filtering for Soroban `getEvents`.
2. Adding multi-contract indexing support for multi-deployment environments.
3. Adding deterministic backfill controls from a specific ledger/cursor.
4. Resolving workspace-wide CI failures (`fmt`/`clippy`) so this branch passes the same checks as GitHub Actions.

## Problem

The indexer previously relied on broad contract event scans and then decoded/discarded many irrelevant records. This increased processing overhead and slowed catch-up.

Additionally, CI checks were failing because:
- Workspace formatting drift existed in the contract crate.
- Workspace `clippy -D warnings` surfaced deprecation/dead-code/style warnings in `contracts/pifp_protocol`.

## Solution

### 1) High-performance event filtering

Updated RPC polling to request only relevant PIFP topics at the filter layer:

- `created`
- `funded`
- `active`
- `verified`
- `expired`
- `cancelled`
- `released`
- `refunded`
- `role_set`
- `role_del`
- `paused`
- `unpaused`

If a node rejects the `topics` filter shape (`-32602`), the indexer automatically falls back to contract-only filtering to preserve reliability.

### 2) Multi-deployment contract support

Added support for indexing multiple contract deployments in one process:

- New env var: `CONTRACT_IDS` (comma-separated)
- Backward-compatible fallback: `CONTRACT_ID` (single contract)

Decode path now drops events outside configured tracked contracts (defense-in-depth).

### 3) Reliable backfilling controls

Added deterministic startup override controls:

- `BACKFILL_START_LEDGER`
- `BACKFILL_CURSOR`

Behavior:
- If either backfill override is set, startup uses those values.
- Otherwise it resumes from persisted cursor/ledger as before.

### 4) CI hardening fixes

To pass workspace CI (`cargo fmt --all --check`, `cargo clippy --all-targets --all-features -D warnings`), this PR includes:

- Formatting alignment in `contracts/pifp_protocol`.
- Contract clippy fixes:
  - deprecated Soroban events API allowance in event emission modules.
  - dead-code allowance for helper not currently called.
  - `?` simplification in optional load path.
  - needless borrow removal in token transfer call.

## Files changed (high-level)

- `backend/indexer/src/rpc.rs`
  - topic-filtered `getEvents` params
  - fallback to contract-only filtering
  - multi-contract aware decoding and filtering
  - benchmark/unit tests
- `backend/indexer/src/config.rs`
  - `CONTRACT_IDS` support
  - `BACKFILL_START_LEDGER` / `BACKFILL_CURSOR`
- `backend/indexer/src/indexer.rs`
  - startup/resume logic updated for multi-contract + backfill overrides
- `backend/indexer/README.md`
  - updated docs for contract list and backfill envs
- `contracts/pifp_protocol/src/{events.rs,rbac.rs,storage.rs,lib.rs,...}`
  - CI-related fmt/clippy compatibility fixes

## Performance evidence

Added benchmark-style unit test (`compare_filtered_vs_broad_decode_speed`) comparing filtered vs broad decode flows on synthetic mixed traffic.

Sample run output:

`decode benchmark: filtered=5000 in 937.287084ms, broad=50000 in 1.789831791s`

This shows lower processing cost when irrelevant events are filtered earlier.

## Testing

Ran locally:

- `cargo fmt --manifest-path contracts/pifp_protocol/Cargo.toml -- --check`
- `cargo clippy --manifest-path contracts/pifp_protocol/Cargo.toml -- -D warnings`
- `cargo test --manifest-path contracts/pifp_protocol/Cargo.toml`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

All passed.

## Backward compatibility

- Existing single-contract deployments continue to work via `CONTRACT_ID`.
- No DB migration required for this PR.
- Runtime behavior is strictly more selective and safer under noisy event streams.

## Follow-ups (optional)

- Add integration benchmark against live RPC datasets for end-to-end sync throughput measurements.
- Add metrics around fallback rate (topic-filter rejected -> contract-only) to monitor RPC compatibility in production.
