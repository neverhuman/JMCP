pub mod projection;
pub mod ranking;
pub mod reads;
pub mod scenes;

use chrono::{DateTime, Utc};
pub use jmcp_domain::{
    ActionSafetyClass, CardLod, CounterValue, DeckRankFactors, DeckRankReason, JituxEvidenceRef,
    PaneCounter, PaneKind, PanePreview, PaneRankReason, PaneRisk, PaneStatus, PaneVm,
    PreparedAction, PreparedTab,
};
pub use projection::{CachedNow, NowProjection};
pub use ranking::{rank_inputs, rank_reason, RankInput, RankedInput};
pub use reads::NowReads;
pub use scenes::queue_blockers::QueueBlockersProjection;

pub fn queue_blockers_panes(reads: &NowReads, now: DateTime<Utc>) -> Vec<PaneVm> {
    scenes::queue_blockers::panes(reads, now)
}

pub fn queue_blockers_projection(reads: &NowReads, now: DateTime<Utc>) -> QueueBlockersProjection {
    scenes::queue_blockers::compose(reads, now)
}
