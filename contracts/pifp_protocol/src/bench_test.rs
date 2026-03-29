extern crate std;

use crate::{PifpProtocol, PifpProtocolClient};
use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env};

fn sample_cid(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
}

/// Deploy + initialise the contract (1 % fee by default).
/// Returns (client, admin, treasury).
fn setup(env: &Env) -> (PifpProtocolClient, Address, Address) {
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    client.initialize(&admin, &treasury, &None);
    (client, admin, treasury)
}

/// Pretty-print one operation's budget snapshot.
fn report(label: &str, cpu: u64, mem: u64) {
    std::println!(
        "\n╔══════════════════════════════════════════╗\
         \n║  BENCHMARK: {:>28}  ║\
         \n╠══════════════════════════════════════════╣\
         \n║  CPU Instructions  : {:>18} ║\
         \n║  Memory Bytes      : {:>18} ║\
         \n╚══════════════════════════════════════════╝",
        label, cpu, mem
    );
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------
#[test]
fn bench_initialize() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);

    env.cost_estimate().budget().reset_default();
    client.initialize(&admin, &treasury, &None);

    let b = env.cost_estimate().budget();
    report("initialize", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// create_project
// ---------------------------------------------------------------------------
#[test]
fn bench_create_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let project_id = BytesN::from_array(&env, &[1u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[2u8; 32]);
    let cid = sample_cid(&env);

    env.cost_estimate().budget().reset_default();
    let _ = client.create_project(&project_id, &creator, &10_000i128, &proof_hash, &cid);

    let b = env.cost_estimate().budget();
    report("create_project", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// deposit
// ---------------------------------------------------------------------------
#[test]
fn bench_deposit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let project_id = BytesN::from_array(&env, &[3u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[4u8; 32]);
    client.create_project(&project_id, &creator, &10_000i128, &proof_hash, &sample_cid(&env));

    let donor = Address::generate(&env);
    env.cost_estimate().budget().reset_default();
    client.deposit(&donor, &project_id, &1_000i128);

    let b = env.cost_estimate().budget();
    report("deposit", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// verify
// ---------------------------------------------------------------------------
#[test]
fn bench_verify() {
    let env = Env::default();
    env.mock_all_auths();

    // verify is a pure skeleton (no storage reads), no initialize needed.
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);

    let project_id = BytesN::from_array(&env, &[5u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[6u8; 32]);

    env.cost_estimate().budget().reset_default();
    client.verify(&project_id, &proof_hash);

    let b = env.cost_estimate().budget();
    report("verify", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// withdraw
// ---------------------------------------------------------------------------
#[test]
fn bench_withdraw() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let project_id = BytesN::from_array(&env, &[7u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[8u8; 32]);
    client.create_project(&project_id, &creator, &10_000i128, &proof_hash, &sample_cid(&env));
    let donor = Address::generate(&env);
    client.deposit(&donor, &project_id, &5_000i128);

    let recipient = Address::generate(&env);
    env.cost_estimate().budget().reset_default();
    client.withdraw(&recipient, &project_id, &500i128);

    let b = env.cost_estimate().budget();
    report("withdraw", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// release  (includes fee split → treasury)
// ---------------------------------------------------------------------------
#[test]
fn bench_release() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = setup(&env); // 1 % fee

    let creator = Address::generate(&env);
    let project_id = BytesN::from_array(&env, &[13u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[14u8; 32]);
    client.create_project(&project_id, &creator, &10_000i128, &proof_hash, &sample_cid(&env));
    client.deposit(&creator, &project_id, &10_000i128);

    env.cost_estimate().budget().reset_default();
    let result = client.release(&project_id, &proof_hash, &creator, &10_000i128);

    let b = env.cost_estimate().budget();
    std::println!(
        "\n╔══════════════════════════════════════════╗\
         \n║  BENCHMARK:                      release  ║\
         \n╠══════════════════════════════════════════╣\
         \n║  CPU Instructions  : {:>18} ║\
         \n║  Memory Bytes      : {:>18} ║\
         \n╠══════════════════════════════════════════╣\
         \n║  Fee Split (1 % of 10 000)                ║\
         \n║    Recipient amount: {:>18} ║\
         \n║    Fee amount      : {:>18} ║\
         \n╚══════════════════════════════════════════╝",
        b.cpu_instruction_cost(),
        b.memory_bytes_cost(),
        result.recipient_amount,
        result.fee_amount,
    );
}

// ---------------------------------------------------------------------------
// set_fee_bps
// ---------------------------------------------------------------------------
#[test]
fn bench_set_fee_bps() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _) = setup(&env);

    env.cost_estimate().budget().reset_default();
    client.set_fee_bps(&admin, &250); // change to 2.5 %

    let b = env.cost_estimate().budget();
    report("set_fee_bps", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// set_treasury
// ---------------------------------------------------------------------------
#[test]
fn bench_set_treasury() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _) = setup(&env);

    let new_treasury = Address::generate(&env);
    env.cost_estimate().budget().reset_default();
    client.set_treasury(&admin, &new_treasury);

    let b = env.cost_estimate().budget();
    report("set_treasury", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// update_metadata
// ---------------------------------------------------------------------------
#[test]
fn bench_update_metadata() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _, _) = setup(&env);

    let creator = Address::generate(&env);
    let project_id = BytesN::from_array(&env, &[9u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    client.create_project(&project_id, &creator, &10_000i128, &proof_hash, &sample_cid(&env));

    let new_cid = Bytes::from_slice(&env, b"bafkreigh2akiscaildcqabab4eupks44qq6y2plwqpk3mvkvbgm7qjlxp4");
    env.cost_estimate().budget().reset_default();
    client.update_metadata(&project_id, &creator, &new_cid);

    let b = env.cost_estimate().budget();
    report("update_metadata", b.cpu_instruction_cost(), b.memory_bytes_cost());
}

// ---------------------------------------------------------------------------
// verify_and_release  (legacy stub)
// ---------------------------------------------------------------------------
#[test]
fn bench_verify_and_release() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);

    let project_id = BytesN::from_array(&env, &[11u8; 32]);
    let proof_hash = BytesN::from_array(&env, &[12u8; 32]);

    env.cost_estimate().budget().reset_default();
    client.verify_and_release(&project_id, &proof_hash);

    let b = env.cost_estimate().budget();
    report("verify_and_release", b.cpu_instruction_cost(), b.memory_bytes_cost());
}
