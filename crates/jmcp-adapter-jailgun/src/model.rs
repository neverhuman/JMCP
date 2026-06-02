use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct JailgunAcceptedRun {
    #[allow(dead_code)]
    pub(crate) run_id: String,
    #[allow(dead_code)]
    pub(crate) status: String,
    #[allow(dead_code)]
    pub(crate) summary_json: String,
    pub(crate) events_jsonl: String,
    #[allow(dead_code)]
    pub(crate) run_url: String,
    pub(crate) summary_url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JailgunSummary {
    #[allow(dead_code)]
    pub(crate) version: u16,
    pub(crate) run_id: String,
    pub(crate) status: String,
    pub(crate) prompt_ref: String,
    pub(crate) events_jsonl: PathBuf,
    #[serde(default)]
    pub(crate) receipt_paths: Vec<PathBuf>,
    #[serde(default)]
    pub(crate) artifacts: Vec<JailgunArtifact>,
    #[serde(default)]
    pub(crate) failures: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JailgunArtifact {
    pub(crate) kind: String,
    pub(crate) path: PathBuf,
    #[serde(default)]
    pub(crate) sha256: Option<String>,
    #[serde(default)]
    pub(crate) receipt_path: Option<PathBuf>,
}
