//! Git operations for itack.

use std::path::Path;

use git2::{FileMode, Oid, Repository, Signature};

use crate::error::Result;

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

/// Read a file from a specific branch without checking it out.
/// Returns None if the file doesn't exist in the branch.
pub fn read_file_from_branch(
    repo_path: &Path,
    branch_name: &str,
    file_path: &Path,
) -> Result<Option<Vec<u8>>> {
    let repo = Repository::discover(repo_path)?;

    // Find the branch
    let branch_ref = format!("refs/heads/{}", branch_name);
    let reference = match repo.find_reference(&branch_ref) {
        Ok(r) => r,
        Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let commit = reference.peel_to_commit()?;
    let tree = commit.tree()?;

    // Convert path to string for tree lookup
    let file_path_str = file_path.to_string_lossy();

    // Navigate the tree to find the file
    match tree.get_path(std::path::Path::new(file_path_str.as_ref())) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            Ok(Some(blob.content().to_vec()))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Remove a file from a specific branch.
/// Returns the commit OID if a commit was created, None if the file didn't exist.
pub fn remove_file_from_branch(
    repo_path: &Path,
    branch_name: &str,
    file_path: &Path,
    message: &str,
) -> Result<Option<Oid>> {
    let repo = Repository::discover(repo_path)?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("itack", "itack@localhost"))?;

    let file_path_str = file_path.to_string_lossy();

    let branch_ref = format!("refs/heads/{}", branch_name);
    let reference = repo.find_reference(&branch_ref)?;
    let parent_commit = reference.peel_to_commit()?;
    let parent_tree = parent_commit.tree()?;

    // Navigate to the parent directory and remove the entry
    let parts: Vec<&str> = file_path_str.split('/').collect();
    let new_tree = if parts.len() == 1 {
        let mut builder = repo.treebuilder(Some(&parent_tree))?;
        builder.remove(parts[0])?;
        let tree_oid = builder.write()?;
        repo.find_tree(tree_oid)?
    } else {
        // Nested path (e.g. ".itack/filename.md")
        let dir_name = parts[0];
        let file_name = parts[1..].join("/");

        let dir_entry = match parent_tree.get_name(dir_name) {
            Some(entry) => entry,
            None => return Ok(None),
        };
        let dir_tree = repo.find_tree(dir_entry.id())?;

        let mut sub_builder = repo.treebuilder(Some(&dir_tree))?;
        if sub_builder.get(&file_name)?.is_none() {
            return Ok(None);
        }
        sub_builder.remove(&file_name)?;
        let sub_tree_oid = sub_builder.write()?;

        let mut builder = repo.treebuilder(Some(&parent_tree))?;
        builder.insert(dir_name, sub_tree_oid, FileMode::Tree.into())?;
        let tree_oid = builder.write()?;
        repo.find_tree(tree_oid)?
    };

    // Check if tree actually changed
    if parent_tree.id() == new_tree.id() {
        return Ok(None);
    }

    let commit_oid = repo.commit(
        Some(&branch_ref),
        &signature,
        &signature,
        message,
        &new_tree,
        &[&parent_commit],
    )?;

    Ok(Some(commit_oid))
}

/// Find an issue file in a branch by issue ID.
/// Returns the relative path if found.
pub fn find_issue_in_branch(
    repo_path: &Path,
    branch_name: &str,
    issue_id: u32,
) -> Result<Option<std::path::PathBuf>> {
    let repo = Repository::discover(repo_path)?;

    // Find the branch
    let branch_ref = format!("refs/heads/{}", branch_name);
    let reference = match repo.find_reference(&branch_ref) {
        Ok(r) => r,
        Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let commit = reference.peel_to_commit()?;
    let tree = commit.tree()?;

    // Look for .itack directory
    let itack_entry = match tree.get_name(".itack") {
        Some(entry) => entry,
        None => return Ok(None),
    };

    let itack_tree = repo.find_tree(itack_entry.id())?;

    // Look for file matching pattern *-issue-{id:03}.md
    let suffix = format!("-issue-{:03}.md", issue_id);
    for entry in itack_tree.iter() {
        if let Some(name) = entry.name()
            && name.ends_with(&suffix)
        {
            return Ok(Some(std::path::PathBuf::from(".itack").join(name)));
        }
    }

    // Fall back to old format
    let old_name = format!("{}.md", issue_id);
    if itack_tree.get_name(&old_name).is_some() {
        return Ok(Some(std::path::PathBuf::from(".itack").join(old_name)));
    }

    Ok(None)
}
