# Event Contracts

This directory holds the generated event-contract surface for JMCP and JCP/1.0.0 traffic.

The canonical source lives in `schemas/jcp/1.0.0/jcp.schema.json`. The copy in this directory is treated as generated contract output and is verified by `just contract-drift`.

## Current Sources

- `jcp-envelope.schema.json`: the shared JCP/1.0.0 envelope shape used before shared routing, approval, audit, and replay.
