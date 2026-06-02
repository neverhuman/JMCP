use jmcp_adapter_jailgun::JailgunAdapter;
use jmcp_adapter_sdk::Adapter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let adapter = JailgunAdapter::default();
    println!("{}", serde_json::to_string_pretty(&adapter.service_card())?);
    Ok(())
}
