use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Duration};
use tokio::process::Command;

#[derive(Clone, Debug)]
pub struct JankuraiAdapter {
    command: PathBuf,
    timeout: Duration,
}

impl Default for JankuraiAdapter {
    fn default() -> Self {
        Self {
            command: std::env::var_os("JMCP_JANKURAI_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("jankurai")),
            timeout: Duration::from_secs(30),
        }
    }
}

impl JankuraiAdapter {
    pub fn new(command: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            command: command.into(),
            timeout,
        }
    }
}

#[async_trait]
impl Adapter for JankuraiAdapter {
    fn service_card(&self) -> ServiceCard {
        ServiceCard {
            name: "jankurai".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subjects: vec!["*/jankurai/*".to_owned()],
            capabilities: vec!["local-cli".to_owned()],
        }
    }

    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        let action = action_for_kind(&work_order.task.kind)?;
        let cwd = work_order
            .task
            .payload
            .get("cwd")
            .and_then(|value| value.as_str())
            .unwrap_or(".");

        let mut command = Command::new(&self.command);
        command.arg(action).current_dir(cwd);

        let output = tokio::time::timeout(self.timeout, command.output())
            .await
            .context("jankurai command timed out")?
            .with_context(|| format!("failed to run {}", self.command.display()))?;

        let digest = digest_output(output.status.code(), &output.stdout, &output.stderr);
        if !output.status.success() {
            anyhow::bail!("jankurai {action} failed with digest {digest}");
        }

        Ok(vec![Evidence {
            kind: format!("jankurai.{action}.digest"),
            uri: format!("sha256:{digest}"),
            captured_at: Utc::now(),
        }])
    }
}

fn action_for_kind(kind: &str) -> Result<&'static str> {
    match kind {
        "jankurai.diff-audit" | "diff-audit" => Ok("diff-audit"),
        "jankurai.proof" | "proof" => Ok("proof"),
        "jankurai.doctor" | "doctor" => Ok("doctor"),
        _ => Err(fail_closed("jankurai")),
    }
}

fn digest_output(status: Option<i32>, stdout: &[u8], stderr: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(status.unwrap_or(-1).to_string().as_bytes());
    hasher.update(b"\0stdout\0");
    hasher.update(stdout);
    hasher.update(b"\0stderr\0");
    hasher.update(stderr);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jmcp_adapter_sdk::Adapter;
    use serde_json::json;
    use std::fs;

    #[tokio::test]
    async fn captures_digest_from_fake_jankurai() {
        let dir = std::env::temp_dir().join(format!("jmcp-jankurai-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("jankurai");
        fs::write(&bin, "#!/usr/bin/env bash\nprintf 'proof-ok'\n").unwrap();
        make_executable(&bin);

        let adapter = JankuraiAdapter::new(&bin, Duration::from_secs(5));
        let work_order = WorkOrder::submit("t/jankurai/e", "jankurai.proof", json!({"cwd": "."}));
        let evidence = adapter.execute(&work_order).await.unwrap();

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].kind, "jankurai.proof.digest");
        assert!(evidence[0].uri.starts_with("sha256:"));
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    fn make_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &std::path::Path) {}
}
