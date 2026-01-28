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

/// Merge source branch into target branch without checkout.
/// Returns the merge commit OID if successful.
/// Returns error if merge has conflicts.
/// If target branch doesn't exist, creates it pointing at source.
pub fn merge_branches(repo_path: &Path, source_branch: &str, target_branch: &str) -> Result<Oid> {
    let repo = Repository::discover(repo_path)?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("itack", "itack@localhost"))?;

    // Find both branch commits
    let source_ref = format!("refs/heads/{}", source_branch);
    let target_ref = format!("refs/heads/{}", target_branch);

    let source_commit = repo
        .find_reference(&source_ref)
        .map_err(|_| ItackError::BranchNotFound(source_branch.to_string()))?
        .peel_to_commit()?;

    // Try to find target branch - if it doesn't exist, create it pointing at source
    let target_commit = match repo.find_reference(&target_ref) {
        Ok(reference) => reference.peel_to_commit()?,
        Err(e) if e.code() == git2::ErrorCode::NotFound => {
            // Target branch doesn't exist - create it pointing at source
            repo.reference(
                &target_ref,
                source_commit.id(),
                false,
                &format!("Create {} from {}", target_branch, source_branch),
            )?;
            return Ok(source_commit.id());
        }
        Err(e) => return Err(e.into()),
    };

    // Check if source is already merged (target contains source)
    if repo.graph_descendant_of(target_commit.id(), source_commit.id())? {
        // Already merged
        return Ok(target_commit.id());
    }

    // Check if fast-forward is possible (source contains target)
    if repo.graph_descendant_of(source_commit.id(), target_commit.id())? {
        // Fast-forward: just update the target ref
        repo.reference(
            &target_ref,
            source_commit.id(),
            true,
            &format!(
                "Fast-forward merge {} into {}",
                source_branch, target_branch
            ),
        )?;
        return Ok(source_commit.id());
    }

    // Need a real merge - find merge base
    let merge_base = repo.merge_base(source_commit.id(), target_commit.id())?;
    let base_commit = repo.find_commit(merge_base)?;
    let base_tree = base_commit.tree()?;

    let source_tree = source_commit.tree()?;
    let target_tree = target_commit.tree()?;

    // Perform three-way merge
    let mut index = repo.merge_trees(&base_tree, &target_tree, &source_tree, None)?;

    if index.has_conflicts() {
        return Err(ItackError::MergeConflict(
            source_branch.to_string(),
            target_branch.to_string(),
        ));
    }

    // Write the merged tree
    let tree_oid = index.write_tree_to(&repo)?;
    let merged_tree = repo.find_tree(tree_oid)?;

    // Create merge commit
    let message = format!("Merge {} into {}", source_branch, target_branch);
    let merge_oid = repo.commit(
        Some(&target_ref),
        &signature,
        &signature,
        &message,
        &merged_tree,
        &[&target_commit, &source_commit],
    )?;

    Ok(merge_oid)
}

/// Read a file from a specific branch without checkout.
pub fn read_file_from_branch(
    repo_path: &Path,
    branch_name: &str,
    file_path: &Path,
) -> Result<Vec<u8>> {
    let repo = Repository::discover(repo_path)?;

    let branch_ref = format!("refs/heads/{}", branch_name);
    let commit = repo
        .find_reference(&branch_ref)
        .map_err(|_| ItackError::BranchNotFound(branch_name.to_string()))?
        .peel_to_commit()?;

    let tree = commit.tree()?;
    let file_path_str = file_path.to_string_lossy();

    let entry = tree
        .get_path(Path::new(&*file_path_str))
        .map_err(|_| ItackError::Other(format!("File not found in branch: {}", file_path_str)))?;

    let blob = repo.find_blob(entry.id())?;
    Ok(blob.content().to_vec())
}

/// Find an issue file in a branch by ID suffix (e.g., "-issue-001.md").
/// Returns the relative path to the file.
pub fn find_issue_file_in_branch(
    repo_path: &Path,
    branch_name: &str,
    issue_id: u32,
) -> Result<std::path::PathBuf> {
    let repo = Repository::discover(repo_path)?;

    let branch_ref = format!("refs/heads/{}", branch_name);
    let commit = repo
        .find_reference(&branch_ref)
        .map_err(|_| ItackError::BranchNotFound(branch_name.to_string()))?
        .peel_to_commit()?;

    let tree = commit.tree()?;
    let suffix = format!("-issue-{:03}.md", issue_id);

    // Walk the tree looking for the issue file
    let mut found_path = None;
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        let name = entry.name().unwrap_or("");
        if name.ends_with(&suffix) {
            let full_path = if root.is_empty() {
                name.to_string()
            } else {
                format!("{}{}", root, name)
            };
            found_path = Some(std::path::PathBuf::from(full_path));
            return git2::TreeWalkResult::Abort;
        }
        git2::TreeWalkResult::Ok
    })?;

    found_path.ok_or_else(|| ItackError::IssueNotFound(issue_id))
}
