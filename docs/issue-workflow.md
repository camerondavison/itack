# itack Workflow

How to work on issues using itack.

## 1. View available issues

```bash
itack list
```

Or see the board overview:

```bash
itack board
```

## 2. Pick an issue

View details of a specific issue:

```bash
itack show <id>
```

## 3. Claim the issue

Assign it to yourself:

```bash
itack claim <id> <your-name>
```

This marks the issue as in-progress and locks it so others can't claim it.

## 4. Create a worktree (optional)

For isolated work, create a git worktree:

```bash
git worktree add ../issue-<id>-<short-description> -b issue-<id>-<short-description>
```

Then work in that directory.

## 5. Work on it

Make your changes. The issue file is at `.itack/<date>-issue-<id>.md` if you need to add notes.

## 6. Mark it done

```bash
itack done <id>
```

## 7. Commit your changes

```bash
git add <changed-files>
git commit -m "Description of changes"
```

## 8. Merge and clean up worktree (if using worktree)

From the main worktree, merge your branch and remove the worktree:

```bash
cd ../main
git merge issue-<id>-<short-description>
git worktree remove ../issue-<id>-<short-description>
```

## Other commands

Release a claim without completing:

```bash
itack release <id>
```

Create a new issue:

```bash
itack create "Issue title"
```

Edit an issue in your editor:

```bash
itack edit <id>
```
