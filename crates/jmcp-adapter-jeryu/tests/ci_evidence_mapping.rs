//! Deterministic invariant (property-style) + integration test for the Jeryu
//! adapter's CI-run -> Evidence mapping.
//!
//! No `proptest` dependency: a tiny inline xorshift64* PRNG generates many
//! pseudo-random `JeryuCiRun` payloads from a fixed seed. Each is deserialized
//! through the crate's public `JeryuCiRun` wire type and fed through the public
//! `JeryuAdapter::execute` path (over an in-test stub `JeryuClient`, so the run
//! is fully offline -- no network, no live forge). For every run we assert the
//! Evidence invariants the mapping must uphold.

use anyhow::Result;
use async_trait::async_trait;
use jmcp_adapter_jeryu::{JeryuCiRun, JeryuClient};
use jmcp_adapter_sdk::Adapter;
use jmcp_domain::WorkOrder;
use serde_json::{json, Value};
use std::sync::Arc;

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

/// A stub forge that is always live and replays a fixed run, so the mapping is
/// exercised without any network.
struct StubClient {
    run: JeryuCiRun,
}

#[async_trait]
impl JeryuClient for StubClient {
    async fn health(&self) -> Result<()> {
        Ok(())
    }

    async fn ci_run_evidence(&self, _run_id: &str) -> Result<JeryuCiRun> {
        Ok(self.run.clone())
    }
}

/// Build a random ci-run JSON in the public `JeryuCiRun` wire shape.
fn random_ci_run_json(rng: &mut Rng) -> Value {
    let run_id = format!("run-{}", rng.below(1_000_000));
    let status = match rng.below(4) {
        0 => Some("success"),
        1 => Some("failed"),
        2 => Some("running"),
        _ => None,
    };
    let commit = if rng.below(2) == 0 {
        Some(format!("{:016x}", rng.next_u64()))
    } else {
        None
    };

    let artifact_count = rng.below(5);
    let kinds = ["junit", "logs", "sbom", "coverage"];
    let artifacts: Vec<Value> = (0..artifact_count)
        .map(|i| {
            let kind = kinds[(rng.below(kinds.len() as u64)) as usize];
            let digest = if rng.below(2) == 0 {
                Some(format!("{:08x}", rng.next_u64()))
            } else {
                None
            };
            let mut obj = serde_json::Map::new();
            obj.insert("kind".to_owned(), json!(kind));
            obj.insert("reference".to_owned(), json!(format!("{kind}-{i}.out")));
            if let Some(d) = digest {
                obj.insert("digest".to_owned(), json!(d));
            }
            Value::Object(obj)
        })
        .collect();

    let mut obj = serde_json::Map::new();
    obj.insert("run_id".to_owned(), json!(run_id));
    if let Some(s) = status {
        obj.insert("status".to_owned(), json!(s));
    }
    if let Some(c) = commit {
        obj.insert("commit_sha".to_owned(), json!(c));
    }
    obj.insert("artifacts".to_owned(), Value::Array(artifacts));
    Value::Object(obj)
}

#[tokio::test]
async fn ci_run_to_evidence_invariants_hold_over_random_inputs() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);

    for i in 0..256u32 {
        let run_json = random_ci_run_json(&mut rng);
        // Exercise the public wire type: a well-formed ci-run JSON must parse.
        let run: JeryuCiRun = serde_json::from_value(run_json.clone()).unwrap_or_else(|err| {
            panic!("iteration {i}: ci-run must deserialize: {err}\n{run_json}")
        });
        let expected_commit = run.commit_sha.clone();
        let artifact_count = run.artifacts.len();

        let adapter = jmcp_adapter_jeryu::JeryuAdapter::new(Arc::new(StubClient { run }));
        let work_order = WorkOrder::submit(
            "t/jeryu/e",
            "jeryu.ci",
            json!({ "run_id": "ignored-by-stub" }),
        );
        let evidence = adapter
            .execute(&work_order)
            .await
            .unwrap_or_else(|err| panic!("iteration {i}: execute must succeed: {err:?}"));

        // Invariant: the run-identity evidence node is always first.
        assert_eq!(
            evidence[0].kind, "jeryu.ci-run",
            "iteration {i}: first evidence is the run node"
        );
        assert!(
            evidence[0].uri.starts_with("jeryu://ci/run/"),
            "iteration {i}: run uri scheme",
        );

        // Invariant: exactly one digest node, always present, sha256-prefixed.
        let digest_nodes: Vec<_> = evidence
            .iter()
            .filter(|e| e.kind == "jeryu.ci-run.digest")
            .collect();
        assert_eq!(
            digest_nodes.len(),
            1,
            "iteration {i}: exactly one digest node"
        );
        assert!(
            digest_nodes[0].uri.starts_with("sha256:"),
            "iteration {i}: digest node is sha256-prefixed",
        );

        // Invariant: a commit node iff the run carried a commit sha.
        let commit_nodes = evidence
            .iter()
            .filter(|e| e.kind == "jeryu.ci-run.commit")
            .count();
        assert_eq!(
            commit_nodes,
            usize::from(expected_commit.is_some()),
            "iteration {i}: commit node presence tracks commit_sha",
        );

        // Invariant: one artifact node per artifact, and every artifact uri is
        // non-empty (either a sha256 digest or the raw reference).
        let artifact_nodes: Vec<_> = evidence
            .iter()
            .filter(|e| e.kind.starts_with("jeryu.artifact."))
            .collect();
        assert_eq!(
            artifact_nodes.len(),
            artifact_count,
            "iteration {i}: one evidence node per artifact",
        );
        for node in &artifact_nodes {
            assert!(
                !node.uri.is_empty(),
                "iteration {i}: artifact uri is non-empty",
            );
        }

        // Invariant: total count is exactly run + digest + (commit?) + artifacts.
        let expected_total = 2 + usize::from(expected_commit.is_some()) + artifact_count;
        assert_eq!(
            evidence.len(),
            expected_total,
            "iteration {i}: evidence count is the sum of its parts",
        );
    }
}
