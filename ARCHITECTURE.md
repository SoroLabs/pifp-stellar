# PIFP Protocol — System Architecture & Threat Model

> **Proof-of-Impact Funding Protocol (PIFP)** — Trust-minimized conditional funding on Stellar/Soroban.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Component Architecture](#2-component-architecture)
3. [Data Model](#3-data-model)
4. [Access Control (RBAC)](#4-access-control-rbac)
5. [Core Flows](#5-core-flows)
6. [Storage Design](#6-storage-design)
7. [Threat Model](#7-threat-model)
8. [Security Properties](#8-security-properties)
9. [Known Limitations & Future Work](#9-known-limitations--future-work)
10. [Deployment Checklist](#10-deployment-checklist)

---

## 1. System Overview

PIFP replaces trust-based donations with **cryptographic accountability**. Funds are locked inside a Soroban smart contract and can only be released when a designated Oracle submits a proof hash that matches the one registered at project creation.

```
┌──────────────┐     register_project     ┌─────────────────────────┐
│ ProjectManager│ ───────────────────────► │                         │
└──────────────┘                           │    PifpProtocol         │
                                           │    (Soroban Contract)   │
┌──────────────┐     deposit               │                         │
│   Donor      │ ───────────────────────► │  • RBAC                 │
└──────────────┘                           │  • Project Registry     │
                                           │  • Proof Verification   │
┌──────────────┐     verify_and_release    │  • Fund Release         │
│   Oracle     │ ───────────────────────► │                         │
└──────────────┘                           └─────────────────────────┘
                                                       │
                                           ┌───────────▼──────────────┐
                                           │  Stellar Token Contract  │
                                           │  (SAC / custom asset)    │
                                           └──────────────────────────┘
```

**Key properties:**

- Non-custodial — funds live in the contract, never in a third-party wallet.
- Permissioned writes — only addresses with the correct RBAC role may mutate state.
- Immutable project config — goal, token, deadline, and proof hash are set once and never changed.
- Event-driven audit trail — every role change and fund movement emits an on-chain event.

---

## 2. Component Architecture

```
contracts/pifp_protocol/src/
├── lib.rs        — Public entry points (contract interface)
├── rbac.rs       — Role-Based Access Control
├── storage.rs    — Persistent & instance storage helpers + TTL management
├── types.rs      — Shared data types (Project, ProjectConfig, ProjectState, Role)
├── invariants.rs — Invariant assertions used in tests
├── test.rs       — Unit & integration tests
└── fuzz_test.rs  — Property-based fuzz tests (proptest)
```

### `lib.rs` — Contract Interface

The single entry point file. Every public `fn` here is a Soroban contract method callable by transactions. Responsibilities:

- Delegates RBAC checks to `rbac.rs` before any mutation.
- Delegates storage reads/writes to `storage.rs`.
- Emits events for off-chain indexers.

### `rbac.rs` — Role-Based Access Control

Manages the role hierarchy and enforces authorization. All role data is stored in **persistent storage** under `RbacKey::Role(address)`.

### `storage.rs` — Storage Abstraction

Abstracts all `env.storage()` calls behind typed helpers. Manages TTL bumping to prevent ledger entry expiry.

### `types.rs` — Data Types

Defines `ProjectConfig` (immutable, written once) and `ProjectState` (mutable, updated on deposits/verification). The split reduces write costs on high-frequency operations.

---

## 3. Data Model

### ProjectConfig (Immutable — written once at registration)

| Field        | Type          | Description                              |
|--------------|---------------|------------------------------------------|
| `id`         | `u64`         | Auto-incremented unique identifier       |
| `creator`    | `Address`     | Address that registered the project      |
| `token`      | `Address`     | Stellar token contract address           |
| `goal`       | `i128`        | Target funding amount (must be > 0)      |
| `proof_hash` | `BytesN<32>`  | Expected proof artifact hash (e.g. IPFS CID digest) |
| `deadline`   | `u64`         | Ledger timestamp by which work must complete |

### ProjectState (Mutable — updated on deposits and verification)

| Field     | Type            | Description                        |
|-----------|-----------------|------------------------------------|
| `balance` | `i128`          | Current funded amount (never < 0)  |
| `status`  | `ProjectStatus` | Lifecycle state (see below)        |

### ProjectStatus — Lifecycle FSM

```
  [Funding] ──deposit──► [Funding]   (balance increases, status unchanged)
      │
      ├──verify_and_release──► [Completed]  (proof matches, funds releasable)
      │
      └──deadline passed ──► [Expired]     (triggered via `expire_project` entry point)

  [Active] ──verify_and_release──► [Completed]
  [Completed] ──(any)──► PANIC (MilestoneAlreadyReleased)
  [Expired]   ──(any)──► PANIC (ProjectNotFound)
```

Valid forward transitions only — status can never regress.

---

## 4. Access Control (RBAC)

### Role Hierarchy

```
SuperAdmin
    │
    ├── Admin          — manage roles, configure protocol parameters
    ├── Oracle         — call verify_and_release; trigger fund releases
    ├── Auditor        — read-only observer (off-chain checks, no on-chain gate)
    └── ProjectManager — register and manage own projects
```

### Role Assignment Rules

| Caller Role | Can Grant           | Cannot Grant  |
|-------------|---------------------|---------------|
| SuperAdmin  | Any role            | —             |
| Admin       | Admin, Oracle, Auditor, ProjectManager | SuperAdmin |
| Others      | —                   | Anything      |

### Invariants

1. **Single SuperAdmin** — stored separately at `RbacKey::SuperAdmin`. Can only be changed via `transfer_super_admin`.
2. **No self-demotion** — `revoke_role` cannot be called on the SuperAdmin address; use `transfer_super_admin`.
3. **One role per address** — granting a new role to an address that already holds one replaces it.
4. **Immutable init** — `init` can be called exactly once; subsequent calls panic with `AlreadyInitialized`.

### Entry Point Authorization Matrix

| Entry Point            | Allowed Roles                              |
|------------------------|---------------------------------------------|
| `init`                 | Any (first caller becomes SuperAdmin)        |
| `grant_role`           | SuperAdmin, Admin (SuperAdmin only for SuperAdmin grant) |
| `revoke_role`          | SuperAdmin, Admin                            |
| `transfer_super_admin` | SuperAdmin only                              |
| `register_project`     | SuperAdmin, Admin, ProjectManager            |
| `set_oracle`           | SuperAdmin, Admin                            |
| `verify_and_release`   | Oracle only (read from storage)              |
| `deposit`              | Any address (no RBAC gate)                   |
| `expire_project`      | Any address (no RBAC gate)                   |
| `get_project`          | Any address (read-only)                      |
| `role_of` / `has_role` | Any address (read-only)                      |

---

## 5. Core Flows

### 5.1 Project Registration

```
creator ──► register_project(creator, token, goal, proof_hash, deadline)
                │
                ├─ creator.require_auth()
                ├─ rbac::require_can_register(creator)   ← RBAC gate
                ├─ validate: goal > 0
                ├─ validate: deadline > now
                ├─ id = get_and_increment_project_id()
                ├─ save ProjectConfig (persistent, immutable)
                ├─ save ProjectState  (persistent, mutable: balance=0, status=Funding)
                └─ return Project
```

### 5.2 Deposit

```
donor ──► deposit(project_id, donator, amount)
              │
              ├─ donator.require_auth()
              ├─ load_project_config(project_id)  ← read token address
              ├─ load_project_state(project_id)   ← read current balance
              ├─ token::transfer(donator → contract, amount)
              ├─ state.balance += amount
              ├─ save_project_state()             ← write ~20 bytes only
              └─ emit event: (donation_received, project_id) → (donator, amount)
```

### 5.3 Oracle Verification & Fund Release

```
oracle ──► verify_and_release(project_id, submitted_proof_hash)
               │
               ├─ oracle = get_oracle()            ← load from instance storage
               ├─ oracle.require_auth()
               ├─ rbac::require_oracle(oracle)     ← RBAC gate
               ├─ load_project_config()            ← read stored proof_hash
               ├─ load_project_state()             ← read status
               ├─ assert status ∈ {Funding, Active}
               ├─ assert submitted_proof_hash == config.proof_hash
               ├─ state.status = Completed
               ├─ save_project_state()
               └─ emit event: (verified,) → project_id
```

---

## 6. Storage Design

PIFP uses two Soroban storage tiers:

### Instance Storage (contract-lifetime TTL)

| Key            | Type      | Description                         |
|----------------|-----------|-------------------------------------|
| `ProjectCount` | `u64`     | Global auto-increment project ID    |
| `OracleKey`    | `Address` | Active oracle address               |

TTL: bumped by **7 days** whenever below 1 day remaining.

### Persistent Storage (per-entry TTL)

| Key               | Type            | Description                     |
|-------------------|-----------------|---------------------------------|
| `ProjConfig(id)`  | `ProjectConfig` | Immutable project configuration |
| `ProjState(id)`   | `ProjectState`  | Mutable project state           |
| `RbacKey::Role(addr)` | `Role`      | RBAC role for an address        |

PIFP exposes several **retrieval helpers** designed to minimise the number of
storage reads and TTL bumps:

- `project_exists(id)` – cheap existence check (no TTL bump).
- `maybe_load_project_config` / `maybe_load_project_state` – return an
  `Option` and only bump TTL when the entry is found.
- `load_project_pair` – atomic two‑entry read with a single call; used by
  high‑frequency operations like `deposit` and `verify_and_release`.
- `maybe_load_project` – convenience wrapper returning a full `Project` or
  `None` if absent.

These helpers underpin the **optimized storage retrieval patterns** that reduce
gas costs and simplify contract logic by centralising dual‑read behaviour.
| `RbacKey::SuperAdmin` | `Address`   | The single SuperAdmin address   |

TTL: bumped by **30 days** whenever below 7 days remaining.

### Why Split Config/State?

Deposits are high-frequency. Writing the full `Project` struct (~150 bytes) on every deposit is wasteful. `ProjectState` is ~20 bytes — separating it reduces ledger write costs by ~87% per deposit.

---

## 7. Threat Model

### 7.1 Trust Boundaries

| Actor          | Trust Level | Notes                                              |
|----------------|-------------|----------------------------------------------------|
| SuperAdmin     | High        | Full protocol control; set at deployment           |
| Admin          | Medium-High | Can configure roles and oracle; cannot elevate to SuperAdmin |
| Oracle         | Medium      | Trusted to verify off-chain proof correctly; single point of failure |
| ProjectManager | Low-Medium  | Can register projects; cannot release funds        |
| Donor          | Untrusted   | Can deposit; cannot affect project config or status |
| Auditor        | Untrusted   | Read-only; no on-chain enforcement needed          |

### 7.2 STRIDE Analysis

#### Spoofing

| Threat | Mitigation |
|--------|------------|
| Impersonating the Oracle to trigger unauthorized fund release | `oracle.require_auth()` + `rbac::require_oracle()` — both address authentication and role check required |
| Claiming SuperAdmin before initialization | `init` checks `RbacKey::SuperAdmin` not set; panics with `AlreadyInitialized` on second call |
| Impersonating a ProjectManager to register malicious projects | `creator.require_auth()` + `rbac::require_can_register()` — role must be pre-granted by Admin/SuperAdmin |

#### Tampering

| Threat | Mitigation |
|--------|------------|
| Modifying `proof_hash` after registration to match a fake proof | `ProjectConfig` is written once and never updated; no update entry point exists |
| Changing project `goal` after funding to prevent completion | `goal` is in immutable `ProjectConfig`; no mutation path |
| Replaying a valid proof on a completed project | `verify_and_release` panics with `MilestoneAlreadyReleased` if `status == Completed` |
| Directly writing to contract storage | Soroban contracts enforce that only the contract itself can write to its own storage |

#### Repudiation

| Threat | Mitigation |
|--------|------------|
| Oracle denies triggering a release | Every `verify_and_release` call emits a `verified` event with `project_id`; events are immutable on-chain |
| Admin denies granting a role | `grant_role` / `revoke_role` emit `role_set` / `role_del` events with the caller address as data |

#### Information Disclosure

| Threat | Mitigation |
|--------|------------|
| Donor identity leak | Donor address is emitted in `donation_received` event; privacy-preserving frontend (e.g. commitment schemes) must be handled off-chain |
| Proof artifact exposure | Only the **hash** of the proof is stored on-chain; the raw proof remains off-chain (e.g. IPFS) |

#### Denial of Service

| Threat | Mitigation |
|--------|------------|
| Flooding contract with zero-value deposits | `deposit` performs a real token transfer — attacker pays token transfer fees |
| Preventing oracle from calling `verify_and_release` by revoking Oracle role | Only SuperAdmin/Admin can revoke; SuperAdmin cannot be removed without explicit transfer |
| Storage expiry causing project data loss | Persistent storage TTL is bumped on every access; 30-day extension with 7-day threshold |

#### Elevation of Privilege

| Threat | Mitigation |
|--------|------------|
| Admin self-escalating to SuperAdmin | `grant_role` checks: only a SuperAdmin can grant `Role::SuperAdmin` |
| ProjectManager granting roles to arbitrary addresses | `grant_role` panics with `NotAuthorized` for any caller without Admin or SuperAdmin role |
| SuperAdmin removal via `revoke_role` | `revoke_role` explicitly guards: if `target == super_admin` → panic `NotAuthorized` |

### 7.3 Attack Vectors & Mitigations

#### AV-1: Oracle Compromise

**Scenario:** The Oracle private key is stolen. An attacker calls `verify_and_release` with a fabricated proof hash.

**Impact:** Funds released to project creator without genuine impact.

**Mitigations:**

- Oracle role can be revoked by SuperAdmin/Admin immediately upon compromise detection.
- `verify_and_release` requires the submitted hash to match the `proof_hash` set at registration — attacker cannot alter the stored hash.
- Future mitigation: ZK-STARK proof verification (placeholder hook exists in `verify_and_release`).

#### AV-2: SuperAdmin Key Loss

**Scenario:** SuperAdmin private key is lost or compromised.

**Impact:** Full protocol control lost or hijacked.

**Mitigations:**

- `transfer_super_admin` allows key rotation.
- Recommend using a multi-sig wallet or hardware security module as the SuperAdmin address.
- Future mitigation: time-locked SuperAdmin operations.

#### AV-3: Malicious Project Registration

**Scenario:** A rogue ProjectManager registers a project with an attacker-controlled `creator` address and a proof hash they already know.

**Impact:** Attacker could collect donations and immediately trigger release.

**Mitigations:**

- ProjectManager role must be explicitly granted by Admin/SuperAdmin — not self-assignable.
- Donors should verify project legitimacy off-chain before depositing.
- `deadline` enforces a time constraint; a suspiciously short deadline is a red flag.

#### AV-4: Proof Hash Pre-image Collision

**Scenario:** Attacker finds a different input that produces the same 32-byte proof hash.

**Impact:** Fake proof accepted as valid.

**Mitigations:**

- Proof hash is a 32-byte value — assumed to be a SHA-256 or similar cryptographic hash produced off-chain.
- The Oracle is responsible for verifying the pre-image before submitting.
- Future mitigation: replace hash comparison with on-chain ZK verification.

#### AV-5: TTL Expiry (Storage Griefing)

**Scenario:** An attacker avoids interacting with a project, letting its storage TTL expire, then registers a new project that reuses the expired ID.

**Impact:** Stale project data, potential ID collision.

**Mitigations:**

- `ProjectCount` is instance storage and never expires with the contract.
- IDs are monotonically increasing — even after expiry a new project gets a fresh ID.
- Project configs and states are bumped on every read/write.

---

## 8. Security Properties

The following invariants **must hold at all times**:

| ID    | Invariant |
|-------|-----------|
| INV-1 | `project.balance >= 0` for all projects |
| INV-2 | `project.goal > 0` for all projects |
| INV-3 | `project.deadline > 0` for all projects |
| INV-4 | A `Completed` project's status is terminal — no further state changes |
| INV-5 | After a deposit of `amount`, `balance_after == balance_before + amount` |
| INV-6 | Project IDs are sequential starting from 0 |
| INV-7 | Status transitions are strictly forward: `Funding → Active | Completed | Expired`;`Active → Completed | Expired`; terminal states have no outbound transitions |
| INV-8 | An address holds at most one RBAC role at a time |
| INV-9 | The SuperAdmin address is always set after `init` and can only change via `transfer_super_admin` |
| INV-10 | `ProjectConfig` fields (`creator`, `token`, `goal`, `proof_hash`, `deadline`) are immutable after registration |

---

## 9. Known Limitations & Future Work

| Item | Description |
|------|-------------|
| **Mocked ZK Verification** | `verify_and_release` currently compares hashes directly. The structure is prepared for ZK-STARK proof verification but the verifier is not yet implemented. |
| **Single Oracle** | One oracle address is stored in instance storage. A compromise requires admin intervention to rotate. Future: multi-oracle quorum or ZK verifier removes oracle trust entirely. |
| **No Fund Withdrawal on Expiry** | Donors cannot reclaim funds after a deadline passes without completion. A `refund` mechanism is planned. |
| **No Pause Mechanism** | There is no emergency pause entry point. The SuperAdmin can revoke the Oracle role to halt new releases, but existing verified projects cannot be halted. |
| **Auditor Role** | The `Auditor` role has no on-chain enforcement gate — it is a semantic label for off-chain tooling only. |

---

## 10. Deployment Checklist

- [ ] Deploy the contract to a Soroban-enabled Stellar network.
- [ ] Call `init(super_admin)` **exactly once** immediately after deployment with a secure multi-sig address as `super_admin`.
- [ ] Call `set_oracle(super_admin, oracle_address)` to register the trusted Oracle.
- [ ] Use `grant_role` to assign `Admin` and `ProjectManager` roles as needed.
- [ ] Verify `has_role(super_admin, SuperAdmin) == true` and `has_role(oracle, Oracle) == true` on-chain before opening to users.
- [ ] Monitor on-chain events (`role_set`, `role_del`, `donation_received`, `verified`) via an off-chain indexer.
- [ ] Store the SuperAdmin key in a hardware security module or multi-sig; never in a hot wallet.
- [ ] Audit TTL thresholds against expected contract lifetime before production deployment.
