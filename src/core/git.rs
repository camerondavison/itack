//! Git operations for itack.

use std::path::Path;

use git2::{FileMode, Oid, Repository, Signature};

use crate::error::{ItackError, Result};

/// Commit a file to a specific branch without checking it out.
/// Creates the branch as orphan if it doesn't exist.
/// Returns the commit OID if a commit was created, None if no changes.
pub fn commit_to_branch(
    repo_path: &Path,
    branch_name: &str,
    file_path: &Path,
    content: &[u8],
    message: &str,
) -> Result<Option<Oid>> {
    let repo = Repository::discover(repo_path)?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("itack", "itack@localhost"))?;

    // Convert file_path to a string for the tree
    let file_path_str = file_path.to_string_lossy();

    // Write the blob
    let blob_oid = repo.blob(content)?;

    // Try to find the branch
    let branch_ref = format!("refs/heads/{}", branch_name);
    let parent_commit = match repo.find_reference(&branch_ref) {
        Ok(reference) => Some(reference.peel_to_commit()?),
        Err(e) if e.code() == git2::ErrorCode::NotFound => None,
        Err(e) => return Err(e.into()),
    };

    // Build the new tree
    let new_tree = if let Some(ref parent) = parent_commit {
        // Update existing tree
        let parent_tree = parent.tree()?;
        let mut builder = repo.treebuilder(Some(&parent_tree))?;

        // Handle nested paths by building tree hierarchy
        build_nested_tree(&repo, &mut builder, &file_path_str, blob_oid)?;

        let tree_oid = builder.write()?;
        repo.find_tree(tree_oid)?
    } else {
        // Create new tree from scratch
        let mut builder = repo.treebuilder(None)?;
        build_nested_tree(&repo, &mut builder, &file_path_str, blob_oid)?;
        let tree_oid = builder.write()?;
        repo.find_tree(tree_oid)?
    };

    // Check if tree is different from parent (no changes)
    if let Some(ref parent) = parent_commit
        && parent.tree()?.id() == new_tree.id()
    {
        return Ok(None);
    }

    // Create the commit
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
    let commit_oid = repo.commit(
        Some(&branch_ref),
        &signature,
        &signature,
        message,
        &new_tree,
        &parents,
    )?;

    Ok(Some(commit_oid))
}

/// Build a nested tree structure for a file path like ".itack/2024-01-28-issue-001.md".
fn build_nested_tree(
    repo: &Repository,
    builder: &mut git2::TreeBuilder,
    path: &str,
    blob_oid: Oid,
) -> Result<()> {
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() == 1 {
        // Simple file at root level
        builder.insert(parts[0], blob_oid, FileMode::Blob.into())?;
    } else {
        // Nested path - we need to build/update subtrees
        let dir_name = parts[0];
        let remaining_path = parts[1..].join("/");

        // Try to get existing subtree
        let existing_tree = builder
            .get(dir_name)?
            .and_then(|entry| repo.find_tree(entry.id()).ok());

        let mut sub_builder = repo.treebuilder(existing_tree.as_ref())?;
        build_nested_tree(repo, &mut sub_builder, &remaining_path, blob_oid)?;
        let sub_tree_oid = sub_builder.write()?;

        builder.insert(dir_name, sub_tree_oid, FileMode::Tree.into())?;
    }

    Ok(())
}

/// Cherry-pick a commit onto the current HEAD branch.
/// Updates working directory, index, and creates a new commit on HEAD.
/// If HEAD is unborn (no commits yet), creates the first commit.
pub fn cherry_pick_to_head(repo_path: &Path, commit_oid: Oid, message: &str) -> Result<Oid> {
    let repo = Repository::discover(repo_path)?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("itack", "itack@localhost"))?;

    let commit = repo.find_commit(commit_oid)?;

    // Check if HEAD exists (repo might have no commits yet)
    let head_commit = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(e.into()),
    };

    if head_commit.is_none() {
        // No commits yet - just checkout the commit's tree and create first commit on HEAD
        let tree = commit.tree()?;

        // Update index to match the tree
        let mut index = repo.index()?;
        index.read_tree(&tree)?;
        index.write()?;

        // Checkout the tree to working directory
        repo.checkout_tree(
            tree.as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )?;

        // Create the first commit on HEAD
        let new_oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[], // No parents for first commit
        )?;

        return Ok(new_oid);
    }

    let head = head_commit.unwrap();

    // Cherry-pick: apply the commit's changes
    repo.cherrypick(&commit, None)?;

    // Check for conflicts
    let index = repo.index()?;
    if index.has_conflicts() {
        repo.cleanup_state()?;
        return Err(ItackError::MergeConflict(
            "cherry-pick".to_string(),
            "HEAD".to_string(),
        ));
    }

    // Write the tree from the index
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Create the commit
    let new_oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&head],
    )?;

    // Clean up cherry-pick state
    repo.cleanup_state()?;

    Ok(new_oid)
}
