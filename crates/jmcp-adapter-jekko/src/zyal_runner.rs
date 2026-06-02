//! CLI transport to the jekko ZYAL engine.
//!
//! Submission to jekko is CLI-only today (`jekko port-run --super` / `--status`);
//! the surface is abstracted behind [`ZyalRunner`] so the adapter is tested with
//! a stub binary, without a live jekko.

use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;

use super::ZyalRunStatus;

/// Default jekko binary used when `JEKKO_BIN` is unset.
const DEFAULT_JEKKO_BIN: &str = "jekko";

/// Options threaded into `jekko port-run`.
#[derive(Clone, Debug, Default)]
pub struct SubmitOpts {
    /// Pass `--live` to drive real per-phase execution (vs the scaffold walk).
    pub live: bool,
    /// `--max-stages N`: stop after N complete phases.
    pub max_stages: Option<u32>,
    /// `--time-budget-hours H`: wall-clock ceiling.
    pub time_budget_hours: Option<f64>,
    /// `--per-phase-timeout-secs N`.
    pub per_phase_timeout_secs: Option<u64>,
}

/// CLI surface of the jekko ZYAL engine, abstracted for testing.
#[async_trait]
pub trait ZyalRunner: Send + Sync {
    /// Start a run from a manifest file under a deterministic `run_id`.
    async fn submit(
        &self,
        manifest_path: &Path,
        db: Option<&str>,
        run_id: &str,
        opts: &SubmitOpts,
    ) -> Result<()>;

    /// Read the current status snapshot for a run.
    async fn status(&self, db: Option<&str>, run_id: &str) -> Result<ZyalRunStatus>;
}

/// Real [`ZyalRunner`] that shells out to the `jekko` binary.
pub struct CliZyalRunner {
    bin: String,
}

impl CliZyalRunner {
    /// Build from `JEKKO_BIN` (default `jekko`).
    pub fn from_env() -> Self {
        let bin = std::env::var("JEKKO_BIN")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_JEKKO_BIN.to_owned());
        Self { bin }
    }

    /// Build with an explicit binary path (used by tests with a stub).
    pub fn with_bin(bin: impl Into<String>) -> Self {
        Self { bin: bin.into() }
    }
}

#[async_trait]
impl ZyalRunner for CliZyalRunner {
    async fn submit(
        &self,
        manifest_path: &Path,
        db: Option<&str>,
        run_id: &str,
        opts: &SubmitOpts,
    ) -> Result<()> {
        let mut cmd = tokio::process::Command::new(&self.bin);
        cmd.arg("port-run")
            .arg("--super")
            .arg(manifest_path)
            .arg("--run-id")
            .arg(run_id);
        if let Some(db) = db {
            cmd.arg("--db").arg(db);
        }
        if opts.live {
            cmd.arg("--live");
        }
        if let Some(n) = opts.max_stages {
            cmd.arg("--max-stages").arg(n.to_string());
        }
        if let Some(h) = opts.time_budget_hours {
            cmd.arg("--time-budget-hours").arg(h.to_string());
        }
        if let Some(s) = opts.per_phase_timeout_secs {
            cmd.arg("--per-phase-timeout-secs").arg(s.to_string());
        }
        let output = cmd
            .output()
            .await
            .with_context(|| format!("spawn `{} port-run --super`", self.bin))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            anyhow::bail!(
                "jekko port-run submit failed ({:?}): {}",
                output.status.code(),
                if stderr.is_empty() {
                    "<no stderr>".to_owned()
                } else {
                    stderr
                }
            );
        }
        Ok(())
    }

    async fn status(&self, db: Option<&str>, run_id: &str) -> Result<ZyalRunStatus> {
        let mut cmd = tokio::process::Command::new(&self.bin);
        cmd.arg("port-run").arg("--status").arg(run_id);
        if let Some(db) = db {
            cmd.arg("--db").arg(db);
        }
        let output = cmd
            .output()
            .await
            .with_context(|| format!("spawn `{} port-run --status`", self.bin))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            anyhow::bail!(
                "jekko port-run --status failed: {}",
                if stderr.is_empty() {
                    "<no stderr>".to_owned()
                } else {
                    stderr
                }
            );
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status: ZyalRunStatus =
            serde_json::from_str(stdout.trim()).context("parse `jekko port-run --status` JSON")?;
        if status.run_id.is_empty() {
            status.run_id = run_id.to_owned();
        }
        Ok(status)
    }
}
