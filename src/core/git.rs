//! Git operations for itack.

use std::path::Path;

use git2::{Repository, Signature};

use crate::error::Result;

/// Commit a file with the given message.
///
/// This uses the git2 crate to add and commit the file.
/// Returns Ok(()) even if there are no changes to commit.
pub fn commit_file(path: &Path, message: &str) -> Result<()> {
    // Open the repository by discovering it from the file path
    let repo = Repository::discover(path)?;

    // Get the path relative to the repo root
    let repo_workdir = repo
        .workdir()
        .ok_or_else(|| git2::Error::from_str("Cannot commit in a bare repository"))?;
    let relative_path = path.strip_prefix(repo_workdir).unwrap_or(path);

    // Stage the file
    let mut index = repo.index()?;
    index.add_path(relative_path)?;
    index.write()?;

    // Check if there are staged changes by comparing index to HEAD
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(e.into()),
    };

    let diff = repo.diff_tree_to_index(head_tree.as_ref(), Some(&index), None)?;
    if diff.deltas().count() == 0 {
        // No changes to commit
        return Ok(());
    }

    // Write the index as a tree
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Get the signature from git config
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("itack", "itack@localhost"))?;

    // Get the parent commit (if any)
    let parent = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(e.into()),
    };

    // Create the commit
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )?;

    Ok(())
}
