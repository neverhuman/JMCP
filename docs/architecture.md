# JMCP V1 Architecture

## System Identity

JMCP is the system. JCP/1.0.0 is the protocol. JPCM is the backbone and transport profile. V1 is intentionally local production-shaped: it must run on a single developer/operator machine while preserving the boundaries, evidence, and failure behavior expected from a production core.

## Default Topology

The V1 default topology is:

- Rust backend as the authoritative runtime.
- Embedded SQLite as the durable local store.
- In-process replayable event bus as the default event backbone.
- React dashboard for visual operation, inspection, and approvals.
- Rust TUI for terminal-first operation and recovery workflows.
- Telegram text intake and approval surface.
- Local Jankurai, Jeryu, and Jekko adapters.
- CI-local parity so local verification and CI verification exercise the same default architecture.

## Runtime Boundaries

The backend owns persistence, protocol validation, approval state, audit records, and replay. User interfaces and adapters are clients of those boundaries, not alternate authorities. All shared messages entering the runtime are represented as JCP/1.0.0 envelopes and carried under JPCM assumptions.

Adapters provide capability at the edge. They may ingest text, produce proposed actions, or execute approved local work. They must not write around policy, approval, replay, or audit components.

## Event Model

The in-process event bus is the default backbone for V1 because it keeps local development deterministic and replayable. Structured runtime/control-plane events include `user.message.received`, `user.attention.requested`, `voice.turn.started`, `voice.turn.transcribed`, `voice.intent.confirmed`, `attention.packet`, `memory.proposed`, `memory.accepted`, `tool.card.published`, `data.card.published`, `agent.card.published`, `service.card.declared`, `incident.opened`, `incident.updated`, `incident.resolved`, and `disaster.mode.entered`. Those records feed the attention inbox, memory promotion path, inventory cards (tool, data, agent, and service cards), and incident/quarantine state. Events require stable identifiers, correlation identifiers, component names, operation names, and explicit timestamps. Replay must reconstruct decisions and observations without re-issuing non-idempotent side effects.

SQLite stores durable state for protocol envelopes, approvals, audit records, adapter observations, and replay checkpoints. The architecture should be able to replace the event bus or database later, but V1 governance treats the embedded defaults as the supported base, not a demo mode.

## Approval Model

Mutating operations require an approval gate. Approval requests are first-class records, visible in the dashboard, TUI, and Telegram approval flow where applicable. Approval decisions are auditable events and must be tied to the original correlation id.

## Failure Model

Failures must preserve enough structure to diagnose without exposing secrets: component, operation, correlation id, diagnostic class, and retryability. Transport errors, validation errors, policy denials, adapter failures, and persistence failures are separate classes.
