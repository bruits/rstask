use lazy_static::lazy_static;
use std::env;

lazy_static! {
    pub static ref FAKE_PTY: bool = env::var("rstask_FAKE_PTY").is_ok();
}

// Build info - will be populated by build script or environment at compile time
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
// These will be set by build.rs or can use option_env! for optional values
pub fn git_commit() -> &'static str {
    option_env!("GIT_COMMIT").unwrap_or("Unknown")
}

pub fn build_date() -> &'static str {
    option_env!("BUILD_DATE").unwrap_or("Unknown")
}

// Status constants
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_RESOLVED: &str = "resolved";
pub const STATUS_DELEGATED: &str = "delegated";
pub const STATUS_DEFERRED: &str = "deferred";
pub const STATUS_PAUSED: &str = "paused";
pub const STATUS_RECURRING: &str = "recurring";
pub const STATUS_TEMPLATE: &str = "template";

// Command constants
pub const CMD_NEXT: &str = "next";
pub const CMD_ADD: &str = "add";
pub const CMD_RM: &str = "rm";
pub const CMD_REMOVE: &str = "remove";
pub const CMD_TEMPLATE: &str = "template";
pub const CMD_LOG: &str = "log";
pub const CMD_START: &str = "start";
pub const CMD_NOTE: &str = "note";
pub const CMD_NOTES: &str = "notes";
pub const CMD_STOP: &str = "stop";
pub const CMD_DONE: &str = "done";
pub const CMD_RESOLVE: &str = "resolve";
pub const CMD_CONTEXT: &str = "context";
pub const CMD_MODIFY: &str = "modify";
pub const CMD_EDIT: &str = "edit";
pub const CMD_UNDO: &str = "undo";
pub const CMD_SYNC: &str = "sync";
pub const CMD_OPEN: &str = "open";
pub const CMD_GIT: &str = "git";
pub const CMD_SHOW_NEXT: &str = "show-next";
pub const CMD_SHOW_PROJECTS: &str = "show-projects";
pub const CMD_SHOW_TAGS: &str = "show-tags";
pub const CMD_SHOW_ACTIVE: &str = "show-active";
pub const CMD_SHOW_PAUSED: &str = "show-paused";
pub const CMD_SHOW_OPEN: &str = "show-open";
pub const CMD_SHOW_RESOLVED: &str = "show-resolved";
pub const CMD_SHOW_TEMPLATES: &str = "show-templates";
pub const CMD_SHOW_UNORGANISED: &str = "show-unorganised";
pub const CMD_COMPLETIONS: &str = "_completions";
pub const CMD_HELP: &str = "help";
pub const CMD_VERSION: &str = "version";
pub const CMD_PRINT_ZSH_COMPLETION: &str = "zsh-completion";
pub const CMD_PRINT_BASH_COMPLETION: &str = "bash-completion";
pub const CMD_PRINT_FISH_COMPLETION: &str = "fish-completion";

// Priority constants
pub const PRIORITY_CRITICAL: &str = "P0";
pub const PRIORITY_HIGH: &str = "P1";
pub const PRIORITY_NORMAL: &str = "P2";
pub const PRIORITY_LOW: &str = "P3";

// Other constants
pub const MAX_TASKS_OPEN: usize = 10000;
pub const TASK_FILENAME_LEN: usize = 40;
pub const MIN_TASKS_SHOWN: usize = 8;
pub const TERMINAL_HEIGHT_MARGIN: usize = 9;
pub const IGNORE_CONTEXT_KEYWORD: &str = "--";
pub const NOTE_MODE_KEYWORD: &str = "/";

// Theme constants (based on taskwarrior dark-256 theme)
pub const TABLE_MAX_WIDTH: usize = 160;
pub const TABLE_COL_GAP: usize = 2;
pub const MODE_HEADER: u8 = 4;
pub const FG_DEFAULT: u8 = 250;
pub const BG_DEFAULT_1: u8 = 233;
pub const BG_DEFAULT_2: u8 = 232;
pub const MODE_DEFAULT: u8 = 0;
pub const FG_ACTIVE: u8 = 233;
pub const BG_ACTIVE: u8 = 250;
pub const BG_PAUSED: u8 = 236;
pub const FG_PRIORITY_CRITICAL: u8 = 160;
pub const FG_PRIORITY_HIGH: u8 = 166;
pub const FG_PRIORITY_NORMAL: u8 = FG_DEFAULT;
pub const FG_PRIORITY_LOW: u8 = 245;
pub const FG_ACTIVE_PRIORITY_CRITICAL: u8 = 124;
pub const FG_ACTIVE_PRIORITY_HIGH: u8 = 130;
pub const FG_ACTIVE_PRIORITY_LOW: u8 = 238;
pub const FG_NOTE: u8 = 240;

// Status arrays
pub const ALL_STATUSES: &[&str] = &[
    STATUS_ACTIVE,
    STATUS_PENDING,
    STATUS_DELEGATED,
    STATUS_DEFERRED,
    STATUS_PAUSED,
    STATUS_RECURRING,
    STATUS_RESOLVED,
    STATUS_TEMPLATE,
];

pub const HIDDEN_STATUSES: &[&str] = &[STATUS_RECURRING, STATUS_RESOLVED, STATUS_TEMPLATE];

pub const NON_RESOLVED_STATUSES: &[&str] = &[
    STATUS_ACTIVE,
    STATUS_PENDING,
    STATUS_DELEGATED,
    STATUS_DEFERRED,
    STATUS_PAUSED,
    STATUS_RECURRING,
    STATUS_TEMPLATE,
];

// Valid status transitions
pub const VALID_STATUS_TRANSITIONS: &[(&str, &str)] = &[
    (STATUS_PENDING, STATUS_ACTIVE),
    (STATUS_ACTIVE, STATUS_PAUSED),
    (STATUS_PAUSED, STATUS_ACTIVE),
    (STATUS_PENDING, STATUS_RESOLVED),
    (STATUS_PAUSED, STATUS_RESOLVED),
    (STATUS_ACTIVE, STATUS_RESOLVED),
    (STATUS_PENDING, STATUS_TEMPLATE),
];

pub const ALL_CMDS: &[&str] = &[
    CMD_NEXT,
    CMD_ADD,
    CMD_RM,
    CMD_REMOVE,
    CMD_TEMPLATE,
    CMD_LOG,
    CMD_START,
    CMD_NOTE,
    CMD_NOTES,
    CMD_STOP,
    CMD_DONE,
    CMD_RESOLVE,
    CMD_CONTEXT,
    CMD_MODIFY,
    CMD_EDIT,
    CMD_UNDO,
    CMD_SYNC,
    CMD_OPEN,
    CMD_GIT,
    CMD_SHOW_NEXT,
    CMD_SHOW_PROJECTS,
    CMD_SHOW_TAGS,
    CMD_SHOW_ACTIVE,
    CMD_SHOW_PAUSED,
    CMD_SHOW_OPEN,
    CMD_SHOW_RESOLVED,
    CMD_SHOW_TEMPLATES,
    CMD_SHOW_UNORGANISED,
    CMD_COMPLETIONS,
    CMD_PRINT_BASH_COMPLETION,
    CMD_PRINT_FISH_COMPLETION,
    CMD_PRINT_ZSH_COMPLETION,
    CMD_HELP,
    CMD_VERSION,
];

// Utility functions
pub fn is_valid_status(status: &str) -> bool {
    ALL_STATUSES.contains(&status)
}

pub fn is_valid_priority(priority: &str) -> bool {
    matches!(
        priority,
        PRIORITY_CRITICAL | PRIORITY_HIGH | PRIORITY_NORMAL | PRIORITY_LOW
    )
}

pub fn is_valid_status_transition(from: &str, to: &str) -> bool {
    VALID_STATUS_TRANSITIONS.contains(&(from, to))
}
