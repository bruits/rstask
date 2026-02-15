mod cli;
mod completions;
mod tui;

use cli::Cli;
use rstask_core::commands::*;
use rstask_core::config::Config;
use rstask_core::constants::*;
use rstask_core::git::ensure_repo_exists;
use rstask_core::local_state::LocalState;
use rstask_core::query::{Query, parse_query};
use rstask_core::taskset::TaskSet;
use std::env;
use std::process;

fn main() {
    // Parse CLI arguments using clap
    let (cmd_name, cmd_args) = Cli::parse_to_command_and_args();

    // Handle TUI command early - it doesn't use the query system
    if cmd_name == "tui" {
        let conf = Config::new();
        match ensure_repo_exists(&conf.repo) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error initializing repository: {}", e);
                process::exit(1);
            }
        }
        if let Err(e) = tui::run_tui(conf) {
            eprintln!("TUI error: {}", e);
            process::exit(1);
        }
        return;
    }

    // Combine command and args for legacy parser
    let mut args = Vec::new();
    if !cmd_name.is_empty() {
        args.push(cmd_name.clone());
    }
    args.extend(cmd_args);

    // Parse the query using the existing query parser
    let query = match parse_query(&args) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Error parsing command: {}", e);
            process::exit(1);
        }
    };

    // Handle _completions command for dynamic completions
    if query.cmd == "_completions" {
        let conf = Config::new();
        if ensure_repo_exists(&conf.repo).is_err() {
            // If repo doesn't exist, just exit silently
            return;
        }

        let completion_type = if args.len() > 1 {
            &args[1]
        } else {
            return;
        };

        match completion_type.as_str() {
            "projects" => {
                if let Ok(ts) = TaskSet::load(&conf.repo, &conf.ids_file, false) {
                    let projects = ts.get_projects();
                    for project in projects {
                        if !project.name.is_empty() {
                            println!("{}", project.name);
                        }
                    }
                }
            }
            "tags" => {
                if let Ok(ts) = TaskSet::load(&conf.repo, &conf.ids_file, false) {
                    let tags = ts.get_tags();
                    for tag in tags {
                        println!("{}", tag);
                    }
                }
            }
            "ids" => {
                if let Ok(ts) = TaskSet::load(&conf.repo, &conf.ids_file, false) {
                    let mut ids: Vec<i32> = ts.tasks().iter().map(|t| t.id).collect();
                    ids.sort();
                    for id in ids {
                        println!("{}", id);
                    }
                }
            }
            _ => {}
        }
        return;
    }

    // Initialize config and ensure repo exists
    let conf = Config::new();
    let repo_was_created = match ensure_repo_exists(&conf.repo) {
        Ok(created) => created,
        Err(e) => {
            eprintln!("Error initializing repository: {}", e);
            process::exit(1);
        }
    };

    // Load state for context
    let mut state = LocalState::load(&conf.state_file);
    let mut ctx = state.context.clone();

    // Check for context override from environment variable
    if let Ok(ctx_from_env) = env::var("RSTASK_CONTEXT")
        && !ctx_from_env.is_empty()
    {
        if query.cmd == CMD_CONTEXT && args.len() >= 2 {
            eprintln!("Error: setting context not allowed while RSTASK_CONTEXT is set");
            process::exit(1);
        }

        let ctx_args: Vec<String> = ctx_from_env
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        ctx = match parse_query(&ctx_args) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("Error parsing RSTASK_CONTEXT: {}", e);
                process::exit(1);
            }
        };
    }

    // Check if we ignore context with the "--" token
    if query.ignore_context {
        ctx = Query::new();
    }

    // Execute the command
    let result = match query.cmd.as_str() {
        "" | CMD_NEXT | CMD_SHOW_NEXT => cmd_next(&conf, &ctx, &query),
        CMD_SHOW_OPEN => cmd_show_open(&conf, &ctx, &query),
        CMD_ADD => cmd_add(&conf, &ctx, &query),
        CMD_RM | CMD_REMOVE => cmd_remove(&conf, &ctx, &query),
        CMD_TEMPLATE => cmd_template(&conf, &ctx, &query),
        CMD_LOG => cmd_log(&conf, &ctx, &query),
        CMD_START => cmd_start(&conf, &ctx, &query),
        CMD_STOP => cmd_stop(&conf, &ctx, &query),
        CMD_DONE | CMD_RESOLVE => cmd_done(&conf, &ctx, &query),
        CMD_CONTEXT => cmd_context(&mut state, &ctx, &query, &args),
        CMD_MODIFY => cmd_modify(&conf, &ctx, &query),
        CMD_EDIT => cmd_edit(&conf, &ctx, &query),
        CMD_NOTE | CMD_NOTES => cmd_note(&conf, &ctx, &query),
        CMD_UNDO => cmd_undo(&conf, &args),
        CMD_SYNC => cmd_sync(conf.repo.to_str().unwrap(), false).map(|_| ()),
        CMD_GIT => {
            // Git command - run git directly in the repo
            if args.len() < 2 {
                eprintln!("Git command requires arguments");
                process::exit(1);
            }
            // Build git args: -C <repo> <subcommand> [args...]
            let mut git_args = vec!["-C", conf.repo.to_str().unwrap()];
            let subcommand_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
            git_args.extend(subcommand_args);
            rstask_core::util::run_cmd("git", &git_args)
        }
        CMD_SHOW_ACTIVE => cmd_show_active(&conf, &ctx, &query),
        CMD_SHOW_PAUSED => cmd_show_paused(&conf, &ctx, &query),
        CMD_OPEN => cmd_open(&conf, &ctx, &query),
        CMD_SHOW => cmd_show(&conf, &ctx, &query),
        CMD_SHOW_PROJECTS => cmd_show_projects(&conf, &ctx, &query),
        CMD_SHOW_TAGS => cmd_show_tags(&conf, &ctx, &query),
        CMD_SHOW_TEMPLATES => cmd_show_templates(&conf, &ctx, &query),
        CMD_SHOW_RESOLVED => cmd_show_resolved(&conf, &ctx, &query),
        CMD_SHOW_UNORGANISED => cmd_show_unorganised(&conf, &ctx, &query),
        _ => {
            eprintln!("Unknown command: {}", query.cmd);
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // Print remote help message if repo was just created and this wasn't a git remote command
    let is_git_remote_command = query.cmd == CMD_GIT && args.len() >= 2 && args[1] == "remote";
    if repo_was_created && !is_git_remote_command {
        println!("\nAdd a remote repository with:\n");
        println!("\trstask git remote add origin <repo>");
        println!();
    }
}
