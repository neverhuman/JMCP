use jmcp_adapter_jankurai::JankuraiAdapter;
use jmcp_adapter_sdk::Adapter;

#[tokio::main]
async fn main() {
    let adapter = JankuraiAdapter::default();
    println!(
        "{}",
        serde_json::to_string(&adapter.service_card()).expect("service card json")
    );
}
