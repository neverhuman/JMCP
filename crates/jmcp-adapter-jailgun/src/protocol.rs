use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::Value;

pub(crate) const JAILGUN_WIRE_VERSION: u64 = 1;

pub(crate) fn run_agent_request(payload: &Value) -> Result<Value> {
    if let Some(request) = payload.get("request") {
        require_wire_version(request, "Jailgun run request")?;
        return Ok(request.clone());
    }
    if let Some(path) = payload.get("request_path").and_then(|value| value.as_str()) {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading Jailgun request_path {path}"))?;
        let request: Value =
            serde_json::from_str(&text).context("Jailgun request_path was not JSON")?;
        require_wire_version(&request, "Jailgun run request")?;
        return Ok(request);
    }
    anyhow::bail!("jailgun work order requires request_path or request")
}

pub(crate) fn require_wire_version(value: &Value, context: &str) -> Result<()> {
    match value.get("version").and_then(|version| version.as_u64()) {
        Some(JAILGUN_WIRE_VERSION) => Ok(()),
        Some(version) => anyhow::bail!(
            "unsupported {context} version {}; expected {}",
            version,
            JAILGUN_WIRE_VERSION
        ),
        None => anyhow::bail!("{context} requires version: {}", JAILGUN_WIRE_VERSION),
    }
}

pub(crate) fn ensure_no_prompt_text(value: &Value) -> Result<()> {
    match value {
        Value::Object(map) => {
            if map.contains_key("prompt_text") || map.contains_key("prompt") {
                anyhow::bail!("Jailgun durable artifact contains prompt text key");
            }
            for child in map.values() {
                ensure_no_prompt_text(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                ensure_no_prompt_text(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn required_path(payload: &Value, key: &str) -> Result<PathBuf> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
        .with_context(|| format!("jailgun work order missing {key}"))
}

pub(crate) fn required_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .with_context(|| format!("jailgun work order missing {key}"))
}

pub(crate) fn payload_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|value| value.as_str())
}
