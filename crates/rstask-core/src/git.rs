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

    // Check if repo is brand new (needed before diff-index to avoid missing HEAD error)
    let objects_dir = repo_path.join(".git/objects");
    let brand_new = if let Ok(entries) = std::fs::read_dir(&objects_dir) {
        entries.count() <= 2
    } else {
        return Err(crate::RstaskError::Other(
            "failed to read git objects directory".to_string(),
        ));
    };

    // Add all files
    let add_status = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "add", "."])
        .status()?;

    if !add_status.success() {
        return Err(crate::RstaskError::Other("git add failed".to_string()));
    }

    // Check for changes -- only if repo has commits (to avoid missing HEAD error)
    if !brand_new {
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
            println!("No changes detected");
            return Ok(());
        }
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
    use std::process::Command;

    let status = Command::new("git")
        .args([
            "-C",
            repo_path,
            "pull",
            "--ff",
            "--no-rebase",
            "--no-edit",
            "--commit",
        ])
        .status()?;

    if !status.success() {
        return Err(crate::RstaskError::Other("git pull failed".to_string()));
    }

    Ok(())
}

pub fn git_push(repo_path: &str) -> Result<()> {
    use std::process::Command;

    let status = Command::new("git")
        .args(["-C", repo_path, "push"])
        .status()?;

    if !status.success() {
        return Err(crate::RstaskError::Other("git push failed".to_string()));
    }

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
