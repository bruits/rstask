use std::env;
use std::path::PathBuf;

/// Configuration for the rstask application
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the git repository
    pub repo: PathBuf,
    /// Path to the rstask local state file
    pub state_file: PathBuf,
    /// Path to the IDs file
    pub ids_file: PathBuf,
    /// Context from environment variable
    pub ctx_from_env_var: Option<String>,
}

impl Config {
    /// Creates a new Config from environment variables
    pub fn new() -> Self {
        let ctx_from_env_var = env::var("RSTASK_CONTEXT").ok();

        let home = home::home_dir()
            .or_else(|| env::var("HOME").ok().map(PathBuf::from))
            .expect("Could not determine home directory");

        let default_repo = home.join(".rstask");
        let repo = env::var("RSTASK_GIT_REPO")
            .map(PathBuf::from)
            .unwrap_or(default_repo);

        let state_file = repo.join(".git").join("rstask").join("state.bin");
        let ids_file = repo.join(".git").join("rstask").join("ids.bin");

        Config {
            repo,
            state_file,
            ids_file,
            ctx_from_env_var,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
