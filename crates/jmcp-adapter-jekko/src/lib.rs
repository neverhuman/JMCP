use anyhow::Result;
use async_trait::async_trait;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};

pub struct JekkoAdapter;

#[async_trait]
impl Adapter for JekkoAdapter {
    fn service_card(&self) -> ServiceCard {
        ServiceCard {
            name: "jekko".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subjects: vec!["*/jekko/*".to_owned()],
            capabilities: vec!["local-cli".to_owned()],
        }
    }

    async fn execute(&self, _work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        Err(fail_closed("jekko"))
    }
}
