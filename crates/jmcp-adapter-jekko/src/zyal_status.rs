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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_jekko_port_run_status_output() {
        // Captured verbatim from `jekko port-run --status` (live-7tab-jekko, the
        // jekko ZYAL engine built from this branch's manifest). This locks the
        // cross-repo contract: if jekko's status JSON shape drifts, this fails
        // here instead of silently mis-reporting progress. Note the extra
        // fields (name/objective/depends_on/summary/timestamps) the parser must
        // tolerate and ignore.
        let real = r#"{"run_id":"e2e-1","phases":[
          {"phase_id":"frame","name":"Frame the objective","objective":"o","depends_on":[],"status":"complete","summary":"scaffold","started_at":"2026-06-02T16:03:20Z","completed_at":"2026-06-02T16:03:20Z","updated_at":"2026-06-02T16:03:20Z"},
          {"phase_id":"produce","name":"Produce candidates with Jailgun","objective":"o","depends_on":["research"],"status":"complete","summary":"scaffold","started_at":"2026-06-02T16:03:20Z","completed_at":"2026-06-02T16:03:20Z","updated_at":"2026-06-02T16:03:20Z"}
        ]}"#;
        let parsed: ZyalRunStatus =
            serde_json::from_str(real).expect("parse real jekko port-run --status JSON");
        assert_eq!(parsed.run_id, "e2e-1");
        assert_eq!(parsed.total(), 2);
        assert_eq!(parsed.completed(), 2);
        assert_eq!(parsed.percent(), 100);
        assert!(parsed.is_terminal());
        assert_eq!(parsed.state_label(), "complete");
    }
}
