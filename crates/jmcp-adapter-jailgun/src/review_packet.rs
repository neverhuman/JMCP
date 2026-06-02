use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use jmcp_domain::Evidence;
use serde_json::Value;

use crate::{
    evidence::file_uri,
    protocol::{
        ensure_no_prompt_text, payload_str, require_wire_version, required_path, required_str,
    },
};

pub(crate) struct ReviewPacketRequest<'a> {
    pub(crate) cwd: &'a str,
    pub(crate) summary_json: PathBuf,
    pub(crate) base: &'a str,
    pub(crate) head: &'a str,
    pub(crate) repo: &'a str,
    pub(crate) output: PathBuf,
    pub(crate) patch_bytes: u64,
}

pub(crate) fn review_packet_request(payload: &Value) -> Result<ReviewPacketRequest<'_>> {
    require_wire_version(payload, "Jailgun review-packet request")?;
    Ok(ReviewPacketRequest {
        cwd: payload_str(payload, "cwd").unwrap_or("."),
        summary_json: required_path(payload, "summary_json")?,
        base: required_str(payload, "base")?,
        head: required_str(payload, "head")?,
        repo: payload_str(payload, "repo").unwrap_or("."),
        output: required_path(payload, "output")?,
        patch_bytes: payload
            .get("patch_bytes")
            .and_then(|value| value.as_u64())
            .unwrap_or(128 * 1024),
    })
}

pub(crate) fn review_packet_evidence(output: &PathBuf) -> Result<Vec<Evidence>> {
    let packet = std::fs::read_to_string(output)
        .with_context(|| format!("reading Jailgun review packet {}", output.display()))?;
    let json: Value = serde_json::from_str(&packet).context("invalid Jailgun review packet")?;
    require_wire_version(&json, "Jailgun review packet")?;
    ensure_no_prompt_text(&json)?;
    Ok(vec![Evidence {
        kind: "jailgun.review_packet".to_owned(),
        uri: file_uri(output),
        captured_at: Utc::now(),
    }])
}
