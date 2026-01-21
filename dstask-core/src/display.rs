use crate::constants::*;
use crate::query::Query;
use crate::table::{RowStyle, Table};
use crate::task::Task;
use crate::taskset::TaskSet;
use crate::util::{get_term_size, stdout_is_tty};
use crate::Result;
use chrono::{Datelike, Utc};

impl Task {
    /// Returns the row style for this task
    pub fn style(&self) -> RowStyle {
        let now = Utc::now();
        let mut style = RowStyle::default();
        let active = self.status == STATUS_ACTIVE;
        let paused = self.status == STATUS_PAUSED;
        let resolved = self.status == STATUS_RESOLVED;

        let get_fg = |normal_color: u8, active_color: u8| -> u8 {
            if active {
                active_color
            } else {
                normal_color
            }
        };

        // Determine foreground color based on priority and due date
        if self.priority == PRIORITY_CRITICAL {
            style.fg = get_fg(FG_PRIORITY_CRITICAL, FG_ACTIVE_PRIORITY_CRITICAL);
        } else if self.due.is_some() && self.due.unwrap() < now && !resolved {
            // Overdue tasks get high priority color
            style.fg = get_fg(FG_PRIORITY_HIGH, FG_ACTIVE_PRIORITY_HIGH);
        } else if self.priority == PRIORITY_HIGH {
            style.fg = get_fg(FG_PRIORITY_HIGH, FG_ACTIVE_PRIORITY_HIGH);
        } else if self.priority == PRIORITY_LOW {
            style.fg = get_fg(FG_PRIORITY_LOW, FG_ACTIVE_PRIORITY_LOW);
        } else {
            style.fg = get_fg(FG_DEFAULT, FG_ACTIVE);
        }

        // Determine background color
        if active {
            style.bg = BG_ACTIVE;
        } else if paused {
            style.bg = BG_PAUSED;
        }

        style
    }

    /// Displays a single task in detail
    pub fn display(&self) {
        let (w, _) = get_term_size();
        let mut table = Table::new(w, vec!["Name".to_string(), "Value".to_string()]);

        table.add_row(
            vec!["ID".to_string(), self.id.to_string()],
            RowStyle::default(),
        );
        table.add_row(
            vec!["Priority".to_string(), self.priority.clone()],
            RowStyle::default(),
        );
        table.add_row(
            vec!["Summary".to_string(), self.summary.clone()],
            RowStyle::default(),
        );
        table.add_row(
            vec!["Status".to_string(), self.status.clone()],
            RowStyle::default(),
        );
        table.add_row(
            vec!["Project".to_string(), self.project.clone()],
            RowStyle::default(),
        );
        table.add_row(
            vec!["Tags".to_string(), self.tags.join(", ")],
            RowStyle::default(),
        );
        table.add_row(
            vec!["UUID".to_string(), self.uuid.clone()],
            RowStyle::default(),
        );
        table.add_row(
            vec!["Created".to_string(), self.created.to_string()],
            RowStyle::default(),
        );

        if let Some(resolved) = self.resolved {
            table.add_row(
                vec!["Resolved".to_string(), resolved.to_string()],
                RowStyle::default(),
            );
        }

        if let Some(due) = self.due {
            table.add_row(
                vec!["Due".to_string(), due.to_string()],
                RowStyle::default(),
            );
        }

        table.render();
    }
}

impl TaskSet {
    /// Displays tasks in "next" view (by priority and creation date)
    pub fn display_by_next(&mut self, ctx: &Query, truncate: bool) -> Result<()> {
        self.sort_by_created_ascending();
        self.sort_by_priority_ascending();

        if stdout_is_tty() {
            ctx.print_context_description();
            self.render_table(truncate)?;

            // Count critical tasks
            let critical_in_view = self
                .tasks()
                .iter()
                .filter(|t| !t.filtered && t.priority == PRIORITY_CRITICAL)
                .count();

            let total_critical = self
                .tasks()
                .iter()
                .filter(|t| {
                    t.priority == PRIORITY_CRITICAL && !HIDDEN_STATUSES.contains(&t.status.as_str())
                })
                .count();

            if critical_in_view < total_critical {
                println!(
                    "\x1b[38;5;{}m{} critical task(s) outside this context! Use `dstask -- P0` to see them.\x1b[0m",
                    FG_PRIORITY_CRITICAL,
                    total_critical - critical_in_view
                );
            }

            Ok(())
        } else {
            self.render_json()
        }
    }

    /// Renders tasks as JSON
    pub fn render_json(&self) -> Result<()> {
        let tasks: Vec<_> = self
            .tasks()
            .iter()
            .filter(|t| !t.filtered)
            .map(|t| t.to_json())
            .collect();
        let json = serde_json::to_string_pretty(&tasks)?;
        println!("{}", json);
        Ok(())
    }

    /// Renders tasks as a table
    pub fn render_table(&self, truncate: bool) -> Result<()> {
        let tasks: Vec<&Task> = self.tasks().iter().filter(|t| !t.filtered).collect();
        let total = tasks.len();

        if self.tasks().is_empty() {
            println!("No tasks found. Run `dstask help` for instructions.");
            return Ok(());
        }

        if tasks.is_empty() {
            return Err(crate::DstaskError::Other(
                "No matching tasks in given context or filter.".to_string(),
            ));
        }

        if tasks.len() == 1 {
            let task = tasks[0];
            task.display();

            if !task.notes.is_empty() {
                println!(
                    "\nNotes on task {}:\n\x1b[38;5;245m{}\x1b[0m\n",
                    task.id, task.notes
                );
            }

            return Ok(());
        }

        // Multiple tasks - show as table
        let (w, h) = get_term_size();
        let max_tasks = (h.saturating_sub(TERMINAL_HEIGHT_MARGIN)).max(MIN_TASKS_SHOWN);

        let display_tasks = if truncate && max_tasks < tasks.len() {
            &tasks[..max_tasks]
        } else {
            &tasks[..]
        };

        let mut table = Table::new(
            w,
            vec![
                "ID".to_string(),
                "Priority".to_string(),
                "Tags".to_string(),
                "Due".to_string(),
                "Project".to_string(),
                "Summary".to_string(),
            ],
        );

        for task in display_tasks {
            let style = task.style();
            table.add_row(
                vec![
                    format!("{:<2}", task.id),
                    task.priority.clone(),
                    task.tags.join(" "),
                    task.parse_due_date_to_str(),
                    task.project.clone(),
                    task.long_summary(),
                ],
                style,
            );
        }

        table.render();

        if truncate && max_tasks < total {
            println!("\n{}/{} tasks shown.", max_tasks, total);
        } else {
            println!("\n{} tasks.", total);
        }

        Ok(())
    }

    /// Displays tasks grouped by week (for show-resolved)
    pub fn display_by_week(&mut self) -> Result<()> {
        self.sort_by_resolved_ascending();

        if stdout_is_tty() {
            let (w, _) = get_term_size();
            let mut table: Option<Table> = None;
            let mut last_week = 0;

            let tasks: Vec<&Task> = self.tasks().iter().filter(|t| !t.filtered).collect();

            for task in &tasks {
                if let Some(resolved) = task.resolved {
                    let week = resolved.iso_week().week();

                    if week != last_week {
                        if let Some(t) = table {
                            if !t.rows.is_empty() {
                                t.render();
                            }
                        }

                        println!(
                            "\n\n> Week {}, starting {}\n",
                            week,
                            resolved.format("%a %-d %b %Y")
                        );

                        table = Some(Table::new(
                            w,
                            vec![
                                "Resolved".to_string(),
                                "Priority".to_string(),
                                "Tags".to_string(),
                                "Due".to_string(),
                                "Project".to_string(),
                                "Summary".to_string(),
                            ],
                        ));
                    }

                    if let Some(ref mut t) = table {
                        t.add_row(
                            vec![
                                resolved.format("%a %-d").to_string(),
                                task.priority.clone(),
                                task.tags.join(" "),
                                task.parse_due_date_to_str(),
                                task.project.clone(),
                                task.long_summary(),
                            ],
                            task.style(),
                        );
                    }

                    last_week = week;
                }
            }

            if let Some(t) = table {
                t.render();
            }

            println!("{} tasks.", tasks.len());
            Ok(())
        } else {
            self.render_json()
        }
    }

    /// Displays projects
    pub fn display_projects(&self) -> Result<()> {
        if stdout_is_tty() {
            self.render_projects_table()
        } else {
            self.render_projects_json()
        }
    }

    fn render_projects_json(&self) -> Result<()> {
        let projects = self.get_projects();
        let json = serde_json::to_string_pretty(&projects)?;
        println!("{}", json);
        Ok(())
    }

    fn render_projects_table(&self) -> Result<()> {
        let projects = self.get_projects();
        let (w, _) = get_term_size();
        let mut table = Table::new(
            w,
            vec![
                "Name".to_string(),
                "Progress".to_string(),
                "Created".to_string(),
            ],
        );

        for project in projects {
            if project.tasks_resolved < project.tasks {
                table.add_row(
                    vec![
                        project.name.clone(),
                        format!("{}/{}", project.tasks_resolved, project.tasks),
                        project.created.format("%a %-d %b %Y").to_string(),
                    ],
                    project.style(),
                );
            }
        }

        table.render();
        Ok(())
    }
}
