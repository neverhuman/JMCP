//! Deterministic invariant (property-style) + integration test for the Jekko
//! adapter's public `worker_run` structured-result -> `JekkoRunOutcome` mapping.
//!
//! No `proptest` dependency: a tiny inline xorshift64* PRNG generates many
//! pseudo-random `structuredContent` payloads (in the router's wire shape) from
//! a fixed seed and feeds each through the public `map_worker_outcome`. The run
//! is fully deterministic and offline -- the mapping is pure (no network, no
//! clock). For every payload we assert the `JekkoRunOutcome` invariants.

use jmcp_adapter_jekko::map_worker_outcome;
use serde_json::{json, Value};

/// Deterministic xorshift64* PRNG (inline; no extra crate, identical stream
/// every run).
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }
}

/// One generated payload plus the facts the test needs to predict the mapping.
struct Generated {
    value: Value,
    status: &'static str,
    job_id: Option<String>,
    summary: Option<String>,
    raw_summary: Option<String>,
    file_change_count: usize,
    failure_count: usize,
}

/// Build a random worker `structuredContent` value in the router wire shape.
fn random_structured(rng: &mut Rng) -> Generated {
    let status = match rng.below(5) {
        0 => "succeeded",
        1 => "failed",
        2 => "running",
        3 => "queued",
        _ => "",
    };

    let job_id = if rng.below(3) == 0 {
        None
    } else {
        Some(format!("job-{}", rng.below(1_000_000)))
    };

    let summary = if rng.below(2) == 0 {
        Some(format!("did thing {}", rng.below(1000)))
    } else {
        None
    };
    let raw_summary = if rng.below(2) == 0 {
        Some(format!("raw cot {}", rng.below(1000)))
    } else {
        None
    };

    let file_change_count = rng.below(4) as usize;
    let file_changes: Vec<Value> = (0..file_change_count)
        .map(|i| {
            let mut obj = serde_json::Map::new();
            obj.insert("path".to_owned(), json!(format!("src/file_{i}.rs")));
            if rng.below(2) == 0 {
                obj.insert(
                    "before_sha256".to_owned(),
                    json!(format!("{:08x}", rng.next_u64())),
                );
            }
            if rng.below(2) == 0 {
                obj.insert(
                    "after_sha256".to_owned(),
                    json!(format!("{:08x}", rng.next_u64())),
                );
            }
            Value::Object(obj)
        })
        .collect();

    let failure_count = rng.below(3) as usize;
    let failures: Vec<Value> = (0..failure_count)
        .map(|i| json!(format!("failure {i}: {}", rng.below(100))))
        .collect();

    let mut obj = serde_json::Map::new();
    if !status.is_empty() {
        obj.insert("status".to_owned(), json!(status));
    }
    if let Some(j) = &job_id {
        obj.insert("job_id".to_owned(), json!(j));
    }
    if let Some(s) = &summary {
        obj.insert("summary".to_owned(), json!(s));
    }
    if let Some(r) = &raw_summary {
        obj.insert("raw_model_summary".to_owned(), json!(r));
    }
    obj.insert(
        "report".to_owned(),
        json!({
            "file_changes": file_changes,
            "failures": failures,
        }),
    );

    Generated {
        value: Value::Object(obj),
        status,
        job_id,
        summary,
        raw_summary,
        file_change_count,
        failure_count,
    }
}

#[test]
fn worker_outcome_invariants_hold_over_random_inputs() {
    let mut rng = Rng::new(0x0BAD_F00D_DEAD_BEEF);

    for i in 0..256u32 {
        let gen = random_structured(&mut rng);
        let outcome = map_worker_outcome(&gen.value);

        // Invariant: success iff status == "succeeded".
        assert_eq!(
            outcome.success,
            gen.status == "succeeded",
            "iteration {i}: success tracks status exactly",
        );

        // Invariant: run_ref is the job_id, else the stable fallback. Never empty.
        let expected_run_ref = gen
            .job_id
            .clone()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "jnoccio-worker".to_owned());
        assert_eq!(
            outcome.run_ref, expected_run_ref,
            "iteration {i}: run_ref maps job_id with fallback",
        );
        assert!(
            !outcome.run_ref.is_empty(),
            "iteration {i}: run_ref non-empty"
        );

        // Invariant: assistant_text prefers summary, falls back to
        // raw_model_summary, else None.
        let expected_text = gen
            .summary
            .clone()
            .filter(|v| !v.is_empty())
            .or_else(|| gen.raw_summary.clone().filter(|v| !v.is_empty()));
        assert_eq!(
            outcome.assistant_text, expected_text,
            "iteration {i}: assistant_text precedence (summary > raw)",
        );

        // Invariant: one artifact per file_change, each a kind:"file".
        assert_eq!(
            outcome.artifacts.len(),
            gen.file_change_count,
            "iteration {i}: one artifact per file change",
        );
        for artifact in &outcome.artifacts {
            assert_eq!(artifact.kind, "file", "iteration {i}: artifact kind");
            assert!(
                !artifact.reference.is_empty(),
                "iteration {i}: artifact reference non-empty",
            );
        }

        // Invariant: error is Some iff status == "failed", and is never empty
        // even when no failures were listed.
        assert_eq!(
            outcome.error.is_some(),
            gen.status == "failed",
            "iteration {i}: error presence tracks failed status",
        );
        if let Some(err) = &outcome.error {
            assert!(!err.is_empty(), "iteration {i}: error message non-empty");
            if gen.failure_count == 0 {
                assert_eq!(
                    err, "jekko worker_run failed",
                    "iteration {i}: empty failures -> default error",
                );
            }
        }
    }
}

#[test]
fn thin_payload_maps_to_deterministic_fallbacks() {
    // A value that does not match the shape must degrade to defaults, never
    // panic -- the mapping is fail-soft.
    let outcome = map_worker_outcome(&json!({}));
    assert_eq!(outcome.run_ref, "jnoccio-worker");
    assert!(!outcome.success);
    assert!(outcome.assistant_text.is_none());
    assert!(outcome.artifacts.is_empty());
    assert!(outcome.error.is_none());

    // A completely wrong-typed value also degrades rather than panicking.
    let from_array = map_worker_outcome(&json!([1, 2, 3]));
    assert_eq!(from_array.run_ref, "jnoccio-worker");
    assert!(!from_array.success);
}
