use std::path::PathBuf;

use chrono::Utc;
use jmcp_domain::Evidence;
use sha2::{Digest, Sha256};

use crate::model::JailgunSummary;

pub(crate) fn evidence_for_summary(
    summary: &JailgunSummary,
    summary_uri: &str,
    events_uri: &str,
) -> Vec<Evidence> {
    let now = Utc::now();
    let mut evidence = vec![
        Evidence {
            kind: "jailgun.run".to_owned(),
            uri: format!("jailgun://run/{}", summary.run_id),
            captured_at: now,
        },
        Evidence {
            kind: "jailgun.summary".to_owned(),
            uri: summary_uri.to_owned(),
            captured_at: now,
        },
        Evidence {
            kind: "jailgun.events".to_owned(),
            uri: events_uri.to_owned(),
            captured_at: now,
        },
        Evidence {
            kind: "jailgun.prompt_ref".to_owned(),
            uri: summary.prompt_ref.clone(),
            captured_at: now,
        },
    ];
    let summary_events_uri = file_uri(&summary.events_jsonl);
    if summary_events_uri != events_uri {
        evidence.push(Evidence {
            kind: "jailgun.events.summary-path".to_owned(),
            uri: summary_events_uri,
            captured_at: now,
        });
    }
    for receipt in &summary.receipt_paths {
        evidence.push(Evidence {
            kind: "jailgun.receipt".to_owned(),
            uri: file_uri(receipt),
            captured_at: now,
        });
    }
    for artifact in &summary.artifacts {
        let uri = artifact
            .sha256
            .as_ref()
            .map(|sha| format!("sha256:{sha}"))
            .unwrap_or_else(|| file_uri(&artifact.path));
        evidence.push(Evidence {
            kind: format!("jailgun.artifact.{}", artifact.kind),
            uri,
            captured_at: now,
        });
        if let Some(receipt) = &artifact.receipt_path {
            evidence.push(Evidence {
                kind: "jailgun.artifact.receipt".to_owned(),
                uri: file_uri(receipt),
                captured_at: now,
            });
        }
    }
    if !summary.failures.is_empty() {
        let digest = hex::encode(Sha256::digest(
            serde_json::to_string(&summary.failures)
                .unwrap_or_default()
                .as_bytes(),
        ));
        evidence.push(Evidence {
            kind: "jailgun.failures.digest".to_owned(),
            uri: format!("sha256:{digest}"),
            captured_at: now,
        });
    }
    evidence
}

pub(crate) fn file_uri(path: &PathBuf) -> String {
    format!("file://{}", path.display())
}
