use super::*;

#[test]
fn maps_succeeded_run_with_file_changes_to_artifacts() {
    let structured = json!({
        "job_id": "job-7",
        "status": "succeeded",
        "summary": "patched the parser",
        "report": {
            "files_read": ["src/lib.rs"],
            "files_changed": ["src/parse.rs"],
            "file_changes": [
                {
                    "path": "src/parse.rs",
                    "before_sha256": "aaa",
                    "after_sha256": "bbb"
                }
            ],
            "commands_run": ["cargo test"],
            "failures": []
        },
        "raw_model_summary": "ignored when summary present"
    });
    let outcome = map_worker_outcome(&structured);
    assert!(outcome.success);
    assert_eq!(outcome.run_ref, "job-7");
    assert_eq!(
        outcome.assistant_text.as_deref(),
        Some("patched the parser")
    );
    assert!(outcome.error.is_none());
    assert_eq!(outcome.artifacts.len(), 1);
    let artifact = &outcome.artifacts[0];
    assert_eq!(artifact.kind, "file");
    assert_eq!(artifact.reference, "src/parse.rs");
    assert_eq!(artifact.digest.as_deref(), Some("bbb"));
}

#[test]
fn failed_run_joins_failures_into_error() {
    let structured = json!({
        "job_id": "job-9",
        "status": "failed",
        "summary": "could not build",
        "report": {
            "failures": ["cargo build failed", "missing dependency"]
        }
    });
    let outcome = map_worker_outcome(&structured);
    assert!(!outcome.success);
    assert_eq!(outcome.run_ref, "job-9");
    assert_eq!(
        outcome.error.as_deref(),
        Some("cargo build failed; missing dependency")
    );
}

#[test]
fn failed_run_without_failures_still_reports_error() {
    let structured = json!({
        "job_id": "job-10",
        "status": "failed"
    });
    let outcome = map_worker_outcome(&structured);
    assert!(!outcome.success);
    assert_eq!(outcome.error.as_deref(), Some("jekko worker_run failed"));
}

#[test]
fn running_status_is_not_success_and_has_no_error() {
    let structured = json!({
        "job_id": "job-running",
        "status": "running"
    });
    let outcome = map_worker_outcome(&structured);
    assert!(!outcome.success);
    assert!(outcome.error.is_none());
    assert_eq!(outcome.run_ref, "job-running");
}

#[test]
fn thin_payload_uses_fallbacks() {
    let outcome = map_worker_outcome(&json!({}));
    assert_eq!(outcome.run_ref, "jnoccio-worker");
    assert!(!outcome.success);
    assert!(outcome.assistant_text.is_none());
    assert!(outcome.artifacts.is_empty());
    assert!(outcome.error.is_none());
}

#[test]
fn falls_back_to_raw_model_summary_when_summary_missing() {
    let structured = json!({
        "job_id": "job-3",
        "status": "succeeded",
        "raw_model_summary": "raw chain of thought summary"
    });
    let outcome = map_worker_outcome(&structured);
    assert!(outcome.success);
    assert_eq!(
        outcome.assistant_text.as_deref(),
        Some("raw chain of thought summary")
    );
}

#[test]
fn file_change_without_after_digest_yields_none_digest() {
    let structured = json!({
        "job_id": "job-4",
        "status": "succeeded",
        "report": {
            "file_changes": [
                { "path": "README.md", "before_sha256": "old" }
            ]
        }
    });
    let outcome = map_worker_outcome(&structured);
    assert_eq!(outcome.artifacts.len(), 1);
    assert_eq!(outcome.artifacts[0].reference, "README.md");
    assert!(outcome.artifacts[0].digest.is_none());
}

#[test]
fn status_helpers_read_running_and_job_id() {
    let structured = json!({ "status": "running", "job_id": "abc" });
    assert_eq!(status_of(&structured), Some("running"));
    assert_eq!(job_id_of(&structured), Some("abc".to_owned()));

    let empty_job = json!({ "status": "running", "job_id": "" });
    assert_eq!(job_id_of(&empty_job), None);

    let missing = json!({});
    assert_eq!(status_of(&missing), None);
    assert_eq!(job_id_of(&missing), None);
}
