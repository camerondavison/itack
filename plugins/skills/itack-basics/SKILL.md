---
name: itack
description: >
  Git-backed issue tracker for multi-agent coordination.
  Use when working with itack issues to:
    (1) View the project board and list issues
    (2) Create new issues with titles and descriptions
    (3) Claim issues to work on them
    (4) Mark issues as done when completed
    (5) Release claimed issues if needed
    (6) Check database health with doctor command
    (7) Search issues by query.
allowed-tools: Bash(itack *)
---

# itack CLI

Git-backed issue tracker for multi-agent coordination.

## Commands

### View Issues

```bash
# Show project board overview
itack board

# List all issues
itack list

# List issues filtered by status
itack list --status open
itack list --status in-progress
itack list --status done
itack list --status wont-fix

# Show details of a specific issue
itack show <id>

# Search issues by title or body
itack search <query>

# Search across all git branches
itack search <query> --all-branches
```

### Create Issues

Issues are automatically committed to git when created.

```bash
# Create a new issue
itack create "Issue title"

# Create with an epic/category
itack create "Issue title" --epic "epic-name"

# Create with a body/description
itack create "Issue title" --body "Detailed description"

# Create with a custom commit message
itack create "Issue title" --message "Custom commit message"
```

### Work on Issues

```bash
# Claim an issue (marks as in-progress)
itack claim <id> <assignee-name>

# Claim with a session ID (e.g., Claude Code session)
itack claim <id> <assignee-name> --session <session-id>

# Mark an issue as done
itack done <id>

# Release a claimed issue without completing
itack release <id>
```

### Edit Issues

Issues are automatically committed to git after editing.

```bash
# Open issue in editor
itack edit <id>

# Edit with a custom commit message
itack edit <id> --message "Custom commit message"
```

### Diagnose Issues

```bash
# Check database health and issue synchronization
itack doctor
```

## Workflow

1. Run `itack board` to see available issues
2. Run `itack show <id>` to view issue details
3. Run `itack claim <id> <name>` to claim an issue
4. Work on the issue
5. Run `itack done <id>` when complete (commit separately with your implementation changes)

## Important: One Issue at a Time

Only claim and work on one issue at a time. If an issue is not claimed, assume someone else is working on it or will pick it up. Do not claim multiple issues simultaneously.

## Configuration

Global configuration is stored in `~/.itack/config.toml`:

```toml
# Default assignee name for claims
default_assignee = "my-name"

# Editor command (defaults to $EDITOR or vi)
editor = "code --wait"

# Branch where itack stores issue data (default: "data/itack")
data_branch = "data/itack"

# Branch to merge data into after commits (default: none)
# Set to "main" to merge into main after each issue change
merge_branch = "main"
```

### Data Branch Behavior

- Issues are always committed to `data_branch` (default: `data/itack`)
- If `merge_branch` is set (default: "main"), changes are merged into that branch after each commit, and files are written to the working directory
- Set `merge_branch = ""` (empty string) for data-only mode, where files are only stored in the data branch, keeping the working directory clean
