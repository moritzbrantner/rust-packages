# Planning Workflow

This repository uses the canonical agent-loop setup for generic planning
workflow rules. See
`~/.codex/skills/moenarch-setup-agent-loop-skills/planning-workflow.md`.

Repo-specific facts stay in `docs/agents/issue-tracker.md`, `docs/agents/triage-labels.md`, and `docs/agents/domain.md`.

Summary:

- GitHub Issues are the durable work queue.
- Substantial future work should default to a GitHub PRD issue instead of direct implementation.
- PRD issues must be labeled `prd` and `ready-for-agent` only when they include acceptance criteria and out-of-scope boundaries.
- Implementation slice issues must begin with YAML frontmatter containing
  `parent`, `blocked_by`, and `scope` before they receive `ready-for-agent`.
  Keep `blocked_by` present as an empty list when the slice has no blockers,
  and list the complete write boundary under `scope`.
- The planning thread should stop after creating the PRD issue unless the user explicitly asks for direct implementation.
- The planning thread should not create implementation slice issues by default.
- The agent-loop handles slicing and routing after the PRD is ready.
- Tiny one-shot changes may be implemented directly.
- Explicit user direction to implement directly wins over the default.
