# Screenshot Evidence

The rendered UX lane keeps reviewable, committed screenshot artifacts here so the proof is not limited to an executable test.

## Committed Artifacts

- `jmcp-cockpit-dashboard.png`
- `jmcp-tui-dashboard.png`
- `apps-web-loading.png`
- `apps-web-empty.png`
- `apps-web-error.png`
- `apps-web-permission-denied.png`
- `apps-web-success.png`

## Live Proof Host

The executable UX proof host lives in `apps/web/`. Its Playwright lane writes fresh artifacts under `target/jankurai/ux-qa/`, while this directory keeps the curated screenshots that help reviewers inspect the intended shape of the cockpit.

The Playwright lane also keeps aria snapshots under `apps/web/tests/rendered-ux.spec.ts-snapshots/` so the rendered states remain reviewable as both pixels and accessibility trees.
