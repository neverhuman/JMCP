use anyhow::Result;
use async_trait::async_trait;
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};

#[async_trait]
pub trait Adapter: Send + Sync {
    fn service_card(&self) -> ServiceCard;
    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>>;
}

pub fn fail_closed(service: &str) -> anyhow::Error {
    anyhow::anyhow!("{service} adapter is not configured; failing closed")
}
