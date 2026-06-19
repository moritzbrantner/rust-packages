# Agent Triage Labels

GitHub Issues labels are the canonical workflow state for this repository.

## Canonical Roles

The five canonical triage roles map exactly to these labels:

| Role | Label | Meaning |
| --- | --- | --- |
| Needs triage | `needs-triage` | Needs review before assignment |
| Needs info | `needs-info` | Waiting on reporter for more information |
| Ready for agent | `ready-for-agent` | Fully specified, ready for an AFK agent |
| Ready for human | `ready-for-human` | Requires human implementation |
| Won't fix | `wontfix` | This will not be worked on |

## Additional Workflow Labels

| Label | Meaning |
| --- | --- |
| `prd` | Product requirements document ready for workflow routing |
| `agent-loop:claimed` | Claimed by the agent-loop master |
| `agent-loop:in-progress` | Work is active in an agent-loop worker |
| `agent-loop:blocked` | Blocked on human input or external access |
| `agent-loop:ready-to-merge` | Worker reports the PR is ready to merge |
| `agent-loop:merged` | Associated PR has been merged |
| `agent-loop:done` | Agent-loop work is complete |
| `agent-loop:failed` | Automation failed and needs review |
