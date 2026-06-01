# Event Contracts

This directory holds authored event-contract sources for JMCP and JCP/1.0.0 traffic.

Keep event envelopes and event payload definitions here so boundary checks can trace the source of truth without relying on generated clients or handwritten transport glue.

## Current Sources

- `jcp-envelope.schema.json`: the shared JCP/1.0.0 envelope shape used before shared routing, approval, audit, and replay.

