# AIUX Mission Deck Session Proof

## Milestone 1: Broker Client

Changed paths:
- `apps/cockpit/src/jitux/client.ts`
- `apps/cockpit/src/jitux/client.test.ts`

Commands:
- `npm --workspace @jmcp/cockpit run typecheck`
  - `tsc --noEmit -p tsconfig.json && tsc --noEmit -p tsconfig.node.json`
  - Result: pass
- `npm --workspace @jmcp/cockpit run test`
  - `6 passed (6)`, `62 passed (62)`
  - Result: pass
- `jankurai audit .`
  - `score=76 raw=79 caps=3 findings=8`
  - Result: pass

Notes:
- Installed npm dependencies with `npm ci` because the isolated worktree had no `node_modules` binaries for `tsc` or `vitest`.
- `openDeckSession` posts `{ prompt, source }` to `/jitux/sessions` and validates `{ sessionId, streamUrl, wsUrl }`.
- `subscribeToDeckFrames` resolves relative stream paths through the cockpit API base before opening `EventSource`.
