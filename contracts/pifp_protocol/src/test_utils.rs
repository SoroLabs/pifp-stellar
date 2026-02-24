extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, BytesN, Env, Vec,
};

use crate::{types::Project, PifpProtocol, PifpProtocolClient, Role};

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

        // Initialize ledger with a standard timestamp
        env.ledger().set(LedgerInfo {
            timestamp: 100_000,
            protocol_version: 22,
            sequence_number: 100,
            network_id: [0u8; 32],
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 1000,
        });

        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let manager = Address::generate(&env);

        client.init(&admin);
        client.grant_role(&admin, &oracle, &Role::Oracle);
        client.grant_role(&admin, &manager, &Role::ProjectManager);

        Self {
            env,
            client,
            admin,
            oracle,
            manager,
        }
    }

    pub fn create_token(&self) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
        let addr = self
            .env
            .register_stellar_asset_contract_v2(self.admin.clone());
        (
            token::Client::new(&self.env, &addr.address()),
            token::StellarAssetClient::new(&self.env, &addr.address()),
        )
    }

    pub fn setup_project(
        &self,
        goal: i128,
    ) -> (
        Project,
        token::Client<'static>,
        token::StellarAssetClient<'static>,
    ) {
        let (token, sac) = self.create_token();
        let tokens = Vec::from_array(&self.env, [token.address.clone()]);
        let project = self.register_project(&tokens, goal);
        (project, token, sac)
    }

    pub fn register_project(&self, tokens: &Vec<Address>, goal: i128) -> Project {
        let proof_hash = self.dummy_proof();
        let deadline = self.env.ledger().timestamp() + 86400;
        self.client
            .register_project(&self.manager, tokens, &goal, &proof_hash, &deadline)
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
