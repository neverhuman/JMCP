# JMCP Now Proof

## Summary

- Removed the duplicate local JITUX model: `crates/jmcp-now/src/contract.rs` is gone.
- `jmcp-now` now consumes canonical `jmcp_domain` JITUX root re-exports for panes, previews, counters, rank reasons/factors, prepared actions, safety classes, pane status/kind/risk, and LOD.
- Preserved the deterministic ranker: weighted sum only, injected `now`, no wall clock or random inputs.
- Preserved the queue-blockers composer as canonical `PaneVm` output, with canonical `PreparedAction`, `JituxEvidenceRef`, and `PaneRankReason` sidecars where `PaneVm` has no action/evidence fields.
- Preserved the `ArcSwap` projection cache keyed on the app event watermark.

## Changed Paths

- `crates/jmcp-now/src/contract.rs` deleted
- `crates/jmcp-now/src/lib.rs`
- `crates/jmcp-now/src/projection.rs`
- `crates/jmcp-now/src/ranking.rs`
- `crates/jmcp-now/src/scenes/queue_blockers.rs`
- `crates/jmcp-now/src/scenes/queue_blockers/actions.rs`
- `crates/jmcp-now/src/scenes/queue_blockers/signals.rs`
- `crates/jmcp-now/tests/jitux_golden.rs`
- `crates/jmcp-now/tests/queue_blockers.rs`
- `crates/jmcp-now/tests/golden/queue_blockers_scene.json` deleted
- `crates/jmcp-now/tests/golden/scene.schema.json` deleted

## Public API

- `jmcp_now::queue_blockers_panes(&NowReads, DateTime<Utc>) -> Vec<jmcp_domain::PaneVm>`
- `jmcp_now::queue_blockers_projection(&NowReads, DateTime<Utc>) -> jmcp_now::QueueBlockersProjection`
- `jmcp_now::NowProjection::{load, refresh_if_stale, refresh_if_stale_at}` returns `Arc<CachedNow>`; `CachedNow` exposes `panes`, `rank_reasons`, `prepared_actions`, and `evidence_refs` as canonical JITUX types.

Broker note: `jmcp_domain` currently re-exports JITUX types at the crate root while keeping the `jitux` module private. The broker can call the APIs above using `jmcp_domain::PaneVm` paths now. If the broker requires the literal `jmcp_domain::jitux::PaneVm` path, the domain owner needs to expose that module path.

## Verification

Implementation commit: `9ce6589 refactor(now): consume canonical jitux types`

- `rtk cargo build -p jmcp-now`: passed, 0 crates compiled, finished in 0.12s.
- `rtk cargo test -p jmcp-now`: passed, 9 tests passed across 4 suites, finished in 0.17s.
- `rtk jankurai audit . --json target/jankurai/now-audit.json --md target/jankurai/now-audit.md`: score=92 raw=92 caps=0 findings=1.

Audit note: the remaining finding is repo-level `HLT-001-DEAD-MARKER` at `.`, with evidence pointing at `crates/jmcp-app/src/microtasks.rs` and broad advisory signals. It is outside this task's owned path and there are no caps.
