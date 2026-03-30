extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Bytes, BytesN, Env, Vec,
};

use crate::{types::{Project, Milestone}, PifpProtocol, PifpProtocolClient, Role};

pub fn setup_test() -> (Env, PifpProtocolClient<'static>, Address) {
    let ctx = TestContext::new();
    let env = ctx.env.clone();
    let client = PifpProtocolClient::new(&env, &ctx.client.address);
    (env, client, ctx.admin)
}

pub fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &addr.address())
}

pub fn dummy_metadata_uri(env: &Env) -> Bytes {
    Bytes::from_slice(
        env,
        b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
    )
}

pub fn dummy_proof(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xabu8; 32])
}

pub struct TestContext {
    pub env: Env,
    pub client: PifpProtocolClient<'static>,
    pub admin: Address,
    pub oracle: Address,
    pub manager: Address,
}

impl TestContext {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let mut ledger = env.ledger().get();
        ledger.timestamp = 100_000;
        ledger.sequence_number = 100;
        env.ledger().set(ledger);

        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let manager = Address::generate(&env);

        client.init(&admin);
        client.grant_role(&admin, &oracle, &Role::Oracle);
        client.grant_role(&admin, &manager, &Role::ProjectManager);

        Self { env, client, admin, oracle, manager }
    }

    pub fn create_token(&self) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
        let addr = self.env.register_stellar_asset_contract_v2(self.admin.clone());
        (
            token::Client::new(&self.env, &addr.address()),
            token::StellarAssetClient::new(&self.env, &addr.address()),
        )
    }

    pub fn setup_project(
        &self,
        goal: i128,
    ) -> (Project, token::Client<'static>, token::StellarAssetClient<'static>) {
        let (token, sac) = self.create_token();
        let tokens = Vec::from_array(&self.env, [token.address.clone()]);
        let project = self.register_project(&tokens, goal, false);
        (project, token, sac)
    }

    pub fn register_project(&self, tokens: &Vec<Address>, goal: i128, is_private: bool) -> Project {
        let proof_hash = self.dummy_proof();
        let metadata_uri = self.dummy_metadata_uri();
        let deadline = self.env.ledger().timestamp() + 86400;
        
        let mut milestones = Vec::new(&self.env);
        milestones.push_back(Milestone {
            label: BytesN::from_array(&self.env, &[0u8; 32]),
            amount_bps: 10000,
            proof_hash: proof_hash.clone(),
        });

        self.client.register_project(
            &self.manager,
            tokens,
            &goal,
            &proof_hash,
            &metadata_uri,
            &deadline,
            &is_private,
            &milestones,
            &0u32, // categories
            &Vec::new(&self.env), // authorized_oracles
            &0u32, // threshold
        )
    }

    pub fn dummy_metadata_uri(&self) -> Bytes {
        Bytes::from_slice(
            &self.env,
            b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        )
    }

    pub fn dummy_proof(&self) -> BytesN<32> {
        BytesN::from_array(&self.env, &[0xabu8; 32])
    }

    pub fn jump_time(&self, seconds: u64) {
        let mut ledger = self.env.ledger().get();
        ledger.timestamp += seconds;
        self.env.ledger().set(ledger);
    }

    pub fn generate_address(&self) -> Address {
        Address::generate(&self.env)
    }
}
