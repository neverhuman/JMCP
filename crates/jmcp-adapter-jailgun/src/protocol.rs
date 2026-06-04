use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::{json, Value};

pub(crate) const JAILGUN_WIRE_VERSION: u64 = 1;

pub(crate) fn run_agent_request(payload: &Value) -> Result<Value> {
    if let Some(request) = payload.get("request") {
        require_wire_version(request, "Jailgun run request")?;
        return canonicalize_run_request(request.clone());
    }
    if let Some(path) = payload.get("request_path").and_then(|value| value.as_str()) {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading Jailgun request_path {path}"))?;
        let request: Value =
            serde_json::from_str(&text).context("Jailgun request_path was not JSON")?;
        require_wire_version(&request, "Jailgun run request")?;
        return canonicalize_run_request(request);
    }
    anyhow::bail!("jailgun work order requires request_path or request")
}

fn canonicalize_run_request(mut request: Value) -> Result<Value> {
    let Some(object) = request.as_object_mut() else {
        anyhow::bail!("Jailgun run request must be a JSON object");
    };
    let canonical = object
        .get("browser")
        .and_then(|browser| browser.get("account_ids"))
        .map(|value| account_ids(value, "browser.account_ids"))
        .transpose()?;
    let top_level = object
        .get("account_ids")
        .map(|value| account_ids(value, "account_ids"))
        .transpose()?;
    let account_alias = object
        .get("account")
        .map(|value| {
            value
                .as_str()
                .map(|account| vec![account.to_owned()])
                .context("account must be a browser account id string")
        })
        .transpose()?;

    let mut selected = canonical;
    for alias in [top_level, account_alias].into_iter().flatten() {
        if let Some(existing) = selected.as_ref() {
            if existing != &alias {
                anyhow::bail!("conflicting Jailgun browser account routing aliases");
            }
        } else {
            selected = Some(alias);
        }
    }
    if let Some(account_ids) = selected {
        reject_duplicate_accounts(&account_ids)?;
        let browser = object
            .entry("browser")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .context("browser must be a JSON object")?;
        browser.insert("account_ids".to_owned(), json!(account_ids));
    }
    object.remove("account_ids");
    object.remove("account");
    Ok(request)
}

fn account_ids(value: &Value, field: &str) -> Result<Vec<String>> {
    let Some(values) = value.as_array() else {
        anyhow::bail!("{field} must be an array of browser account ids");
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(str::to_owned)
                .with_context(|| format!("{field}[{index}] must be a string"))
        })
        .collect()
}

fn reject_duplicate_accounts(account_ids: &[String]) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for account_id in account_ids {
        if !seen.insert(account_id) {
            anyhow::bail!("duplicate browser account id requested: {account_id}");
        }
    }
    Ok(())
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
