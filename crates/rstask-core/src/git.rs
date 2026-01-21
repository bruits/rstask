// Placeholder for git module - to be implemented
use crate::Result;
use git2::Repository;
use std::io::{self, Write};
use std::path::Path;

fn is_stdout_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}

fn confirm_or_abort(message: &str) -> Result<()> {
    eprint!("{} [y/n] ", message);
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let normalized = input.trim().to_lowercase();
    if normalized == "y" || normalized == "yes" {
        Ok(())
    } else {
        Err(crate::RstaskError::Other("Aborted.".to_string()))
    }
}

pub fn ensure_repo_exists(repo_path: &Path) -> Result<()> {
    // Check for git required
    if std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_err()
    {
        return Err(crate::RstaskError::Other(
            "git required, please install".to_string(),
        ));
    }

    let git_dir = repo_path.join(".git");

    if !git_dir.exists() {
        if is_stdout_tty() {
            confirm_or_abort(&format!(
                "Could not find dstask repository at {} -- create?",
                repo_path.display()
            ))?;
        }

        std::fs::create_dir_all(repo_path)?;
        Repository::init(repo_path)?;

        println!("\nAdd a remote repository with:\n");
        println!("\trstask git remote add origin <repo>");
        println!();
    }
    Ok(())
}

pub fn git_commit(repo_path: &Path, message: &str) -> Result<()> {
    use std::process::Command;

    // Add all files
    let add_status = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "add", "."])
        .status()?;

    if !add_status.success() {
        return Err(crate::RstaskError::Other("git add failed".to_string()));
    }

    // Check if there are changes to commit
    let diff_status = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "diff-index",
            "--quiet",
            "HEAD",
            "--",
        ])
        .status();

    // If diff-index returns 0, no changes
    if let Ok(status) = diff_status
        && status.success()
    {
        eprintln!("No changes detected");
        return Ok(());
    }

    // Commit with output shown
    let commit_status = Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "commit",
            "--no-gpg-sign",
            "-m",
            message,
        ])
        .status()?;

    if !commit_status.success() {
        return Err(crate::RstaskError::Other("git commit failed".to_string()));
    }

    Ok(())
}

pub fn git_pull(repo_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;

    // Fetch from origin
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["master"], None, None)?;

    // Merge FETCH_HEAD into current branch
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.0.is_up_to_date() {
        // Already up to date
        Ok(())
    } else if analysis.0.is_fast_forward() {
        // Fast-forward merge
        let refname = "refs/heads/master";
        let mut reference = repo.find_reference(refname)?;
        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
        repo.set_head(refname)?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        Ok(())
    } else {
        // Would require actual merge - for now just error
        Err(crate::RstaskError::Git(git2::Error::from_str(
            "merge required",
        )))
    }
}

pub fn git_push(repo_path: &str) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut remote = repo.find_remote("origin")?;

    // Push master branch
    remote.push(&["refs/heads/master:refs/heads/master"], None)?;

    Ok(())
}

pub fn git_reset(repo_path: &Path) -> Result<()> {
    let repo = Repository::open(repo_path)?;

    // Reset to HEAD~1 (one commit back)
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;

    let parent = head_commit.parent(0).map_err(|_| {
        crate::RstaskError::Git(git2::Error::from_str("no parent commit to reset to"))
    })?;

    repo.reset(parent.as_object(), git2::ResetType::Hard, None)?;
    Ok(())
}
