mod common;

use chrono::{Datelike, Local, NaiveDate, Weekday};

// Helper functions for date handling

fn get_test_date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn get_current_date() -> NaiveDate {
    Local::now().date_naive()
}

fn get_relative_date(days: i64) -> NaiveDate {
    Local::now()
        .date_naive()
        .checked_add_signed(chrono::Duration::days(days))
        .unwrap()
}

fn get_next_weekday(weekday: Weekday) -> NaiveDate {
    let now = Local::now().date_naive();
    let current_weekday = now.weekday();
    let days_until =
        (weekday.num_days_from_monday() as i64 - current_weekday.num_days_from_monday() as i64 + 7)
            % 7;
    let days_until = if days_until == 0 { 7 } else { days_until };
    now.checked_add_signed(chrono::Duration::days(days_until))
        .unwrap()
}

fn assert_date_equal(expected: NaiveDate, actual: NaiveDate, msg: &str) {
    assert_eq!(
        expected, actual,
        "{}: expected {}, got {}",
        msg, expected, actual
    );
}

#[test]
fn test_add_task_with_full_date() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task with full date", "due:2025-07-01"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_test_date(2025, 7, 1), task_due, "Full date parsing");
    assert_eq!(tasks[0].summary, "Task with full date");
}

#[test]
fn test_add_task_with_month_day() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task with month-day", "due:07-01"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_due = tasks[0].due.unwrap().date_naive();
    let current_year = Local::now().year();
    assert_date_equal(
        get_test_date(current_year, 7, 1),
        task_due,
        "Month-day parsing",
    );
    assert_eq!(tasks[0].summary, "Task with month-day");
}

#[test]
fn test_add_task_with_day() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task with day only", "due:15"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_due = tasks[0].due.unwrap().date_naive();
    let now = Local::now().date_naive();
    assert_date_equal(
        get_test_date(now.year(), now.month(), 15),
        task_due,
        "Day-only parsing",
    );
    assert_eq!(tasks[0].summary, "Task with day only");
}

#[test]
fn test_add_task_with_today() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task due today", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task_due, "Today keyword");
    assert_eq!(tasks[0].summary, "Task due today");
}

#[test]
fn test_add_task_with_yesterday() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task due yesterday", "due:yesterday"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_relative_date(-1), task_due, "Yesterday keyword");
    assert_eq!(tasks[0].summary, "Task due yesterday");
}

#[test]
fn test_add_task_with_tomorrow() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task due tomorrow", "due:tomorrow"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_relative_date(1), task_due, "Tomorrow keyword");
    assert_eq!(tasks[0].summary, "Task due tomorrow");
}

#[test]
fn test_add_task_with_weekdays() {
    let (_repo, cmd) = test_setup!();

    let weekdays = [
        ("monday", Weekday::Mon),
        ("tuesday", Weekday::Tue),
        ("wednesday", Weekday::Wed),
        ("thursday", Weekday::Thu),
        ("friday", Weekday::Fri),
        ("saturday", Weekday::Sat),
        ("sunday", Weekday::Sun),
    ];

    for (weekday_name, weekday) in &weekdays {
        let result = cmd.run(&[
            "add",
            &format!("Task due {}", weekday_name),
            &format!("due:{}", weekday_name),
        ]);
        result.assert_success();

        let result = cmd.run(&["next", &format!("due:{}", weekday_name)]);
        result.assert_success();

        let tasks = result.parse_tasks();
        let task_due = tasks[0].due.unwrap().date_naive();
        let expected_date = get_next_weekday(*weekday);
        assert_date_equal(
            expected_date,
            task_due,
            &format!("Weekday parsing: {}", weekday_name),
        );
        assert_eq!(tasks[0].summary, format!("Task due {}", weekday_name));
    }
}

#[test]
fn test_filter_tasks_by_exact_date() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:2025-07-01"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 2", "due:2025-08-01"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 3"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:2025-07-01"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_test_date(2025, 7, 1), task_due, "Exact date filter");
    assert_eq!(tasks[0].summary, "Task 1");
}

#[test]
fn test_filter_tasks_by_today() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task due today", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task due tomorrow", "due:tomorrow"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:today"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task_due, "Filter by today");
    assert_eq!(tasks[0].summary, "Task due today");
}

#[test]
fn test_filter_tasks_by_overdue() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Overdue task", "due:yesterday"]);
    result.assert_success();

    let result = cmd.run(&["add", "Today task", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["add", "Future task", "due:tomorrow"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:overdue"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].summary, "Overdue task");
    assert_eq!(tasks[1].summary, "Today task");
}

#[test]
fn test_filter_tasks_by_this_weekdays() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "This Monday task", "due:this-monday"]);
    result.assert_success();

    let result = cmd.run(&["add", "Next Monday task", "due:this-friday"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:this-monday"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].summary, "This Monday task");
}

#[test]
fn test_filter_tasks_by_next_weekdays() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Next Monday task", "due:next-monday"]);
    result.assert_success();

    let result = cmd.run(&["add", "This Monday task", "due:next-friday"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:next-monday"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].summary, "Next Monday task");
}

#[test]
fn test_filter_tasks_due_after() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:yesterday"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 2", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 3", "due:tomorrow"]);
    result.assert_success();

    let result = cmd.run(&["next", "due.after:today"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].summary, "Task 2");
    assert_eq!(tasks[1].summary, "Task 3");
    let task1_due = tasks[0].due.unwrap().date_naive();
    let task2_due = tasks[1].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task1_due, "After filter - task 2");
    assert_date_equal(get_relative_date(1), task2_due, "After filter - task 3");
}

#[test]
fn test_filter_tasks_due_before() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:yesterday"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 2", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 3", "due:tomorrow"]);
    result.assert_success();

    let result = cmd.run(&["next", "due.before:today"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].summary, "Task 1");
    let task1_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_relative_date(-1), task1_due, "Before filter - task 1");
    assert_eq!(tasks[1].summary, "Task 2");
    let task2_due = tasks[1].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task2_due, "Before filter - task 2");
}

#[test]
fn test_filter_tasks_due_on() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:yesterday"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 2", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 3", "due:tomorrow"]);
    result.assert_success();

    let result = cmd.run(&["next", "due.on:today"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].summary, "Task 2");
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task_due, "On filter");
}

#[test]
fn test_filter_tasks_due_after_with_full_date() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:2025-06-01"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 2", "due:2025-07-01"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 3", "due:2025-08-01"]);
    result.assert_success();

    let result = cmd.run(&["next", "due.after:2025-06-15"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].summary, "Task 2");
    assert_eq!(tasks[1].summary, "Task 3");
}

#[test]
fn test_modify_command_with_due_dates() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:2025-06-01"]);
    result.assert_success();

    let result = cmd.run(&["modify", "1", "due:2025-06-18"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:2025-06-18"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_test_date(2025, 6, 18), task_due, "Modified due date");
}

#[test]
fn test_templates_with_due_dates() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["template", "Template 1", "due:2025-10-31"]);
    result.assert_success();

    let result = cmd.run(&["add", "template:1", "task with due date from template"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:2025-10-31"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_test_date(2025, 10, 31), task_due, "Template due date");
}

#[test]
fn test_due_dates_merge_with_context() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["context", "due:2025-09-01", "+work"]);
    result.assert_success();

    let result = cmd.run(&["add", "new task with context"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:2025-09-01"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_test_date(2025, 9, 1), task_due, "Context due date");
    assert_eq!(tasks[0].summary, "new task with context");
    assert!(tasks[0].tags.contains(&"work".to_string()));
}

#[test]
fn test_next_command_shows_due_dates() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task without due date"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task with due date", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 2);

    let task_with_due = tasks
        .iter()
        .find(|t| t.summary == "Task with due date")
        .expect("Task with due date should exist");

    let task_due = task_with_due.due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task_due, "Next shows due dates");
}

#[test]
fn test_show_resolved_displays_due_dates() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Completed task", "due:today"]);
    result.assert_success();

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    let task_id = tasks[0].id.to_string();

    let result = cmd.run(&["done", &task_id]);
    result.assert_success();

    let result = cmd.run(&["show-resolved"]);
    result.assert_success();

    let resolved_tasks = result.parse_tasks();
    assert_eq!(resolved_tasks.len(), 1);
    assert_eq!(resolved_tasks[0].summary, "Completed task");
    let task_due = resolved_tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task_due, "Resolved task due date");
}

#[test]
fn test_invalid_date_formats() {
    let (_repo, cmd) = test_setup!();

    let invalid_formats = [
        "due:invalid-date",
        "due:13-32",
        "due:2025-13-01",
        "due:2025-02-30",
        "due:32",
        "due:next-funday",
        "due:this-xyz",
        "due.afber:today",
    ];

    let mut failed_count = 0;
    for format in &invalid_formats {
        let result = cmd.run(&["add", "Task with invalid date", format]);
        if !result.success() {
            failed_count += 1;
        }
    }

    assert_eq!(
        failed_count,
        invalid_formats.len(),
        "All invalid formats should fail"
    );
}

#[test]
fn test_case_insensitive_due_keywords() {
    let (_repo, cmd) = test_setup!();

    let case_variations = ["TODAY", "Today", "TOMORROW", "Tomorrow", "MONDAY", "Monday"];

    let mut failed_count = 0;
    for variation in &case_variations {
        let result = cmd.run(&[
            "add",
            &format!("Task with {}", variation),
            &format!("due:{}", variation),
        ]);
        if !result.success() {
            failed_count += 1;
        }
    }

    let result = cmd.run(&["next"]);
    result.assert_success();

    let tasks = result.parse_tasks();

    for task in &tasks {
        assert!(task.due.is_some(), "Task should have due date set");
    }
    assert_eq!(failed_count, 0, "All case variations should succeed");
}

#[test]
fn test_combined_due_filters() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["add", "Task 1", "due:today", "+urgent"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 2", "due:tomorrow", "+urgent"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 3", "due:today", "+normal"]);
    result.assert_success();

    let result = cmd.run(&["next", "due:today", "+urgent"]);
    result.assert_success();

    let tasks = result.parse_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].summary, "Task 1");
    let task_due = tasks[0].due.unwrap().date_naive();
    assert_date_equal(get_current_date(), task_due, "Combined filters");
}

#[test]
fn test_multiple_due_dates() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&[
        "add",
        "Task with multiple due dates",
        "due:today",
        "due:tomorrow",
    ]);

    assert!(
        !result.success(),
        "Multiple due dates should not be allowed"
    );
}

#[test]
fn test_add_multiple_due_dates_with_context() {
    let (_repo, cmd) = test_setup!();

    let result = cmd.run(&["context", "due:today", "+urgent"]);
    result.assert_success();

    let result = cmd.run(&["add", "Task 1", "due:tomorrow"]);

    assert!(
        !result.success(),
        "Cannot add task with due date when context has due date"
    );
}
