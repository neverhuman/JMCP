use jmcp_adapter_jeryu::JeryuAdapter;
use jmcp_adapter_sdk::Adapter;

#[tokio::main]
async fn main() {
    let adapter = JeryuAdapter::default();
    println!(
        "{}",
        serde_json::to_string(&adapter.service_card()).expect("service card json")
    );
}
