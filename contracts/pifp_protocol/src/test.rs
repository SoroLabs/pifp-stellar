// contracts/pifp_protocol/src/test.rs  (replaces existing test.rs)
// Multi-asset funding tests — covers all new behaviour on top of RBAC.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _},
    token::{Client as TokenClient, StellarAssetClient},
    Address, BytesN, Env, Vec,
};

use crate::{PifpProtocol, PifpProtocolClient, Role};

// ─── Setup ───────────────────────────────────────────────

fn setup() -> (Env, PifpProtocolClient<'static>, Address, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let id      = env.register_contract(None, PifpProtocol);
    let client  = PifpProtocolClient::new(&env, &id);

    let super_admin = Address::generate(&env);
    let oracle      = Address::generate(&env);
    let pm          = Address::generate(&env);

    let xlm  = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    let usdc = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();

    client.init(&super_admin);
    client.grant_role(&super_admin, &pm,     &Role::ProjectManager);
    client.grant_role(&super_admin, &oracle, &Role::Oracle);

    (env, client, super_admin, oracle, pm, xlm, usdc)
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

fn bytes32(env: &Env) -> BytesN<32> { BytesN::from_array(env, &[0xabu8; 32]) }
fn future(env: &Env) -> u64 { env.ledger().timestamp() + 86_400 }

fn tok2(env: &Env, a: &Address, b: &Address) -> Vec<Address> {
    let mut v = Vec::new(env);
    v.push_back(a.clone());
    v.push_back(b.clone());
    v
}

fn tok1(env: &Env, a: &Address) -> Vec<Address> {
    let mut v = Vec::new(env);
    v.push_back(a.clone());
    v
}

// ─── register_project ────────────────────────────────────

#[test]
fn register_two_tokens() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &1_000i128, &bytes32(&env), &future(&env));
    assert_eq!(p.accepted_tokens.len(), 2);
    assert_eq!(p.donation_count, 0);
}

#[test]
fn register_single_token() {
    let (env, client, _, _, pm, xlm, _) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &500i128, &bytes32(&env), &future(&env));
    assert_eq!(p.accepted_tokens.len(), 1);
}

#[test]
#[should_panic]
fn register_empty_tokens_panics() {
    let (env, client, _, _, pm, _, _) = setup();
    client.register_project(&pm, &Vec::new(&env), &100i128, &bytes32(&env), &future(&env));
}

#[test]
#[should_panic]
fn register_eleven_tokens_panics() {
    let (env, client, _, _, pm, _, _) = setup();
    let mut v: Vec<Address> = Vec::new(&env);
    for _ in 0..11 { v.push_back(Address::generate(&env)); }
    client.register_project(&pm, &v, &100i128, &bytes32(&env), &future(&env));
}

// ─── deposit success ─────────────────────────────────────

#[test]
fn deposit_xlm_updates_balance() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let donor = Address::generate(&env);
    mint(&env, &xlm, &donor, 1_000);

    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &xlm, &600i128);

    assert_eq!(client.get_token_balance(&p.id, &xlm),  600);
    assert_eq!(client.get_token_balance(&p.id, &usdc), 0);
}

#[test]
fn deposit_usdc_updates_balance() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let donor = Address::generate(&env);
    mint(&env, &usdc, &donor, 500);

    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &usdc, &500i128);

    assert_eq!(client.get_token_balance(&p.id, &usdc), 500);
    assert_eq!(client.get_token_balance(&p.id, &xlm),  0);
}

#[test]
fn deposits_accumulate_per_token() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let d1 = Address::generate(&env);
    let d2 = Address::generate(&env);
    mint(&env, &xlm,  &d1, 300);
    mint(&env, &xlm,  &d2, 700);
    mint(&env, &usdc, &d1, 400);

    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &1_000i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &d1, &xlm,  &300i128);
    client.deposit(&p.id, &d2, &xlm,  &700i128);
    client.deposit(&p.id, &d1, &usdc, &400i128);

    assert_eq!(client.get_token_balance(&p.id, &xlm),  1_000);
    assert_eq!(client.get_token_balance(&p.id, &usdc), 400);
}

#[test]
fn get_project_balances_returns_both_tokens() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let donor = Address::generate(&env);
    mint(&env, &xlm,  &donor, 100);
    mint(&env, &usdc, &donor, 200);

    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &xlm,  &100i128);
    client.deposit(&p.id, &donor, &usdc, &200i128);

    let bals = client.get_project_balances(&p.id);
    assert_eq!(bals.balances.len(), 2);
    assert_eq!(bals.balances.get(0).unwrap().balance, 100); // XLM
    assert_eq!(bals.balances.get(1).unwrap().balance, 200); // USDC
}

#[test]
fn donation_count_increments_each_deposit() {
    let (env, client, _, _, pm, xlm, _) = setup();
    let donor = Address::generate(&env);
    mint(&env, &xlm, &donor, 300);

    let p = client.register_project(&pm, &tok1(&env, &xlm), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &xlm, &100i128);
    client.deposit(&p.id, &donor, &xlm, &100i128);
    client.deposit(&p.id, &donor, &xlm, &100i128);

    let updated = client.get_project(&p.id);
    assert_eq!(updated.donation_count, 3);
}

// ─── deposit failures ────────────────────────────────────

#[test]
#[should_panic]
fn deposit_unlisted_token_panics() {
    let (env, client, _, _, pm, xlm, _) = setup();
    let donor = Address::generate(&env);
    let other = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    mint(&env, &other, &donor, 100);

    let p = client.register_project(&pm, &tok1(&env, &xlm), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &other, &100i128);
}

#[test]
#[should_panic]
fn deposit_zero_panics() {
    let (env, client, _, _, pm, xlm, _) = setup();
    let donor = Address::generate(&env);
    let p = client.register_project(&pm, &tok1(&env, &xlm), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &xlm, &0i128);
}

#[test]
#[should_panic]
fn deposit_negative_panics() {
    let (env, client, _, _, pm, xlm, _) = setup();
    let donor = Address::generate(&env);
    let p = client.register_project(&pm, &tok1(&env, &xlm), &500i128, &bytes32(&env), &future(&env));
    client.deposit(&p.id, &donor, &xlm, &-1i128);
}

#[test]
#[should_panic]
fn deposit_into_completed_project_panics() {
    let (env, client, _, oracle, pm, xlm, _) = setup();
    let donor = Address::generate(&env);
    mint(&env, &xlm, &donor, 200);

    let proof = bytes32(&env);
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &proof, &future(&env));
    client.deposit(&p.id, &donor, &xlm, &100i128);
    client.verify_and_release(&oracle, &p.id, &proof);
    client.deposit(&p.id, &donor, &xlm, &100i128); // must panic
}

// ─── is_token_accepted ───────────────────────────────────

#[test]
fn is_token_accepted_true_and_false() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &bytes32(&env), &future(&env));

    assert!(client.is_token_accepted(&p.id, &xlm));
    assert!(!client.is_token_accepted(&p.id, &usdc));
}

// ─── whitelist_token / remove_token ──────────────────────

#[test]
fn whitelist_adds_token() {
    let (env, client, super_admin, _, pm, xlm, usdc) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &bytes32(&env), &future(&env));
    assert!(!client.is_token_accepted(&p.id, &usdc));

    client.whitelist_token(&super_admin, &p.id, &usdc);
    assert!(client.is_token_accepted(&p.id, &usdc));
}

#[test]
#[should_panic]
fn whitelist_duplicate_panics() {
    let (env, client, super_admin, _, pm, xlm, _) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &bytes32(&env), &future(&env));
    client.whitelist_token(&super_admin, &p.id, &xlm); // XLM already there
}

#[test]
fn remove_token_shrinks_list() {
    let (env, client, super_admin, _, pm, xlm, usdc) = setup();
    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &100i128, &bytes32(&env), &future(&env));

    client.remove_token(&super_admin, &p.id, &usdc);
    assert!(client.is_token_accepted(&p.id, &xlm));
    assert!(!client.is_token_accepted(&p.id, &usdc));
}

#[test]
#[should_panic]
fn remove_last_token_panics() {
    let (env, client, super_admin, _, pm, xlm, _) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &bytes32(&env), &future(&env));
    client.remove_token(&super_admin, &p.id, &xlm);
}

#[test]
#[should_panic]
fn remove_unlisted_token_panics() {
    let (env, client, super_admin, _, pm, xlm, usdc) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &bytes32(&env), &future(&env));
    client.remove_token(&super_admin, &p.id, &usdc);
}

#[test]
#[should_panic]
fn pm_cannot_whitelist() {
    let (env, client, _, _, pm, xlm, usdc) = setup();
    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &bytes32(&env), &future(&env));
    client.whitelist_token(&pm, &p.id, &usdc); // pm is not admin
}

// ─── verify_and_release (multi-asset) ────────────────────

#[test]
fn release_transfers_all_balances_to_creator() {
    let (env, client, _, oracle, pm, xlm, usdc) = setup();
    let donor = Address::generate(&env);
    mint(&env, &xlm,  &donor, 1_000);
    mint(&env, &usdc, &donor, 500);

    let proof = bytes32(&env);
    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &500i128, &proof, &future(&env));
    client.deposit(&p.id, &donor, &xlm,  &1_000i128);
    client.deposit(&p.id, &donor, &usdc, &500i128);

    client.verify_and_release(&oracle, &p.id, &proof);

    // Contract balances drained
    assert_eq!(client.get_token_balance(&p.id, &xlm),  0);
    assert_eq!(client.get_token_balance(&p.id, &usdc), 0);

    // Creator received funds
    assert_eq!(TokenClient::new(&env, &xlm).balance(&pm),  1_000);
    assert_eq!(TokenClient::new(&env, &usdc).balance(&pm), 500);
}

#[test]
fn release_skips_zero_balance_tokens() {
    let (env, client, _, oracle, pm, xlm, usdc) = setup();
    let donor = Address::generate(&env);
    mint(&env, &xlm, &donor, 300);

    let proof = bytes32(&env);
    let p = client.register_project(&pm, &tok2(&env, &xlm, &usdc), &100i128, &proof, &future(&env));
    client.deposit(&p.id, &donor, &xlm, &300i128); // only XLM donated

    // Should not panic because USDC balance is 0 — just skips it
    client.verify_and_release(&oracle, &p.id, &proof);

    let updated = client.get_project(&p.id);
    assert_eq!(updated.status, crate::ProjectStatus::Completed);
}

#[test]
#[should_panic]
fn release_wrong_proof_panics() {
    let (env, client, _, oracle, pm, xlm, _) = setup();
    let proof = bytes32(&env);
    let bad   = BytesN::from_array(&env, &[0x00u8; 32]);

    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &proof, &future(&env));
    client.verify_and_release(&oracle, &p.id, &bad);
}

#[test]
#[should_panic]
fn double_release_panics() {
    let (env, client, _, oracle, pm, xlm, _) = setup();
    let proof = bytes32(&env);

    let p = client.register_project(&pm, &tok1(&env, &xlm), &100i128, &proof, &future(&env));
    client.verify_and_release(&oracle, &p.id, &proof);
    client.verify_and_release(&oracle, &p.id, &proof); // must panic
}