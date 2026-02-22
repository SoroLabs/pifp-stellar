## Summary

Exposes the `get_project_balances` contract entry point as required by issue #57.

## Changes

- **`lib.rs`**: Added `get_project_balances(env, project_id) -> ProjectBalances` entry point. Calls `storage::get_all_balances` internally. Panics with `Error::ProjectNotFound` if the project ID is invalid.
- **`test_events.rs`**: Added `test_get_project_balances` — registers a project with two accepted tokens, deposits distinct amounts for each, and asserts the returned `ProjectBalances` matches exactly.

## Notes

- `storage::get_all_balances` was already fully implemented; this PR simply makes it publicly reachable via a contract entry point.
- `ProjectBalances` and `TokenBalance` types were already defined in `types.rs`.
- All 4 tests pass.

Closes #57
