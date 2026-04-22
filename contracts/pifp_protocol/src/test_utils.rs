extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Bytes, BytesN, Env, Vec, IntoVal, Val,
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

        let mut ledger = env.ledger().get();
        ledger.timestamp = 100_000;
        ledger.sequence_number = 100;
        env.ledger().set(ledger);

        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let manager = Address::generate(&env);

        env.mock_auths(&[
            MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "init",
                    args: (&admin,).into_val(&env),
                    sub_invocations: &[],
                },
            },
        ]);
        client.init(&admin);

        env.mock_auths(&[
            MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "grant_role",
                    args: (&admin, &oracle, Role::Oracle).into_val(&env),
                    sub_invocations: &[],
                },
            },
        ]);
        client.grant_role(&admin, &oracle, &Role::Oracle);

        env.mock_auths(&[
            MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "grant_role",
                    args: (&admin, &manager, Role::ProjectManager).into_val(&env),
                    sub_invocations: &[],
                },
            },
        ]);
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

        self.mock_auth(
            &self.manager,
            "register_project",
            (
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
            ),
        );

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

    pub fn mock_auth(&self, address: &Address, fn_name: &str, args: impl IntoVal<Env, Vec<Val>>) {
        self.env.mock_auths(&[
            MockAuth {
                address: address,
                invoke: &MockAuthInvoke {
                    contract: &self.client.address,
                    fn_name: fn_name,
                    args: args.into_val(&self.env),
                    sub_invocations: &[],
                },
            },
        ]);
    }

    pub fn mock_auth_with_sub_invocations(
        &self,
        address: &Address,
        fn_name: &str,
        args: impl IntoVal<Env, Vec<Val>>,
        sub_invocations: Vec<MockAuthInvoke>,
    ) {
        let mut sub_inv_refs = std::vec::Vec::new();
        for i in 0..sub_invocations.len() {
            sub_inv_refs.push(sub_invocations.get(i).unwrap());
        }

        self.env.mock_auths(&[
            MockAuth {
                address: address,
                invoke: &MockAuthInvoke {
                    contract: &self.client.address,
                    fn_name: fn_name,
                    args: args.into_val(&self.env),
                    sub_invocations: &sub_inv_refs,
                },
            },
        ]);
    }

    pub fn mock_deposit_auth(&self, donator: &Address, project_id: u64, token: &Address, amount: i128) {
        self.env.mock_auths(&[
            MockAuth {
                address: donator,
                invoke: &MockAuthInvoke {
                    contract: &self.client.address,
                    fn_name: "deposit",
                    args: (project_id, donator, token, amount).into_val(&self.env),
                    sub_invocations: &[
                        MockAuthInvoke {
                            contract: token,
                            fn_name: "transfer",
                            args: (donator, &self.client.address, amount).into_val(&self.env),
                            sub_invocations: &[],
                        }
                    ],
                },
            },
        ]);
    }
}
