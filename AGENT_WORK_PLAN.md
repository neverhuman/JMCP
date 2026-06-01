# JMCP V1 Core Work Plan

## Board

| ID | Lane | Status | Owner | Deliverable | Verification |
| --- | --- | --- | --- | --- | --- |
| GOV-001 | Instructions | Done | Codex | `AGENTS.md` references RTK and scope rules | Manual read |
| GOV-002 | Coordination | Done | Codex | `AGENT_CHAT.md` append-only log | Manual read |
| GOV-003 | Coordination | Done | Codex | `AGENT_WORK_PLAN.md` task board | Manual read |
| GOV-004 | Standards | Done | Codex | `agent/JANKURAI_STANDARD.md` | Manual read |
| GOV-005 | Maps | Done | Codex | `agent/owner-map.json`, `agent/test-map.json` | JSON parse |
| GOV-006 | Policies | Done | Codex | `agent/generated-zones.toml`, `agent/proof-lanes.toml`, `agent/audit-policy.toml` | TOML parse |
| DOC-001 | Architecture | Done | Codex | `docs/architecture.md` | Link/path read |
| DOC-002 | Testing | Done | Codex | `docs/testing.md` | Link/path read |
| DOC-003 | Security | Done | Codex | `docs/security.md` | Link/path read |
| DOC-004 | Operations | Done | Codex | `docs/operations.md` | Link/path read |
| DOC-005 | Protocol | Done | Codex | `docs/protocol-decisions.md` | Link/path read |
| PAP-001 | Paper | Done | Codex | `paper/jmcp-v1-architecture-security-reproducibility.tex` | Text scan |
| CORE-001 | Protocol | Done | Codex | `crates/jcp-core` envelope, hash, subject, schema, signature stubs | `cargo test`, conformance |
| CORE-002 | Domain | Done | Codex | `crates/jmcp-domain` work, lease, approval, evidence, attention state | `cargo test` |
| CORE-003 | Store/App/API | Done | Codex | SQLite event store/projections, app use cases, Axum API | `cargo test`, `cargo check` |
| CORE-004 | Adapters | Done | Codex | Adapter SDK plus fail-closed Jankurai/Jeryu/Jekko crates | `cargo check` |
| CORE-005 | Telegram | Done | Codex | Text approval parser with wrong-user, expired, forged-token tests | `cargo test` |
| CORE-006 | Binaries | Done | Codex | `jmcpd`, `jmcpctl`, `jmcp-tui` | `cargo check --workspace --all-targets` |
| UI-001 | Cockpit | Done | Codex | Vite/React dashboard for Now, Work, Evidence, Systems, Tools/Data, Memory-lite, Replay, Approvals | `npm run build`, `npm test` |
| CI-001 | Local CI | Done | Codex | `Justfile`, `scripts/`, `ops/ci/` parity gates | `just fast`, `just ci` |
| CI-002 | Security | Done | Codex | gitleaks, cargo-audit, cargo-deny, npm audit, zizmor, SBOM hooks | `just security` |
| CI-003 | GitHub Actions | Done | Codex | Workflows delegate to local scripts | `just fast`, actionlint in fast gate |
| SCHEMA-001 | Canonical Inputs | Done | Codex | Copied JCP and JPCM schemas under `schemas/`; preserved `tips/v6/` | `just conformance` |

## Open Follow-Up Work

- Replace stub signatures with real key-backed signing before networked multi-tenant use.
- Add Postgres/NATS implementations behind the existing store/bus boundaries when V1 needs distributed runtime.
- Expand adapter live-smoke tests once local `jankurai`, `jeryu`, and `jekko` CLIs are available in CI.
- Repair local Jeryu SSH/project access before first GitHub push; `jeryu repo adopt` succeeded, but `git push jeryu main` and `jeryu sync` currently fail on local GitLab authorization.
