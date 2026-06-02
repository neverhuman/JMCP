//! Status snapshot model for a ZYAL run (parsed from `jekko port-run --status`).

use serde::Deserialize;

/// One phase row from `jekko port-run --status`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ZyalPhase {
    #[serde(default)]
    pub phase_id: String,
    pub status: String,
}

/// Parsed `--status` snapshot for a run.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ZyalRunStatus {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub phases: Vec<ZyalPhase>,
}

impl ZyalRunStatus {
    /// Total phases known for the run.
    pub fn total(&self) -> usize {
        self.phases.len()
    }

    /// Count of phases in the `complete` state.
    pub fn completed(&self) -> usize {
        self.phases
            .iter()
            .filter(|p| p.status == "complete")
            .count()
    }

    /// Percent complete in `0..=100`. An empty run is 0%.
    pub fn percent(&self) -> u8 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        ((self.completed() * 100) / total) as u8
    }

    /// True once every phase has reached a terminal state
    /// (`complete` | `blocked` | `failed`). An empty run is never terminal.
    pub fn is_terminal(&self) -> bool {
        !self.phases.is_empty()
            && self
                .phases
                .iter()
                .all(|p| matches!(p.status.as_str(), "complete" | "blocked" | "failed"))
    }

    /// Coarse run-state label for evidence: `failed` if any phase failed, else
    /// `blocked` if any is blocked, else `complete` if all complete, else
    /// `partial` (and `unknown` for an empty run).
    pub fn state_label(&self) -> &'static str {
        if self.phases.is_empty() {
            return "unknown";
        }
        if self.phases.iter().any(|p| p.status == "failed") {
            return "failed";
        }
        if self.phases.iter().any(|p| p.status == "blocked") {
            return "blocked";
        }
        if self.phases.iter().all(|p| p.status == "complete") {
            return "complete";
        }
        "partial"
    }
}
