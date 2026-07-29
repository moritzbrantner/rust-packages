# Agent-Driven Rust Releases

This guide defines the non-secret release architecture for future capability
repositories. Phase A publishes nothing.

## Bootstrap once per repository

1. Create a GitHub `release` environment without required human reviewers when
   repository and organization policy permit. Do not weaken an existing broader
   policy silently.
2. In crates.io, bind the exact owner, repository, workflow filename, and
   `release` environment as a trusted publisher. This binding and the GitHub
   environment cannot be configured from the Phase A checkout.
3. Prefer a small repository-local workflow. The inspected reusable workflow
   `package-publish.yml@workflow-standard-v1.3` uses a long-lived token and is
   not suitable for this OIDC path.
4. Grant only `contents: read` and `id-token: write` during package publication.
   A separate job may receive the minimum write permission needed to create
   verified tags/releases.
5. Use crates.io's official authentication action
   `rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18`
   (release `v1.0.5`) or a newer officially documented immutable revision after
   review. Follow the current
   [crates.io trusted-publishing documentation](https://crates.io/docs/trusted-publishing)
   rather than remembered workflow syntax.

The first crate for a new trusted-publisher identity still needs crates.io's
documented bootstrap path. If no trusted binding exists, the release issue must
record the one-time manual/local bootstrap or human configuration blocker.
Never add a placeholder token or secret.

## Exact release contract

Copy `docs/repository-split/release-plan.example.json`, then record the release
repository, immutable source and default-branch base SHAs, release issue, old
and new versions, required features, compatibility/deprecation packages,
dependency order, registry, owners, checks, tags, consumer checks, and
downstream repositories. Dependency safety is derived from each reviewed Cargo
manifest; a release plan's self-reported dependency data is not trusted.
`publish: false` is allowed for a
reviewed architecture-only example; real release entries use exact bumped
versions and expected tags.

Run the non-publishing preflight:

```bash
python3 scripts/check_release_plan.py \
  --check path/to/release-plan.json \
  --expected-sha "$(git rev-parse HEAD)" \
  --expected-base-sha "$(git merge-base origin/main HEAD)"
python3 scripts/release_preflight.py \
  --check path/to/release-plan.json \
  --expected-sha "$(git rev-parse HEAD)" \
  --expected-base-sha "$(git merge-base origin/main HEAD)" \
  --print-order
```

The preflight never authenticates or publishes. A target repository should copy
the validator with its reviewed ownership data or adopt an equivalent
standard-library implementation.

## Release pull request and gates

The scoped release PR contains only version bumps, changelog/release notes,
Cargo-produced lockfile changes, metadata, and the exact manifest. Required
checks, reviews, and threads must be satisfied before ordinary auto-merge or
merge. Administrator bypass is forbidden.

For each package, select the deterministic subset of:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --no-default-features
cargo doc --workspace --no-deps
cargo deny --workspace --all-features --locked check
cargo package -p PACKAGE --locked
cargo package -p PACKAGE --locked --list
```

The release issue separates deterministic package gates, feature compile gates,
resource-backed integrations, and optional real-model checks. Inspect package
file lists for models, credentials, caches, local output, large fixtures, and
unlicensed data. Never use `--no-verify` or an unexplained `--allow-dirty`.

Candidate consumers use isolated worktrees or temporary patches that are never
committed. Foundation/domain issues select applicable consumers from
`docs/CONSUMER_RELEASE_MATRIX.md`.

## Topological, idempotent publication

For each package in validated order:

1. Query crates.io for the exact name/version.
2. If present, verify metadata and mark the step complete.
3. If absent, run the package gates and publish through trusted OIDC.
4. Poll the crates.io API/index with bounded retries until the exact version is
   resolvable.
5. Package the next dependent against the registry-visible prerequisite.

Never publish dependent crates concurrently. A rerun resumes at the first
unpublished package. If a later package fails, retain published versions,
record the partial state, fix forward, and resume. Do not republish, overwrite,
delete, or automatically yank.

After registry verification, create immutable package tags
`<package-name>-v<version>` (or a documented lockstep workspace tag), then a
GitHub Release listing packages, source commit, compatibility notes, and
downstream issues. Do not attach caches or model artifacts.

Finally create one scoped dependency-update PR per consumer, remove temporary
patches, prove registry-only resolution in a clean checkout, run the documented
consumer gate, and record evidence in the release issue.

## Local fallback

Local `cargo publish` is permitted only when the release issue and manifest
explicitly select it, CI is unavailable, an authenticated Cargo credential
already exists, the checkout is the exact release commit, and every normal gate
passes. Never inspect, print, request, paste, or pass the credential value in a
command argument.
