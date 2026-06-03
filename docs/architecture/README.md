# Architecture

JMCP architecture docs describe the local production-shaped V1 runtime:
backend-owned persistence, JCP/JPCM protocol envelopes, approval gates,
replayable events, UI clients, adapter boundaries, and local proof lanes.

- Start with `docs/architecture.md` for the system topology and runtime model.
- Use `docs/boundaries.md` for ownership and direct-access rules.
- Use `docs/testing.md` and `agent/proof-lanes.toml` to map architecture claims
  to local proof commands.
- Add dated decision records under `docs/decisions/` when a durable architecture
  choice needs context, alternatives, and consequences.
