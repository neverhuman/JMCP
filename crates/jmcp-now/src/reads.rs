use jmcp_app::{AppResult, AppState};
use jmcp_domain::{
    ApprovalChallenge, AttentionPacket, AutonomousActionCard, IncidentRecord, Lease, WorkOrder,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NowReads {
    pub work_orders: Vec<WorkOrder>,
    pub leases: Vec<Lease>,
    pub attention_packets: Vec<AttentionPacket>,
    pub approval_challenges: Vec<ApprovalChallenge>,
    pub incidents: Vec<IncidentRecord>,
    pub autonomous_actions: Vec<AutonomousActionCard>,
}

impl NowReads {
    pub fn from_state(state: &AppState) -> AppResult<Self> {
        Ok(Self {
            work_orders: state.list_work_orders()?,
            leases: state.list_leases()?,
            attention_packets: state.attention_packets()?,
            approval_challenges: state.list_approval_challenges()?,
            incidents: state.incidents()?,
            autonomous_actions: state.list_autonomous_actions()?,
        })
    }
}
