# Jankurai ratchet baseline

`main.repo-score.json` is the **accepted floor**: the `jankurai audit` result that
new commits may not regress below. It is seeded/bumped — never lowered — via:

```sh
ops/ci/jankurai-ratchet.sh --accept   # re-audit a clean tree and accept it as the floor
```

The ratchet (`ops/ci/jankurai-ratchet.sh`, run by `ops/git-hooks/{pre-commit,pre-push}`
and the CI ratchet job) fails any change that lowers the final score, adds a new
applied cap, or raises the hard-finding count versus this file. Activate the local
hooks with `git config core.hooksPath ops/git-hooks`.

Seeding is intentionally deferred until the in-flight jankurai remediation reaches a
stable, improved committed state, so the floor reflects real conformance — not a
mid-refactor working tree. Until `main.repo-score.json` exists the ratchet no-ops
(it cannot regress against an absent floor); the CI job seeds it on first green main.
