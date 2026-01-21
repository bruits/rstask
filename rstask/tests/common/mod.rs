#![allow(dead_code)]

use rstask_core::task::Task;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

pub struct TestRepo {
    pub dir: TempDir,
    pub path: PathBuf,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp directory");
        let path = dir.path().to_path_buf();

        // Initialize git repo
        let status = Command::new("git")
            .arg("init")
            .current_dir(&path)
            .status()
            .expect("Failed to initialize git repo");

        assert!(status.success(), "git init failed");

        TestRepo { dir, path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub struct TestCmd {
    repo_path: PathBuf,
    binary_path: PathBuf,
    context: Option<String>,
}

impl TestCmd {
    pub fn new(repo: &TestRepo) -> Self {
        let binary_path = std::env::current_exe()
            .expect("Failed to get test executable path")
            .parent()
            .expect("Failed to get parent directory")
            .parent()
            .expect("Failed to get parent directory")
            .join("rstask");

        TestCmd {
            repo_path: repo.path().to_path_buf(),
            binary_path,
            context: None,
        }
    }

    pub fn new_with_context(repo: &TestRepo, context: &str) -> Self {
        let binary_path = std::env::current_exe()
            .expect("Failed to get test executable path")
            .parent()
            .expect("Failed to get parent directory")
            .parent()
            .expect("Failed to get parent directory")
            .join("rstask");

        TestCmd {
            repo_path: repo.path().to_path_buf(),
            binary_path,
            context: Some(context.to_string()),
        }
    }

    pub fn run(&self, args: &[&str]) -> TestResult {
        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args).env("RSTASK_GIT_REPO", &self.repo_path);

        if let Some(ctx) = &self.context {
            cmd.env("RSTASK_CONTEXT", ctx);
        } else {
            cmd.env("RSTASK_CONTEXT", "");
        }

        let output = cmd.output().expect("Failed to execute command");

        TestResult { output }
    }
}

pub struct TestResult {
    output: Output,
}

impl TestResult {
    pub fn success(&self) -> bool {
        self.output.status.success()
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    pub fn assert_success(&self) {
        if !self.success() {
            panic!(
                "Command failed with status: {:?}\nstdout: {}\nstderr: {}",
                self.output.status,
                self.stdout(),
                self.stderr()
            );
        }
    }

    pub fn assert_failure(&self) {
        if self.success() {
            panic!(
                "Command succeeded when failure was expected\nstdout: {}\nstderr: {}",
                self.stdout(),
                self.stderr()
            );
        }
    }

    pub fn parse_tasks(&self) -> Vec<Task> {
        let stdout = self.stdout();
        if stdout.trim().is_empty() {
            return Vec::new();
        }
        serde_json::from_str(&stdout).expect("Failed to parse tasks from JSON")
    }

    pub fn parse_projects(&self) -> Vec<String> {
        let stdout = self.stdout();
        if stdout.trim().is_empty() {
            return Vec::new();
        }
        serde_json::from_str(&stdout).expect("Failed to parse projects from JSON")
    }
}

#[macro_export]
macro_rules! test_setup {
    () => {{
        let repo = $crate::common::TestRepo::new();
        let cmd = $crate::common::TestCmd::new(&repo);
        (repo, cmd)
    }};
}
