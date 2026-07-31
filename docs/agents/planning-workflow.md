# Planning Workflow

This repository defines its agent-loop planning contract below. Use
`moenarch-setup-agent-loop-skills` to initialize or repair the repository's
Agent Loop setup; do not depend on a user-local skill path for planning policy.

Repo-specific facts stay in `docs/agents/issue-tracker.md`, `docs/agents/triage-labels.md`, and `docs/agents/domain.md`.

Summary:

- GitHub Issues are the durable work queue.
- Substantial future work should default to a GitHub PRD issue instead of direct implementation.
- PRD issues must be labeled `prd` and `ready-for-agent` only when they include acceptance criteria and out-of-scope boundaries.
- Implementation slice issues must begin with canonical YAML frontmatter:

  ```yaml
  ---
  parent: 123
  blocked_by: []
  scope:
    - crates/example/**
  ---
  ```

  `parent` is the parent PRD issue number, `blocked_by` is the complete list of
  blocking issue numbers (or an empty list), and `scope` is the complete worker
  write boundary. All three keys are required before a slice receives
  `ready-for-agent`.
- The planning thread should stop after creating the PRD issue unless the user explicitly asks for direct implementation.
- The planning thread should not create implementation slice issues by default.
- The agent-loop handles slicing and routing after the PRD is ready.
- Tiny one-shot changes may be implemented directly.
- Explicit user direction to implement directly wins over the default.
