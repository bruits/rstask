use crate::Result;
use crate::constants::*;
use crate::date_util::parse_due_date_arg;
use crate::util::slice_contains;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Query {
    pub cmd: String,
    pub ids: Vec<i32>,
    pub tags: Vec<String>,
    pub anti_tags: Vec<String>,
    pub project: String,
    pub anti_projects: Vec<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    #[serde(default)]
    pub due: Option<DateTime<Utc>>,
    pub date_filter: String,
    pub priority: String,
    pub template: i32,
    pub text: String,
    pub ignore_context: bool,
    pub note: String,
}

impl Query {
    /// Creates an empty query
    pub fn new() -> Self {
        Self::default()
    }

    /// Prints context description with color
    pub fn print_context_description(&self) {
        let env_var_notification = if std::env::var("RSTASK_CONTEXT").is_ok() {
            " (set by RSTASK_CONTEXT)"
        } else {
            ""
        };

        let query_str = self.to_string();
        if !query_str.is_empty() {
            println!(
                "\x1b[33mActive context{}: {}\x1b[0m",
                env_var_notification, query_str
            );
        }
    }

    /// Returns true if the query has filter operators
    pub fn has_operators(&self) -> bool {
        !self.tags.is_empty()
            || !self.anti_tags.is_empty()
            || !self.project.is_empty()
            || !self.anti_projects.is_empty()
            || self.due.is_some()
            || !self.date_filter.is_empty()
            || !self.priority.is_empty()
            || self.template > 0
    }

    /// Merges another query into this one, used for applying context
    pub fn merge(&self, q2: &Query) -> Query {
        let mut q = self.clone();

        for tag in &q2.tags {
            if !q.tags.contains(tag) {
                q.tags.push(tag.clone());
            }
        }

        for tag in &q2.anti_tags {
            if !q.anti_tags.contains(tag) {
                q.anti_tags.push(tag.clone());
            }
        }

        if !q2.project.is_empty() {
            if !q.project.is_empty() && q.project != q2.project {
                panic!("Could not apply context, project conflict");
            }
            q.project = q2.project.clone();
        }

        if q2.due.is_some() {
            if q.due.is_some() && q.due != q2.due {
                panic!("Could not apply context, date filter conflict");
            }
            q.due = q2.due;
            q.date_filter = q2.date_filter.clone();
        }

        if !q2.priority.is_empty() {
            if !q.priority.is_empty() {
                panic!("Could not apply context, priority conflict");
            }
            q.priority = q2.priority.clone();
        }

        q
    }
}

/// Parses command line arguments into a Query
pub fn parse_query(args: &[String]) -> Result<Query> {
    let mut query = Query::new();
    let mut words = Vec::new();
    let mut notes_mode_activated = false;
    let mut notes = Vec::new();
    let mut ids_exhausted = false;
    let mut due_date_set = false;

    for item in args {
        let lc_item = item.to_lowercase();

        if notes_mode_activated {
            notes.push(item.clone());
            continue;
        }

        // Check for command
        if query.cmd.is_empty() && slice_contains(ALL_CMDS, &lc_item.as_str()) {
            query.cmd = lc_item;
            continue;
        }

        // Check for ID (only before any other token)
        if !ids_exhausted && let Ok(id) = item.parse::<i32>() {
            query.ids.push(id);
            continue;
        }

        // Check for special keywords
        if item == IGNORE_CONTEXT_KEYWORD {
            query.ignore_context = true;
        } else if item == NOTE_MODE_KEYWORD {
            notes_mode_activated = true;
        } else if let Some(proj) = lc_item.strip_prefix("project:") {
            if query.project.is_empty() {
                query.project = proj.to_string();
            }
        } else if let Some(proj) = lc_item.strip_prefix("+project:") {
            if query.project.is_empty() {
                query.project = proj.to_string();
            }
        } else if let Some(proj) = lc_item.strip_prefix("-project:") {
            query.anti_projects.push(proj.to_string());
        } else if lc_item.starts_with("due.") || lc_item.starts_with("due:") {
            if due_date_set {
                return Err(crate::RstaskError::Parse(
                    "Query should only have one due date".to_string(),
                ));
            }
            let (date_filter, due_date) = parse_due_date_arg(&lc_item)?;
            query.date_filter = date_filter;
            query.due = Some(due_date.with_timezone(&Utc));
            due_date_set = true;
        } else if let Some(template_str) = lc_item.strip_prefix("template:") {
            if let Ok(template_id) = template_str.parse::<i32>() {
                query.template = template_id;
            }
        } else if let Some(tag) = lc_item.strip_prefix('+') {
            if !tag.is_empty() {
                query.tags.push(tag.to_string());
            }
        } else if let Some(tag) = lc_item.strip_prefix('-') {
            if !tag.is_empty() {
                query.anti_tags.push(tag.to_string());
            }
        } else if query.priority.is_empty() && is_valid_priority(item) {
            query.priority = item.clone();
        } else {
            words.push(item.clone());
        }

        ids_exhausted = true;
    }

    query.text = words.join(" ");
    query.note = notes.join(" ");

    Ok(query)
}

impl fmt::Display for Query {
    /// Reconstructs the query as a string
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut args = Vec::new();

        for id in &self.ids {
            args.push(id.to_string());
        }

        for tag in &self.tags {
            args.push(format!("+{}", tag));
        }

        for tag in &self.anti_tags {
            args.push(format!("-{}", tag));
        }

        if !self.project.is_empty() {
            args.push(format!("project:{}", self.project));
        }

        for project in &self.anti_projects {
            args.push(format!("-project:{}", project));
        }

        if let Some(due) = &self.due {
            let mut due_arg = "due".to_string();
            if !self.date_filter.is_empty() {
                due_arg.push('.');
                due_arg.push_str(&self.date_filter);
            }
            due_arg.push(':');
            due_arg.push_str(&due.format("%Y-%m-%d").to_string());
            args.push(due_arg);
        }

        if !self.priority.is_empty() {
            args.push(self.priority.clone());
        }

        if self.template > 0 {
            args.push(format!("template:{}", self.template));
        }

        if !self.text.is_empty() {
            args.push(format!("\"{}\"", self.text));
        }

        write!(f, "{}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_basic() {
        let args = vec![
            "add".to_string(),
            "have".to_string(),
            "an".to_string(),
            "adventure".to_string(),
        ];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "add");
        assert_eq!(query.text, "have an adventure");
        assert!(query.tags.is_empty());
        assert!(query.anti_tags.is_empty());
    }

    #[test]
    fn test_parse_query_with_tags() {
        let args = vec![
            "add".to_string(),
            "+x".to_string(),
            "-y".to_string(),
            "have".to_string(),
            "an".to_string(),
            "adventure".to_string(),
        ];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "add");
        assert_eq!(query.tags, vec!["x".to_string()]);
        assert_eq!(query.anti_tags, vec!["y".to_string()]);
        assert_eq!(query.text, "have an adventure");
    }

    #[test]
    fn test_parse_query_with_note() {
        let args = vec![
            "add".to_string(),
            "floss".to_string(),
            "project:p".to_string(),
            "+health".to_string(),
            "/".to_string(),
            "every".to_string(),
            " day".to_string(),
        ];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "add");
        assert_eq!(query.project, "p");
        assert_eq!(query.tags, vec!["health".to_string()]);
        assert_eq!(query.text, "floss");
        assert_eq!(query.note, "every  day");
    }

    #[test]
    fn test_parse_query_with_id_and_modify() {
        let args = vec![
            "16".to_string(),
            "modify".to_string(),
            "+project:p".to_string(),
            "-project:x".to_string(),
            "-fun".to_string(),
        ];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "modify");
        assert_eq!(query.ids, vec![16]);
        assert_eq!(query.project, "p");
        assert_eq!(query.anti_projects, vec!["x".to_string()]);
        assert_eq!(query.anti_tags, vec!["fun".to_string()]);
    }

    #[test]
    fn test_parse_query_ignore_context() {
        let args = vec!["--".to_string(), "show-resolved".to_string()];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "show-resolved");
        assert!(query.ignore_context);
    }

    #[test]
    fn test_parse_query_priority() {
        let args = vec![
            "add".to_string(),
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
        ];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "add");
        assert_eq!(query.priority, "P1");
        assert_eq!(query.text, "P2 P3");
    }

    #[test]
    fn test_parse_query_template() {
        let args = vec![
            "add".to_string(),
            "My".to_string(),
            "Task".to_string(),
            "template:1".to_string(),
            "/".to_string(),
            "Test".to_string(),
            "Note".to_string(),
        ];
        let query = parse_query(&args).unwrap();

        assert_eq!(query.cmd, "add");
        assert_eq!(query.template, 1);
        assert_eq!(query.text, "My Task");
        assert_eq!(query.note, "Test Note");
    }
}
