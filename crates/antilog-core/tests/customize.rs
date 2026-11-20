use antilog_core::customize::suite::reach_suite;
use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine};

#[tokio::test]
async fn the_fake_store_keeps_connectors_skills_and_plugins() {
    let deps = FakeDeps::new(FakeMachine::default(), FakeCodeHost::default());
    reach_suite(&deps).await;
}
