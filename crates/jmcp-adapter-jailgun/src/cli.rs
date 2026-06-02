use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tokio::{process::Command, time::sleep};

pub(crate) async fn run_checked(
    command_path: &PathBuf,
    timeout: Duration,
    mut command: Command,
    operation: &str,
) -> Result<()> {
    let output = run_with_retry(timeout, &mut command)
        .await
        .with_context(|| format!("failed to run {}", command_path.display()))?;
    if !output.status.success() {
        let digest = digest_output(output.status.code(), &output.stdout, &output.stderr);
        anyhow::bail!("jailgun {operation} failed with digest {digest}");
    }
    Ok(())
}

async fn run_with_retry(timeout: Duration, command: &mut Command) -> Result<std::process::Output> {
    let mut last_error = None;
    for attempt in 0..5 {
        match tokio::time::timeout(timeout, command.output()).await {
            Err(_) => return Err(anyhow::anyhow!("jailgun command timed out")),
            Ok(Err(err)) => {
                let busy = err.raw_os_error() == Some(26);
                if busy && attempt < 4 {
                    last_error = Some(err);
                    sleep(Duration::from_millis(50 * (attempt + 1) as u64)).await;
                    continue;
                }
                return Err(err.into());
            }
            Ok(Ok(output)) => return Ok(output),
        }
    }
    Err(last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow::anyhow!("jailgun command failed to start")))
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
