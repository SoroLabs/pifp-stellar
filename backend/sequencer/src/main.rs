use pifp_sequencer::{
    prover::ExternalProverClient,
    sequencer::Sequencer,
};

fn main() {
    let prover = ExternalProverClient::new(
        "https://gpu-prover-cluster.internal",
        "https://soroban-rpc.testnet.stellar.org",
    );
    let sequencer = Sequencer::new(prover);
    println!(
        "pifp-sequencer ready; bootstrap_balance={}",
        sequencer.balance_of("bootstrap")
    );
}

