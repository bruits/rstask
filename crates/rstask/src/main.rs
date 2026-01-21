use rstask_core::commands::*;
use rstask_core::config::Config;
use rstask_core::constants::*;
use rstask_core::git::ensure_repo_exists;
use rstask_core::local_state::LocalState;
use rstask_core::query::{Query, parse_query};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let query = match parse_query(&args) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Error parsing command: {}", e);
            process::exit(1);
        }
    };

    // Handle commands that don't require initialization
    match query.cmd.as_str() {
        CMD_HELP => {
            cmd_help(&args);
            return;
        }
        CMD_VERSION => {
            cmd_version();
            return;
        }
        CMD_PRINT_BASH_COMPLETION => {
            print!("{}", include_str!("../completions/bash.sh"));
            return;
        }
        CMD_PRINT_ZSH_COMPLETION => {
            print!("{}", include_str!("../completions/zsh.sh"));
            return;
        }
        CMD_PRINT_FISH_COMPLETION => {
            print!("{}", include_str!("../completions/completions.fish"));
            return;
        }
        _ => {}
    }

    // Initialize config and ensure repo exists
    let conf = Config::new();
    if let Err(e) = ensure_repo_exists(&conf.repo) {
        eprintln!("Error initializing repository: {}", e);
        process::exit(1);
    }

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
        CMD_SYNC => cmd_sync(conf.repo.to_str().unwrap()),
        CMD_GIT => {
            // Git command - run git directly in the repo
            if args.len() < 2 {
                eprintln!("Git command requires arguments");
                process::exit(1);
            }
            // Use git2-rs or run git command directly
            rstask_core::util::run_cmd("git", &["-C", conf.repo.to_str().unwrap()]).and_then(|_| {
                // Now run the actual git subcommand
                let git_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
                rstask_core::util::run_cmd("git", &git_args)
            })
        }
        CMD_SHOW_ACTIVE => cmd_show_active(&conf, &ctx, &query),
        CMD_SHOW_PAUSED => cmd_show_paused(&conf, &ctx, &query),
        CMD_OPEN => cmd_open(&conf, &ctx, &query),
        CMD_SHOW_PROJECTS => cmd_show_projects(&conf, &ctx, &query),
        CMD_SHOW_TAGS => cmd_show_tags(&conf, &ctx, &query),
        CMD_SHOW_TEMPLATES => cmd_show_templates(&conf, &ctx, &query),
        CMD_SHOW_RESOLVED => cmd_show_resolved(&conf, &ctx, &query),
        CMD_SHOW_UNORGANISED => cmd_show_unorganised(&conf, &ctx, &query),
        CMD_COMPLETIONS => {
            eprintln!("Completions command not yet implemented");
            process::exit(1);
        }
        _ => {
            eprintln!("Unknown command: {}", query.cmd);
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
