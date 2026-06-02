# Visual Review

Rendered UX evidence for the JMCP cockpit proof host lives here:

- `apps-web-loading.png`
- `apps-web-empty.png`
- `apps-web-error.png`
- `apps-web-permission-denied.png`
- `apps-web-success.png`

Review notes:

- the state switcher is visible before interaction;
- each required proof state is captured as a full-frame screenshot;
- the success state reuses the live cockpit UI instead of a separate mock shell;
- aria snapshots for the proof states live in `apps/web/tests/rendered-ux.spec.ts-snapshots/`.
