use std::str::FromStr;

use jcp_core::{Envelope, LocalSigner, Subject};
use jmcp_domain::{
    AutonomousActionCard, AutonomousActionManifestMetadata, AutonomousActionMode,
    AutonomousActionOverrides, AutonomousActionSafetyDefaults, AutonomousActionSubject,
    AutonomousWorkOrderKind, WorkOrder,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{AppError, AppResult, AppState};

const SUBMITTED_BY: &str = "jmcp.full_auto";
const ZYAL_RUN_KIND: &str = "zyal.run";

struct ActionDefinition {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    subject: &'static str,
    manifest_path: &'static str,
    manifest_source: &'static str,
    max_stages: u32,
    time_budget_hours: f64,
    per_phase_timeout_secs: u64,
    poll_timeout_secs: u64,
}

const ACTIONS: &[ActionDefinition] = &[
    ActionDefinition {
        id: "repo-bank-bug-scan",
        title: "Repo Bank Bug Scan",
        summary: "Bounded ZYAL, Jekko, and jnoccio-router bug scan with evidence-only output.",
        subject: "jmcp/zyal/repo-bank-bug-scan",
        manifest_path: "agent/zyal/repo-bank-bug-scan.zyal",
        manifest_source: include_str!("../../../agent/zyal/repo-bank-bug-scan.zyal"),
        max_stages: 4,
        time_budget_hours: 0.5,
        per_phase_timeout_secs: 600,
        poll_timeout_secs: 1800,
    },
    ActionDefinition {
        id: "harden-jekko-with-jailgun",
        title: "Harden Jekko With Jailgun",
        summary: "Inspect Jekko hardening against Jailgun evidence boundaries.",
        subject: "jmcp/zyal/harden-jekko-with-jailgun",
        manifest_path: "agent/zyal/harden-jekko-with-jailgun.zyal",
        manifest_source: include_str!("../../../agent/zyal/harden-jekko-with-jailgun.zyal"),
        max_stages: 4,
        time_budget_hours: 0.5,
        per_phase_timeout_secs: 600,
        poll_timeout_secs: 1800,
    },
    ActionDefinition {
        id: "cache-reduction-validity-check",
        title: "Cache Reduction Validity Check",
        summary: "Verify cache-reduction safety claims with bounded local evidence.",
        subject: "jmcp/zyal/cache-reduction-validity-check",
        manifest_path: "agent/zyal/cache-reduction-validity-check.zyal",
        manifest_source: include_str!("../../../agent/zyal/cache-reduction-validity-check.zyal"),
        max_stages: 4,
        time_budget_hours: 0.5,
        per_phase_timeout_secs: 600,
        poll_timeout_secs: 1800,
    },
];

impl AppState {
    pub fn list_autonomous_actions(&self) -> AppResult<Vec<AutonomousActionCard>> {
        ACTIONS.iter().map(action_card).collect()
    }

    pub fn submit_autonomous_action(
        &self,
        id: &str,
        overrides: AutonomousActionOverrides,
    ) -> AppResult<WorkOrder> {
        let action = action_by_id(id)?;
        let payload = action_payload(action, overrides)?;
        let subject = Subject::from_str(action.subject)?;
        let signer = LocalSigner::load_or_create_default()?;
        let envelope = signer.sign(Envelope::new(subject, ZYAL_RUN_KIND, payload));
        self.submit_envelope(envelope)
    }
}

pub(crate) fn validate_autonomous_action_id(id: &str) -> AppResult<()> {
    action_by_id(id).map(|_| ())
}

fn action_by_id(id: &str) -> AppResult<&'static ActionDefinition> {
    ACTIONS
        .iter()
        .find(|action| action.id == id)
        .ok_or_else(|| AppError::State(format!("unknown autonomous action: {id}")))
}

fn action_card(action: &ActionDefinition) -> AppResult<AutonomousActionCard> {
    let manifest = manifest_value(action)?;
    Ok(AutonomousActionCard {
        id: action.id.to_owned(),
        title: action.title.to_owned(),
        summary: action.summary.to_owned(),
        mode: AutonomousActionMode::FullAuto,
        subject: AutonomousActionSubject(action.subject.to_owned()),
        work_order_kind: AutonomousWorkOrderKind(ZYAL_RUN_KIND.to_owned()),
        manifest: manifest_metadata(action, &manifest)?,
        safety: safety_defaults(action),
    })
}

fn action_payload(
    action: &ActionDefinition,
    overrides: AutonomousActionOverrides,
) -> AppResult<Value> {
    if overrides.live == Some(true) {
        return Err(AppError::State(
            "autonomous action live=true override requires an approval policy".to_owned(),
        ));
    }

    let manifest = manifest_value(action)?;
    let max_stages = bounded_u32(overrides.max_stages, action.max_stages, "maxStages")?
        .unwrap_or(action.max_stages);
    let time_budget_hours = bounded_f64(
        overrides.time_budget_hours,
        action.time_budget_hours,
        "timeBudgetHours",
    )?
    .unwrap_or(action.time_budget_hours);
    let per_phase_timeout_secs = bounded_u64(
        overrides.per_phase_timeout_secs,
        action.per_phase_timeout_secs,
        "perPhaseTimeoutSecs",
    )?
    .unwrap_or(action.per_phase_timeout_secs);
    let poll_timeout_secs = bounded_u64(
        overrides.poll_timeout_secs,
        action.poll_timeout_secs,
        "pollTimeoutSecs",
    )?
    .unwrap_or(action.poll_timeout_secs);

    let mut payload = json!({
        "manifest": manifest,
        "run_id": overrides.run_id.unwrap_or_else(|| format!("jmcp-full-auto-{}", action.id)),
        "live": false,
        "max_stages": max_stages,
        "time_budget_hours": time_budget_hours,
        "per_phase_timeout_secs": per_phase_timeout_secs,
        "poll_timeout_secs": poll_timeout_secs,
        "metadata": {
            "submitted_by": SUBMITTED_BY,
            "autonomous_action_id": action.id,
            "manifest_path": action.manifest_path,
            "mode": "evidence_oriented"
        }
    });

    if let Some(db) = overrides.db {
        payload["db"] = json!(db);
    }
    if let Some(metadata) = overrides.metadata {
        payload["metadata"]["overrides"] = metadata;
    }
    Ok(payload)
}

fn manifest_value(action: &ActionDefinition) -> AppResult<Value> {
    let body = manifest_body(action.manifest_source)?;
    serde_json::from_str(body).map_err(|err| {
        AppError::State(format!(
            "autonomous action manifest {} is invalid: {err}",
            action.manifest_path
        ))
    })
}

fn manifest_metadata(
    action: &ActionDefinition,
    manifest: &Value,
) -> AppResult<AutonomousActionManifestMetadata> {
    Ok(AutonomousActionManifestMetadata {
        path: action.manifest_path.to_owned(),
        manifest_id: required_manifest_string(manifest, "id")?.to_owned(),
        name: required_manifest_job_string(manifest, "name")?.to_owned(),
        objective: required_manifest_job_string(manifest, "objective")?.to_owned(),
        sha256: format!(
            "sha256:{}",
            hex::encode(Sha256::digest(action.manifest_source.as_bytes()))
        ),
    })
}

fn manifest_body(source: &str) -> AppResult<&str> {
    let body_start = source
        .find(">>>\n")
        .map(|index| index + ">>>\n".len())
        .ok_or_else(|| AppError::State("autonomous action manifest missing open marker".into()))?;
    let body_end = source
        .find("\n<<<END_ZYAL ")
        .ok_or_else(|| AppError::State("autonomous action manifest missing close marker".into()))?;
    if body_end <= body_start {
        return Err(AppError::State(
            "autonomous action manifest body is empty".into(),
        ));
    }
    Ok(&source[body_start..body_end])
}

fn required_manifest_string<'a>(manifest: &'a Value, key: &str) -> AppResult<&'a str> {
    manifest
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            AppError::State(format!(
                "autonomous action manifest missing string field {key}"
            ))
        })
}

fn required_manifest_job_string<'a>(manifest: &'a Value, key: &str) -> AppResult<&'a str> {
    manifest
        .get("job")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            AppError::State(format!(
                "autonomous action manifest missing string field job.{key}"
            ))
        })
}

fn safety_defaults(action: &ActionDefinition) -> AutonomousActionSafetyDefaults {
    AutonomousActionSafetyDefaults {
        evidence_oriented: true,
        live: false,
        max_stages: action.max_stages,
        time_budget_hours: action.time_budget_hours,
        per_phase_timeout_secs: action.per_phase_timeout_secs,
        poll_timeout_secs: action.poll_timeout_secs,
        submitted_by: SUBMITTED_BY.to_owned(),
    }
}

fn bounded_u32(value: Option<u32>, cap: u32, field: &str) -> AppResult<Option<u32>> {
    match value {
        Some(value) if value == 0 || value > cap => Err(AppError::State(format!(
            "autonomous action override {field} must be between 1 and {cap}"
        ))),
        other => Ok(other),
    }
}

fn bounded_u64(value: Option<u64>, cap: u64, field: &str) -> AppResult<Option<u64>> {
    match value {
        Some(value) if value == 0 || value > cap => Err(AppError::State(format!(
            "autonomous action override {field} must be between 1 and {cap}"
        ))),
        other => Ok(other),
    }
}

fn bounded_f64(value: Option<f64>, cap: f64, field: &str) -> AppResult<Option<f64>> {
    match value {
        Some(value) if !value.is_finite() || value <= 0.0 || value > cap => {
            Err(AppError::State(format!(
                "autonomous action override {field} must be greater than 0 and no more than {cap}"
            )))
        }
        other => Ok(other),
    }
}
