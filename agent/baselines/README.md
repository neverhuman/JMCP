# Jankurai ratchet baseline

The accepted **floor** is `agent/repo-score-baseline.json` (a compact summary —
`{score, raw_score, caps_applied, rule_counts}` — shared with the CI audit lane).
New commits may not regress below it: a lower score, a newly-applied cap, or more
total findings is rejected.

It is seeded/bumped — never lowered — via:

```sh
ops/ci/jankurai-ratchet.sh --accept   # re-audit, accept the current (improved) result as the floor
git config core.hooksPath ops/git-hooks   # activate the local pre-commit/pre-push hooks
```

`ops/ci/jankurai-ratchet.sh` (run by `ops/git-hooks/{pre-commit,pre-push}` and the
CI ratchet step) re-audits and exits non-zero on any regression versus the floor.
It reads the compact summary format above; do **not** point `jankurai audit
--baseline` at it (that flag needs a full native report with `report_fingerprint`).

The floor is bumped to a stable, improved committed state as the jankurai
remediation lands, so it reflects real conformance — never a mid-refactor working
tree. While the floor still carries a cap, the ratchet reports it as a regression
only if a *new* cap appears or the score/finding totals worsen.
