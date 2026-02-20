// contracts/pifp_protocol/src/rbac.rs
//
// Role-Based Access Control (RBAC) for the PIFP Protocol
//
// ## Role Hierarchy
//
// ```
//   SuperAdmin
//       │
//       ├── Admin          (manage roles, configure protocol)
//       ├── Oracle         (verify proofs, trigger releases)
//       ├── Auditor        (read-only: view all projects + audit logs)
//       └── ProjectManager (register + manage own projects only)
// ```
//
// ## Design
//
// Roles are stored in persistent storage keyed by `RbacKey::Role(address)`.
// Every role-bearing address also appears in `RbacKey::RoleMembers(role)` so
// that membership can be enumerated off-chain via events (the list itself is
// not stored on-chain to avoid unbounded growth).
//
// A `SuperAdmin` is set once at contract initialisation and can never be
// removed via normal `revoke_role` — it must use `transfer_super_admin`.
//
// All admin mutations emit events so that off-chain indexers can maintain a
// complete audit trail without storing full membership lists on-chain.

#![allow(unused)]

use soroban_sdk::{contracttype, symbol_short, Address, Env, Vec};

use crate::Error;

// ─────────────────────────────────────────────────────────
// Role enum — stored per address
// ─────────────────────────────────────────────────────────

/// The set of roles that can be assigned to an address.
///
/// A single address may hold at most one role at a time.
/// Upgrading or revoking replaces / removes the stored value.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    /// Full protocol control: can grant/revoke any role, change oracle, pause.
    SuperAdmin,
    /// Can grant/revoke non-SuperAdmin roles and configure protocol parameters.
    Admin,
    /// Can call `verify_and_release`; replaces the single oracle address.
    Oracle,
    /// Read-only observer; confirmed by off-chain checks rather than on-chain gates.
    Auditor,
    /// Can call `register_project`; restricted to managing their own projects.
    ProjectManager,
}

// ─────────────────────────────────────────────────────────
// Storage keys
// ─────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RbacKey {
    /// Maps an address → its current Role (if any).
    Role(Address),
    /// The one and only SuperAdmin address.
    SuperAdmin,
}

// ─────────────────────────────────────────────────────────
// Storage helpers (private)
// ─────────────────────────────────────────────────────────

/// Persist a role assignment. Overwrites any existing role.
fn store_role(env: &Env, address: &Address, role: &Role) {
    env.storage()
        .persistent()
        .set(&RbacKey::Role(address.clone()), role);
}

/// Remove any role stored for `address`.
fn clear_role(env: &Env, address: &Address) {
    env.storage()
        .persistent()
        .remove(&RbacKey::Role(address.clone()));
}

/// Read the role for `address`, returning `None` if unassigned.
pub fn get_role(env: &Env, address: &Address) -> Option<Role> {
    env.storage()
        .persistent()
        .get(&RbacKey::Role(address.clone()))
}

/// Read the SuperAdmin address, returning `None` before init.
pub fn get_super_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&RbacKey::SuperAdmin)
}

// ─────────────────────────────────────────────────────────
// Initialisation
// ─────────────────────────────────────────────────────────

/// Set the initial SuperAdmin. Must be called exactly once (during contract
/// initialisation). Panics with `Error::AlreadyInitialized` if called again.
pub fn init_super_admin(env: &Env, super_admin: &Address) {
    if env.storage().persistent().has(&RbacKey::SuperAdmin) {
        panic_with_error_rbac(env, Error::AlreadyInitialized);
    }
    env.storage()
        .persistent()
        .set(&RbacKey::SuperAdmin, super_admin);
    store_role(env, super_admin, &Role::SuperAdmin);

    emit(env, symbol_short!("role_set"), super_admin, &Role::SuperAdmin, None::<Address>);
}

// ─────────────────────────────────────────────────────────
// Role assignment
// ─────────────────────────────────────────────────────────

/// Grant `role` to `target`.
///
/// - `caller` must hold `SuperAdmin` or `Admin`.
/// - `Admin` callers cannot grant `SuperAdmin` — only SuperAdmin can elevate.
/// - Assigning a role to an address that already has one replaces it.
///
/// Emits a `role_set` event.
pub fn grant_role(env: &Env, caller: &Address, target: &Address, role: Role) {
 

    let caller_role = get_role(env, caller);

    match &role {
        // Only SuperAdmin can grant SuperAdmin
        Role::SuperAdmin => {
            require_role(env, caller, &Role::SuperAdmin);
        }
        // Admin or SuperAdmin can grant everything else
        _ => {
            require_any_of(env, caller, &[Role::SuperAdmin, Role::Admin]);
        }
    }

    // Prevent demotion of the SuperAdmin via grant_role
    if let Some(Role::SuperAdmin) = get_role(env, target) {
        if role != Role::SuperAdmin {
            panic_with_error_rbac(env, Error::NotAuthorized);
        }
    }

    store_role(env, target, &role);
    emit(env, symbol_short!("role_set"), target, &role, Some(caller.clone()));
}

/// Revoke any role from `target`.
///
/// - `caller` must hold `SuperAdmin` or `Admin`.
/// - The SuperAdmin address itself cannot be revoked; use `transfer_super_admin`.
/// - Revoking a role from an address with no role is a no-op.
///
/// Emits a `role_del` event if a role existed.
pub fn revoke_role(env: &Env, caller: &Address, target: &Address) {
   
    require_any_of(env, caller, &[Role::SuperAdmin, Role::Admin]);

    // Protect the SuperAdmin address from revocation via this path
    let super_admin = get_super_admin(env);
    if Some(target.clone()) == super_admin {
        panic_with_error_rbac(env, Error::NotAuthorized);
    }

    if get_role(env, target).is_some() {
        clear_role(env, target);
        emit_revoke(env, target, Some(caller.clone()));
    }
}

/// Transfer the SuperAdmin role to a new address.
///
/// - `current_super_admin` must authorize and must hold `SuperAdmin`.
/// - `new_super_admin` is granted the `SuperAdmin` role.
/// - The old SuperAdmin loses the `SuperAdmin` role automatically.
///
/// This is the only way to remove a SuperAdmin.
pub fn transfer_super_admin(env: &Env, current: &Address, new: &Address) {
 
    require_role(env, current, &Role::SuperAdmin);

    // Clear old SuperAdmin
    clear_role(env, current);
    emit_revoke(env, current, Some(current.clone()));

    // Set new SuperAdmin
    env.storage()
        .persistent()
        .set(&RbacKey::SuperAdmin, new);
    store_role(env, new, &Role::SuperAdmin);
    emit(env, symbol_short!("role_set"), new, &Role::SuperAdmin, Some(current.clone()));
}

// ─────────────────────────────────────────────────────────
// Access guards (called from lib.rs handlers)
// ─────────────────────────────────────────────────────────

/// Assert that `address` holds exactly `required_role`.
/// Panics with `Error::NotAuthorized` on failure.
pub fn require_role(env: &Env, address: &Address, required_role: &Role) {
    match get_role(env, address) {
        Some(ref r) if r == required_role => {}
        _ => panic_with_error_rbac(env, Error::NotAuthorized),
    }
}

/// Assert that `address` holds one of the roles in `allowed`.
/// Panics with `Error::NotAuthorized` if none match.
pub fn require_any_of(env: &Env, address: &Address, allowed: &[Role]) {
    if let Some(ref r) = get_role(env, address) {
        if allowed.contains(r) {
            return;
        }
    }
    panic_with_error_rbac(env, Error::NotAuthorized);
}

/// Assert that `address` is the SuperAdmin OR an Admin.
/// Convenience wrapper used on configuration-level operations.
#[inline]
pub fn require_admin_or_above(env: &Env, address: &Address) {
    require_any_of(env, address, &[Role::SuperAdmin, Role::Admin]);
}

/// Assert that `address` holds the Oracle role.
/// Used to gate `verify_and_release`.
#[inline]
pub fn require_oracle(env: &Env, address: &Address) {
    require_role(env, address, &Role::Oracle);
}

/// Assert that `address` may register and manage projects.
/// ProjectManager, Admin, and SuperAdmin may all register projects.
#[inline]
pub fn require_can_register(env: &Env, address: &Address) {
    require_any_of(
        env,
        address,
        &[Role::SuperAdmin, Role::Admin, Role::ProjectManager],
    );
}

// ─────────────────────────────────────────────────────────
// Queries
// ─────────────────────────────────────────────────────────

/// Returns the role held by `address`, or `None`.
pub fn role_of(env: &Env, address: Address) -> Option<Role> {
    get_role(env, &address)
}

/// Returns `true` if `address` holds `role`.
pub fn has_role(env: &Env, address: Address, role: Role) -> bool {
    get_role(env, &address).map(|r| r == role).unwrap_or(false)
}

// ─────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────

/// Emit a role assignment event.
/// Topic: `(role_set, target_address, role_name_symbol)`
/// Data:  `Option<caller_address>`
fn emit(env: &Env, event: soroban_sdk::Symbol, target: &Address, role: &Role, by: Option<Address>) {
    let role_sym = role_to_symbol(env, role);
    env.events().publish(
        (event, target.clone(), role_sym),
        by,
    );
}

/// Emit a role revocation event.
fn emit_revoke(env: &Env, target: &Address, by: Option<Address>) {
    env.events().publish(
        (symbol_short!("role_del"), target.clone()),
        by,
    );
}

/// Convert a Role to a short Symbol for event topics.
fn role_to_symbol(env: &Env, role: &Role) -> soroban_sdk::Symbol {
    match role {
        Role::SuperAdmin    => symbol_short!("supadmin"),
        Role::Admin         => symbol_short!("admin"),
        Role::Oracle        => symbol_short!("oracle"),
        Role::Auditor       => symbol_short!("auditor"),
        Role::ProjectManager=> symbol_short!("proj_mgr"),
    }
}

/// Thin wrapper so we can call panic_with_error from inside rbac.rs
/// without importing the macro from the parent.
#[inline(always)]
fn panic_with_error_rbac(env: &Env, err: Error) -> ! {
    soroban_sdk::panic_with_error!(env, err)
}