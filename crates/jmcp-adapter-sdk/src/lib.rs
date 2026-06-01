use anyhow::Result;
use async_trait::async_trait;
use jmcp_domain::{Evidence, Lease, ServiceCard, WorkOrder};

#[async_trait]
pub trait Adapter: Send + Sync {
    fn service_card(&self) -> ServiceCard;
    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>>;
}

pub fn fail_closed(service: &str) -> anyhow::Error {
    anyhow::anyhow!("{service} adapter is not configured; failing closed")
}

pub fn require_valid_lease(work_order: &WorkOrder, lease: &Lease, holder: &str) -> Result<()> {
    lease.validate_for(work_order.id, holder)?;
    Ok(())
}

pub async fn execute_with_lease<A>(
    adapter: &A,
    work_order: &WorkOrder,
    lease: &Lease,
    holder: &str,
) -> Result<Vec<Evidence>>
where
    A: Adapter,
{
    require_valid_lease(work_order, lease, holder)?;
    adapter.execute(work_order).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use jmcp_domain::Lease;
    use serde_json::json;

    #[test]
    fn rejects_missing_matching_lease_before_adapter_side_effects() {
        let work_order = WorkOrder::submit("t/s/e", "demo", json!({}));
        let lease = Lease {
            work_order_id: work_order.id,
            holder: "other".to_owned(),
            expires_at: Utc::now() + Duration::minutes(5),
        };

        assert!(require_valid_lease(&work_order, &lease, "adapter").is_err());
    }
}
