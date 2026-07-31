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

## Routing Label

| Label | Meaning |
| --- | --- |
| `prd` | Product requirements document ready for workflow routing |

The `prd` label classifies planning issues; it is not operational state.

## Agent Loop Operational Labels

The complete operational label set is:

| Label | Meaning |
| --- | --- |
| `ready-for-agent` | Fully specified and available for the agent loop |
| `agent-loop:active` | Work is active in the agent loop |
| `agent-loop:blocked` | Blocked on human input or external access |
| `agent-loop:ready-to-merge` | Worker reports the PR is ready to merge |

Closed issues and merged pull requests record completion through native GitHub
state. Operational labels describe only transient queue state.
