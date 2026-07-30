# Agent Instructions

## Project Purpose

This repository is a Rust-first multimodal analysis workspace for video, audio,
image, text, vector, data, math, animation, 3D, and ComfyUI interoperability.

It contains:

* reusable Rust crates under `crates/`
* a root Rust facade crate in `src/lib.rs`
* workspace integration and public API tests under `tests/`
* Bun-managed TypeScript packages under `packages/`
* a Vite prototype app under `prototypes/web/video-analysis-web`
* vendored/reference projects used to compare behavior

The main engineering goal is to keep the Rust crate surfaces composable,
testable, benchmarkable, and usable from frontend/package surfaces without
letting prototypes, generated outputs, or runtime-specific concerns leak into
core crates.

## Operating Model

For non-trivial work, act as a coordinator:

1. Inspect the repository and current git state.
2. Identify the smallest safe scope.
3. Decide whether subagents are useful.
4. Plan before editing.
5. Use one implementation path.
6. Run the narrowest meaningful checks.
7. Repair failures within the repair budget.
8. Summarize changed files, checks, risks, and follow-up PRs.

Always check git state before and after edits:

```bash
git status --short --branch
```

Prefer exact search before broad reading:

```bash
rg --files | sed -n '1,200p'
rg "<symbol-or-text>" crates packages prototypes tests scripts docs
cargo metadata --no-deps
```

Use `cargo metadata --no-deps` whenever workspace membership, crate boundaries,
features, or dependency relationships matter.

## Agent skills

This repository uses GitHub Issues as the source of truth for agent workflow.
Agents should use the configured issue tracker and labels instead of local
markdown task lists for triage, PRDs, assignment, blocking state, and
completion state.

Read the agent setup docs before running workflow skills:

* Issue tracker: [docs/agents/issue-tracker.md](docs/agents/issue-tracker.md)
* Triage labels: [docs/agents/triage-labels.md](docs/agents/triage-labels.md)
* Domain context: [docs/agents/domain.md](docs/agents/domain.md)
* Planning workflow: [docs/agents/planning-workflow.md](docs/agents/planning-workflow.md)

The triage labels in `docs/agents/triage-labels.md` are canonical for this
repo. Domain context starts with `CONTEXT.md` and the ADRs under `docs/adr/`.

### Planning workflow

Substantial new work should be planned into GitHub PRD issues instead of
implemented directly. See `docs/agents/planning-workflow.md`.

## Subagent Policy

Use subagents for independent investigation, comparison, review, or audit work.
Do not use subagents for many agents editing the same files.

### When To Use Subagents

Use subagents when the task involves any of these:

* multiple crates or package surfaces
* dependency conflicts or feature-flag questions
* public API design or crate boundary decisions
* benchmark/reference implementation comparison
* UI plus Rust integration
* release-readiness, preflight, or broad audit work
* applying the Crate Surface Audit Protocol
* reviewing a large or risky diff
* deciding how to split a change into PRs

Do not use subagents for small, obvious, one-file fixes.

### Subagent Roles

Use these roles conceptually, or map them to custom `.codex/agents/*.toml`
agents if available.

#### `repo-explorer`

Read-only. Use for repository mapping.

Responsibilities:

* map relevant files, crates, packages, tests, scripts, and docs
* identify ownership boundaries
* find existing patterns before new code is written
* report exact file paths and symbols
* avoid proposing edits unless asked

#### `crate-surface-auditor`

Read-only unless explicitly promoted to worker by the coordinator.

Responsibilities:

* apply `docs/CRATE_SURFACE_AUDIT_PROTOCOL.md`
* verify public crate operations
* identify missing tests, examples, docs, and UI paths
* distinguish primary workflows from debug/inspection helpers
* recommend the smallest complete crate-surface improvement

When the user says:

```text
Apply the Crate Surface Audit Protocol to <crate-name>
```

follow `docs/CRATE_SURFACE_AUDIT_PROTOCOL.md` exactly.

Audit one crate at a time. Verify every package-surface operation. Make the
primary workflow the default UI path. Move inspection/debug helpers into a Debug
operation tab. Update tests and docs.

#### `dependency-mapper`

Read-only. Use for dependency/version/features work.

Responsibilities:

* inspect `Cargo.toml`, workspace dependencies, feature flags, and lockfile impact
* identify public exposure of dependency types
* find duplicate dependency versions and incompatibilities
* recommend adapter, replacement, feature-gating, or reimplementation options
* avoid dependency additions unless clearly justified

#### `test-planner`

Read-only. Use before or after implementation when verification is unclear.

Responsibilities:

* identify the narrowest meaningful checks
* map changed surfaces to Rust, TypeScript, UI, WASM, e2e, snapshot, or benchmark checks
* distinguish required checks from expensive/optional checks
* recommend fixtures or smoke tests when coverage is missing

#### `rust-reviewer`

Read-only. Use after a diff exists.

Responsibilities:

* review correctness, API stability, error handling, test coverage, feature flags,
  dependency impact, and performance risks
* ignore style-only comments unless they affect maintainability or consistency
* give actionable findings with exact file paths
* recommend whether the change is ready, needs repair, or should be split

#### `implementation-worker`

The only role that should edit files.

Responsibilities:

* implement the smallest scoped change
* avoid unrelated refactors
* preserve public APIs unless the task explicitly requires API changes
* add or update tests with behavior changes
* run narrow checks first
* stop after the repair budget is exhausted

Use only one implementation worker at a time.

### Subagent Coordination Rules

The coordinator must consolidate subagent findings before editing.

Good pattern:

1. Spawn read-only explorers/reviewers for independent areas.
2. Wait for all results.
3. Merge findings into one plan.
4. Assign at most one implementation worker.
5. Run checks.
6. Ask one reviewer to inspect the final diff if the change is risky.

Bad pattern:

* multiple workers editing overlapping files
* subagents making broad speculative rewrites
* subagents adding dependencies independently
* reviewers blocking on subjective style comments
* recursive subagent delegation without a clear reason

### Required Subagent Output

Each subagent should return:

* scope inspected
* relevant files/symbols
* findings
* recommended action
* risks or unknowns
* suggested checks

For audit/review work, also return whether the change should be:

* one PR
* split into multiple PRs
* blocked until more information is available

## Planning And Repair Loop

For non-trivial work, write a short plan before editing.

Default loop:

1. Explore relevant files.
2. Plan the smallest safe change.
3. Edit.
4. Run narrow checks.
5. Fix failures.
6. Rerun checks.
7. Stop after at most 3 repair cycles.
8. Handoff with a clear summary.

A repair cycle is one attempt to fix a failed check and rerun it.

If the same failure remains after 3 repair cycles, stop and report:

* failing command
* relevant error output
* likely cause
* files already changed
* suggested next step

Do not hide failing checks.

## Ownership Boundaries

### Rust

* `crates/`: reusable Rust crates.

  * Keep library logic composable.
  * Avoid app/runtime concerns in core crates.
  * Prefer clear feature gates over unconditional heavy dependencies.
  * Avoid exposing third-party dependency types in public APIs unless intentional.

* `src/lib.rs`: root `video-analysis` facade crate.

  * Keep facade exports deliberate.
  * Do not turn the facade into a dumping ground for app logic.

* `tests/`: workspace integration and public API smoke tests.

  * Use for cross-crate behavior and public API verification.
  * Keep fixtures small and intentional.

### Frontend And Packages

* `packages/`: reusable frontend packages.

  * Build outputs are not checked in.
  * Regenerate `packages/video-analysis-ui/dist/` with `bun run ui:build`.

* `packages/text-core-wasm/`: WASM package generated through `wasm-pack`.

  * `packages/text-core-wasm/pkg/` is generated output.
  * Regenerate through the package build flow.

* `prototypes/`: runnable experiments and local app surfaces.

  * The web app lives in `prototypes/web/video-analysis-web`.
  * Prototype code may exercise package surfaces but should not define core crate architecture.

### References And Vendor Trees

* `references/pyscenedetect/`: vendored upstream behavior reference.

  * Treat as read-only unless the task explicitly asks to sync or document it.

* `vendor/whisper.cpp/`: vendored runtime source.

  * Treat as read-only unless the task explicitly targets the vendor tree.

### Data, Fixtures, And Outputs

* `use-case-output/`: small reviewed report snapshots only.

  * Large generated media outputs are local artifacts and ignored.

* `tests/fixtures/`: checked-in fixtures.

  * Keep fixtures small and intentional.
  * Do not refresh fixtures unless the requested behavior requires it.

## Commands

### Install Prerequisites

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked --version 0.14.0
bun install
```

### Standard Root Commands

```bash
bun run dev              # start the Vite prototype app
bun run test             # fastest meaningful Rust + frontend unit/API tests
bun run lint             # clippy and TypeScript type checks
bun run format           # write Rust formatting changes
bun run format:check     # check Rust formatting without mutation
bun run build            # Rust workspace build plus UI/web production builds
bun run preflight        # broad local CI/preflight mirror
bun run verify           # full local baseline: scripts/check.sh
bun run hygiene          # lightweight repo status and ignore audit
bun run hygiene:generated # fail on tracked generated/local artifacts
bun run snapshot:check   # verify reviewed generated docs are fresh
```

### Lower-Level Checks

```bash
scripts/check-fast.sh
scripts/check-preflight.sh
scripts/check.sh
scripts/check_e2e_external_tools.sh
cargo doc --workspace --no-deps
```

Use `scripts/check-fast.sh` as the normal changed-aware local iteration gate.

Repository capability boundaries and future release plans are checked with:

```bash
python3 scripts/generate_repository_split_inventory.py --check
python3 scripts/check_repository_boundaries.py --check
python3 scripts/check_release_plan.py --check docs/repository-split/release-plan.example.json
python3 scripts/release_preflight.py --check docs/repository-split/release-plan.example.json --print-order
```

The release preflight validates and prints order only; it never publishes.

Use `scripts/check-preflight.sh` before PR/release-oriented handoff or when
touching UI routing, builds, e2e behavior, or cross-surface integration.

Use `scripts/check.sh` before full release-oriented handoff or when the task
needs external-tool coverage.

For big changes, run `scripts/check-preflight.sh` and store exact-head local
verification evidence before handoff. GitHub Actions state is informational and
is not a readiness gate.

Benchmark checks belong to `bun run bench`, `performance-ci`, or explicit
benchmark commands. Do not add benchmarks to the default fast local gate unless
the task explicitly asks for it.

Release and publish work is authorized only by an exact release issue and
validated manifest. Use:

```bash
docs/RELEASE_CHECKLIST.md
docs/AGENT_DRIVEN_RELEASES.md
```

Agents may automate or publish only the exact packages and versions authorized
by that release contract.

## Check Selection

Prefer the narrowest check that proves the changed surface.

### Rust crate-only changes

Start with:

```bash
cargo check -p <crate>
cargo test -p <crate>
```

Then run broader checks if the change affects public APIs, shared utilities, or
workspace-level behavior.

### Workspace dependency or feature changes

Use:

```bash
cargo metadata --no-deps
cargo check --workspace
bun run lint
```

Also regenerate dependency docs if crate membership or internal dependencies
changed.

### Frontend/package changes

Use the relevant package checks first, then:

```bash
bun run test
bun run lint
```

Run production builds only when the change affects bundling, routing, generated
package output, or release readiness.

### WASM changes

Check the Rust crate, the wasm-pack flow, and any package surface depending on
the generated WASM package.

### UI prototype changes

Use the Vite/prototype checks. Run preflight when routing, build behavior, or
browser behavior changes.

### Documentation-only changes

Run only formatting, link/snapshot checks, or docs generation when relevant.
Do not run expensive build/test suites for pure documentation edits unless the
docs include generated snapshots or command examples that must be verified.

## Generated And Local Files

Do not manually edit generated or local-only directories:

* `target/`
* `.cargo-target/`
* `node_modules/`
* `.external-test-tools/`
* `.audio-tools/`
* `.model-runtime/`
* `.test-corpora/`
* `coverage/`
* Playwright reports
* test-result folders

Build outputs are not checked in. This includes:

* `packages/video-analysis-ui/dist/`
* `packages/*-wasm/pkg/`
* `prototypes/web/video-analysis-web/dist/`

Generated local app data such as this is regenerated by dev/build scripts and
ignored:

```text
prototypes/web/video-analysis-web/public/workspace-architecture.json
```

Reviewed generated documentation snapshots must pass:

```bash
bun run snapshot:check
```

The allowlist lives in:

```text
scripts/generated_snapshots.allow
```

Generated/local artifact exceptions live in:

```text
scripts/generated_artifacts.allow
```

Regenerate dependency docs after workspace crate membership or internal
dependency changes:

```bash
python3 scripts/generate_dependency_chart.py
```

## Expensive Or Opt-In Work

`scripts/check-fast.sh` is the normal changed-aware local iteration gate. It
intentionally skips browser e2e, production web builds, and benchmarks.

Force broader checks with:

```bash
CHECK_FAST_SCOPE=workspace
CHECK_FAST_FRONTEND=all
CHECK_FAST_PROGRESS=all
```

`scripts/check-preflight.sh` is the broad local CI/preflight mirror. Run it
before PR/release-oriented handoff or when touching UI routing, build, e2e, or
cross-surface behavior.

`scripts/check.sh` includes external-tool coverage. Run it before full
release-oriented handoff or when the task needs those checks.

`scripts/check-e2e.sh` requires local external tools. Run setup only when the
task needs those checks:

```bash
scripts/setup_e2e_external_tools.sh fast
```

`scripts/check-e2e-slow.sh` is reserved for dedicated radiance/Nerfstudio
runners.

Feature flags named `external-tests` require real tools, models, or network
access and are outside the default contributor gate.

## Dependency Policy

Before adding a dependency, check whether an existing crate or package already
solves the problem.

For Rust dependencies:

* prefer workspace-level dependency declarations when shared
* avoid duplicate versions unless unavoidable
* avoid exposing third-party types in public APIs unless that is the crate's purpose
* document why new heavy dependencies are needed
* feature-gate optional integrations
* keep core crates independent from app/runtime dependencies

For TypeScript dependencies:

* avoid adding frontend dependencies for prototype-only convenience unless they
  belong to the package surface
* keep generated package outputs out of source control
* verify type-checking and build behavior when package exports change

When dependency conflicts appear, prefer this order:

1. adapter boundary
2. feature-gating
3. dependency replacement
4. small internal reimplementation
5. larger architecture change

Use a dependency-mapping subagent before choosing options 3-5.

## Benchmark And Reference Policy

Benchmarks and reference projects should support crate quality without bloating
core crate APIs.

* Keep benchmark harnesses separate from core library logic.
* Keep reference implementations read-only unless explicitly updating them.
* Use references to define expected behavior, not as runtime dependencies.
* Do not require large media, models, or external tools for the default local gate.
* Put expensive or external benchmark runs behind explicit commands.

When comparing with reference implementations, report:

* behavior being compared
* input fixture or scenario
* relevant upstream behavior
* local behavior
* gaps or deliberate differences

## PR-Sized Change Policy

Prefer small, reviewable changes.

Split work when it combines:

* dependency changes plus behavior changes
* public API changes plus UI integration
* benchmark infrastructure plus algorithm changes
* generated snapshots plus unrelated code edits
* multiple crates with independent purposes
* refactoring plus feature work

At handoff, recommend a PR split if the diff is too broad.

A good handoff includes:

* changed files
* why they changed
* commands run
* passing/failing checks
* risks
* follow-up PRs

## Crate Surface Audit Protocol

When the user says:

```text
Apply the Crate Surface Audit Protocol to <crate-name>
```

follow:

```text
docs/CRATE_SURFACE_AUDIT_PROTOCOL.md
```

Rules:

* audit one crate at a time
* verify every package-surface operation
* make the primary workflow the default UI path
* move inspection/debug helpers into a Debug operation tab
* update tests and docs
* avoid broad unrelated refactors
* finish with a crate-specific summary and follow-up list

Use subagents for this when useful:

* `repo-explorer` to map the crate and package surfaces
* `crate-surface-auditor` to apply the protocol
* `test-planner` to identify checks
* `rust-reviewer` to review the final diff

Only one implementation worker should edit files.

## Final Handoff Format

Every non-trivial task should end with:

```text
Summary:
- ...

Changed files:
- ...

Checks run:
- ...

Results:
- ...

Risks / unresolved issues:
- ...

Suggested follow-up:
- ...
```

If no files changed, say so explicitly.

If checks were not run, explain why.

If generated files were intentionally not refreshed, say so and give the command
the user should run.
