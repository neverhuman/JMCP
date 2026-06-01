# Install jankurai

Run `jankurai init --profile rust-ts-postgres --ide all --mode advisory --dry-run`, review the plan, then rerun with `--yes`.

For Rust services that want runtime repair packets, an optional `witness-rt` crate can emit packets that feed the Rust witness and diagnose flows.

## Managed Hooks

After install, point Git at the repo-managed hooks path:

```bash
git config core.hooksPath tools/jankurai-hooks
```

That path contains the tracked `pre-commit`, `prepare-commit-msg`, and `pre-push` hooks. The `pre-commit` and `pre-push` hooks compare the current advisory report against `agent/repo-score-baseline.json`.
