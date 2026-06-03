# Boundaries

JMCP keeps runtime truth behind explicit Rust, SQL, protocol, and adapter boundaries. The machine-readable boundary manifest is `agent/boundaries.toml`; this doc is the agent-readable companion for reviews and repair work.

## Runtime Ownership

- Rust backend owns protocol validation, approvals, audit records, replay checkpoints, persistence, and adapter effect routing.
- SQLite is the embedded durable store for V1. Durable database truth belongs in `db/migrations`, `db/constraints`, and backend-owned transactions.
- React dashboard, Rust TUI, Telegram, and voice surfaces are clients of the runtime. They may request, inspect, and approve work, but they do not write around policy or persistence.
- Local Jankurai, Jeryu, and Jekko adapters are edge capabilities. Mutating effects must route through approval, policy, replay, and audit components.
- JCP/1.0.0 schemas and JPCM transport profile artifacts define the cross-runtime message contract.

## Data Boundary

Direct database access from a UI, adapter edge, script, or generated client is a bug unless a boundary manifest entry and proof lane explicitly authorize it. Database changes must preserve:

- migration and constraint evidence under `db/`;
- application-owned transactions in Rust;
- replay-safe idempotency for side effects;
- audit records that include component, operation, correlation id, diagnostic class, and retryability without secrets.

## Contract Boundary

Public protocol or event changes must update the source schema first, regenerate declared outputs through the documented command, and run the route in `agent/test-map.json`. Handwritten contract drift is acceptable only as a temporary, dated review finding with a follow-up owner.

## Adapter Boundary

Adapters may observe local tools, collect text, or propose work. They must not:

- bypass approvals for mutating operations;
- duplicate committed side effects during replay;
- log secrets or private tokens;
- persist durable truth outside the backend-owned store;
- widen generated zones or schema contracts without updating `agent/generated-zones.toml` and proof routing.
