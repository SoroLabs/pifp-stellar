extern crate std;
use soroban_sdk::{Env, testutils::Events};

#[test]
fn test_debug_events() {
    let env = Env::default();
    let events = env.events().all();
    std::println!("{:?}", events);
}
