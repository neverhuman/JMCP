use jmcp_domain::{Attention, AttentionLevel, WorkOrder, WorkOrderStatus};

pub trait AttentionPolicy: Send + Sync {
    fn evaluate(&self, work_order: &WorkOrder) -> Vec<Attention>;
}

#[derive(Clone, Debug, Default)]
pub struct DefaultAttentionPolicy;

impl AttentionPolicy for DefaultAttentionPolicy {
    fn evaluate(&self, work_order: &WorkOrder) -> Vec<Attention> {
        match work_order.status {
            WorkOrderStatus::Failed => vec![Attention {
                level: AttentionLevel::Page,
                reason: "work order failed".to_owned(),
            }],
            WorkOrderStatus::AwaitingApproval => vec![Attention {
                level: AttentionLevel::Warn,
                reason: "approval required".to_owned(),
            }],
            _ => Vec::new(),
        }
    }
}
