use crate::{
    config::Config,
    constants::*,
    error::{Result, RstaskError},
    git::git_commit,
    local_state::LocalState,
    query::Query,
    task::Task,
    taskset::TaskSet,
    util::stdout_is_tty,
};
use chrono::Utc;
use std::io::{self, Write};
use termimad::MadSkin;

/// Add a new task to the task database
pub fn cmd_add(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    if query.text.is_empty() && query.template == 0 {
        return Err(RstaskError::Parse(
            "task description or template required".to_string(),
        ));
    }

    if !query.date_filter.is_empty() && query.date_filter != "in" && query.date_filter != "on" {
        return Err(RstaskError::Parse(
            "cannot use date filter with add command".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    if query.template > 0 {
        // Create task from template
        let template = ts.must_get_by_id(query.template).clone();
        let merged_query = query.merge(ctx);

        let task_summary = if !query.text.is_empty() {
            query.text.clone()
        } else {
            template.summary.clone()
        };

        let mut task = Task {
            write_pending: true,
            status: STATUS_PENDING.to_string(),
            summary: task_summary,
            tags: template.tags.clone(),
            project: template.project.clone(),
            priority: template.priority.clone(),
            due: template.due,
            notes: template.notes.clone(),
            ..Default::default()
        };

        task.modify(&merged_query);
        task = ts.must_load_task(task)?;
        ts.save_pending_changes()?;
        git_commit(&conf.repo, &format!("Added {}", task.summary), false)?;

        if template.status != STATUS_TEMPLATE {
            println!(
                "\nYou've copied an open task!\n\
                To learn more about creating templates enter 'rstask help template'\n"
            );
        }
    } else if !query.text.is_empty() {
        // Create new task from scratch
        ctx.print_context_description();
        let merged_query = query.merge(ctx);

        let mut task = Task {
            write_pending: true,
            status: STATUS_PENDING.to_string(),
            summary: merged_query.text.clone(),
            tags: merged_query.tags.clone(),
            project: merged_query.project.clone(),
            priority: merged_query.priority.clone(),
            due: merged_query.due,
            notes: merged_query.note.clone(),
            ..Default::default()
        };

        task = ts.must_load_task(task)?;
        ts.save_pending_changes()?;

        // Print feedback message
        println!("Added {}: {}", task.id, task.summary);

        git_commit(
            &conf.repo,
            &format!("Added {}: {}", task.id, task.summary),
            false,
        )?;
    }

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Set or display the current context
pub fn cmd_context(
    state: &mut LocalState,
    ctx: &Query,
    query: &Query,
    args: &[String],
) -> Result<()> {
    if args.len() < 2 {
        println!("{}", ctx);
    } else if args[1] == "none" {
        state.set_context(Query::default())?;
    } else {
        state.set_context(query.clone())?;
    }

    state.save()?;
    Ok(())
}

/// Mark tasks as done/resolved
pub fn cmd_done(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    if query.ids.is_empty() {
        return Err(RstaskError::Parse(
            "at least one task ID required".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    // iterate over IDs instead of filtering; it's clearer and enables us to
    // test each ID exists, and ignore context/operators
    for id in &query.ids {
        let task = ts.must_get_by_id(*id);

        if task.status == STATUS_RESOLVED {
            return Err(RstaskError::Other(format!(
                "task {} is already resolved",
                id
            )));
        }

        let mut task = task.clone();
        task.status = STATUS_RESOLVED.to_string();
        task.resolved = Some(Utc::now());
        task.write_pending = true;

        ts.must_update_task(task)?;
    }

    ts.save_pending_changes()?;

    let task_word = if query.ids.len() == 1 {
        "task"
    } else {
        "tasks"
    };
    git_commit(
        &conf.repo,
        &format!("Resolved {} {}", query.ids.len(), task_word),
        false,
    )?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Edit a task in $EDITOR
pub fn cmd_edit(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    use crate::util::edit_string;

    if query.ids.len() != 1 {
        return Err(RstaskError::Parse(
            "exactly one task ID required".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;
    let task = ts.must_get_by_id(query.ids[0]);

    // Serialize task to markdown with frontmatter for editing
    let markdown_str = crate::frontmatter::task_to_markdown(task)?;
    let edited = edit_string(&markdown_str)?;

    // Parse edited markdown
    let edited_task =
        crate::frontmatter::task_from_markdown(&edited, &task.uuid, &task.status, task.id)?;

    // Validate UUID hasn't changed (should be guaranteed by task_from_markdown)
    if edited_task.uuid != task.uuid {
        return Err(RstaskError::Parse("task ID must not be edited".to_string()));
    }

    let mut edited_task = edited_task;
    edited_task.write_pending = true;
    ts.must_update_task(edited_task)?;
    ts.save_pending_changes()?;
    git_commit(&conf.repo, "Edited task", false)?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Display help text
pub fn cmd_help(args: &[String]) {
    let cmd = if args.len() >= 3 {
        args[2].as_str()
    } else {
        ""
    };

    crate::help::show_help(cmd);
}

/// Log a completed task immediately
pub fn cmd_log(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    if query.text.is_empty() {
        return Err(RstaskError::Parse("task description required".to_string()));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    ctx.print_context_description();
    let merged_query = query.merge(ctx);

    let task = Task {
        write_pending: true,
        status: STATUS_RESOLVED.to_string(),
        summary: merged_query.text.clone(),
        tags: merged_query.tags.clone(),
        project: merged_query.project.clone(),
        priority: merged_query.priority.clone(),
        due: merged_query.due,
        resolved: Some(Utc::now()),
        ..Default::default()
    };

    let task = ts.must_load_task(task)?;
    ts.save_pending_changes()?;
    git_commit(&conf.repo, &format!("Added {}", task.summary), false)?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Modify existing tasks
/// Modify one or more tasks
pub fn cmd_modify(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    use crate::preferences::BulkCommitStrategy;

    if !query.has_operators() {
        return Err(RstaskError::Parse("no operations specified".to_string()));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    if query.ids.is_empty() {
        // Apply to all tasks in context
        ts.filter(ctx);

        if crate::util::stdout_is_tty() {
            let task_count = ts.tasks().len();
            crate::util::confirm_or_abort(&format!(
                "no IDs specified. Apply to all {} tasks in current context?",
                task_count
            ))?;
        }

        let tasks_to_modify: Vec<_> = ts.tasks().iter().map(|t| (*t).clone()).collect();
        let task_count = tasks_to_modify.len();

        for mut task in tasks_to_modify {
            task.modify(query);
            task.write_pending = true;
            ts.must_update_task(task.clone())?;
            ts.save_pending_changes()?;

            if conf.preferences.bulk_commit_strategy == BulkCommitStrategy::PerTask {
                git_commit(&conf.repo, &format!("Modified {}", task.summary), false)?;
            }
        }

        if conf.preferences.bulk_commit_strategy == BulkCommitStrategy::Single && task_count > 0 {
            let task_word = if task_count == 1 { "task" } else { "tasks" };
            git_commit(
                &conf.repo,
                &format!("Modified {} {}", task_count, task_word),
                false,
            )?;
        }
    } else {
        // Apply to specified task IDs
        let task_count = query.ids.len();

        for id in &query.ids {
            let task = ts.must_get_by_id(*id);
            let mut task = task.clone();
            task.modify(query);
            task.write_pending = true;
            ts.must_update_task(task.clone())?;
            ts.save_pending_changes()?;

            if conf.preferences.bulk_commit_strategy == BulkCommitStrategy::PerTask {
                git_commit(&conf.repo, &format!("Modified {}", task.summary), false)?;
            }
        }

        if conf.preferences.bulk_commit_strategy == BulkCommitStrategy::Single && task_count > 0 {
            let task_word = if task_count == 1 { "task" } else { "tasks" };
            git_commit(
                &conf.repo,
                &format!("Modified {} {}", task_count, task_word),
                false,
            )?;
        }
    }

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Show next/pending tasks (default view)
pub fn cmd_next(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    let filter_query = if !query.ids.is_empty() {
        // addressing task by ID, ignores context
        if query.has_operators() {
            return Err(RstaskError::Parse(
                "operators not valid when addressing task by ID".to_string(),
            ));
        }
        query.clone()
    } else {
        // apply context
        query.merge(ctx)
    };

    ts.filter(&filter_query);
    ts.display_by_next(ctx, true)?;

    Ok(())
}

/// Edit task notes in $EDITOR
pub fn cmd_note(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    use crate::util::edit_string;

    if query.ids.len() != 1 {
        return Err(RstaskError::Parse(
            "exactly one task ID required".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;
    let task = ts.must_get_by_id(query.ids[0]);

    // Edit notes (notes is already a String)
    let edited = edit_string(&task.notes)?;

    let mut task = task.clone();
    task.notes = edited;
    task.write_pending = true;

    ts.must_update_task(task)?;
    ts.save_pending_changes()?;
    git_commit(&conf.repo, "Updated task notes", false)?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Open/reopen tasks (move from resolved to pending)
pub fn cmd_open(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    if query.ids.is_empty() {
        return Err(RstaskError::Parse(
            "at least one task ID required".to_string(),
        ));
    }

    if query.has_operators() {
        return Err(RstaskError::Parse(
            "operators not valid in this context".to_string(),
        ));
    }

    let ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    for id in &query.ids {
        let task = ts.must_get_by_id(*id);

        // Extract URLs from task summary and notes
        let text = format!("{} {}", task.summary, task.notes);
        let urls = crate::util::extract_urls(&text);

        if urls.is_empty() {
            return Err(RstaskError::Other(format!(
                "no URLs found in task {}",
                task.id
            )));
        }

        for url in urls {
            crate::util::open_browser(&url)?;
        }
    }

    Ok(())
}

/// Remove/delete tasks
pub fn cmd_remove(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    if query.ids.is_empty() {
        return Err(RstaskError::Parse(
            "at least one task ID required".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    // Print tasks that will be removed (like Go version)
    for id in &query.ids {
        let task = ts.must_get_by_id(*id);
        println!("{}", task);
    }

    // Confirm deletion only if we have a TTY (interactive terminal)
    if stdout_is_tty() {
        println!();
        print!(
            "The above {} task(s) will be deleted without checking subtasks. Continue? (y/N): ",
            query.ids.len()
        );
        io::stdout().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;

        if response.trim().to_lowercase() != "y" {
            println!("Cancelled");
            return Ok(());
        }
    }

    for id in &query.ids {
        let uuid = ts.must_get_by_id(*id).uuid.clone();
        ts.delete_task(&uuid)?;
    }

    let task_word = if query.ids.len() == 1 {
        "task"
    } else {
        "tasks"
    };
    git_commit(
        &conf.repo,
        &format!("Removed {} {}", query.ids.len(), task_word),
        false,
    )?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Show active tasks
pub fn cmd_show_active(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, true)?;
    let merged_query = query.merge(ctx);

    ts.filter(&merged_query);
    ts.filter_by_status(STATUS_ACTIVE);
    ts.display_by_next(ctx, true)?;

    Ok(())
}

/// Show tasks grouped by project
pub fn cmd_show_projects(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, true)?;
    let merged_query = query.merge(ctx);

    ts.filter(&merged_query);
    ts.display_projects()?;

    Ok(())
}

/// Show open tasks (pending + active + paused)
pub fn cmd_show_open(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;
    let merged_query = query.merge(ctx);

    ts.filter(&merged_query);
    // Don't filter by status - open means not resolved
    ts.display_by_next(ctx, false)?;

    Ok(())
}

/// Show a single task with rendered markdown notes
pub fn cmd_show(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    let ts = TaskSet::load(&conf.repo, &conf.ids_file, true)?;

    // Get the task ID from the query
    if query.ids.is_empty() {
        return Err(RstaskError::Parse(
            "show command requires a task ID".to_string(),
        ));
    }

    let id = query.ids[0];
    let task = ts
        .get_by_id(id)
        .ok_or_else(|| RstaskError::TaskNotFound(format!("Task with ID {} not found", id)))?;

    // Display task metadata
    task.display();

    // Render notes with termimad if present
    if !task.notes.is_empty() {
        println!("\nNotes:");
        println!("{}", "─".repeat(80));

        let skin = MadSkin::default();
        skin.print_text(&task.notes);

        println!("{}", "─".repeat(80));
    }

    Ok(())
}

/// Show paused tasks
pub fn cmd_show_paused(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, true)?;
    let merged_query = query.merge(ctx);

    ts.filter(&merged_query);
    ts.filter_by_status(STATUS_PAUSED);
    ts.display_by_next(ctx, true)?;

    Ok(())
}

/// Show resolved tasks
pub fn cmd_show_resolved(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, true)?;
    let merged_query = query.merge(ctx);

    ts.unhide();
    ts.filter(&merged_query);
    ts.filter_by_status(STATUS_RESOLVED);
    ts.display_by_week()?;

    Ok(())
}

/// Show all tags in use
pub fn cmd_show_tags(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, true)?;
    let merged_query = query.merge(ctx);

    ts.filter(&merged_query);

    // Collect all unique tags
    let mut tags: Vec<String> = Vec::new();
    for task in ts.tasks() {
        if !task.filtered {
            for tag in &task.tags {
                if !tags.contains(tag) {
                    tags.push(tag.clone());
                }
            }
        }
    }

    tags.sort();
    for tag in tags {
        println!("{}", tag);
    }

    Ok(())
}

/// Show template tasks
pub fn cmd_show_templates(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    ts.unhide();
    ts.filter_by_status(STATUS_TEMPLATE);

    let merged_query = query.merge(ctx);
    ts.filter(&merged_query);
    ts.display_by_next(ctx, true)?;

    Ok(())
}

/// Show unorganised tasks (no project, no tags)
pub fn cmd_show_unorganised(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    // Go version explicitly rejects using query/context for show-unorganised
    if !query.ids.is_empty() || query.has_operators() {
        return Err(RstaskError::Other(
            "query/context not used for show-unorganised".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    // Don't filter by query or context - show ALL unorganised tasks
    ts.filter_unorganised();
    ts.display_by_next(ctx, true)?;

    Ok(())
}

/// Start/activate a task
pub fn cmd_start(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    if query.ids.is_empty() {
        return Err(RstaskError::Parse(
            "at least one task ID required".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    for id in &query.ids {
        let task = ts.must_get_by_id(*id);

        if task.status != STATUS_PENDING && task.status != STATUS_PAUSED {
            return Err(RstaskError::InvalidStatusTransition(
                task.status.clone(),
                STATUS_ACTIVE.to_string(),
            ));
        }

        let mut task = task.clone();
        task.status = STATUS_ACTIVE.to_string();
        task.write_pending = true;

        ts.must_update_task(task)?;
    }

    ts.save_pending_changes()?;

    let task_word = if query.ids.len() == 1 {
        "task"
    } else {
        "tasks"
    };
    git_commit(
        &conf.repo,
        &format!("Started {} {}", query.ids.len(), task_word),
        false,
    )?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Stop/pause an active task
pub fn cmd_stop(conf: &Config, _ctx: &Query, query: &Query) -> Result<()> {
    if query.ids.is_empty() {
        return Err(RstaskError::Parse(
            "at least one task ID required".to_string(),
        ));
    }

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    for id in &query.ids {
        let task = ts.must_get_by_id(*id);

        if task.status != STATUS_ACTIVE {
            return Err(RstaskError::InvalidStatusTransition(
                task.status.clone(),
                STATUS_PAUSED.to_string(),
            ));
        }

        let mut task = task.clone();
        task.status = STATUS_PAUSED.to_string();
        task.write_pending = true;

        ts.must_update_task(task)?;
    }

    ts.save_pending_changes()?;

    let task_word = if query.ids.len() == 1 {
        "task"
    } else {
        "tasks"
    };
    git_commit(
        &conf.repo,
        &format!("Stopped {} {}", query.ids.len(), task_word),
        false,
    )?;

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Sync repository with git remote
pub fn cmd_sync(repo_path: &str, quiet: bool) -> Result<String> {
    use crate::git::{git_pull, git_push};

    // Pull with fast-forward, creating merge commits if needed
    let pull_summary = git_pull(repo_path, quiet)?;

    // Push changes
    let push_summary = git_push(repo_path, quiet)?;

    Ok(format!("{}, {}", pull_summary, push_summary))
}

/// Automatically sync if configured to do so
fn auto_sync_if_enabled(conf: &Config) -> Result<()> {
    use crate::preferences::SyncFrequency;

    if conf.preferences.sync_frequency == SyncFrequency::AfterEveryModification {
        cmd_sync(conf.repo.to_str().unwrap(), false).map(|_| ())?;
    }

    Ok(())
}

/// Create a template task
pub fn cmd_template(conf: &Config, ctx: &Query, query: &Query) -> Result<()> {
    use crate::preferences::BulkCommitStrategy;

    let mut ts = TaskSet::load(&conf.repo, &conf.ids_file, false)?;

    if !query.ids.is_empty() {
        // Convert existing task(s) to template(s)
        let task_count = query.ids.len();

        for id in &query.ids {
            let task = ts.must_get_by_id(*id);
            let mut task = task.clone();
            task.status = STATUS_TEMPLATE.to_string();
            task.write_pending = true;
            ts.must_update_task(task.clone())?;

            if conf.preferences.bulk_commit_strategy == BulkCommitStrategy::PerTask {
                git_commit(
                    &conf.repo,
                    &format!("Changed {} to Template", task.summary),
                    false,
                )?;
            }
        }
        ts.save_pending_changes()?;

        if conf.preferences.bulk_commit_strategy == BulkCommitStrategy::Single && task_count > 0 {
            let task_word = if task_count == 1 { "task" } else { "tasks" };
            git_commit(
                &conf.repo,
                &format!("Changed {} {} to Template", task_count, task_word),
                false,
            )?;
        }
    } else if !query.text.is_empty() {
        // Create new template
        let merged_query = query.merge(ctx);

        let mut task = Task {
            write_pending: true,
            status: STATUS_TEMPLATE.to_string(),
            summary: merged_query.text.clone(),
            tags: merged_query.tags.clone(),
            project: merged_query.project.clone(),
            priority: merged_query.priority.clone(),
            due: merged_query.due,
            notes: merged_query.note.clone(),
            ..Default::default()
        };

        task = ts.must_load_task(task)?;
        ts.save_pending_changes()?;
        git_commit(
            &conf.repo,
            &format!("Created template: {}", task.summary),
            false,
        )?;
    } else {
        return Err(RstaskError::Parse(
            "task ID or description required for template".to_string(),
        ));
    }

    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Undo last git commit
pub fn cmd_undo(conf: &Config, args: &[String]) -> Result<()> {
    use crate::git::git_reset;

    // Default to 1 commit
    let count = if args.len() > 2 {
        args[2].parse::<usize>().unwrap_or(1)
    } else {
        1
    };

    for _ in 0..count {
        git_reset(&conf.repo)?;
    }

    println!("Undone {} commit(s)", count);
    auto_sync_if_enabled(conf)?;
    Ok(())
}

/// Display version information
pub fn cmd_version() {
    println!("rstask {}", env!("CARGO_PKG_VERSION"));
}
