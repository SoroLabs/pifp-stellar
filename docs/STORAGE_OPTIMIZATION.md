# Smart Contract Storage Layout Optimization

> Analysis of the PIFP Protocol (`contracts/pifp_protocol`) storage patterns,
> gas cost comparisons, and recommendations for future attribute additions.
>
> Soroban SDK version: **25.3.0**

---

## 1. Soroban Storage Tiers — Cost Model

Soroban exposes three storage tiers. PIFP uses two of them.

| Tier | Scope | TTL | Write cost | Read cost | Use in PIFP |
|------|-------|-----|-----------|-----------|-------------|
| **Instance** | Whole contract | Shared with contract | Cheapest (single ledger entry) | Cheapest | Global config, counters, pause flag |
| **Persistent** | Per key | Independent per entry | Higher (separate ledger entry) | Higher | Per-project data, balances, whitelist |
| Temporary | Per key | Short, auto-expires | Cheapest per entry | Cheapest | Not used |

### Key cost drivers

- **Ledger entry creation** is the most expensive operation. Each new `DataKey`
  variant written for the first time creates a new ledger entry.
- **Ledger entry writes** cost proportionally to the serialized byte size of the
  value stored.
- **TTL extension** (`extend_ttl`) is a separate fee charged per ledger entry
  per bump call. Bumping many small entries is cheaper than bumping one large one
  only when the entries have independent lifetimes.
- **Instance storage** shares a single ledger entry for all instance-tier keys,
  so a single `extend_ttl` call covers all of them simultaneously.

---

## 2. Current Storage Layout

### 2.1 Instance-tier keys

All instance keys share one ledger entry and one TTL bump.

| `DataKey` variant | Value type | Size (approx.) | Bump policy |
|-------------------|-----------|----------------|-------------|
| `ProjectCount` | `u64` | 8 bytes | On every project creation |
| `IsPaused` | `bool` | 1 byte | On pause/unpause |
| `ProtocolConfig` | `ProtocolConfig` (address + u32) | ~36 bytes | On config update |

TTL constants:
```
INSTANCE_LIFETIME_THRESHOLD = 1 day  (17,280 ledgers)
INSTANCE_BUMP_AMOUNT        = 7 days (120,960 ledgers)
```

Because all three keys share one ledger entry, bumping any one of them extends
the TTL for all three at no extra cost. This is already optimal.

### 2.2 Persistent-tier keys

Each key is an independent ledger entry with its own TTL.

| `DataKey` variant | Value type | Size (approx.) | Written by |
|-------------------|-----------|----------------|-----------|
| `ProjConfig(id)` | `ProjectConfig` | ~150 bytes | `register_project`, `extend_deadline` |
| `ProjState(id)` | `ProjectState` | ~20 bytes | `deposit`, `verify_and_release`, `expire_project`, `cancel` |
| `TokenBalance(id, token)` | `i128` | 16 bytes | `deposit`, `drain_token_balance` |
| `DonatorBalance(id, token, donator)` | `i128` | 16 bytes | `deposit`, `refund` |
| `Whitelist(id, address)` | `()` | 0 bytes | `add_to_whitelist` |

TTL constants:
```
PERSISTENT_LIFETIME_THRESHOLD = 7 days  (120,960 ledgers)
PERSISTENT_BUMP_AMOUNT        = 30 days (518,400 ledgers)
```

---

## 3. Gas Analysis

### 3.1 Config / State split — the most impactful optimization

The single biggest optimization already in place is splitting the full `Project`
struct into `ProjectConfig` (~150 bytes) and `ProjectState` (~20 bytes).

**Deposit path write cost comparison:**

| Approach | Bytes written per deposit | Relative cost |
|----------|--------------------------|---------------|
| Write full `Project` struct | ~200 bytes | 100% (baseline) |
| Write only `ProjectState` (current) | ~20 bytes | **~10%** |

Deposits are the highest-frequency write operation. At scale this is an ~87%
reduction in ledger write fees per deposit.

The `load_project_pair` helper reads both entries atomically and bumps both TTLs
in a single call, avoiding the double-bump overhead that would occur if callers
read config and state separately.

### 3.2 Instance vs Persistent — when to use each

**Use Instance when:**
- The value must live as long as the contract (global counters, admin config).
- Multiple small values can be grouped under the same contract entry.
- The value is read on nearly every transaction (pause flag, protocol config).

**Use Persistent when:**
- The value is scoped to a specific entity (project, donator, token).
- The value may need to expire independently (e.g. a completed project's data
  can eventually be allowed to expire without affecting other projects).
- The value is large enough that sharing an instance entry would bloat every
  instance read.

**Cost trap to avoid:** Storing per-project data in instance storage would mean
every instance bump pays for the full serialized size of all projects combined.
The current design correctly keeps per-project data in persistent storage.

### 3.3 `DonatorBalance` key structure

`DonatorBalance(project_id: u64, token: Address, donator: Address)` creates one
ledger entry per unique `(project, token, donator)` triple. For a project with
`T` accepted tokens and `D` unique donators this is up to `T × D` entries.

Each entry stores only 16 bytes (`i128`), but the key itself serializes to
roughly `8 + 32 + 32 = 72 bytes`. The key overhead dominates the value.

This is acceptable because:
- Entries are only created when a donator actually deposits (lazy creation).
- Entries can be deleted after a refund or reclaim, recovering the rent.
- Independent TTLs mean a donator's entry can expire without affecting others.

### 3.4 `Whitelist` key — zero-value flag pattern

`Whitelist(project_id, address)` stores `()` (unit type, 0 bytes). The entry's
existence is the signal; the value is irrelevant. This is the correct pattern
for set membership in Soroban — cheaper than storing a `bool` and semantically
cleaner.

### 3.5 TTL bump frequency

Every read and write bumps the TTL of the accessed entry. This is a conservative
strategy that prevents accidental expiry at the cost of paying bump fees on every
access. For a production system with predictable access patterns, a lazy bump
(only bump when TTL is near threshold) is already implemented via the
`PERSISTENT_LIFETIME_THRESHOLD` guard in `bump_persistent`.

The current thresholds are reasonable:
- 7-day threshold / 30-day bump gives a 23-day "safe zone" between bumps.
- 1-day threshold / 7-day bump for instance storage is aggressive but instance
  bumps are cheap since they cover all instance keys at once.

---

## 4. Identified Inefficiencies

### 4.1 Double TTL bump in `maybe_load_project`

`maybe_load_project` calls `maybe_load_project_config` (which bumps the config
TTL), then manually bumps `ProjState` separately. This is correct but results in
two separate bump calls. The `load_project_pair` path avoids this by bumping both
in sequence after both reads succeed. `maybe_load_project` should be refactored
to mirror `load_project_pair`'s pattern.

### 4.2 `add_to_token_balance` double-reads

`add_to_token_balance` calls `get_token_balance` (read + bump) then
`set_token_balance` (write + bump), resulting in two bump calls for one logical
operation. This is a minor inefficiency; a single internal read-modify-write
helper that bumps once would be marginally cheaper.

### 4.3 `get_all_balances` N reads for N tokens

`get_all_balances` iterates over `accepted_tokens` and calls `get_token_balance`
for each, resulting in N separate persistent reads and N TTL bumps. For projects
with the maximum 10 accepted tokens this is 10 reads. This is unavoidable given
the current key structure but is worth noting for the 10-token cap justification.

---

## 5. Plan for Future Attribute Additions

When adding new fields to existing types or new storage keys, follow these rules
to minimize gas impact.

### 5.1 Adding fields to `ProjectState` (mutable, high-frequency)

`ProjectState` is written on every deposit. Keep it small.

**Rule:** Only add a field to `ProjectState` if it changes on deposits or
verification. Current size is ~20 bytes; aim to stay under 64 bytes.

```rust
// Good: small scalar that changes on deposit
pub struct ProjectState {
    pub status: ProjectStatus,
    pub donation_count: u32,
    pub refund_expiry: u64,
    // OK to add: milestone_index: u32 (~4 bytes)
}
```

### 5.2 Adding fields to `ProjectConfig` (immutable, written once)

`ProjectConfig` is written once at registration. Size matters less here, but
keep it under 256 bytes to avoid hitting ledger entry size limits.

```rust
// Good: written once, never mutated
pub struct ProjectConfig {
    // existing fields...
    // OK to add: category: u8, tags: Bytes (bounded), min_donation: i128
}
```

### 5.3 Adding a new per-project scalar

Prefer adding it to `ProjectState` or `ProjectConfig` rather than creating a new
`DataKey` variant. A new key means a new ledger entry, a new TTL to manage, and
an additional bump call on every access.

Only create a new `DataKey` variant when:
- The value is optional and most projects won't have it (sparse data).
- The value has a different access frequency than config/state.
- The value is large enough that including it in config/state would significantly
  increase write costs for operations that don't need it.

### 5.4 Adding per-donator metadata

Follow the `DonatorBalance` pattern: `NewKey(project_id: u64, donator: Address)`.
Do not store donator lists in `ProjectState` — a `Vec<Address>` grows unboundedly
and would make every deposit write proportionally more expensive.

### 5.5 Milestone support (reserved `MilestoneNotFound`, `InvalidMilestones`)

When milestones are added, the recommended layout is:

```rust
// New persistent key — only exists for projects with milestones
DataKey::Milestone(project_id: u64, milestone_index: u32)
```

Store a `MilestoneState` struct per milestone rather than a `Vec<Milestone>` in
`ProjectConfig`. This keeps individual milestone writes cheap and allows
milestones to expire independently after completion.

### 5.6 Temporary storage — when to consider it

Temporary storage is not currently used. It is appropriate for:
- Short-lived nonces or replay-protection flags (expire after one epoch).
- Cached computation results that are only valid for the current transaction.

Do not use temporary storage for anything that must survive across transactions.

---

## 6. Summary of Recommendations

| Priority | Recommendation | Impact |
|----------|---------------|--------|
| Low | Refactor `maybe_load_project` to avoid double TTL bump on config | Minor gas saving |
| Low | Combine read+bump in `add_to_token_balance` into a single bump | Minor gas saving |
| Medium | Document the 10-token cap as a gas-driven design constraint | Clarity |
| Future | Use sparse `DataKey::Milestone(id, index)` for milestone data | Avoids unbounded state growth |
| Future | Keep `ProjectState` under 64 bytes as new fields are added | Preserves deposit cost |
| Future | Prefer extending existing structs over new `DataKey` variants for dense data | Reduces ledger entry count |
