use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use rstask_core::commands::cmd_sync;
use rstask_core::config::Config;
use rstask_core::constants::*;
use rstask_core::frontmatter::{task_from_markdown, task_to_markdown};
use rstask_core::git::{git_commit, git_reset};
use rstask_core::local_state::LocalState;
use rstask_core::query::{parse_query, Query};
use rstask_core::task::Task;
use rstask_core::taskset::TaskSet;
use rstask_core::util::{edit_string, extract_urls, open_browser};
use std::io;

use chrono::Utc;
use mdfrier::MdFrier;

/// Which view the TUI is currently showing
#[derive(Debug, Clone, PartialEq)]
enum View {
    /// Main task list
    List,
    /// Detailed view of a single task
    Detail,
    /// Editing the note of a task
    EditNote,
}

/// Which status filter tab is active
#[derive(Debug, Clone, Copy, PartialEq)]
enum StatusTab {
    All,
    Pending,
    Active,
    Paused,
    Resolved,
}

impl StatusTab {
    fn label(&self) -> &str {
        match self {
            StatusTab::All => "All",
            StatusTab::Pending => "Pending",
            StatusTab::Active => "Active",
            StatusTab::Paused => "Paused",
            StatusTab::Resolved => "Resolved",
        }
    }

    fn next(&self) -> Self {
        match self {
            StatusTab::All => StatusTab::Pending,
            StatusTab::Pending => StatusTab::Active,
            StatusTab::Active => StatusTab::Paused,
            StatusTab::Paused => StatusTab::Resolved,
            StatusTab::Resolved => StatusTab::All,
        }
    }

    fn prev(&self) -> Self {
        match self {
            StatusTab::All => StatusTab::Resolved,
            StatusTab::Pending => StatusTab::All,
            StatusTab::Active => StatusTab::Pending,
            StatusTab::Paused => StatusTab::Active,
            StatusTab::Resolved => StatusTab::Paused,
        }
    }
}

/// A status message shown temporarily at the bottom
struct StatusMessage {
    text: String,
    is_error: bool,
}

/// State for the URL selection popup
struct UrlPopup {
    /// URLs extracted from the task
    urls: Vec<String>,
    /// Which URLs are checked for opening
    checked: Vec<bool>,
    /// Current cursor position
    cursor: usize,
}

impl UrlPopup {
    fn new(urls: Vec<String>) -> Self {
        let len = urls.len();
        UrlPopup {
            urls,
            checked: vec![false; len],
            cursor: 0,
        }
    }

    fn toggle_current(&mut self) {
        if !self.urls.is_empty() {
            self.checked[self.cursor] = !self.checked[self.cursor];
        }
    }

    fn toggle_all(&mut self) {
        let all_checked = self.checked.iter().all(|&c| c);
        for c in &mut self.checked {
            *c = !all_checked;
        }
    }

    fn checked_urls(&self) -> Vec<&str> {
        self.urls
            .iter()
            .zip(self.checked.iter())
            .filter(|(_, checked)| **checked)
            .map(|(url, _)| url.as_str())
            .collect()
    }

    fn any_checked(&self) -> bool {
        self.checked.iter().any(|&c| c)
    }
}

/// State for the inline note editor
struct NoteEditor {
    /// Lines of text being edited
    lines: Vec<String>,
    /// Cursor row (line index)
    cursor_row: usize,
    /// Cursor column (byte offset within the line)
    cursor_col: usize,
    /// Scroll offset for the editor view
    scroll: usize,
    /// UUID of the task being edited
    task_uuid: String,
}

impl NoteEditor {
    fn new(notes: &str, task_uuid: &str) -> Self {
        let lines: Vec<String> = if notes.is_empty() {
            vec![String::new()]
        } else {
            notes.lines().map(|l| l.to_string()).collect()
        };
        // If the note ends with a newline, add an empty trailing line
        let lines = if notes.ends_with('\n') && !notes.is_empty() {
            let mut l = lines;
            l.push(String::new());
            l
        } else if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        NoteEditor {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll: 0,
            task_uuid: task_uuid.to_string(),
        }
    }

    /// Get the full note text from the editor buffer
    fn to_string(&self) -> String {
        self.lines.join("\n")
    }

    /// Ensure cursor_col is valid for current line
    fn clamp_cursor_col(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    /// Ensure the cursor is visible within the scroll window
    fn ensure_cursor_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.cursor_row < self.scroll {
            self.scroll = self.cursor_row;
        }
        if self.cursor_row >= self.scroll + visible_height {
            self.scroll = self.cursor_row - visible_height + 1;
        }
    }
}

/// What a confirmation popup is confirming
#[derive(Debug, Clone)]
enum ConfirmAction {
    /// Remove a task (stores uuid and summary for display)
    RemoveTask { uuid: String, summary: String },
    /// Undo last git commit
    Undo,
}

/// State for confirmation popup
struct ConfirmPopup {
    action: ConfirmAction,
    message: String,
}

impl ConfirmPopup {
    fn new(action: ConfirmAction) -> Self {
        let message = match &action {
            ConfirmAction::RemoveTask { summary, .. } => {
                format!("Remove task \"{}\"?", summary)
            }
            ConfirmAction::Undo => "Undo last commit? This cannot be reversed.".to_string(),
        };
        ConfirmPopup { action, message }
    }
}

/// State for the add-task input mode
struct AddTaskInput {
    /// Raw input text (summary + inline tags/project/priority)
    text: String,
    /// Cursor position (byte offset)
    cursor: usize,
    /// Whether to immediately resolve the task (log mode)
    resolve_immediately: bool,
}

impl AddTaskInput {
    fn new() -> Self {
        AddTaskInput {
            text: String::new(),
            cursor: 0,
            resolve_immediately: false,
        }
    }
}

/// State for the context management popup
struct ContextPopup {
    /// Input text for setting a new context
    text: String,
    /// Cursor position
    cursor: usize,
}

impl ContextPopup {
    fn new(current_context: &Query) -> Self {
        // Pre-fill with the current context as text
        let text = context_to_display_string(current_context);
        let cursor = text.len();
        ContextPopup { text, cursor }
    }
}

/// Convert a Query context to a display string
fn context_to_display_string(q: &Query) -> String {
    let mut parts = Vec::new();
    for tag in &q.tags {
        parts.push(format!("+{}", tag));
    }
    for tag in &q.anti_tags {
        parts.push(format!("-{}", tag));
    }
    if !q.project.is_empty() {
        parts.push(format!("project:{}", q.project));
    }
    if !q.priority.is_empty() {
        parts.push(q.priority.clone());
    }
    parts.join(" ")
}

/// Application state
struct App {
    conf: Config,
    /// All non-resolved tasks (unfiltered)
    all_tasks: Vec<Task>,
    /// Indices into all_tasks that pass the current filter
    filtered_indices: Vec<usize>,
    /// List widget state (selection)
    list_state: ListState,
    /// Current view
    view: View,
    /// Search/filter input string
    filter_text: String,
    /// Whether the filter input is focused
    filter_active: bool,
    /// Status tab filter
    status_tab: StatusTab,
    /// Status bar message
    status_message: Option<StatusMessage>,
    /// Should the app quit?
    should_quit: bool,
    /// Show the help popup
    show_help: bool,
    /// Note editor state (active when view == EditNote)
    note_editor: Option<NoteEditor>,
    /// URL selection popup state
    url_popup: Option<UrlPopup>,
    /// Confirmation popup state
    confirm_popup: Option<ConfirmPopup>,
    /// Add task input state
    add_input: Option<AddTaskInput>,
    /// Context management popup state
    context_popup: Option<ContextPopup>,
    /// Local state for context persistence
    local_state: LocalState,
    /// Whether we need to suspend/resume TUI for external editor
    editor_request: Option<String>,
    /// Cached mdfrier parser for markdown rendering
    frier: MdFrier,
}

impl App {
    fn new(conf: Config) -> Result<Self, rstask_core::error::RstaskError> {
        let local_state = LocalState::load(&conf.state_file);
        let mut app = App {
            conf,
            all_tasks: Vec::new(),
            filtered_indices: Vec::new(),
            list_state: ListState::default(),
            view: View::List,
            filter_text: String::new(),
            filter_active: false,
            status_tab: StatusTab::All,
            status_message: None,
            should_quit: false,
            show_help: false,
            note_editor: None,
            url_popup: None,
            confirm_popup: None,
            add_input: None,
            context_popup: None,
            local_state,
            editor_request: None,
            frier: MdFrier::new().expect("failed to initialize markdown parser"),
        };
        app.reload_tasks()?;
        Ok(app)
    }

    /// Load tasks from disk
    fn reload_tasks(&mut self) -> Result<(), rstask_core::error::RstaskError> {
        let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, true)?;
        ts.sort_by_created_ascending();
        ts.sort_by_priority_ascending();

        // Collect all tasks except templates and recurring
        self.all_tasks = ts
            .all_tasks()
            .iter()
            .filter(|t| t.status != STATUS_TEMPLATE && t.status != STATUS_RECURRING)
            .cloned()
            .collect();

        self.apply_filter();
        Ok(())
    }

    /// Recompute filtered_indices from all_tasks based on filter_text + status_tab
    fn apply_filter(&mut self) {
        // Parse filter text using the same query parser as the CLI
        let filter_query = if self.filter_text.is_empty() {
            None
        } else {
            let tokens: Vec<String> = self
                .filter_text
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            parse_query(&tokens).ok()
        };

        self.filtered_indices = self
            .all_tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| {
                // Status tab filter
                let status_ok = match self.status_tab {
                    StatusTab::All => task.status != STATUS_RESOLVED,
                    StatusTab::Pending => task.status == STATUS_PENDING,
                    StatusTab::Active => task.status == STATUS_ACTIVE,
                    StatusTab::Paused => task.status == STATUS_PAUSED,
                    StatusTab::Resolved => task.status == STATUS_RESOLVED,
                };
                if !status_ok {
                    return false;
                }

                // Query-based filter
                match &filter_query {
                    Some(q) => task.matches_filter(q),
                    None => true,
                }
            })
            .map(|(i, _)| i)
            .collect();

        // Fix selection
        if self.filtered_indices.is_empty() {
            self.list_state.select(None);
        } else {
            let sel = self.list_state.selected().unwrap_or(0);
            if sel >= self.filtered_indices.len() {
                self.list_state
                    .select(Some(self.filtered_indices.len() - 1));
            } else if self.list_state.selected().is_none() {
                self.list_state.select(Some(0));
            }
        }
    }

    /// Get the currently selected task (if any)
    fn selected_task(&self) -> Option<&Task> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered_indices.get(i))
            .map(|&idx| &self.all_tasks[idx])
    }

    fn set_status(&mut self, msg: &str, is_error: bool) {
        self.status_message = Some(StatusMessage {
            text: msg.to_string(),
            is_error,
        });
    }

    /// Perform a task action that changes status
    fn change_task_status(&mut self, new_status: &str) {
        let task = match self.selected_task() {
            Some(t) => t.clone(),
            None => {
                self.set_status("No task selected", true);
                return;
            }
        };

        if !is_valid_status_transition(&task.status, new_status) {
            self.set_status(
                &format!("Cannot transition from {} to {}", task.status, new_status),
                true,
            );
            return;
        }

        let result = (|| -> Result<(), rstask_core::error::RstaskError> {
            let include_resolved = task.status == STATUS_RESOLVED;
            let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, include_resolved)?;
            let mut t = ts
                .get_by_uuid(&task.uuid)
                .ok_or_else(|| rstask_core::error::RstaskError::TaskNotFound(task.uuid.clone()))?
                .clone();
            t.status = new_status.to_string();
            t.write_pending = true;
            if new_status == STATUS_RESOLVED {
                t.resolved = Some(Utc::now());
            }
            ts.must_update_task(t)?;
            ts.save_pending_changes()?;

            let verb = match new_status {
                STATUS_ACTIVE => "Started",
                STATUS_PAUSED => "Stopped",
                STATUS_RESOLVED => "Resolved",
                _ if task.status == STATUS_RESOLVED => "Reopened",
                _ => "Updated",
            };
            git_commit(&self.conf.repo, &format!("{} {}", verb, task.summary), true)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                let verb = match new_status {
                    STATUS_ACTIVE if task.status == STATUS_RESOLVED => "Reopened (active)",
                    STATUS_ACTIVE => "Started",
                    STATUS_PAUSED if task.status == STATUS_RESOLVED => "Reopened (paused)",
                    STATUS_PAUSED => "Paused",
                    STATUS_RESOLVED => "Resolved",
                    _ if task.status == STATUS_RESOLVED => "Reopened",
                    _ => "Updated",
                };
                self.set_status(&format!("{}: {}", verb, task.summary), false);
                let _ = self.reload_tasks();
            }
            Err(e) => {
                self.set_status(&format!("Error: {}", e), true);
            }
        }
    }

    /// Cycle priority of selected task
    fn cycle_priority(&mut self) {
        let task = match self.selected_task() {
            Some(t) => t.clone(),
            None => {
                self.set_status("No task selected", true);
                return;
            }
        };

        let new_priority = match task.priority.as_str() {
            PRIORITY_CRITICAL => PRIORITY_HIGH,
            PRIORITY_HIGH => PRIORITY_NORMAL,
            PRIORITY_NORMAL => PRIORITY_LOW,
            PRIORITY_LOW => PRIORITY_CRITICAL,
            _ => PRIORITY_NORMAL,
        };

        let result = (|| -> Result<(), rstask_core::error::RstaskError> {
            let include_resolved = task.status == STATUS_RESOLVED;
            let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, include_resolved)?;
            let mut t = ts
                .get_by_uuid(&task.uuid)
                .ok_or_else(|| rstask_core::error::RstaskError::TaskNotFound(task.uuid.clone()))?
                .clone();
            t.priority = new_priority.to_string();
            t.write_pending = true;
            ts.must_update_task(t)?;
            ts.save_pending_changes()?;
            git_commit(
                &self.conf.repo,
                &format!("Changed priority of {} to {}", task.summary, new_priority),
                true,
            )?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.set_status(
                    &format!("Priority: {} -> {}", task.priority, new_priority),
                    false,
                );
                let _ = self.reload_tasks();
            }
            Err(e) => {
                self.set_status(&format!("Error: {}", e), true);
            }
        }
    }

    /// Sync with remote git repository (pull + push), then reload tasks
    fn sync(&mut self) {
        self.set_status("Syncing...", false);
        let repo_path = self.conf.repo.to_str().unwrap().to_string();
        match cmd_sync(&repo_path, true) {
            Ok(summary) => match self.reload_tasks() {
                Ok(()) => self.set_status(&format!("Synced: {}", summary), false),
                Err(e) => self.set_status(&format!("Synced but reload failed: {}", e), true),
            },
            Err(e) => {
                self.set_status(&format!("Sync failed: {}", e), true);
            }
        }
    }

    /// Handle input events
    fn handle_event(&mut self, event: Event) {
        if let Event::Key(key) = event {
            // Global quit
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.should_quit = true;
                return;
            }

            // Help popup toggle
            if self.show_help {
                self.show_help = false;
                return;
            }

            // URL popup input
            if self.url_popup.is_some() {
                self.handle_url_popup_input(key);
                return;
            }

            // Confirmation popup input
            if self.confirm_popup.is_some() {
                self.handle_confirm_popup_input(key);
                return;
            }

            // Add task input mode
            if self.add_input.is_some() {
                self.handle_add_input(key);
                return;
            }

            // Context popup input
            if self.context_popup.is_some() {
                self.handle_context_popup_input(key);
                return;
            }

            // If filter input is active, handle text input
            if self.filter_active {
                self.handle_filter_input(key);
                return;
            }

            // View-specific input
            match self.view {
                View::List => self.handle_list_input(key),
                View::Detail => self.handle_detail_input(key),
                View::EditNote => self.handle_edit_note_input(key),
            }
        }
    }

    fn handle_filter_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_active = false;
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
                self.apply_filter();
            }
            KeyCode::Char(c) => {
                self.filter_text.push(c);
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn handle_list_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                if !self.filtered_indices.is_empty() {
                    self.list_state.select(Some(0));
                }
            }
            KeyCode::Char('G') | KeyCode::End => {
                if !self.filtered_indices.is_empty() {
                    self.list_state
                        .select(Some(self.filtered_indices.len() - 1));
                }
            }
            KeyCode::Enter => {
                if self.selected_task().is_some() {
                    self.view = View::Detail;
                }
            }
            KeyCode::Char('/') => {
                self.filter_active = true;
                self.status_message = None;
            }
            KeyCode::Char('c') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.filter_text.clear();
                self.apply_filter();
                self.set_status("Filter cleared", false);
            }
            KeyCode::Tab => {
                self.status_tab = self.status_tab.next();
                self.apply_filter();
            }
            KeyCode::BackTab => {
                self.status_tab = self.status_tab.prev();
                self.apply_filter();
            }
            // Actions
            KeyCode::Char('s') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_task_status(STATUS_ACTIVE);
            }
            KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_task_status(STATUS_PAUSED);
            }
            KeyCode::Char('d') => {
                self.change_task_status(STATUS_RESOLVED);
            }
            KeyCode::Char('P') | KeyCode::Char('p')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.cycle_priority();
            }
            KeyCode::Char('r') => match self.reload_tasks() {
                Ok(()) => self.set_status("Tasks reloaded", false),
                Err(e) => self.set_status(&format!("Reload error: {}", e), true),
            },
            KeyCode::Char('S') | KeyCode::Char('s')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.sync();
            }
            // Add task
            KeyCode::Char('a') => {
                self.add_input = Some(AddTaskInput::new());
                self.status_message = None;
            }
            // Remove task
            KeyCode::Char('x') => {
                self.request_remove_task();
            }
            // Undo
            KeyCode::Char('u') => {
                self.confirm_popup = Some(ConfirmPopup::new(ConfirmAction::Undo));
            }
            // Edit with $EDITOR
            KeyCode::Char('E') | KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.request_editor();
            }
            // Context
            KeyCode::Char('C') | KeyCode::Char('c')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                let ctx = self.local_state.get_context().clone();
                self.context_popup = Some(ContextPopup::new(&ctx));
            }
            _ => {}
        }
    }

    fn handle_detail_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
                self.view = View::List;
            }
            KeyCode::Char('?') => {
                self.show_help = true;
            }
            // Enter edit mode for notes
            KeyCode::Char('e') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                if let Some(task) = self.selected_task() {
                    let editor = NoteEditor::new(&task.notes, &task.uuid);
                    self.note_editor = Some(editor);
                    self.view = View::EditNote;
                    self.set_status("Editing notes | Ctrl+S: save | Esc: cancel", false);
                }
            }
            // Open URLs found in task
            KeyCode::Char('o') => {
                self.open_task_urls();
            }
            // Edit with $EDITOR
            KeyCode::Char('E') | KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.request_editor();
            }
            // Actions work in detail view too
            KeyCode::Char('s') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_task_status(STATUS_ACTIVE);
            }
            KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.change_task_status(STATUS_PAUSED);
            }
            KeyCode::Char('d') => {
                self.change_task_status(STATUS_RESOLVED);
                if self.selected_task().is_none() {
                    self.view = View::List;
                }
            }
            KeyCode::Char('P') | KeyCode::Char('p')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.cycle_priority();
            }
            KeyCode::Char('S') | KeyCode::Char('s')
                if key.modifiers.contains(KeyModifiers::SHIFT) =>
            {
                self.sync();
            }
            _ => {}
        }
    }

    fn handle_edit_note_input(&mut self, key: KeyEvent) {
        let editor = match self.note_editor.as_mut() {
            Some(e) => e,
            None => {
                self.view = View::Detail;
                return;
            }
        };

        // Ctrl+S to save
        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.save_note();
            return;
        }

        // Esc to cancel
        if key.code == KeyCode::Esc {
            self.note_editor = None;
            self.view = View::Detail;
            self.set_status("Edit cancelled", false);
            return;
        }

        match key.code {
            KeyCode::Char(c) => {
                editor.lines[editor.cursor_row].insert(editor.cursor_col, c);
                editor.cursor_col += c.len_utf8();
            }
            KeyCode::Enter => {
                // Split current line at cursor
                let rest = editor.lines[editor.cursor_row][editor.cursor_col..].to_string();
                editor.lines[editor.cursor_row].truncate(editor.cursor_col);
                editor.cursor_row += 1;
                editor.lines.insert(editor.cursor_row, rest);
                editor.cursor_col = 0;
            }
            KeyCode::Backspace => {
                if editor.cursor_col > 0 {
                    // Find the previous char boundary
                    let prev = editor.lines[editor.cursor_row][..editor.cursor_col]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    editor.lines[editor.cursor_row].remove(prev);
                    editor.cursor_col = prev;
                } else if editor.cursor_row > 0 {
                    // Merge with previous line
                    let current_line = editor.lines.remove(editor.cursor_row);
                    editor.cursor_row -= 1;
                    editor.cursor_col = editor.lines[editor.cursor_row].len();
                    editor.lines[editor.cursor_row].push_str(&current_line);
                }
            }
            KeyCode::Delete => {
                let line_len = editor.lines[editor.cursor_row].len();
                if editor.cursor_col < line_len {
                    editor.lines[editor.cursor_row].remove(editor.cursor_col);
                } else if editor.cursor_row + 1 < editor.lines.len() {
                    // Merge next line into current
                    let next_line = editor.lines.remove(editor.cursor_row + 1);
                    editor.lines[editor.cursor_row].push_str(&next_line);
                }
            }
            KeyCode::Left => {
                if editor.cursor_col > 0 {
                    // Move to previous char boundary
                    editor.cursor_col = editor.lines[editor.cursor_row][..editor.cursor_col]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                } else if editor.cursor_row > 0 {
                    editor.cursor_row -= 1;
                    editor.cursor_col = editor.lines[editor.cursor_row].len();
                }
            }
            KeyCode::Right => {
                let line_len = editor.lines[editor.cursor_row].len();
                if editor.cursor_col < line_len {
                    // Move to next char boundary
                    let rest = &editor.lines[editor.cursor_row][editor.cursor_col..];
                    let next_char_len = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
                    editor.cursor_col += next_char_len;
                } else if editor.cursor_row + 1 < editor.lines.len() {
                    editor.cursor_row += 1;
                    editor.cursor_col = 0;
                }
            }
            KeyCode::Up => {
                if editor.cursor_row > 0 {
                    editor.cursor_row -= 1;
                    editor.clamp_cursor_col();
                }
            }
            KeyCode::Down => {
                if editor.cursor_row + 1 < editor.lines.len() {
                    editor.cursor_row += 1;
                    editor.clamp_cursor_col();
                }
            }
            KeyCode::Home => {
                editor.cursor_col = 0;
            }
            KeyCode::End => {
                editor.cursor_col = editor.lines[editor.cursor_row].len();
            }
            KeyCode::Tab => {
                // Insert 4 spaces
                editor.lines[editor.cursor_row].insert_str(editor.cursor_col, "    ");
                editor.cursor_col += 4;
            }
            _ => {}
        }
    }

    /// Save the edited note back to the task
    fn save_note(&mut self) {
        let (note_text, task_uuid) = match &self.note_editor {
            Some(editor) => (editor.to_string(), editor.task_uuid.clone()),
            None => return,
        };

        let result = (|| -> Result<String, rstask_core::error::RstaskError> {
            let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, true)?;
            let mut task = ts
                .get_by_uuid(&task_uuid)
                .ok_or_else(|| rstask_core::error::RstaskError::TaskNotFound(task_uuid.clone()))?
                .clone();
            let summary = task.summary.clone();
            task.notes = note_text;
            task.write_pending = true;
            ts.must_update_task(task)?;
            ts.save_pending_changes()?;
            git_commit(
                &self.conf.repo,
                &format!("Updated notes for {}", summary),
                true,
            )?;
            Ok(summary)
        })();

        match result {
            Ok(summary) => {
                self.note_editor = None;
                self.view = View::Detail;
                self.set_status(&format!("Notes saved for {}", summary), false);
                let _ = self.reload_tasks();
            }
            Err(e) => {
                self.set_status(&format!("Save failed: {}", e), true);
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0) as i32;
        let new = (current + delta).clamp(0, self.filtered_indices.len() as i32 - 1) as usize;
        self.list_state.select(Some(new));
    }

    fn open_task_urls(&mut self) {
        let task = match self.selected_task() {
            Some(t) => t.clone(),
            None => return,
        };

        let text = format!("{} {}", task.summary, task.notes);
        let urls = extract_urls(&text);

        if urls.is_empty() {
            self.set_status("No URLs found in task", true);
            return;
        }

        if urls.len() == 1 {
            match open_browser(&urls[0]) {
                Ok(()) => self.set_status(&format!("Opened {}", urls[0]), false),
                Err(e) => self.set_status(&format!("Failed to open URL: {}", e), true),
            }
            return;
        }

        // Multiple URLs — show selection popup
        self.url_popup = Some(UrlPopup::new(urls));
    }

    fn handle_url_popup_input(&mut self, key: KeyEvent) {
        let popup = match self.url_popup.as_mut() {
            Some(p) => p,
            None => return,
        };

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.url_popup = None;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if popup.cursor + 1 < popup.urls.len() {
                    popup.cursor += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if popup.cursor > 0 {
                    popup.cursor -= 1;
                }
            }
            KeyCode::Char(' ') => {
                popup.toggle_current();
            }
            KeyCode::Char('a') => {
                popup.toggle_all();
            }
            KeyCode::Enter => {
                // Open checked URLs, or the currently highlighted one if none checked
                let urls_to_open: Vec<String> = if popup.any_checked() {
                    popup.checked_urls().iter().map(|s| s.to_string()).collect()
                } else {
                    vec![popup.urls[popup.cursor].clone()]
                };

                let count = urls_to_open.len();
                let mut errors = Vec::new();
                for url in &urls_to_open {
                    if let Err(e) = open_browser(url) {
                        errors.push(format!("{}", e));
                    }
                }

                self.url_popup = None;
                if errors.is_empty() {
                    self.set_status(
                        &format!("Opened {} URL{}", count, if count == 1 { "" } else { "s" }),
                        false,
                    );
                } else {
                    self.set_status(
                        &format!("Failed to open some URLs: {}", errors.join(", ")),
                        true,
                    );
                }
            }
            _ => {}
        }
    }

    fn handle_confirm_popup_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let action = self.confirm_popup.take().unwrap().action;
                self.execute_confirmed_action(action);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirm_popup = None;
                self.set_status("Cancelled", false);
            }
            _ => {}
        }
    }

    fn execute_confirmed_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::RemoveTask { uuid, summary } => {
                self.do_remove_task(&uuid, &summary);
            }
            ConfirmAction::Undo => {
                self.do_undo();
            }
        }
    }

    fn handle_add_input(&mut self, key: KeyEvent) {
        let input = match self.add_input.as_mut() {
            Some(i) => i,
            None => return,
        };

        match key.code {
            KeyCode::Esc => {
                self.add_input = None;
                self.set_status("Add cancelled", false);
            }
            KeyCode::Enter => {
                if input.text.trim().is_empty() {
                    self.add_input = None;
                    self.set_status("Add cancelled (empty)", false);
                } else {
                    let text = input.text.clone();
                    let resolve = input.resolve_immediately;
                    self.add_input = None;
                    self.do_add_task(&text, resolve);
                }
            }
            KeyCode::Tab => {
                // Toggle resolve-immediately checkbox
                input.resolve_immediately = !input.resolve_immediately;
            }
            KeyCode::Backspace => {
                if input.cursor > 0 {
                    let prev = input.text[..input.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    input.text.remove(prev);
                    input.cursor = prev;
                }
            }
            KeyCode::Left => {
                if input.cursor > 0 {
                    input.cursor = input.text[..input.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            KeyCode::Right => {
                if input.cursor < input.text.len() {
                    let rest = &input.text[input.cursor..];
                    let next_len = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
                    input.cursor += next_len;
                }
            }
            KeyCode::Home => {
                input.cursor = 0;
            }
            KeyCode::End => {
                input.cursor = input.text.len();
            }
            KeyCode::Char(c) => {
                input.text.insert(input.cursor, c);
                input.cursor += c.len_utf8();
            }
            _ => {}
        }
    }

    fn handle_context_popup_input(&mut self, key: KeyEvent) {
        let popup = match self.context_popup.as_mut() {
            Some(p) => p,
            None => return,
        };

        match key.code {
            KeyCode::Esc => {
                self.context_popup = None;
                self.set_status("Context unchanged", false);
            }
            KeyCode::Enter => {
                let text = popup.text.trim().to_string();
                self.context_popup = None;
                self.do_set_context(&text);
            }
            KeyCode::Backspace => {
                if popup.cursor > 0 {
                    let prev = popup.text[..popup.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    popup.text.remove(prev);
                    popup.cursor = prev;
                }
            }
            KeyCode::Left => {
                if popup.cursor > 0 {
                    popup.cursor = popup.text[..popup.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
            }
            KeyCode::Right => {
                if popup.cursor < popup.text.len() {
                    let rest = &popup.text[popup.cursor..];
                    let next_len = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
                    popup.cursor += next_len;
                }
            }
            KeyCode::Home => {
                popup.cursor = 0;
            }
            KeyCode::End => {
                popup.cursor = popup.text.len();
            }
            KeyCode::Char(c) => {
                popup.text.insert(popup.cursor, c);
                popup.cursor += c.len_utf8();
            }
            _ => {}
        }
    }

    /// Request to remove the currently selected task
    fn request_remove_task(&mut self) {
        let task = match self.selected_task() {
            Some(t) => t.clone(),
            None => {
                self.set_status("No task selected", true);
                return;
            }
        };
        self.confirm_popup = Some(ConfirmPopup::new(ConfirmAction::RemoveTask {
            uuid: task.uuid.clone(),
            summary: task.summary.clone(),
        }));
    }

    /// Actually remove a task after confirmation
    fn do_remove_task(&mut self, uuid: &str, summary: &str) {
        let result = (|| -> Result<(), rstask_core::error::RstaskError> {
            let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, true)?;
            ts.delete_task(uuid)?;
            git_commit(&self.conf.repo, &format!("Removed {}", summary), true)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.set_status(&format!("Removed: {}", summary), false);
                let _ = self.reload_tasks();
                // If we were in detail view, go back to list
                if self.view == View::Detail {
                    self.view = View::List;
                }
            }
            Err(e) => {
                self.set_status(&format!("Remove failed: {}", e), true);
            }
        }
    }

    /// Undo last git commit
    fn do_undo(&mut self) {
        match git_reset(&self.conf.repo) {
            Ok(()) => {
                self.set_status("Undone: last commit reverted", false);
                let _ = self.reload_tasks();
            }
            Err(e) => {
                self.set_status(&format!("Undo failed: {}", e), true);
            }
        }
    }

    /// Add a new task from the input text
    fn do_add_task(&mut self, text: &str, resolve: bool) {
        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
        let query = match parse_query(&tokens) {
            Ok(q) => q,
            Err(e) => {
                self.set_status(&format!("Parse error: {}", e), true);
                return;
            }
        };

        if query.text.is_empty() {
            self.set_status("No task summary provided", true);
            return;
        }

        // Merge with context
        let ctx = self.local_state.get_context().clone();
        let merged = query.merge(&ctx);

        let result = (|| -> Result<String, rstask_core::error::RstaskError> {
            let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, false)?;
            let task = Task {
                summary: merged.text.clone(),
                tags: merged.tags.clone(),
                project: merged.project.clone(),
                priority: if merged.priority.is_empty() {
                    PRIORITY_NORMAL.to_string()
                } else {
                    merged.priority.clone()
                },
                status: if resolve {
                    STATUS_RESOLVED.to_string()
                } else {
                    STATUS_PENDING.to_string()
                },
                resolved: if resolve { Some(Utc::now()) } else { None },
                due: merged.due,
                notes: merged.note.clone(),
                created: Utc::now(),
                write_pending: true,
                ..Default::default()
            };
            let summary = task.summary.clone();
            ts.must_load_task(task)?;
            ts.save_pending_changes()?;
            let verb = if resolve { "Logged" } else { "Added" };
            git_commit(&self.conf.repo, &format!("{} {}", verb, summary), true)?;
            Ok(summary)
        })();

        match result {
            Ok(summary) => {
                let verb = if resolve { "Logged" } else { "Added" };
                self.set_status(&format!("{}: {}", verb, summary), false);
                let _ = self.reload_tasks();
            }
            Err(e) => {
                self.set_status(&format!("Add failed: {}", e), true);
            }
        }
    }

    /// Request to open selected task in $EDITOR
    fn request_editor(&mut self) {
        let task = match self.selected_task() {
            Some(t) => t.clone(),
            None => {
                self.set_status("No task selected", true);
                return;
            }
        };
        self.editor_request = Some(task.uuid.clone());
    }

    /// Actually run the external editor (called from main loop with terminal suspended)
    fn run_external_editor(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let uuid = match self.editor_request.take() {
            Some(u) => u,
            None => return Ok(()),
        };

        let mut ts = TaskSet::load(&self.conf.repo, &self.conf.ids_file, true)?;
        let task = ts
            .get_by_uuid(&uuid)
            .ok_or_else(|| rstask_core::error::RstaskError::TaskNotFound(uuid.clone()))?
            .clone();

        let markdown = task_to_markdown(&task)?;
        let edited = edit_string(&markdown)?;

        if edited.trim() == markdown.trim() {
            self.set_status("No changes made", false);
            return Ok(());
        }

        let mut updated = task_from_markdown(&edited, &task.uuid, &task.status, task.id)?;
        updated.write_pending = true;
        let summary = updated.summary.clone();
        ts.must_update_task(updated)?;
        ts.save_pending_changes()?;
        git_commit(&self.conf.repo, &format!("Edited {}", summary), true)?;
        self.set_status(&format!("Saved: {}", summary), false);
        self.reload_tasks()?;
        Ok(())
    }

    /// Set context from text input
    fn do_set_context(&mut self, text: &str) {
        if text.is_empty() || text == "none" {
            // Clear context
            match self.local_state.set_context(Query::default()) {
                Ok(()) => {
                    if let Err(e) = self.local_state.save() {
                        self.set_status(&format!("Failed to save context: {}", e), true);
                        return;
                    }
                    self.set_status("Context cleared", false);
                }
                Err(e) => {
                    self.set_status(&format!("Failed to clear context: {}", e), true);
                }
            }
            return;
        }

        let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
        let query = match parse_query(&tokens) {
            Ok(q) => q,
            Err(e) => {
                self.set_status(&format!("Parse error: {}", e), true);
                return;
            }
        };

        match self.local_state.set_context(query.clone()) {
            Ok(()) => {
                if let Err(e) = self.local_state.save() {
                    self.set_status(&format!("Failed to save context: {}", e), true);
                    return;
                }
                let display = context_to_display_string(&query);
                self.set_status(&format!("Context set: {}", display), false);
            }
            Err(e) => {
                self.set_status(&format!("Invalid context: {}", e), true);
            }
        }
    }
}

// -- Rendering --

fn ui(f: &mut Frame, app: &mut App) {
    let term_width = f.area().width as usize;

    // Compute help hint text so we can determine its height
    let hint_text = build_help_hint(app);
    let hint_height = if term_width > 0 {
        ((hint_text.len() + term_width - 1) / term_width).max(1) as u16
    } else {
        1
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),           // Tabs + filter
            Constraint::Min(5),              // Main content
            Constraint::Length(1),           // Status bar
            Constraint::Length(hint_height), // Help hint (wraps)
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);

    match app.view {
        View::List => draw_list(f, app, chunks[1]),
        View::Detail => draw_detail(f, app, chunks[1]),
        View::EditNote => draw_edit_note(f, app, chunks[1]),
    }

    draw_status_bar(f, app, chunks[2]);
    draw_help_hint(f, &hint_text, chunks[3]);

    if app.show_help {
        draw_help_popup(f);
    }

    if let Some(ref popup) = app.url_popup {
        draw_url_popup(f, popup);
    }

    if app.confirm_popup.is_some() {
        draw_confirm_popup(f, app);
    }

    if app.add_input.is_some() {
        draw_add_input(f, app);
    }

    if app.context_popup.is_some() {
        draw_context_popup(f, app);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    // Use a vertical layout: row 1 = tabs + context, row 2 = filter (with bottom border)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let width = area.width as usize;

    // Tab bar — use compact labels on narrow screens
    let tabs = [
        StatusTab::All,
        StatusTab::Pending,
        StatusTab::Active,
        StatusTab::Paused,
        StatusTab::Resolved,
    ];
    let compact = width < 50;
    let tab_spans: Vec<Span> = tabs
        .iter()
        .map(|tab| {
            let label = if compact {
                match tab {
                    StatusTab::All => "All",
                    StatusTab::Pending => "Pend",
                    StatusTab::Active => "Act",
                    StatusTab::Paused => "Pau",
                    StatusTab::Resolved => "Res",
                }
            } else {
                tab.label()
            };
            let style = if *tab == app.status_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Span::styled(format!(" {} ", label), style)
        })
        .collect();

    let sep = if compact { "|" } else { " | " };
    let mut tab_line: Vec<Span> = Vec::new();
    for (i, span) in tab_spans.into_iter().enumerate() {
        tab_line.push(span);
        if i < tabs.len() - 1 {
            tab_line.push(Span::styled(sep, Style::default().fg(Color::DarkGray)));
        }
    }

    // Show active context if set
    let ctx_display = context_to_display_string(app.local_state.get_context());
    if !ctx_display.is_empty() {
        tab_line.push(Span::styled("  ctx:", Style::default().fg(Color::DarkGray)));
        tab_line.push(Span::styled(
            ctx_display,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let tabs_widget = Paragraph::new(Line::from(tab_line));
    f.render_widget(tabs_widget, chunks[0]);

    // Filter display
    let filter_content = if app.filter_active {
        Line::from(vec![
            Span::styled(" / ", Style::default().fg(Color::Yellow)),
            Span::styled(&app.filter_text, Style::default().fg(Color::White)),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ])
    } else if !app.filter_text.is_empty() {
        Line::from(vec![
            Span::styled(" filter: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.filter_text, Style::default().fg(Color::Yellow)),
        ])
    } else {
        Line::from("")
    };

    let filter_widget =
        Paragraph::new(filter_content).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(filter_widget, chunks[1]);
}

fn priority_color(priority: &str) -> Color {
    match priority {
        PRIORITY_CRITICAL => Color::Red,
        PRIORITY_HIGH => Color::Yellow,
        PRIORITY_NORMAL => Color::White,
        PRIORITY_LOW => Color::DarkGray,
        _ => Color::White,
    }
}

fn status_color(status: &str) -> Color {
    match status {
        STATUS_ACTIVE => Color::Green,
        STATUS_PAUSED => Color::Yellow,
        STATUS_PENDING => Color::Blue,
        STATUS_RESOLVED => Color::DarkGray,
        _ => Color::White,
    }
}

fn status_indicator(status: &str) -> &str {
    match status {
        STATUS_ACTIVE => ">>",
        STATUS_PAUSED => "||",
        STATUS_PENDING => "  ",
        STATUS_RESOLVED => "ok",
        _ => "  ",
    }
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .map(|&idx| {
            let task = &app.all_tasks[idx];
            let pri_color = priority_color(&task.priority);
            let st_color = status_color(&task.status);

            let mut spans = vec![
                Span::styled(
                    if task.status == STATUS_RESOLVED {
                        match task.resolved {
                            Some(dt) => format!("{} ", dt.format("%b %-d")),
                            None => "    ".to_string(),
                        }
                    } else {
                        format!("{:>3} ", task.id)
                    },
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{} ", status_indicator(&task.status)),
                    Style::default().fg(st_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} ", task.priority),
                    Style::default().fg(pri_color).add_modifier(Modifier::BOLD),
                ),
            ];

            // Summary
            let summary_style = if task.status == STATUS_ACTIVE {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            spans.push(Span::styled(&task.summary, summary_style));

            // Project
            if !task.project.is_empty() {
                spans.push(Span::styled(
                    format!("  [{}]", task.project),
                    Style::default().fg(Color::Cyan),
                ));
            }

            // Tags
            if !task.tags.is_empty() {
                spans.push(Span::styled(
                    format!("  +{}", task.tags.join(" +")),
                    Style::default().fg(Color::Magenta),
                ));
            }

            // Due date
            let due_str = task.parse_due_date_to_str();
            if !due_str.is_empty() {
                let due_color = if task.due.is_some() && task.due.unwrap() < Utc::now() {
                    Color::Red
                } else {
                    Color::Yellow
                };
                spans.push(Span::styled(
                    format!("  due:{}", due_str),
                    Style::default().fg(due_color),
                ));
            }

            // Notes indicator
            if !task.notes.is_empty() {
                spans.push(Span::styled(
                    " [notes]",
                    Style::default().fg(Color::DarkGray),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let task_count = app.filtered_indices.len();
    let total_count = app.all_tasks.len();
    let title = if task_count == total_count {
        format!(" Tasks ({}) ", task_count)
    } else {
        format!(" Tasks ({}/{}) ", task_count, total_count)
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let task = match app.selected_task() {
        Some(t) => t.clone(),
        None => {
            let msg = Paragraph::new("No task selected")
                .block(Block::default().title(" Detail ").borders(Borders::ALL));
            f.render_widget(msg, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(3)])
        .split(area);

    // Metadata section
    let pri_color = priority_color(&task.priority);
    let st_color = status_color(&task.status);

    let mut meta_lines = vec![
        Line::from(vec![
            Span::styled("  Summary: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &task.summary,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("       ID: ", Style::default().fg(Color::DarkGray)),
            Span::styled(task.id.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("   Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &task.status,
                Style::default().fg(st_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Priority: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &task.priority,
                Style::default().fg(pri_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Project: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if task.project.is_empty() {
                    "-"
                } else {
                    &task.project
                },
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("     Tags: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if task.tags.is_empty() {
                    "-".to_string()
                } else {
                    task.tags
                        .iter()
                        .map(|t| format!("+{}", t))
                        .collect::<Vec<_>>()
                        .join(" ")
                },
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Created: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                task.created.format("%Y-%m-%d %H:%M").to_string(),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    if let Some(due) = task.due {
        let due_color = if due < Utc::now() {
            Color::Red
        } else {
            Color::Yellow
        };
        meta_lines.push(Line::from(vec![
            Span::styled("      Due: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                due.format("%Y-%m-%d %H:%M").to_string(),
                Style::default().fg(due_color),
            ),
        ]));
    }

    let meta = Paragraph::new(meta_lines).block(
        Block::default()
            .title(" Task Detail ")
            .borders(Borders::ALL),
    );
    f.render_widget(meta, chunks[0]);

    // Notes section — rendered as markdown
    let block = Block::default().title(" Notes ").borders(Borders::ALL);
    let inner = block.inner(chunks[1]);
    f.render_widget(block, chunks[1]);

    if task.notes.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "No notes. Press 'e' to add notes.",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(empty, inner);
    } else if inner.width > 0 {
        let theme = mdfrier::ratatui::DefaultTheme;
        let md_lines = app.frier.parse(inner.width, &task.notes, &theme);
        let ratatui_lines: Vec<Line> = md_lines
            .into_iter()
            .map(|md_line| {
                let (line, _tags) = mdfrier::ratatui::render_line(md_line, &theme);
                line
            })
            .collect();
        let preview_widget = Paragraph::new(ratatui_lines).wrap(Wrap { trim: false });
        f.render_widget(preview_widget, inner);
    }
}

fn draw_edit_note(f: &mut Frame, app: &mut App, area: Rect) {
    let task = match app.selected_task() {
        Some(t) => t,
        None => {
            let msg = Paragraph::new("No task selected")
                .block(Block::default().title(" Edit Notes ").borders(Borders::ALL));
            f.render_widget(msg, area);
            return;
        }
    };

    let task_summary = task.summary.clone();

    let editor = match app.note_editor.as_mut() {
        Some(e) => e,
        None => return,
    };

    // Split area: top line for task summary, rest for editor + preview
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    // Task summary bar
    let summary_line = Line::from(vec![
        Span::styled(
            "  Editing notes for: ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            &task_summary,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let summary_widget = Paragraph::new(summary_line).block(
        Block::default()
            .title(" Note Editor ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(summary_widget, main_chunks[0]);

    // Always split horizontally: editor left, preview right
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    draw_editor_pane(f, editor, split[0]);
    draw_preview_pane(f, editor, &mut app.frier, split[1]);
}

fn draw_editor_pane(f: &mut Frame, editor: &mut NoteEditor, area: Rect) {
    let block = Block::default()
        .title(" Editor ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height as usize;
    editor.ensure_cursor_visible(visible_height);

    // Build lines with cursor highlight
    let mut lines: Vec<Line> = Vec::new();
    let end = (editor.scroll + visible_height).min(editor.lines.len());
    for row in editor.scroll..end {
        let line_text = &editor.lines[row];
        if row == editor.cursor_row {
            // Show cursor on this line
            let col = editor.cursor_col.min(line_text.len());
            let before = &line_text[..col];
            let cursor_char = if col < line_text.len() {
                &line_text[col..col
                    + line_text[col..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1)]
            } else {
                " "
            };
            let after = if col < line_text.len() {
                let char_len = line_text[col..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(0);
                &line_text[col + char_len..]
            } else {
                ""
            };

            lines.push(Line::from(vec![
                Span::raw(before.to_string()),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(after.to_string()),
            ]));
        } else {
            lines.push(Line::raw(line_text.to_string()));
        }
    }

    // Add line numbers gutter
    let gutter_width = format!("{}", editor.lines.len()).len() + 1;
    let mut numbered_lines: Vec<Line> = Vec::new();
    for (i, line) in lines.into_iter().enumerate() {
        let line_num = editor.scroll + i + 1;
        let mut spans = vec![Span::styled(
            format!("{:>width$} ", line_num, width = gutter_width),
            Style::default().fg(Color::DarkGray),
        )];
        spans.extend(line.spans);
        numbered_lines.push(Line::from(spans));
    }

    let editor_widget = Paragraph::new(numbered_lines);
    f.render_widget(editor_widget, inner);
}

fn draw_preview_pane(f: &mut Frame, editor: &mut NoteEditor, frier: &mut MdFrier, area: Rect) {
    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let note_text = editor.to_string();
    let width = inner.width as usize;

    if width == 0 || note_text.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "No content to preview.",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(empty, inner);
        return;
    }

    let theme = mdfrier::ratatui::DefaultTheme;
    let md_lines = frier.parse(width as u16, &note_text, &theme);
    let ratatui_lines: Vec<Line> = md_lines
        .into_iter()
        .map(|md_line| {
            let (line, _tags) = mdfrier::ratatui::render_line(md_line, &theme);
            line
        })
        .collect();

    let preview_widget = Paragraph::new(ratatui_lines);
    f.render_widget(preview_widget, inner);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (text, style) = match &app.status_message {
        Some(msg) if msg.is_error => (
            msg.text.clone(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Some(msg) => (msg.text.clone(), Style::default().fg(Color::Green)),
        None => (String::new(), Style::default().fg(Color::DarkGray)),
    };

    let bar = Paragraph::new(Span::styled(text, style));
    f.render_widget(bar, area);
}

fn build_help_hint(app: &App) -> String {
    let segments: Vec<&str> = if app.filter_active {
        vec!["Type to filter", "Enter/Esc: confirm"]
    } else {
        match app.view {
            View::List => {
                vec![
                    "?: help",
                    "q: quit",
                    "/: filter",
                    "Enter: detail",
                    "a: add",
                    "s: start",
                    "p: pause",
                    "d: done",
                    "x: remove",
                    "P: priority",
                    "E: editor",
                    "u: undo",
                    "C: context",
                    "Tab: status",
                    "r: reload",
                    "S: sync",
                    "c: clear",
                ]
            }
            View::Detail => {
                vec![
                    "?: help",
                    "Esc: back",
                    "e: edit",
                    "E: editor",
                    "o: open URLs",
                    "s: start",
                    "p: pause",
                    "d: done",
                    "P: priority",
                    "S: sync",
                ]
            }
            View::EditNote => {
                vec!["Ctrl+S: save", "Esc: cancel", "arrows: move", "Tab: indent"]
            }
        }
    };

    let mut result = String::from(" ");
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            result.push_str(" | ");
        }
        result.push_str(seg);
    }
    result
}

fn draw_help_hint(f: &mut Frame, hint_text: &str, area: Rect) {
    let hint_widget = Paragraph::new(Span::styled(
        hint_text,
        Style::default().fg(Color::DarkGray),
    ))
    .wrap(Wrap { trim: false });
    f.render_widget(hint_widget, area);
}

fn draw_url_popup(f: &mut Frame, popup: &UrlPopup) {
    // Size the popup based on content
    let max_url_len = popup
        .urls
        .iter()
        .map(|u| u.len())
        .max()
        .unwrap_or(20)
        .min(80);
    let width = (max_url_len + 10).min(f.area().width as usize - 4) as u16;
    // +5 for title, footer, borders, header line, blank line
    let height = (popup.urls.len() + 6).min(f.area().height as usize - 2) as u16;

    let area = centered_rect_abs(width, height, f.area());
    f.render_widget(Clear, area);

    let mut lines = vec![
        Line::from(Span::styled(
            "Select URLs to open",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (i, url) in popup.urls.iter().enumerate() {
        let checkbox = if popup.checked[i] { "[x] " } else { "[ ] " };
        let is_cursor = i == popup.cursor;

        let style = if is_cursor {
            Style::default()
                .fg(Color::White)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let checkbox_style = if is_cursor {
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD)
        } else if popup.checked[i] {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(checkbox.to_string(), checkbox_style),
            Span::styled(url.clone(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Space: toggle | a: all | Enter: open | Esc: close",
        Style::default().fg(Color::DarkGray),
    )));

    let popup_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Open URLs ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(popup_widget, area);
}

fn draw_confirm_popup(f: &mut Frame, app: &App) {
    let popup = match &app.confirm_popup {
        Some(p) => p,
        None => return,
    };

    let width = (popup.message.len() + 6).max(30).min(60) as u16;
    let height = 5;
    let area = centered_rect_abs(width, height, f.area());
    f.render_widget(Clear, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &popup.message,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " y: confirm | n/Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);
}

fn draw_add_input(f: &mut Frame, app: &App) {
    let input = match &app.add_input {
        Some(i) => i,
        None => return,
    };

    let width = (f.area().width as usize * 70 / 100).max(40).min(80) as u16;
    let height = 7;
    let area = centered_rect_abs(width, height, f.area());
    f.render_widget(Clear, area);

    let resolve_indicator = if input.resolve_immediately {
        "[x] Log (resolve immediately)"
    } else {
        "[ ] Log (resolve immediately)"
    };

    let col = input.cursor.min(input.text.len());
    let before = &input.text[..col];
    let cursor_char = if col < input.text.len() {
        let ch_len = input.text[col..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        &input.text[col..col + ch_len]
    } else {
        " "
    };
    let after = if col < input.text.len() {
        let ch_len = input.text[col..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        &input.text[col + ch_len..]
    } else {
        ""
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  > ", Style::default().fg(Color::Yellow)),
            Span::raw(before.to_string()),
            Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(after.to_string()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", resolve_indicator),
            Style::default().fg(if input.resolve_immediately {
                Color::Green
            } else {
                Color::DarkGray
            }),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " Enter: add | Tab: toggle log | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Add Task (+tag project:X P0-P3 due:date) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);
}

fn draw_context_popup(f: &mut Frame, app: &App) {
    let popup = match &app.context_popup {
        Some(p) => p,
        None => return,
    };

    let current = context_to_display_string(app.local_state.get_context());
    let current_display = if current.is_empty() {
        "(none)".to_string()
    } else {
        current
    };

    let width = 60u16.min(f.area().width - 4);
    let height = 8;
    let area = centered_rect_abs(width, height, f.area());
    f.render_widget(Clear, area);

    let col = popup.cursor.min(popup.text.len());
    let before = &popup.text[..col];
    let cursor_char = if col < popup.text.len() {
        let ch_len = popup.text[col..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        &popup.text[col..col + ch_len]
    } else {
        " "
    };
    let after = if col < popup.text.len() {
        let ch_len = popup.text[col..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        &popup.text[col + ch_len..]
    } else {
        ""
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Current: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&current_display, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  > ", Style::default().fg(Color::Yellow)),
            Span::raw(before.to_string()),
            Span::styled(
                cursor_char.to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ),
            Span::raw(after.to_string()),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Enter: set | empty/none: clear | Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Context (+tag project:X P0-P3) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);
}

fn draw_help_popup(f: &mut Frame) {
    let area = centered_rect(80, 80, f.area());

    f.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Navigation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    j/Down    ", Style::default().fg(Color::White)),
            Span::styled("Move down", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    k/Up      ", Style::default().fg(Color::White)),
            Span::styled("Move up", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    g/Home    ", Style::default().fg(Color::White)),
            Span::styled("Go to top", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    G/End     ", Style::default().fg(Color::White)),
            Span::styled("Go to bottom", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    Enter     ", Style::default().fg(Color::White)),
            Span::styled("Show task detail", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    Esc/q     ", Style::default().fg(Color::White)),
            Span::styled("Back / Quit", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Filtering",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    /         ", Style::default().fg(Color::White)),
            Span::styled(
                "Start typing a filter",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    c         ", Style::default().fg(Color::White)),
            Span::styled("Clear filter", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    Tab       ", Style::default().fg(Color::White)),
            Span::styled("Next status tab", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    Shift+Tab ", Style::default().fg(Color::White)),
            Span::styled("Previous status tab", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Actions",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    a         ", Style::default().fg(Color::White)),
            Span::styled(
                "Add new task (Tab to toggle log mode)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    s         ", Style::default().fg(Color::White)),
            Span::styled(
                "Start task (set active)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    p         ", Style::default().fg(Color::White)),
            Span::styled("Pause task", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    d         ", Style::default().fg(Color::White)),
            Span::styled(
                "Mark task done (resolve)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    P         ", Style::default().fg(Color::White)),
            Span::styled(
                "Cycle priority (P0->P1->P2->P3->P0)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    x         ", Style::default().fg(Color::White)),
            Span::styled(
                "Remove task (with confirmation)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    u         ", Style::default().fg(Color::White)),
            Span::styled(
                "Undo last commit (with confirmation)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    E         ", Style::default().fg(Color::White)),
            Span::styled("Edit task in $EDITOR", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    C         ", Style::default().fg(Color::White)),
            Span::styled(
                "Set/clear context filter",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    r         ", Style::default().fg(Color::White)),
            Span::styled(
                "Reload tasks from disk",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("    S         ", Style::default().fg(Color::White)),
            Span::styled(
                "Sync with remote (pull + push)",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Detail View",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    e         ", Style::default().fg(Color::White)),
            Span::styled("Edit task notes", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    E         ", Style::default().fg(Color::White)),
            Span::styled("Edit task in $EDITOR", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    o         ", Style::default().fg(Color::White)),
            Span::styled("Open URLs in browser", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    Esc/q     ", Style::default().fg(Color::White)),
            Span::styled("Back to list", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "  Note Editor",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("    Ctrl+S    ", Style::default().fg(Color::White)),
            Span::styled("Save notes", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("    Esc       ", Style::default().fg(Color::White)),
            Span::styled("Cancel editing", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(help, area);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Helper to create a centered rect with absolute width/height
fn centered_rect_abs(width: u16, height: u16, r: Rect) -> Rect {
    let w = width.min(r.width);
    let h = height.min(r.height);
    let x = r.x + (r.width.saturating_sub(w)) / 2;
    let y = r.y + (r.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Entry point for the TUI
pub fn run_tui(conf: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(conf)?;

    // Main loop
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // Handle external editor request — need to suspend TUI
        if app.editor_request.is_some() {
            // Leave alternate screen and disable raw mode
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            // Run the editor
            if let Err(e) = app.run_external_editor() {
                app.set_status(&format!("Editor error: {}", e), true);
            }

            // Re-enter alternate screen and enable raw mode
            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.clear()?;
            continue;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            let ev = event::read()?;
            app.handle_event(ev);
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
