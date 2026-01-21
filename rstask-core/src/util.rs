use crate::Result;
use crate::constants::*;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use uuid::Uuid;

/// Prints an error message in red and exits
pub fn exit_fail(msg: &str) -> ! {
    eprintln!("\x1b[31m{}\x1b[0m", msg);
    std::process::exit(1);
}

/// Asks for user confirmation or exits
pub fn confirm_or_abort(msg: &str) -> Result<()> {
    eprint!("{} [y/n] ", msg);
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let normalized = input.trim().to_lowercase();
    if normalized == "y" || normalized == "yes" {
        Ok(())
    } else {
        exit_fail("Aborted.");
    }
}

/// Generates a new UUID v4 string
pub fn must_get_uuid4_string() -> String {
    Uuid::new_v4().to_string()
}

/// Validates a UUID v4 string
pub fn is_valid_uuid4_string(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

/// Runs a command with stdin/stdout/stderr inherited
pub fn run_cmd(name: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(name)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(crate::RstaskError::Other(format!(
            "Command {} failed with status: {}",
            name, status
        )));
    }

    Ok(())
}

/// Creates a temporary filename for editing
pub fn make_temp_filename(id: i32, summary: &str, ext: &str) -> String {
    let mut truncated = String::new();
    let mut prev_was_hyphen = true; // Start true to skip leading hyphens

    for c in summary.chars().take(21) {
        // Skip multi-byte UTF-8 characters
        if c.len_utf8() != 1 {
            continue;
        }

        // Skip punctuation
        if c.is_ascii_punctuation() {
            continue;
        }

        // Convert spaces and other non-alphanumeric to hyphens
        if !c.is_alphanumeric() {
            if !prev_was_hyphen {
                truncated.push('-');
                prev_was_hyphen = true;
            }
            continue;
        }

        truncated.push(c);
        prev_was_hyphen = false;
    }

    let lowered = truncated.to_lowercase();
    format!("rstask.*.{}-{}.{}", id, lowered, ext)
}

/// Opens an editor to edit bytes, returns the edited content
pub fn must_edit_bytes(data: &[u8], tmp_filename: &str) -> Result<Vec<u8>> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    let editor_parts: Vec<&str> = editor.split_whitespace().collect();

    if editor_parts.is_empty() {
        return Err(crate::RstaskError::Other("EDITOR is empty".to_string()));
    }

    let mut tmpfile = tempfile::Builder::new()
        .prefix("")
        .suffix(tmp_filename)
        .tempfile()?;

    tmpfile.write_all(data)?;
    tmpfile.flush()?;

    let path = tmpfile.path().to_path_buf();

    let mut cmd = Command::new(editor_parts[0]);
    if editor_parts.len() > 1 {
        cmd.args(&editor_parts[1..]);
    }
    cmd.arg(&path);

    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(crate::RstaskError::Other(
            "Failed to run $EDITOR".to_string(),
        ));
    }

    let edited = std::fs::read(&path)?;
    Ok(edited)
}

/// Opens an editor to edit a string, returns the edited content
pub fn edit_string(content: &str) -> Result<String> {
    let bytes = must_edit_bytes(content.as_bytes(), "rstask-edit.txt")?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Checks if a slice contains an item
pub fn slice_contains<T: PartialEq>(haystack: &[T], needle: &T) -> bool {
    haystack.contains(needle)
}

/// Checks if subset is contained in superset
pub fn slice_contains_all(subset: &[String], superset: &[String]) -> bool {
    subset.iter().all(|item| superset.contains(item))
}

/// Deduplicates strings in a vector (preserves order)
pub fn deduplicate_strings(strings: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    strings.retain(|s| seen.insert(s.clone()));
}

/// Opens a URL in the default browser
pub fn must_open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    let cmd = "xdg-open";

    #[cfg(target_os = "windows")]
    let cmd = "cmd";

    #[cfg(target_os = "macos")]
    let cmd = "open";

    #[cfg(target_os = "windows")]
    let args = ["/c", "start", "", url];

    #[cfg(not(target_os = "windows"))]
    let args = [url];

    Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| crate::RstaskError::Other("Failed to open browser".to_string()))?;

    Ok(())
}

/// Gets terminal size (width, height)
pub fn get_term_size() -> (usize, usize) {
    terminal_size::terminal_size()
        .map(|(w, h)| (w.0 as usize, h.0 as usize))
        .unwrap_or((80, 24))
}

/// Checks if stdout is a TTY
pub fn stdout_is_tty() -> bool {
    *FAKE_PTY || termion::is_tty(&std::io::stdout())
}

/// Gets the repository path for a given status
pub fn get_repo_path(repo: &std::path::Path, status: &str) -> std::path::PathBuf {
    repo.join(status)
}

/// Gets the repository path or exits on error
pub fn must_get_repo_path(
    repo: &std::path::Path,
    status: &str,
    filename: &str,
) -> std::path::PathBuf {
    repo.join(status).join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_uuid4_string() {
        assert!(is_valid_uuid4_string(
            "550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(!is_valid_uuid4_string("invalid-uuid"));
        assert!(!is_valid_uuid4_string(""));
    }

    #[test]
    fn test_make_temp_filename() {
        assert_eq!(make_temp_filename(1, "& &", "md"), "rstask.*.1-.md");
        assert_eq!(
            make_temp_filename(99, "A simple summary!", "md"),
            "rstask.*.99-a-simple-summary.md"
        );
        assert_eq!(
            make_temp_filename(1, "& that's that.", "md"),
            "rstask.*.1-thats-that.md"
        );
        assert_eq!(
            make_temp_filename(2147483647, "J's $100, != €100", "md"),
            "rstask.*.2147483647-js-100-100.md"
        );
    }

    #[test]
    fn test_slice_contains_all() {
        assert!(slice_contains_all(&[], &[]));
        assert!(slice_contains_all(
            &["one".to_string()],
            &["one".to_string()]
        ));
        assert!(!slice_contains_all(
            &["one".to_string()],
            &["two".to_string()]
        ));
        assert!(!slice_contains_all(&["one".to_string()], &[]));
        assert!(slice_contains_all(
            &["one".to_string()],
            &["one".to_string(), "two".to_string()]
        ));
        assert!(slice_contains_all(
            &["two".to_string(), "one".to_string()],
            &["three".to_string(), "one".to_string(), "two".to_string()]
        ));
        assert!(!slice_contains_all(
            &["apple".to_string(), "two".to_string(), "one".to_string()],
            &["three".to_string(), "one".to_string(), "two".to_string()]
        ));
        assert!(slice_contains_all(
            &[],
            &["three".to_string(), "one".to_string(), "two".to_string()]
        ));
    }

    #[test]
    fn test_deduplicate_strings() {
        let mut vec = vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
            "c".to_string(),
        ];
        deduplicate_strings(&mut vec);
        assert_eq!(vec, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
