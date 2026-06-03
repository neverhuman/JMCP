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

## Milestone 2: Deck Live Session Lifecycle

Changed paths:
- `apps/cockpit/src/jitux/store.ts`
- `apps/cockpit/src/jitux/components/NowCommandDeck.tsx`
- `apps/cockpit/src/jitux/session-channel.ts`
- `apps/cockpit/src/styles-deck.css`
- `apps/cockpit/src/jitux/components/NowCommandDeck.test.tsx`

Commands:
- `npm --workspace @jmcp/cockpit run typecheck`
  - `tsc --noEmit -p tsconfig.json && tsc --noEmit -p tsconfig.node.json`
  - Result: pass
- `npm --workspace @jmcp/cockpit run test`
  - `6 passed (6)`, `68 passed (68)`
  - Result: pass
- `jankurai audit .`
  - `score=92 raw=92 caps=0 findings=1`
  - Result: pass

Notes:
- `deckStore.igniteQueueBlockers(runtime)` paints a labeled cached snapshot immediately.
- `NowCommandDeck` starts a broker session only while the active deck is mounted.
- Live broker frames stream through `subscribeToDeckFrames(streamUrl, frame => store.applyFrames([frame]))` behavior inside the store reducer path.
- Session open and first-frame stream errors retain the cached snapshot and update the caption/trace as degraded.
- Deactivation aborts pending session open; barge-in closes the active stream through `deckStore.stopLiveQueueBlockers("barge_in")`.
- Mobile deck clearance is deck-side only: `.command-deck[data-mobile-clearance="voice-bar"]` reserves bottom space with `safe-area-inset-bottom`.
- `session-channel.ts` publishes `{ sessionId, streamUrl }` when the deck opens a broker session. No voice bridge files consume it in this pass.
- The live-session controller and trace shaping live in `session-channel.ts`, keeping `store.ts` below the audit size threshold while preserving the deck store API.
