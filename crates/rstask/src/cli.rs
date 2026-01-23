use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(name = "rstask")]
#[command(author, version, about = "A simple, git-based task manager", long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// Ignore the current context filter
    #[arg(long = "no-context", short = 'n', global = true)]
    pub no_context: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show most important tasks (default command)
    ///
    /// Display a list of non-resolved tasks in the current context, most recent last.
    /// If no command is specified, 'next' is the default.
    ///
    /// Examples:
    ///   rstask next
    ///   rstask next +work
    ///   rstask next project:website
    ///   rstask -n next    # Bypass context
    #[command(visible_alias = "show-next")]
    Next {
        /// Task filters and query parameters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Add a new task
    ///
    /// Tags (+tag), project (project:name), and priority (P0-P3) can be added
    /// anywhere in the task description. Use / to separate task from notes.
    ///
    /// Examples:
    ///   rstask add Fix bug +urgent P1 project:web
    ///   rstask add Buy milk / at the store
    ///   rstask add template:5 New task from template
    Add {
        /// Task description and attributes
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Remove a task (delete from filesystem)
    ///
    /// Examples:
    ///   rstask remove 15
    ///   rstask rm 15
    #[command(visible_alias = "rm")]
    Remove {
        /// Task IDs and filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Create or manage task templates
    ///
    /// Templates are reusable task definitions. Use template:<id> when adding
    /// tasks to create from a template.
    ///
    /// Examples:
    ///   rstask template Weekly review / checklist items
    ///   rstask template 34 project:home
    Template {
        /// Task ID or template description
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Log an already completed task
    Log {
        /// Task description and attributes
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Start working on a task (mark as active)
    ///
    /// Can either start an existing task by ID, or add and start a new task.
    ///
    /// Examples:
    ///   rstask start 15
    ///   rstask start Fix bug +urgent
    Start {
        /// Task IDs or task description for quick add
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Stop working on a task (mark as paused)
    Stop {
        /// Task IDs and optional note
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Mark a task as done (resolve)
    ///
    /// Examples:
    ///   rstask done 15
    ///   rstask done 15 Fixed by restarting server
    #[command(visible_alias = "resolve")]
    Done {
        /// Task IDs and optional closing note
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Set or view the current context filter
    ///
    /// Context is a persistent filter applied to most commands. Use "none" to clear.
    ///
    /// Examples:
    ///   rstask context +work -bug
    ///   rstask context project:website
    ///   rstask context none
    Context {
        /// Context filter or "none" to clear
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Modify task attributes (tags, project, priority)
    ///
    /// Examples:
    ///   rstask modify 15 +urgent -later P1
    ///   rstask modify 15 project:website
    Modify {
        /// Task IDs and attribute modifications
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Edit a task in your text editor
    Edit {
        /// Task IDs to edit
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Add or edit notes for a task (markdown supported)
    ///
    /// If text is provided, it's appended to notes. Otherwise, opens in editor.
    ///
    /// Examples:
    ///   rstask note 15
    ///   rstask note 15 This is a note
    #[command(visible_alias = "notes")]
    Note {
        /// Task ID and optional note text
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Undo last n commits
    Undo {
        /// Number of commits to undo (default: 1)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Synchronize with remote git repository
    Sync,

    /// Run git commands in the task repository
    Git {
        /// Git command and arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Display a single task with full details and rendered markdown notes
    ///
    /// Examples:
    ///   rstask show 15
    Show {
        /// Task ID to display
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Open all URLs found in task summary and notes in browser
    ///
    /// Examples:
    ///   rstask open 15
    Open {
        /// Task IDs to open
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show all non-resolved tasks
    #[command(name = "show-open")]
    ShowOpen {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show active tasks
    #[command(name = "show-active")]
    ShowActive {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show paused tasks
    #[command(name = "show-paused")]
    ShowPaused {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show resolved tasks
    #[command(name = "show-resolved")]
    ShowResolved {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show task templates
    #[command(name = "show-templates")]
    ShowTemplates {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Show unorganised tasks (no tags or project)
    #[command(name = "show-unorganised")]
    ShowUnorganised {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all projects with completion status
    #[command(name = "show-projects")]
    ShowProjects {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all tags in use
    #[command(name = "show-tags")]
    ShowTags {
        /// Task filters
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Generate shell completions
    #[command(name = "completions")]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Internal command for dynamic completions (hidden)
    #[command(name = "_completions", hide = true)]
    Complete {
        /// Completion type: projects, tags, or ids
        #[arg(value_parser = ["projects", "tags", "ids"])]
        completion_type: String,
    },
}

impl Cli {
    /// Parse command line arguments and return the command name and args
    pub fn parse_to_command_and_args() -> (String, Vec<String>) {
        let cli = Cli::parse();

        // Helper to prepend "--" if no-context flag is set
        let maybe_add_context_bypass = |mut args: Vec<String>| -> Vec<String> {
            if cli.no_context && !args.contains(&"--".to_string()) {
                args.insert(0, "--".to_string());
            }
            args
        };

        match cli.command {
            Some(Commands::Next { args }) => ("next".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Add { args }) => ("add".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Remove { args }) => {
                ("remove".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::Template { args }) => {
                ("template".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::Log { args }) => ("log".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Start { args }) => ("start".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Stop { args }) => ("stop".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Done { args }) => ("done".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Context { args }) => ("context".to_string(), args),
            Some(Commands::Modify { args }) => {
                ("modify".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::Edit { args }) => ("edit".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Note { args }) => ("note".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Undo { args }) => ("undo".to_string(), args),
            Some(Commands::Sync) => ("sync".to_string(), vec![]),
            Some(Commands::Git { args }) => {
                let mut full_args = vec!["git".to_string()];
                full_args.extend(args);
                ("git".to_string(), full_args)
            }
            Some(Commands::Show { args }) => ("show".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::Open { args }) => ("open".to_string(), maybe_add_context_bypass(args)),
            Some(Commands::ShowOpen { args }) => {
                ("show-open".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::ShowActive { args }) => {
                ("show-active".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::ShowPaused { args }) => {
                ("show-paused".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::ShowResolved { args }) => {
                ("show-resolved".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::ShowTemplates { args }) => {
                ("show-templates".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::ShowUnorganised { args }) => (
                "show-unorganised".to_string(),
                maybe_add_context_bypass(args),
            ),
            Some(Commands::ShowProjects { args }) => {
                ("show-projects".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::ShowTags { args }) => {
                ("show-tags".to_string(), maybe_add_context_bypass(args))
            }
            Some(Commands::Completions { shell }) => {
                // Generate enhanced completions with dynamic data
                crate::completions::generate_completions(shell, &mut std::io::stdout());
                std::process::exit(0);
            }
            Some(Commands::Complete { completion_type }) => {
                ("_completions".to_string(), vec![completion_type.clone()])
            }
            None => {
                // No subcommand provided - default to "next" command
                ("next".to_string(), vec![])
            }
        }
    }
}
