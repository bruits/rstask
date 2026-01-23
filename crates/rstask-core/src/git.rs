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

pub fn ensure_repo_exists(repo_path: &Path) -> Result<bool> {
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

        // Return true to indicate repo was just created
        return Ok(true);
    }
    Ok(false)
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

fn get_current_branch(repo_path: &str) -> Result<String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["-C", repo_path, "branch", "--show-current"])
        .output()?;

    if !output.status.success() {
        return Err(crate::RstaskError::Other(
            "failed to get current branch".to_string(),
        ));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if branch.is_empty() {
        return Err(crate::RstaskError::Other("not on a branch".to_string()));
    }

    Ok(branch)
}

fn has_upstream_branch(repo_path: &str, branch: &str) -> Result<bool> {
    use std::process::Command;

    let output = Command::new("git")
        .args([
            "-C",
            repo_path,
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{upstream}}", branch),
        ])
        .output()?;

    Ok(output.status.success())
}

fn has_remote(repo_path: &str) -> Result<bool> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["-C", repo_path, "remote"])
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let remotes = String::from_utf8_lossy(&output.stdout);
    Ok(!remotes.trim().is_empty())
}

pub fn git_pull(repo_path: &str) -> Result<()> {
    use std::process::Command;

    // Check if a remote is configured
    if !has_remote(repo_path)? {
        return Err(crate::RstaskError::Other(
            "No remote configured. Add a remote with: rstask git remote add origin <url>"
                .to_string(),
        ));
    }

    // Get current branch name
    let branch = get_current_branch(repo_path)?;

    // Check if upstream is set
    let has_upstream = has_upstream_branch(repo_path, &branch)?;

    let status = if has_upstream {
        // Normal pull
        Command::new("git")
            .args([
                "-C",
                repo_path,
                "pull",
                "--ff",
                "--no-rebase",
                "--no-edit",
                "--commit",
                "--allow-unrelated-histories",
            ])
            .status()?
    } else {
        // First pull - set upstream tracking
        Command::new("git")
            .args([
                "-C",
                repo_path,
                "pull",
                "--set-upstream",
                "origin",
                &branch,
                "--ff",
                "--no-rebase",
                "--no-edit",
                "--commit",
                "--allow-unrelated-histories",
            ])
            .status()?
    };

    if !status.success() {
        return Err(crate::RstaskError::Other(
            "git pull failed. Make sure the remote is set up correctly with: rstask git remote add origin <url>".to_string()
        ));
    }

    Ok(())
}

pub fn git_push(repo_path: &str) -> Result<()> {
    use std::process::Command;

    // Check if a remote is configured
    if !has_remote(repo_path)? {
        return Err(crate::RstaskError::Other(
            "No remote configured. Add a remote with: rstask git remote add origin <url>"
                .to_string(),
        ));
    }

    // Get current branch name
    let branch = get_current_branch(repo_path)?;

    // Check if upstream is set
    let has_upstream = has_upstream_branch(repo_path, &branch)?;

    let status = if has_upstream {
        // Normal push
        Command::new("git")
            .args(["-C", repo_path, "push"])
            .status()?
    } else {
        // First push - set upstream tracking
        Command::new("git")
            .args(["-C", repo_path, "push", "-u", "origin", &branch])
            .status()?
    };

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
