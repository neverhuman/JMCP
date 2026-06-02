# Ownership Notes

The flat `agent/owner-map.json` is the authoritative path-to-owner map. This file keeps the richer routing notes that were previously embedded in grouped JSON objects.

## Grouped Surfaces

- `adapters`: local Jankurai, Jeryu, and Jekko adapter surfaces under `crates/**` and `apps/**`.
- `core-runtime`: Rust backend, embedded SQLite integration, in-process replayable event bus, and JCP/1.0.0 runtime enforcement under `crates/**`.
- `dashboard`: React dashboard, operator approvals, and runtime observability under `apps/**`.
- `governance`: agent coordination, documentation governance, proof-lane definitions, audit policy, and architecture/security/testing/operations documentation under `AGENT_CHAT.md`, `AGENT_WORK_PLAN.md`, `AGENTS.md`, `agent/**`, `docs/**`, and `paper/**`.
- `tui`: Rust TUI and local operator workflows across `apps/**` and `crates/**`.
