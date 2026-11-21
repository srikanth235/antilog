use antilog_core::customize::suite::reach_suite;
use antilog_core::fakes::FakeDeps;

#[tokio::test]
async fn the_fake_store_keeps_connectors_skills_and_plugins() {
    let deps = FakeDeps::new();
    reach_suite(&deps).await;
}
