# JMCP Now Proof

- 2026-06-03T00:00:00Z initialized jmcp-now contract, ranker, read boundary, scene composer, projection cache, and event watermark hook; proof pending first build.
- 2026-06-03T00:00:00Z feat(now): contract and deterministic ranker; `cargo build -p jmcp-now` passed; `cargo test -p jmcp-now` passed (3 tests); `jankurai audit .` score=92 caps=0 findings=1.
- 2026-06-03T00:00:00Z feat(now): golden scene/schema and queue blocker tests; `cargo build -p jmcp-now` passed; `cargo test -p jmcp-now` passed (8 tests, 1 ignored writer); `jankurai audit .` score=92 caps=0 findings=1.

## Final Summary

Changed paths:

- `Cargo.toml`
- `Cargo.lock`
- `crates/jmcp-app/src/lib.rs`
- `crates/jmcp-store/src/replay.rs`
- `crates/jmcp-now/**`

Final proof:

- `cargo build -p jmcp-now`: passed, 0 crates compiled, finished in 0.14s.
- `cargo test -p jmcp-now`: passed, 8 tests passed, 1 ignored writer, 4 suites.
- `jankurai audit . --full --json target/jankurai/now-audit.json --md target/jankurai/now-audit.md`: passed, score=92 raw=92 caps=0 findings=1.

Audit note:

- Remaining finding is repo-level shape `HLT-001-DEAD-MARKER` at `.`, not a new `crates/jmcp-now` product-code cap.
