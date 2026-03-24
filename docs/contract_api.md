# PIFP Smart Contract API Reference

This document provides a comprehensive reference for interacting with the `PifpProtocol` smart contract on the Soroban network.

## 1. Types & Enums

### Core Types

- **`Project`**: The main structure representing a project.
  - `id`: `u64` - Unique project identifier.
  - `creator`: `Address` - Address of the project creator.
  - `accepted_tokens`: `Vec<Address>` - List of token addresses accepted for deposits.
  - `goal`: `i128` - Funding target amount.
  - `proof_hash`: `BytesN<32>` - Cryptographic hash of the expected proof artifact.
  - `deadline`: `u64` - Ledger timestamp deadline.
  - `status`: `ProjectStatus` - Current state of the project.
  - `donation_count`: `u32` - Number of unique donors.

- **`ProjectBalances`**:
  - `balances`: `Map<Address, i128>` - Current funded amount per accepted token.

### Enums

- **`ProjectStatus`**: 
  - `Funding` (0): Actively accepting deposits. Goal not yet reached.
  - `Active` (1): Goal reached. Still active and waiting for verification or expiration.
  - `Expired` (2): Deadline passed, funds can be refunded.
  - `Completed` (3): Proof verified, funds released to creator.

- **`Role`**:
  - `SuperAdmin` (0), `Admin` (1), `Oracle` (2), `ProjectManager` (3), `Auditor` (4)

- **`Error`**: See the [Error Catalogue](../ARCHITECTURE.md#error-catalogue) in `ARCHITECTURE.md` for error code definitions.

---

## 2. API Reference

All invocations assume you are using the Soroban CLI and have `--source`, `--network`, and `--id` set appropriately for your network.

### Initialization

#### `init`
Initialise the contract and set the first SuperAdmin. Must be called exactly once immediately after deployment.

- **Signature**: `fn init(env: Env, super_admin: Address)`
- **Parameters**:
  - `super_admin` (`Address`): The address to grant the initial `SuperAdmin` role.
- **Returns**: `void`
- **Events**: None.
- **Errors**: `AlreadyInitialized` (8)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    --source admin_wallet \
    -- init --super_admin <ADMIN_ADDRESS>
  ```

---

### Role Management

#### `grant_role`
Grant a specific role to an address.

- **Signature**: `fn grant_role(env: Env, caller: Address, target: Address, role: Role)`
- **Parameters**:
  - `caller` (`Address`): The admin performing the operation. Must hold `SuperAdmin` or `Admin`.
  - `target` (`Address`): The recipient address.
  - `role` (`Role`): The role to assign (0-4). Only `SuperAdmin` can grant `SuperAdmin` (0).
- **Returns**: `void`
- **Events**: `role_set` (emitted by RBAC module).
- **Errors**: `NotAuthorized` (6)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    --source admin_wallet \
    -- grant_role --caller <ADMIN_ADDRESS> \
                   --target <TARGET_ADDRESS> \
                   --role 3
  ```

#### `revoke_role`
Revoke any role currently held by the target address.

- **Signature**: `fn revoke_role(env: Env, caller: Address, target: Address)`
- **Parameters**:
  - `caller` (`Address`): Must hold `SuperAdmin` or `Admin`. Cannot revoke SuperAdmin.
  - `target` (`Address`): The address to lose its role.
- **Returns**: `void`
- **Events**: `role_del` (emitted by RBAC module).
- **Errors**: `NotAuthorized` (6)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    --source admin_wallet \
    -- revoke_role --caller <ADMIN_ADDRESS> --target <TARGET_ADDRESS>
  ```

#### `transfer_super_admin`
Atomically transfer the SuperAdmin role.

- **Signature**: `fn transfer_super_admin(env: Env, current_super_admin: Address, new_super_admin: Address)`
- **Parameters**:
  - `current_super_admin` (`Address`): The current SuperAdmin issuing the transfer.
  - `new_super_admin` (`Address`): The new SuperAdmin.
- **Returns**: `void`
- **Events**: `role_del`, `role_set` (emitted by RBAC module).
- **Errors**: `NotAuthorized` (6)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    --source current_admin \
    -- transfer_super_admin --current_super_admin <OLD_ADMIN> \
                             --new_super_admin <NEW_ADMIN>
  ```

#### `role_of`
Query the role currently held by an address.

- **Signature**: `fn role_of(env: Env, address: Address) -> Option<Role>`
- **Parameters**: `address` (`Address`)
- **Returns**: `Option<Role>` (The numeric Enum role or null if none).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID -- role_of --address <ADDRESS>
  ```

#### `has_role`
Check if an address holds a specific role.

- **Signature**: `fn has_role(env: Env, address: Address, role: Role) -> bool`
- **Parameters**:
  - `address` (`Address`): Address to check.
  - `role` (`Role`): Target role.
- **Returns**: `bool`
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    -- has_role --address <ADDRESS> --role 3
  ```

#### `set_oracle`
Grant the Oracle role to an address. (Syntactic sugar for `grant_role(env, caller, oracle, Role::Oracle)`).

- **Signature**: `fn set_oracle(env: Env, caller: Address, oracle: Address)`
- **Parameters**:
  - `caller` (`Address`): Admin or SuperAdmin.
  - `oracle` (`Address`): Address to receive the Oracle role.
- **Returns**: `void`
- **Events**: `role_set` (emitted by RBAC module).
- **Errors**: `NotAuthorized` (6)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source admin_wallet \
    -- set_oracle --caller <ADMIN_ADDRESS> --oracle <ORACLE_ADDRESS>
  ```

---

### Emergency Control

#### `pause` / `unpause`
Halt or resume the protocol. Halts registrations, deposits, verifications.

- **Signature**: `fn pause(env: Env, caller: Address)` / `fn unpause(env: Env, caller: Address)`
- **Parameters**: `caller` (`Address`) - Admin or SuperAdmin.
- **Returns**: `void`
- **Events**: `paused` / `unpaused`
- **Errors**: `NotAuthorized` (6)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source admin_wallet \
    -- pause --caller <ADMIN_ADDRESS>
  ```

#### `is_paused`
Query whether the protocol is currently paused.

- **Signature**: `fn is_paused(env: Env) -> bool`
- **Returns**: `bool`
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID -- is_paused
  ```

---

### Project Lifecycle

#### `register_project`
Register a new project. Required to start accepting funds.

- **Signature**: `fn register_project(env: Env, creator: Address, accepted_tokens: Vec<Address>, goal: i128, proof_hash: BytesN<32>, deadline: u64) -> Project`
- **Parameters**:
  - `creator` (`Address`): Address of the caller. Must hold Admin, SuperAdmin, or ProjectManager role.
  - `accepted_tokens` (`Vec<Address>`): SAC Token addresses acceptable for donation. Max length 10.
  - `goal` (`i128`): Funding target (>0).
  - `proof_hash` (`BytesN<32>`): 32-byte cryptographic hash of the proof artifact that the oracle will later supply.
  - `deadline` (`u64`): Ledger closing timestamp indicating the expiry of the project. Minimum is current time, max 5 years.
- **Returns**: `Project` struct representing the created project.
- **Events**: `created` (`ProjectCreated`)
- **Errors**: `ProtocolPaused` (19), `NotAuthorized` (6), `EmptyAcceptedTokens` (17), `TooManyTokens` (10), `DuplicateToken` (12), `InvalidGoal` (7), `InvalidDeadline` (13).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source manager_wallet \
    -- register_project \
      --creator <MANAGER_ADDRESS> \
      --accepted_tokens '["<TOKEN_CONTRACT_A>"]' \
      --goal 5000000000 \
      --proof_hash 0000000000000000000000000000000000000000000000000000000000000000 \
      --deadline 1790000000
  ```

#### `get_project`
Retrieve a full Project configuration and state from storage.

- **Signature**: `fn get_project(env: Env, id: u64) -> Project`
- **Parameters**: `id` (`u64`) - Auto-incremented project ID.
- **Returns**: `Project` struct.
- **Errors**: `ProjectNotFound` (1).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID -- get_project --id 1
  ```

#### `get_balance`
Return the current unreleased balance of a specific token for a project.

- **Signature**: `fn get_balance(env: Env, project_id: u64, token: Address) -> i128`
- **Parameters**:
  - `project_id` (`u64`)
  - `token` (`Address`) - Supported token contract address.
- **Returns**: `i128` (0 if no deposits).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    -- get_balance --project_id 1 --token <TOKEN_CONTRACT>
  ```

#### `get_project_balances`
Return all token balances related to the specified project.

- **Signature**: `fn get_project_balances(env: Env, project_id: u64) -> ProjectBalances`
- **Parameters**: `project_id` (`u64`)
- **Returns**: `ProjectBalances` struct (map of Token Address -> i128 Amount).
- **Errors**: `ProjectNotFound` (1)
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    -- get_project_balances --project_id 1
  ```

#### `deposit`
Transfer funds from a donor to the contract, associating them with a project. 
The donor must have signed an auth payload or `soroban-cli` must supply `--source`. The token must also have had an `approve` granted to the protocol (if invoking via custom frontend wrapper or cross-contract call).

- **Signature**: `fn deposit(env: Env, project_id: u64, donator: Address, token: Address, amount: i128)`
- **Parameters**:
  - `project_id` (`u64`)
  - `donator` (`Address`): The account providing funds.
  - `token` (`Address`): A token defined in `accepted_tokens`.
  - `amount` (`i128`): Amount to deposit (> 0).
- **Returns**: `void`
- **Events**: `funded` (`ProjectFunded`), optionally `active` (`ProjectActive`) if goal reached.
- **Errors**: `ProtocolPaused` (19), `InvalidAmount` (11), `ProjectExpired` (14), `ProjectNotActive` (15), `NotAuthorized` (6 - if token not accepted, or using old error. Modern uses 23 `TokenNotAccepted`).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source donor \
    -- deposit \
      --project_id 1 \
      --donator <DONOR_ADDRESS> \
      --token <TOKEN_CONTRACT> \
      --amount 1000000000
  ```

#### `refund`
Reclaim deposited tokens from a project after its deadline has passed unverified.

- **Signature**: `fn refund(env: Env, donator: Address, project_id: u64, token: Address)`
- **Parameters**:
  - `donator` (`Address`)
  - `project_id` (`u64`)
  - `token` (`Address`) - Target refund token.
- **Returns**: `void`
- **Events**: `refunded`
- **Errors**: `ProjectNotExpired` (21), `InsufficientBalance` (4).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source donor \
    -- refund \
      --donator <DONOR_ADDRESS> \
      --project_id 1 \
      --token <TOKEN_CONTRACT>
  ```

#### `verify_and_release`
Verify project completion proof and trigger a final release of all associated token balances to the constructor/creator.

- **Signature**: `fn verify_and_release(env: Env, oracle: Address, project_id: u64, submitted_proof_hash: BytesN<32>)`
- **Parameters**:
  - `oracle` (`Address`): The calling oracle.
  - `project_id` (`u64`): The target project.
  - `submitted_proof_hash` (`BytesN<32>`): Proof matching what was set during registration.
- **Returns**: `void`
- **Events**: `verified` (`ProjectVerified`), `released` (`FundsReleased`) per token.
- **Errors**: `ProtocolPaused` (19), `NotAuthorized` (6), `ProjectExpired` (14), `MilestoneAlreadyReleased` (3), `VerificationFailed` (16).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID --source oracle_wallet \
    -- verify_and_release \
      --oracle <ORACLE_ADDRESS> \
      --project_id 1 \
      --submitted_proof_hash <32_BYTE_HEX>
  ```

#### `expire_project`
Permissionlessly force the status of a project past its deadline to `Expired`. Normally checked lazily on deposit/verify, but explicit calls maintain on-chain indexer clarity.

- **Signature**: `fn expire_project(env: Env, project_id: u64)`
- **Parameters**: `project_id` (`u64`)
- **Returns**: `void`
- **Events**: `expired` (`ProjectExpired`)
- **Errors**: `InvalidTransition` (22), `ProjectNotExpired` (21).
- **CLI Example**:
  ```bash
  soroban contract invoke --id $CONTRACT_ID \
    -- expire_project --project_id 1
  ```
