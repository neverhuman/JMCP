# Jankurai Adapter Standard

## Purpose

Jankurai is a local JMCP adapter class. In V1 it must behave as a local, inspectable, replay-friendly component that participates in JCP/1.0.0 through the JPCM profile without requiring network availability or external service state.

## Required Properties

- **Local first:** Default execution must work on a developer machine with embedded SQLite and the in-process event bus.
- **Protocol bounded:** Adapter messages must be represented as JCP/1.0.0 envelopes before they enter shared routing, approval, or audit paths.
- **Replayable:** Inputs, decisions, side effects, and emitted events must have stable identifiers suitable for event-bus replay.
- **Approval aware:** Any operation that can mutate durable state, call an external endpoint, or release user-visible output must expose an approval decision point.
- **Deterministic tests:** Unit and integration tests must not rely on wall-clock timing, remote services, or ordering not guaranteed by JPCM.
- **Auditable errors:** Errors must preserve adapter name, operation, correlation id, and non-secret diagnostic class.

## Interface Expectations

Each local adapter should document:

- supported JCP/1.0.0 message types;
- JPCM transport assumptions;
- persistence tables or event streams used;
- approval gates;
- replay behavior;
- failure classes;
- test fixtures.

## Security Boundary

Jankurai must not become a privileged bypass around JMCP governance. It may request actions, emit events, and provide local computation, but policy, approval, replay, and audit remain JMCP-owned concerns.

