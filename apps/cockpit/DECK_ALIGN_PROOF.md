# AIUX Mission Deck Alignment Proof

## Scope

- Work stayed under `apps/cockpit`.
- Voice assistant files were not edited.
- The deck presentation uses canonical `jitux/types.ts`, `jitux/guards.ts`, and `jitux/reducer.ts`.
- No copied `protocol.ts`, `protocol-primitives.ts`, or duplicate deck reducer was added.

## Changed Paths

- `apps/cockpit/src/App.tsx`
- `apps/cockpit/src/App.test.tsx`
- `apps/cockpit/src/views.tsx`
- `apps/cockpit/src/styles.css`
- `apps/cockpit/src/styles-base.css`
- `apps/cockpit/src/styles-deck.css`
- `apps/cockpit/src/jitux/client.ts`
- `apps/cockpit/src/jitux/scheduler.ts`
- `apps/cockpit/src/jitux/store.ts`
- `apps/cockpit/src/jitux/layout/deck.ts`
- `apps/cockpit/src/jitux/layout/flip.ts`
- `apps/cockpit/src/jitux/components/AnswerCaptionStream.tsx`
- `apps/cockpit/src/jitux/components/DataLoom.tsx`
- `apps/cockpit/src/jitux/components/DeckCardView.tsx`
- `apps/cockpit/src/jitux/components/DeckViewport.tsx`
- `apps/cockpit/src/jitux/components/EvidenceRibbon.tsx`
- `apps/cockpit/src/jitux/components/FocusPane.tsx`
- `apps/cockpit/src/jitux/components/NowCommandDeck.test.tsx`
- `apps/cockpit/src/jitux/components/NowCommandDeck.tsx`
- `apps/cockpit/src/jitux/components/PreparedActionRail.tsx`
- `apps/cockpit/src/jitux/components/TraceRibbon.tsx`

## Verification

- `npm --workspace @jmcp/cockpit run typecheck`
  - Result: pass
  - Output: `tsc --noEmit -p tsconfig.json && tsc --noEmit -p tsconfig.node.json`
- `npm --workspace @jmcp/cockpit run test`
  - Result: pass
  - Output: `Test Files 5 passed (5); Tests 56 passed (56)`
- `jankurai audit .`
  - Result: pass
  - Output: `score=92 raw=92 caps=0 findings=1`

## Runtime Alignment

- `createQueueBlockerFrames` emits canonical `JituxFrame` values only.
- `deckStore.applyFrames` and `deckStore.dispatch` reduce frames through `reduceJituxFrame`.
- Render tests assert guard acceptance, guard rejection, ranked order, LOD states, reduced-motion list mode, and Now rail purple takeover.
- Prepared actions render canonical `PreparedAction.safety`, `ready`, and `requiresApproval`.
- Evidence renders canonical `EvidenceRef` fields.
