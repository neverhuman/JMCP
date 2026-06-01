use jmcp_adapter_jekko::JekkoAdapter;
use jmcp_adapter_sdk::Adapter;

#[tokio::main]
async fn main() {
    let adapter = JekkoAdapter::default();
    println!(
        "{}",
        serde_json::to_string(&adapter.service_card()).expect("service card json")
    );
}
