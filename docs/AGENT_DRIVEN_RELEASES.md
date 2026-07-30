# Agent-Driven Rust Releases

This guide defines the non-secret release architecture for future capability
repositories. Phase A publishes nothing.

## Publication environment

Local Cargo publication is authorized. Use only Cargo's normal
already-configured credential mechanism; never inspect, print, request, paste,
copy, or pass a token as a command-line argument. GitHub Actions trusted
publishing may remain as an optional alternative, but hosted execution and OIDC
are not prerequisites for release progress.

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

The exact live release issue must contain this fenced JSON contract. Human prose
alone is not publication authorization:

```json
{
  "release_authorization": {
    "authorization": "publish",
    "repository": "moritzbrantner/TARGET_REPOSITORY",
    "release_issue": "https://github.com/moritzbrantner/TARGET_REPOSITORY/issues/123",
    "source_sha": "FULL_RELEASE_COMMIT_SHA",
    "default_branch_base_sha": "FULL_BASE_SHA",
    "required_checks": ["cargo package --locked"],
    "packages": [
      {"name": "package-name", "version": "1.2.3"}
    ]
  }
}
```

Run the non-publishing preflight:

```bash
python3 scripts/check_release_plan.py \
  --check path/to/release-plan.json
python3 scripts/release_preflight.py \
  --check path/to/release-plan.json \
  --print-order
```

The preflight never authenticates or publishes. For a publishable plan it reads
the actual local Git head/base and fetches the exact live issue with `gh`; the
plan must exactly match the issue-authorized repository, issue URL, SHAs,
checks, packages, and versions. Caller flags cannot self-assert those values.

## Release pull request and gates

The scoped release PR contains only version bumps, changelog/release notes,
Cargo-produced lockfile changes, metadata, and the exact manifest. Required
local checks, reviews, and threads must be satisfied before ordinary merge.
Hosted workflow state is informational only. Administrator bypass is forbidden.

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
3. If absent, run `cargo package`, inspect its contents, run the authorized
   release and consumer gates, then publish with normal `cargo publish`.
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

## Credential safety

Publication requires an existing authenticated Cargo credential, a clean exact
release checkout, the live issue authorization above, a validated manifest, and
passing exact-head local gates. The credential value is never read back or
logged. Local verification receipts and diagnostics contain commands and exit
codes only, never environment values or registry credentials.
