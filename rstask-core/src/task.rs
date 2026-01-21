use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::constants::*;
use crate::date_util::format_due_date;
use crate::query::Query;
use crate::util::{is_valid_uuid4_string, must_get_repo_path};
use crate::{Result, RstaskError};

// Custom serialization module for DateTime fields to match Go's RFC3339 format
mod datetime_rfc3339 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

// Custom serialization for optional DateTime fields
mod optional_datetime_rfc3339 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    // Zero date constant matching Go's "0001-01-01T00:00:00Z"
    const ZERO_DATE_STR: &str = "0001-01-01T00:00:00Z";

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(dt) => serializer.serialize_str(&dt.to_rfc3339()),
            None => serializer.serialize_str(ZERO_DATE_STR),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == ZERO_DATE_STR || s.starts_with("0001-01-01") {
            Ok(None)
        } else {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| Some(dt.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom)
        }
    }
}

/// JSON representation of a task (matches Go version output)
#[derive(Debug, Clone, Serialize)]
pub struct TaskJson {
    pub uuid: String,
    pub status: String,
    pub id: i32,
    pub summary: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub project: String,
    pub priority: String,
    pub created: String,
    pub resolved: String,
    pub due: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubTask {
    pub summary: String,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Task {
    #[serde(skip)]
    pub uuid: String,

    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub status: String,

    #[serde(skip)]
    pub write_pending: bool,

    #[serde(skip)]
    pub id: i32,

    #[serde(skip)]
    pub deleted: bool,

    pub summary: String,

    #[serde(default)]
    pub notes: String,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(default)]
    pub project: String,

    #[serde(default)]
    pub priority: String,

    #[serde(default, rename = "delegatedto")]
    pub delegated_to: String,

    #[serde(default)]
    pub subtasks: Vec<SubTask>,

    #[serde(default)]
    pub dependencies: Vec<String>,

    #[serde(with = "datetime_rfc3339")]
    pub created: DateTime<Utc>,

    #[serde(with = "optional_datetime_rfc3339", default)]
    pub resolved: Option<DateTime<Utc>>,

    #[serde(with = "optional_datetime_rfc3339", default)]
    pub due: Option<DateTime<Utc>>,

    #[serde(skip)]
    pub filtered: bool,
}

impl Task {
    /// Creates a new task with default values
    pub fn new(summary: String) -> Self {
        Task {
            uuid: Uuid::new_v4().to_string(),
            status: STATUS_PENDING.to_string(),
            write_pending: true,
            id: 0,
            deleted: false,
            summary,
            notes: String::new(),
            tags: Vec::new(),
            project: String::new(),
            priority: PRIORITY_NORMAL.to_string(),
            delegated_to: String::new(),
            subtasks: Vec::new(),
            dependencies: Vec::new(),
            created: Utc::now(),
            resolved: None,
            due: None,
            filtered: false,
        }
    }

    /// Converts task to JSON representation (matches Go version)
    pub fn to_json(&self) -> TaskJson {
        TaskJson {
            uuid: self.uuid.clone(),
            status: self.status.clone(),
            id: self.id,
            summary: self.summary.clone(),
            notes: self.notes.clone(),
            tags: self.tags.clone(),
            project: self.project.clone(),
            priority: self.priority.clone(),
            created: self.created.to_rfc3339(),
            resolved: self
                .resolved
                .map(|r| r.to_rfc3339())
                .unwrap_or_else(|| "0001-01-01T00:00:00Z".to_string()),
            due: self
                .due
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "0001-01-01T00:00:00Z".to_string()),
        }
    }

    /// Checks equality of core properties (ignores ephemeral fields)
    pub fn equals(&self, other: &Task) -> bool {
        self.uuid == other.uuid
            && self.status == other.status
            && self.summary == other.summary
            && self.notes == other.notes
            && self.tags == other.tags
            && self.project == other.project
            && self.priority == other.priority
            && self.delegated_to == other.delegated_to
            && self.subtasks == other.subtasks
            && self.dependencies == other.dependencies
            && self.created == other.created
            && self.resolved == other.resolved
            && self.due == other.due
    }

    /// Checks if task matches a filter query
    pub fn matches_filter(&self, query: &Query) -> bool {
        // IDs were specified but none match
        if !query.ids.is_empty() && !query.ids.contains(&self.id) {
            return false;
        }

        // Must have all specified tags
        for tag in &query.tags {
            if !self.tags.contains(tag) {
                return false;
            }
        }

        // Must not have any anti-tags
        for tag in &query.anti_tags {
            if self.tags.contains(tag) {
                return false;
            }
        }

        // Must not be in anti-projects
        if query.anti_projects.contains(&self.project) {
            return false;
        }

        // Must match project if specified
        if !query.project.is_empty() && self.project != query.project {
            return false;
        }

        // Check due date filter
        if let Some(query_due) = &query.due {
            match self.due {
                None => return false,
                Some(task_due) => match query.date_filter.as_str() {
                    "after" if task_due < *query_due => return false,
                    "before" if task_due > *query_due => return false,
                    "on" | "in" if task_due.date_naive() != query_due.date_naive() => return false,
                    "" if task_due.date_naive() != query_due.date_naive() => return false,
                    _ => {}
                },
            }
        }

        // Check priority
        if !query.priority.is_empty() && self.priority != query.priority {
            return false;
        }

        // Check text search
        if !query.text.is_empty() {
            let search_text = query.text.to_lowercase();
            let summary_lower = self.summary.to_lowercase();
            let notes_lower = self.notes.to_lowercase();
            if !summary_lower.contains(&search_text) && !notes_lower.contains(&search_text) {
                return false;
            }
        }

        true
    }

    /// Normalizes task data (lowercase tags/project, sort, deduplicate)
    pub fn normalise(&mut self) {
        self.project = self.project.to_lowercase();

        // Lowercase all tags
        for tag in &mut self.tags {
            *tag = tag.to_lowercase();
        }

        // Sort tags
        self.tags.sort();

        // Deduplicate tags
        self.tags.dedup();

        // Resolved tasks should not have IDs
        if self.status == STATUS_RESOLVED {
            self.id = 0;
        }

        // Default priority
        if self.priority.is_empty() {
            self.priority = PRIORITY_NORMAL.to_string();
        }
    }

    /// Validates task data
    pub fn validate(&self) -> Result<()> {
        if !is_valid_uuid4_string(&self.uuid) {
            return Err(RstaskError::InvalidUuid(self.uuid.clone()));
        }

        if !is_valid_status(&self.status) {
            return Err(RstaskError::InvalidStatus(self.status.clone()));
        }

        if !is_valid_priority(&self.priority) {
            return Err(RstaskError::InvalidPriority(self.priority.clone()));
        }

        for dep_uuid in &self.dependencies {
            if !is_valid_uuid4_string(dep_uuid) {
                return Err(RstaskError::InvalidUuid(dep_uuid.clone()));
            }
        }

        Ok(())
    }

    /// Returns summary with last note if available
    pub fn long_summary(&self) -> String {
        let notes = self.notes.trim();
        if let Some(last_note) = notes.lines().last()
            && !last_note.is_empty()
        {
            return format!("{} {} {}", self.summary, NOTE_MODE_KEYWORD, last_note);
        }
        self.summary.clone()
    }

    /// Modifies task based on query
    pub fn modify(&mut self, query: &Query) {
        // Add tags
        for tag in &query.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }

        // Remove anti-tags
        self.tags.retain(|tag| !query.anti_tags.contains(tag));

        // Set project
        if !query.project.is_empty() {
            self.project = query.project.clone();
        }

        // Remove anti-projects
        if query.anti_projects.contains(&self.project) {
            self.project.clear();
        }

        // Set priority
        if !query.priority.is_empty() {
            self.priority = query.priority.clone();
        }

        // Set due date
        if let Some(due) = query.due {
            self.due = Some(due);
        }

        // Append note
        if !query.note.is_empty() {
            if !self.notes.is_empty() {
                self.notes.push('\n');
            }
            self.notes.push_str(&query.note);
        }

        self.write_pending = true;
    }

    /// Saves task to disk
    pub fn save_to_disk(&mut self, repo_path: &Path) -> Result<()> {
        self.write_pending = false;

        let filepath = must_get_repo_path(repo_path, &self.status, &format!("{}.yml", self.uuid));

        if self.deleted {
            // Delete the task file
            if filepath.exists() {
                std::fs::remove_file(&filepath)?;
            }
        } else {
            // Save task to disk
            // Create a copy and clear status for serialization
            let mut task_copy = self.clone();
            task_copy.status.clear();

            let yaml_data = serde_yaml::to_string(&task_copy)?;

            // Ensure directory exists
            if let Some(parent) = filepath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&filepath, yaml_data)?;
        }

        // Delete task from other status directories
        for status in ALL_STATUSES {
            if *status == self.status {
                continue;
            }

            let other_filepath =
                must_get_repo_path(repo_path, status, &format!("{}.yml", self.uuid));
            if other_filepath.exists() {
                std::fs::remove_file(&other_filepath)?;
            }
        }

        Ok(())
    }

    /// Deletes task from disk
    pub fn delete_from_disk(&self, repo_path: &Path) -> Result<()> {
        // Delete from current status directory
        let filepath = must_get_repo_path(repo_path, &self.status, &format!("{}.yml", self.uuid));
        if filepath.exists() {
            std::fs::remove_file(&filepath)?;
        }

        // Also check other status directories
        for status in ALL_STATUSES {
            if *status == self.status {
                continue;
            }
            let other_filepath =
                must_get_repo_path(repo_path, status, &format!("{}.yml", self.uuid));
            if other_filepath.exists() {
                std::fs::remove_file(&other_filepath)?;
            }
        }

        Ok(())
    }

    /// Parses due date to a display string
    pub fn parse_due_date_to_str(&self) -> String {
        match self.due {
            Some(due) => format_due_date(due.with_timezone(&chrono::Local)),
            None => String::new(),
        }
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.id > 0 {
            write!(f, "{}: {}", self.id, self.summary)
        } else {
            write!(f, "{}", self.summary)
        }
    }
}

/// Unmarshals a task from disk
pub fn unmarshal_task(
    path: &Path,
    filename: &str,
    ids: &std::collections::HashMap<String, i32>,
    status: &str,
) -> Result<Task> {
    if filename.len() != TASK_FILENAME_LEN {
        return Err(RstaskError::Parse(format!(
            "filename does not encode UUID {} (wrong length)",
            filename
        )));
    }

    let uuid = &filename[0..36];
    if !is_valid_uuid4_string(uuid) {
        return Err(RstaskError::Parse(format!(
            "filename does not encode UUID {}",
            filename
        )));
    }

    let id = ids.get(uuid).copied().unwrap_or(0);

    let data = std::fs::read_to_string(path)?;
    let mut task: Task = serde_yaml::from_str(&data)?;

    task.uuid = uuid.to_string();
    task.status = status.to_string();
    task.id = id;

    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_serialization_format() {
        let task = Task {
            summary: "test task".to_string(),
            tags: vec!["work".to_string()],
            project: "myproject".to_string(),
            priority: "P1".to_string(),
            notes: String::new(),
            delegated_to: String::new(),
            subtasks: Vec::new(),
            dependencies: Vec::new(),
            created: Utc::now(),
            resolved: None,
            due: None,
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&task).unwrap();
        eprintln!("YAML output:\n{}", yaml);

        // Check that created is in RFC3339 format (contains 'T' and timezone)
        assert!(yaml.contains("created:"));
        assert!(
            yaml.contains('T'),
            "created should be in RFC3339 format with 'T'"
        );
        assert!(
            yaml.contains("notes: ''") || yaml.contains("notes: \"\""),
            "notes should be serialized as empty string"
        );
        assert!(
            yaml.contains("delegatedto:"),
            "delegatedto field should exist"
        );
    }

    #[test]
    fn test_parse_go_yaml_with_local_timezone() {
        let go_yaml = r#"
summary: go created task
notes: ""
tags:
- work
project: myproject
priority: P1
delegatedto: ""
subtasks: []
dependencies: []
created: 2026-01-21T03:08:06.14017135+01:00
resolved: 0001-01-01T00:00:00Z
due: 0001-01-01T00:00:00Z
"#;

        let task: Task = serde_yaml::from_str(go_yaml).unwrap();
        eprintln!("Parsed task: {:?}", task);
        eprintln!("Created timestamp: {}", task.created.to_rfc3339());

        assert_eq!(task.summary, "go created task");
        assert_eq!(task.priority, "P1");
        assert!(task.resolved.is_none());
        assert!(task.due.is_none());
    }

    #[test]
    fn test_task_modify_adds_note() {
        let mut task = Task::new("Test".to_string());
        let query = Query {
            note: "Test Note".to_string(),
            ..Default::default()
        };
        task.modify(&query);
        assert_eq!(task.notes, "Test Note");
    }

    #[test]
    fn test_task_modify_appends_note() {
        let mut task = Task::new("Test".to_string());
        task.notes = "Start Note".to_string();
        let query = Query {
            note: "Query Note".to_string(),
            ..Default::default()
        };
        task.modify(&query);
        assert_eq!(task.notes, "Start Note\nQuery Note");
    }

    #[test]
    fn test_task_modify_priority() {
        let mut task = Task::new("Test".to_string());
        let query = Query {
            priority: "P1".to_string(),
            ..Default::default()
        };
        task.modify(&query);
        assert_eq!(task.priority, "P1");
    }

    #[test]
    fn test_task_modify_removes_project() {
        let mut task = Task::new("Test".to_string());
        task.project = "myproject".to_string();
        let query = Query {
            anti_projects: vec!["myproject".to_string()],
            ..Default::default()
        };
        task.modify(&query);
        assert_eq!(task.project, "");
    }

    #[test]
    fn test_task_normalise() {
        let mut task = Task::new("Test".to_string());
        task.project = "MyProject".to_string();
        task.tags = vec!["B".to_string(), "A".to_string(), "B".to_string()];
        task.normalise();

        assert_eq!(task.project, "myproject");
        assert_eq!(task.tags, vec!["a".to_string(), "b".to_string()]);
    }
}
