# Fork CI policy

`antoninomariolacommare/evolutionary-world` runs quality checks without publishing
upstream releases, installers or websites. Keep the exact repository guards when
syncing workflows from upstream; missing secrets are not the deployment guard.

## Retained checks

`pipeline.yml` retains pull requests targeting `main`, pushes to `main`, and manual
runs. Rust formatting, Clippy, tests/doctests, performance checks, native/WASM
fingerprints and determinism, frontend lint/tests/build, and desktop TypeScript
compilation/tests keep their existing commands and matrices. Source-key caching
and recording of successful checks remain enabled.

The main-branch validation gate still checks required results, including successful
cached test runs. On forks the frontend is built even when a previous production
bundle exists, so skipping `ship-web` cannot leave the gate without a frontend
result. The upstream `[skip-bump]` marker no longer suppresses fork checks.

## Publishing disabled on forks

The following jobs require `github.repository == '1homsi/thehumanbox'`:

- `bump-version` and `publish-release` (version commits and desktop releases).
- `ship-web` and `deploy-frontend` (both Cloudflare deployment paths).
- `desktop-release.yml`'s `build` job (installer matrix, including manual dispatch).

The four pipeline publishing jobs also explicitly require a push to `main`.
Workflow defaults grant only `contents: read` and `actions: read`; publishing
permissions remain scoped to the guarded jobs. Quality checks use the automatic
GitHub token and require no upstream PAT, Cloudflare credentials or analytics key.
Fork frontend builds receive an empty analytics key and can upload validation
artifacts without one.

`cache-cleanup.yml` is unchanged: it deletes PR caches on PR closure using the
automatic token's `actions: write` permission. It does not check out PR code or
publish releases/sites.

## Validation performed for this change

- `actionlint` 1.7.12: all three workflows pass static validation.
- GitHub `@actions/expressions` 0.3.61: explicit conditions pass nine combinations
  of repository (this fork, upstream, another fork) and event (PR, main push,
  manual dispatch), including skipped deployment, stale bundle flags, missing
  secrets and the inherited skip marker.
- Extracted Bash steps pass six cached-bundle scenarios, six test-selection
  scenarios and twelve gate scenarios. A skipped deployment with successful
  checks passes; failed or missing required checks fail the gate. GitHub responses
  and job results are fixtures in these local checks.
- Quality commands/matrices are unchanged; `git diff --check` passes.

These are local static/extracted-script checks, not execution of the full workflow
on GitHub runners. Rust and desktop suites were not repeated for workflow-only
changes. Actual GitHub run status is reported separately in the PR.
