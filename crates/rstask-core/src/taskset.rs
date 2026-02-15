// TaskSet - collection of tasks with filtering and loading capabilities
use crate::Result;
use crate::constants::*;
use crate::local_state::{load_ids, save_ids};
use crate::query::Query;
use crate::table::RowStyle;
use crate::task::{Task, unmarshal_task};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub tasks: usize,
    pub tasks_resolved: usize,
    pub active: bool,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub resolved: DateTime<Utc>,
    pub priority: String,
}

impl Project {
    pub fn style(&self) -> RowStyle {
        let mut style = RowStyle::default();

        if self.active {
            style.fg = FG_ACTIVE;
            style.bg = BG_ACTIVE;
        } else if self.priority == PRIORITY_CRITICAL {
            style.fg = FG_PRIORITY_CRITICAL;
        } else if self.priority == PRIORITY_HIGH {
            style.fg = FG_PRIORITY_HIGH;
        } else if self.priority == PRIORITY_LOW {
            style.fg = FG_PRIORITY_LOW;
        }

        style
    }
}

pub struct TaskSet {
    tasks: Vec<Task>,
    tasks_by_id: HashMap<i32, usize>,
    tasks_by_uuid: HashMap<String, usize>,
    ids_file_path: PathBuf,
    repo_path: PathBuf,
}

impl TaskSet {
    pub fn new(repo_path: PathBuf, ids_file_path: PathBuf) -> Self {
        TaskSet {
            tasks: Vec::new(),
            tasks_by_id: HashMap::new(),
            tasks_by_uuid: HashMap::new(),
            ids_file_path,
            repo_path,
        }
    }

    /// Loads tasks from the repository
    pub fn load(repo_path: &Path, ids_file_path: &Path, include_resolved: bool) -> Result<Self> {
        let mut ts = TaskSet::new(repo_path.to_path_buf(), ids_file_path.to_path_buf());
        let ids = load_ids(ids_file_path);

        let statuses = if include_resolved {
            ALL_STATUSES
        } else {
            NON_RESOLVED_STATUSES
        };

        for status in statuses {
            let dir = repo_path.join(status);

            if !dir.exists() {
                continue;
            }

            // Collect all entries first
            let mut entries: Vec<_> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();

            // Sort entries to prioritize .md files over .yml files
            // This ensures if both formats exist for the same task, .md is loaded
            entries.sort_by(|a, b| {
                let a_name = a.file_name();
                let b_name = b.file_name();
                let a_str = a_name.to_string_lossy();
                let b_str = b_name.to_string_lossy();

                // .md files should come before .yml files
                match (a_str.ends_with(".md"), b_str.ends_with(".md")) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a_str.cmp(&b_str),
                }
            });

            for entry in entries {
                let filename = entry.file_name();
                let filename_str = filename.to_string_lossy();

                // Skip hidden files
                if filename_str.starts_with('.') {
                    continue;
                }

                let path = entry.path();
                match unmarshal_task(&path, &filename_str, &ids, status) {
                    Ok(task) => {
                        ts.load_task(task)?;
                    }
                    Err(e) => {
                        eprintln!("Warning: error loading task: {}", e);
                    }
                }
            }
        }

        // hide some tasks by default. This is useful for things like templates and
        // recurring tasks which are shown either directly or with show- commands
        for task in &mut ts.tasks {
            if HIDDEN_STATUSES.contains(&task.status.as_str()) {
                task.filtered = true;
            }
        }

        Ok(ts)
    }

    /// Loads a task into the set
    pub fn load_task(&mut self, mut task: Task) -> Result<()> {
        task.normalise();

        if task.uuid.is_empty() {
            task.uuid = crate::util::must_get_uuid4_string();
        }

        task.validate()?;

        // Don't overwrite existing tasks
        if self.tasks_by_uuid.contains_key(&task.uuid) {
            return Ok(());
        }

        // Remove ID if already taken
        if task.id > 0 && self.tasks_by_id.contains_key(&task.id) {
            task.id = 0;
        }

        // Assign ID if needed (for non-resolved tasks)
        if task.id == 0 && task.status != STATUS_RESOLVED {
            for id in 1..=MAX_TASKS_OPEN as i32 {
                if !self.tasks_by_id.contains_key(&id) {
                    task.id = id;
                    break;
                }
            }
        }

        // Set created time if not set
        if task.created == DateTime::<Utc>::from_timestamp(0, 0).unwrap() {
            task.created = Utc::now();
            task.write_pending = true;
        }

        let idx = self.tasks.len();
        self.tasks_by_uuid.insert(task.uuid.clone(), idx);
        if task.id > 0 {
            self.tasks_by_id.insert(task.id, idx);
        }
        self.tasks.push(task);
        Ok(())
    }

    /// Assigns IDs to tasks
    pub fn assign_ids(&mut self) -> Result<()> {
        let mut ids = load_ids(&self.ids_file_path);
        let mut next_id = 1;

        // Find next available ID
        while ids.values().any(|&id| id == next_id) {
            next_id += 1;
        }

        for (idx, task) in self.tasks.iter_mut().enumerate() {
            if task.status != STATUS_RESOLVED && task.id == 0 {
                ids.insert(task.uuid.clone(), next_id);
                task.id = next_id;
                self.tasks_by_id.insert(next_id, idx);
                next_id += 1;
            }
        }

        save_ids(&self.ids_file_path, &ids)?;
        Ok(())
    }

    /// Filters tasks by a query
    pub fn filter(&mut self, query: &Query) {
        for task in &mut self.tasks {
            if !task.matches_filter(query) {
                task.filtered = true;
            }
        }
    }

    /// Returns unfiltered tasks only
    pub fn tasks(&self) -> Vec<&Task> {
        self.tasks.iter().filter(|t| !t.filtered).collect()
    }

    /// Returns all tasks regardless of filtered status
    pub fn all_tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// Returns mutable reference to tasks
    pub fn tasks_mut(&mut self) -> &mut Vec<Task> {
        &mut self.tasks
    }

    /// Saves all pending changes
    pub fn save_pending_changes(&mut self) -> Result<()> {
        let mut ids = std::collections::HashMap::new();

        for task in &mut self.tasks {
            if task.write_pending {
                task.save_to_disk(&self.repo_path)?;
            }

            // Build IDs map for all tasks with IDs
            if task.id > 0 {
                ids.insert(task.uuid.clone(), task.id);
            }
        }

        // Save IDs map to disk
        save_ids(&self.ids_file_path, &ids)?;
        Ok(())
    }

    /// Gets a task by ID
    pub fn get_by_id(&self, id: i32) -> Option<&Task> {
        self.tasks_by_id.get(&id).map(|&idx| &self.tasks[idx])
    }

    /// Gets a mutable task by ID
    pub fn get_by_id_mut(&mut self, id: i32) -> Option<&mut Task> {
        self.tasks_by_id
            .get(&id)
            .copied()
            .map(move |idx| &mut self.tasks[idx])
    }

    /// Gets a task by UUID
    pub fn get_by_uuid(&self, uuid: &str) -> Option<&Task> {
        self.tasks_by_uuid.get(uuid).map(|&idx| &self.tasks[idx])
    }

    /// Updates an existing task
    pub fn update_task(&mut self, mut task: Task) -> Result<()> {
        task.normalise();
        task.validate()?;

        let idx = *self
            .tasks_by_uuid
            .get(&task.uuid)
            .ok_or_else(|| crate::RstaskError::TaskNotFound(task.uuid.clone()))?;

        let old = &self.tasks[idx];

        // Validate status transition
        if old.status != task.status
            && !crate::constants::is_valid_status_transition(&old.status, &task.status)
        {
            return Err(crate::RstaskError::InvalidStatusTransition(
                old.status.clone(),
                task.status.clone(),
            ));
        }

        // Check for incomplete checklist
        if old.status != task.status
            && task.status == STATUS_RESOLVED
            && task.notes.contains("- [ ] ")
        {
            return Err(crate::RstaskError::Other(
                "Refusing to resolve task with incomplete checklist".to_string(),
            ));
        }

        // Clear ID for resolved tasks
        if task.status == STATUS_RESOLVED {
            task.id = 0;
        }

        // Assign a new ID when un-resolving (resolved -> non-resolved)
        if old.status == STATUS_RESOLVED && task.status != STATUS_RESOLVED && task.id == 0 {
            for id in 1..=MAX_TASKS_OPEN as i32 {
                if !self.tasks_by_id.contains_key(&id) {
                    task.id = id;
                    self.tasks_by_id.insert(id, idx);
                    break;
                }
            }
        }

        // Set resolved time
        if task.status == STATUS_RESOLVED && task.resolved.is_none() {
            task.resolved = Some(Utc::now());
        }

        // Clear resolved time when un-resolving
        if old.status == STATUS_RESOLVED && task.status != STATUS_RESOLVED {
            task.resolved = None;
        }

        task.write_pending = true;
        self.tasks[idx] = task;

        Ok(())
    }

    /// Sorts tasks by creation date (then by ID for stability)
    pub fn sort_by_created_ascending(&mut self) {
        self.tasks
            .sort_by(|a, b| a.created.cmp(&b.created).then_with(|| a.id.cmp(&b.id)));
    }

    pub fn sort_by_created_descending(&mut self) {
        self.tasks.sort_by(|a, b| b.created.cmp(&a.created));
    }

    /// Sorts tasks by priority (P0 > P1 > P2 > P3)
    pub fn sort_by_priority_ascending(&mut self) {
        self.tasks.sort_by(|a, b| a.priority.cmp(&b.priority));
    }

    pub fn sort_by_priority_descending(&mut self) {
        self.tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Sorts tasks by resolved date
    pub fn sort_by_resolved_ascending(&mut self) {
        self.tasks.sort_by(|a, b| match (a.resolved, b.resolved) {
            (Some(ar), Some(br)) => ar.cmp(&br),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    pub fn sort_by_resolved_descending(&mut self) {
        self.tasks.sort_by(|a, b| match (a.resolved, b.resolved) {
            (Some(ar), Some(br)) => br.cmp(&ar),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    /// Filters to show only specified status
    pub fn filter_by_status(&mut self, status: &str) {
        for task in &mut self.tasks {
            if task.status != status {
                task.filtered = true;
            }
        }
    }

    /// Filters to show only organized tasks (with tags or project)
    pub fn filter_organised(&mut self) {
        for task in &mut self.tasks {
            if task.tags.is_empty() && task.project.is_empty() {
                task.filtered = true;
            }
        }
    }

    /// Filters to show only unorganized tasks
    pub fn filter_unorganised(&mut self) {
        for task in &mut self.tasks {
            if !task.tags.is_empty() || !task.project.is_empty() {
                task.filtered = true;
            }
        }
    }

    /// Unhides tasks with hidden statuses
    pub fn unhide(&mut self) {
        for task in &mut self.tasks {
            if HIDDEN_STATUSES.contains(&task.status.as_str()) {
                task.filtered = false;
            }
        }
    }

    /// Gets all tags in use
    pub fn get_tags(&self) -> Vec<String> {
        let mut tagset = std::collections::HashSet::new();

        for task in self.tasks() {
            for tag in &task.tags {
                tagset.insert(tag.clone());
            }
        }

        let mut tags: Vec<String> = tagset.into_iter().collect();
        tags.sort();
        tags
    }

    /// Gets all projects with statistics
    pub fn get_projects(&self) -> Vec<Project> {
        let mut projects_map: HashMap<String, Project> = HashMap::new();

        for task in &self.tasks {
            if task.project.is_empty() {
                continue;
            }

            let project = projects_map
                .entry(task.project.clone())
                .or_insert_with(|| Project {
                    name: task.project.clone(),
                    tasks: 0,
                    tasks_resolved: 0,
                    active: false,
                    created: Utc::now(),
                    resolved: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
                    priority: PRIORITY_LOW.to_string(),
                });

            project.tasks += 1;

            if project.created == DateTime::<Utc>::from_timestamp(0, 0).unwrap()
                || task.created < project.created
            {
                project.created = task.created;
            }

            if let Some(task_resolved) = task.resolved
                && task_resolved > project.resolved
            {
                project.resolved = task_resolved;
            }

            if task.status == STATUS_RESOLVED {
                project.tasks_resolved += 1;
            }

            if task.status == STATUS_ACTIVE {
                project.active = true;
            }

            if task.status != STATUS_RESOLVED && task.priority < project.priority {
                project.priority = task.priority.clone();
            }
        }

        let mut names: Vec<String> = projects_map.keys().cloned().collect();
        names.sort();

        names
            .into_iter()
            .map(|name| projects_map.remove(&name).unwrap())
            .collect()
    }

    /// Returns the total number of tasks
    pub fn num_total(&self) -> usize {
        self.tasks.len()
    }

    // "Must" helper methods that panic on error (for commands that should exit on failure)

    /// Gets a task by ID, panics if not found
    pub fn must_get_by_id(&self, id: i32) -> &Task {
        self.get_by_id(id)
            .unwrap_or_else(|| panic!("task with ID {} not found", id))
    }

    /// Loads a task into the set, returns the loaded task, panics on error
    pub fn must_load_task(&mut self, mut task: Task) -> Result<Task> {
        // Generate UUID if needed before loading
        if task.uuid.is_empty() {
            task.uuid = crate::util::must_get_uuid4_string();
        }
        let uuid = task.uuid.clone();

        self.load_task(task)?;

        // Return the newly loaded task
        Ok(self
            .get_by_uuid(&uuid)
            .unwrap_or_else(|| panic!("task {} not found after loading", uuid))
            .clone())
    }

    /// Updates a task, panics on error
    pub fn must_update_task(&mut self, task: Task) -> Result<()> {
        self.update_task(task)
    }

    /// Apply modifications from a query to filtered tasks
    pub fn apply_modifications(&mut self, query: &Query) -> Result<()> {
        for task in &mut self.tasks {
            if !task.filtered {
                task.modify(query);
            }
        }
        Ok(())
    }

    /// Delete a task by UUID
    pub fn delete_task(&mut self, uuid: &str) -> Result<()> {
        let idx = *self
            .tasks_by_uuid
            .get(uuid)
            .ok_or_else(|| crate::RstaskError::TaskNotFound(uuid.to_string()))?;

        let task = &self.tasks[idx];

        // Delete from disk
        task.delete_from_disk(&self.repo_path)?;

        // Remove from in-memory structures
        let id = task.id;
        self.tasks.remove(idx);
        self.tasks_by_uuid.remove(uuid);
        if id > 0 {
            self.tasks_by_id.remove(&id);
        }

        // Rebuild indices since we removed an element
        self.rebuild_indices();

        Ok(())
    }

    /// Rebuild task indices after removal
    fn rebuild_indices(&mut self) {
        self.tasks_by_uuid.clear();
        self.tasks_by_id.clear();

        for (idx, task) in self.tasks.iter().enumerate() {
            self.tasks_by_uuid.insert(task.uuid.clone(), idx);
            if task.id > 0 {
                self.tasks_by_id.insert(task.id, idx);
            }
        }
    }
}
