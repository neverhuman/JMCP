# JMCP V1 Protocol Decisions

## Naming Decision

JMCP is the system. JCP/1.0.0 is the protocol. JPCM is the backbone and transport profile. Documentation, tests, and paper claims should use these names consistently.

## Envelope Decision

Shared runtime messages use JCP/1.0.0 envelopes before entering routing, approval, audit, replay, or adapter execution paths. Envelopes must carry message type, version, stable id, correlation id, source component, target component or capability, timestamp, and payload classification.

## Transport Profile Decision

JPCM is the V1 transport profile. In V1 it is implemented locally through the in-process replayable event bus by default. The profile defines ordering assumptions, delivery expectations, replay metadata, and idempotency requirements independently from any future remote transport.

## Persistence Decision

Embedded SQLite is the V1 durable store. It records protocol envelopes, approvals, audit evidence, replay checkpoints, and adapter observations. This makes the local default inspectable and reproducible.

## Approval Decision

Approval is protocol-visible. Mutating or externally visible operations must include approval state in their JCP/JPCM flow. Approval decisions are not UI-only state.

## Compatibility Decision

JCP/1.0.0 is a stable V1 compatibility target. Breaking changes require a new protocol version, a migration note, and tests proving old-version rejection or compatibility behavior.

