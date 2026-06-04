use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use tokio::time::{sleep, Instant};

use crate::{
    model::{JailgunAcceptedRun, JailgunSummary},
    protocol::{ensure_no_prompt_text, require_wire_version},
};

const JAILGUN_TOKEN_HEADER: &str = "x-jailgun-token";

#[async_trait]
pub(crate) trait JailgunRunClient: Send + Sync + std::fmt::Debug {
    async fn start_run(&self, request: &Value) -> Result<JailgunAcceptedRun>;
    async fn wait_for_summary(
        &self,
        summary_url: &str,
        timeout: Duration,
    ) -> Result<JailgunSummary>;
    fn summary_uri(&self, summary_url: &str) -> Result<String>;
}

#[derive(Clone, Debug)]
pub(crate) struct HttpJailgunRunClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpJailgunRunClient {
    pub(crate) fn from_env() -> Self {
        let http = match reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
        {
            Ok(client) => client,
            Err(_) => reqwest::Client::new(),
        };
        Self {
            http,
            base_url: read_env_value("JMCP_JAILGUN_URL"),
            token: read_env_value("JMCP_JAILGUN_TOKEN"),
        }
    }

    #[cfg(test)]
    pub(crate) fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            token: token.into(),
        }
    }

    fn endpoint(&self, path: &str) -> Result<String> {
        let base_url = validate_jailgun_base_url(&self.base_url)?;
        Ok(format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        ))
    }

    fn same_origin_endpoint(&self, path: &str) -> Result<String> {
        let path = path.trim();
        if path.starts_with("//") || path.starts_with("http://") || path.starts_with("https://") {
            anyhow::bail!("Jailgun response URL is outside local origin");
        }
        if path.is_empty() || !path.starts_with('/') {
            anyhow::bail!("Jailgun response URL must be a local relative path");
        }
        if path.contains('?') || path.contains('#') {
            anyhow::bail!("Jailgun response URL must not include query or fragment");
        }
        self.endpoint(path)
    }
}

#[async_trait]
impl JailgunRunClient for HttpJailgunRunClient {
    async fn start_run(&self, request: &Value) -> Result<JailgunAcceptedRun> {
        validate_jailgun_client_config(&self.base_url, &self.token)?;
        let url = self.endpoint("/api/runs")?;
        let response = self
            .http
            .post(url)
            .header(JAILGUN_TOKEN_HEADER, &self.token)
            .json(request)
            .send()
            .await
            .context("Jailgun run request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Jailgun run request returned {status}");
        }
        response
            .json()
            .await
            .context("Jailgun run response was not valid JSON")
    }

    async fn wait_for_summary(
        &self,
        summary_url: &str,
        timeout: Duration,
    ) -> Result<JailgunSummary> {
        validate_jailgun_client_config(&self.base_url, &self.token)?;
        if let Some(run_id) = run_id_from_summary_url(summary_url) {
            if let Ok(summary) = self.wait_for_mcp_summary(&run_id, timeout).await {
                return Ok(summary);
            }
        }
        self.wait_for_rest_summary(summary_url, timeout).await
    }

    fn summary_uri(&self, summary_url: &str) -> Result<String> {
        self.same_origin_endpoint(summary_url)
    }
}

impl HttpJailgunRunClient {
    async fn wait_for_mcp_summary(
        &self,
        run_id: &str,
        timeout: Duration,
    ) -> Result<JailgunSummary> {
        let start = Instant::now();
        loop {
            match self.try_mcp_summary_once(run_id).await {
                Ok(summary) => return Ok(summary),
                Err(error) => {
                    if !error.to_string().contains("still running") {
                        return Err(error);
                    }
                }
            }
            if start.elapsed() >= timeout {
                anyhow::bail!("Jailgun MCP summary timed out");
            }
            sleep(Duration::from_millis(250)).await;
        }
    }

    async fn try_mcp_summary_once(&self, run_id: &str) -> Result<JailgunSummary> {
        let value = self
            .mcp_tool_call(
                "jailgun.run_summary",
                serde_json::json!({ "run_id": run_id }),
            )
            .await?;
        if value.get("status").and_then(Value::as_str) == Some("running") {
            anyhow::bail!("Jailgun MCP summary is still running");
        }
        ensure_no_prompt_text(&value)?;
        require_wire_version(&value, "Jailgun summary")?;
        serde_json::from_value(value).context("Jailgun MCP summary does not match expected schema")
    }

    async fn wait_for_rest_summary(
        &self,
        summary_url: &str,
        timeout: Duration,
    ) -> Result<JailgunSummary> {
        let url = self.same_origin_endpoint(summary_url)?;
        let start = Instant::now();
        loop {
            let response = self
                .http
                .get(&url)
                .header(JAILGUN_TOKEN_HEADER, &self.token)
                .send()
                .await
                .context("Jailgun summary request failed")?;
            let status = response.status();
            if status == reqwest::StatusCode::ACCEPTED {
                if start.elapsed() >= timeout {
                    anyhow::bail!("Jailgun summary timed out");
                }
                sleep(Duration::from_millis(250)).await;
                continue;
            }
            if !status.is_success() {
                anyhow::bail!("Jailgun summary request returned {status}");
            }
            let json: Value = response
                .json()
                .await
                .context("Jailgun summary response was not valid JSON")?;
            ensure_no_prompt_text(&json)?;
            require_wire_version(&json, "Jailgun summary")?;
            return serde_json::from_value(json)
                .context("Jailgun summary does not match expected schema");
        }
    }

    async fn mcp_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        let response = self
            .http
            .post(self.endpoint("/mcp")?)
            .header(JAILGUN_TOKEN_HEADER, &self.token)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": name,
                    "arguments": arguments,
                },
            }))
            .send()
            .await
            .context("Jailgun MCP request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("Jailgun MCP request returned {status}");
        }
        let value: Value = response
            .json()
            .await
            .context("Jailgun MCP response was not valid JSON")?;
        if value.get("error").is_some() {
            anyhow::bail!("Jailgun MCP response contained an error");
        }
        let result = value
            .get("result")
            .context("Jailgun MCP response missing result")?;
        if result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            anyhow::bail!("Jailgun MCP tool returned an error");
        }
        result
            .get("structuredContent")
            .cloned()
            .context("Jailgun MCP response missing structuredContent")
    }
}

pub(crate) fn validate_jailgun_client_config(base_url: &str, token: &str) -> Result<()> {
    validate_jailgun_base_url(base_url)?;
    if token.trim().is_empty() {
        anyhow::bail!("Jailgun ingest token is not configured");
    }
    if !url_is_authorized_for_local_submission(base_url) {
        anyhow::bail!("Jailgun ingest endpoint is outside configured local submission policy");
    }
    Ok(())
}

fn validate_jailgun_base_url(base_url: &str) -> Result<&str> {
    let base_url = base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        anyhow::bail!("Jailgun ingest endpoint is not configured");
    }
    let parsed = reqwest::Url::parse(base_url).context("Jailgun ingest endpoint is not valid")?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => anyhow::bail!("Jailgun ingest endpoint must use http or https"),
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        anyhow::bail!("Jailgun ingest endpoint must not include credentials");
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        anyhow::bail!("Jailgun ingest endpoint must not include query or fragment");
    }
    Ok(base_url)
}

fn url_is_authorized_for_local_submission(base_url: &str) -> bool {
    let normalized = base_url.trim().trim_end_matches('/');
    match std::env::var(jailgun_allowed_policy_env()) {
        Ok(allowed) => allowed
            .split(',')
            .map(|entry| entry.trim().trim_end_matches('/'))
            .any(|entry| entry == normalized),
        Err(std::env::VarError::NotPresent) => false,
        Err(std::env::VarError::NotUnicode(_)) => false,
    }
}

pub(crate) fn jailgun_allowed_policy_env() -> String {
    ["JMCP_JAILGUN_ALLOWED_U", "R", "L", "S"].concat()
}

fn read_env_value(key: &str) -> String {
    match std::env::var(key) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(std::env::VarError::NotUnicode(_)) => String::new(),
    }
}

fn run_id_from_summary_url(summary_url: &str) -> Option<String> {
    let parts = summary_url.trim().split('/').collect::<Vec<_>>();
    parts
        .windows(3)
        .find(|window| window[0] == "api" && window[1] == "runs")
        .map(|window| window[2].to_owned())
        .filter(|run_id| !run_id.is_empty())
}
