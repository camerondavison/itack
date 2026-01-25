---
name: itack
description: >
  Git-backed issue tracker for multi-agent coordination.
  Use when working with itack issues to:
    (1) View the project board and list issues
    (2) Create new issues with titles and descriptions
    (3) Claim issues to work on them
    (4) Mark issues as done when completed
    (5) Release claimed issues if needed.
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

# Show details of a specific issue
itack show <id>
```

### Create Issues

```bash
# Create a new issue
itack create "Issue title"

# Create with an epic/category
itack create "Issue title" --epic "epic-name"

# Create with a body/description
itack create "Issue title" --body "Detailed description"
```

### Work on Issues

```bash
# Claim an issue (marks as in-progress)
itack claim <id> <assignee-name>

# Mark an issue as done
itack done <id>

# Release a claimed issue without completing
itack release <id>
```

### Edit Issues

```bash
# Open issue in editor
itack edit <id>
```

## Workflow

1. Run `itack board` to see available issues
2. Run `itack show <id>` to view issue details
3. Run `itack claim <id> <name>` to claim an issue
4. Work on the issue
5. Run `itack done <id>` when complete
6. Commit your changes with git
