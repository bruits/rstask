use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io::Write;

use crate::cli::Cli;

/// Generate enhanced shell completions with dynamic task data
pub fn generate_completions<W: Write>(shell: Shell, buf: &mut W) {
    match shell {
        Shell::Bash => {
            let script = include_str!("../completions/bash.sh");
            let _ = buf.write_all(script.as_bytes());
        }
        Shell::Zsh => {
            let script = include_str!("../completions/zsh.sh");
            let _ = buf.write_all(script.as_bytes());
        }
        Shell::Fish => {
            let script = include_str!("../completions/fish.fish");
            let _ = buf.write_all(script.as_bytes());
        }
        _ => {
            // Fall back to basic clap completions for other shells
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "rstask", buf);
        }
    }
}
