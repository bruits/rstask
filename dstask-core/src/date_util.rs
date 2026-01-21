use crate::Result;
use chrono::{Datelike, Days, Local, NaiveDate, TimeZone, Weekday};

/// Returns the start of day (midnight) for a given time
pub fn start_of_day(t: chrono::DateTime<Local>) -> chrono::DateTime<Local> {
    Local
        .with_ymd_and_hms(t.year(), t.month(), t.day(), 0, 0, 0)
        .unwrap()
}

/// Parses weekday strings (full names and abbreviations)
fn weekday_str_to_time(date_str: &str, selector: &str) -> Option<chrono::DateTime<Local>> {
    let weekday = match date_str.to_lowercase().as_str() {
        "sun" | "sunday" => Weekday::Sun,
        "mon" | "monday" => Weekday::Mon,
        "tue" | "tues" | "tuesday" => Weekday::Tue,
        "wed" | "wednesday" => Weekday::Wed,
        "thu" | "thur" | "thurs" | "thursday" => Weekday::Thu,
        "fri" | "friday" => Weekday::Fri,
        "sat" | "saturday" => Weekday::Sat,
        _ => return None,
    };

    let now = Local::now();
    let now_weekday = now.weekday();
    let days_difference =
        weekday.num_days_from_monday() as i64 - now_weekday.num_days_from_monday() as i64;

    let target_date = match selector {
        "next" => start_of_day(now + Days::new((days_difference + 7) as u64)),
        "this" | "" => {
            if days_difference < 0 {
                start_of_day(now + Days::new((days_difference + 7) as u64))
            } else {
                start_of_day(now + Days::new(days_difference as u64))
            }
        }
        _ => return None,
    };

    Some(target_date)
}

/// Parses a date string into a DateTime
/// Supports: "today", "tomorrow", "yesterday", "[next-]monday", "YYYY-MM-DD", "MM-DD", "DD"
pub fn parse_str_to_date(date_str: &str) -> Result<chrono::DateTime<Local>> {
    let now = Local::now();
    let lower = date_str.trim().to_lowercase();

    match lower.as_str() {
        "today" => return Ok(start_of_day(now)),
        "tomorrow" => return Ok(start_of_day(now + Days::new(1))),
        "yesterday" => return Ok(start_of_day(now - Days::new(1))),
        _ => {}
    }

    // Check for next-[weekday], this-[weekday]
    if let Some((selector, rest)) = lower.split_once('-') {
        if let Some(date) = weekday_str_to_time(rest, selector) {
            return Ok(date);
        }
    }

    // Check for [weekday]
    if let Some(date) = weekday_str_to_time(&lower, "") {
        return Ok(date);
    }

    // Try YYYY-MM-DD
    if let Ok(naive_date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
        return Ok(Local
            .from_local_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap())
            .unwrap());
    }

    // Try MM-DD
    if let Ok(naive_date) = NaiveDate::parse_from_str(&date_str, "%m-%d") {
        let with_year = naive_date.with_year(now.year()).unwrap();
        return Ok(Local
            .from_local_datetime(&with_year.and_hms_opt(0, 0, 0).unwrap())
            .unwrap());
    }

    // Try DD (day of month)
    if let Ok(day) = date_str.parse::<u32>() {
        if day >= 1 && day <= 31 {
            if let Some(naive_date) = NaiveDate::from_ymd_opt(now.year(), now.month(), day) {
                return Ok(Local
                    .from_local_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap())
                    .unwrap());
            }
        }
    }

    Err(crate::DstaskError::Parse(format!(
        "Invalid due date format: {}\nExpected format: YYYY-MM-DD, MM-DD or DD, relative date like 'next-monday', 'today', etc.",
        date_str
    )))
}

/// Parses a due date argument like "due:today" or "due.before:2024-12-25"
pub fn parse_due_date_arg(due_str: &str) -> Result<(String, chrono::DateTime<Local>)> {
    let parts: Vec<&str> = due_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(crate::DstaskError::Parse(format!(
            "Invalid due query format: {}\nExpected format: due:YYYY-MM-DD, due:MM-DD, due:DD, due:next-monday, due:today, etc.",
            due_str
        )));
    }

    let date_str = parts[1];

    // Special case: overdue
    if date_str == "overdue" {
        return Ok(("before".to_string(), start_of_day(Local::now())));
    }

    // Check for date filter (due.before, due.after, etc.)
    let tag_parts: Vec<&str> = parts[0].splitn(2, '.').collect();
    let date_filter = if tag_parts.len() == 2 {
        let filter = tag_parts[1];
        match filter {
            "after" | "before" | "on" | "in" => filter.to_string(),
            _ => {
                return Err(crate::DstaskError::Parse(format!(
                    "Invalid date filter format: {}\nValid filters are: after, before, on, in",
                    filter
                )));
            }
        }
    } else {
        String::new()
    };

    let due_date = parse_str_to_date(date_str)?;
    Ok((date_filter, due_date))
}

/// Formats a due date for display
pub fn format_due_date(due: chrono::DateTime<Local>) -> String {
    let now = Local::now();

    if due.date_naive() == now.date_naive() {
        return "today".to_string();
    }

    if due.date_naive() == (now + Days::new(1)).date_naive() {
        return "tomorrow".to_string();
    }

    if due.date_naive() == (now - Days::new(1)).date_naive() {
        return "yesterday".to_string();
    }

    let days_until = (due.date_naive() - now.date_naive()).num_days();

    match days_until {
        0..=6 if due > now => {
            // Within a week in the future
            due.format("%a %-d").to_string()
        }
        _ if due.year() == now.year() => {
            // Same year
            due.format("%-d %b").to_string()
        }
        _ => {
            // Different year
            due.format("%-d %b %Y").to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_dates() {
        let today = start_of_day(Local::now());
        let tomorrow = start_of_day(Local::now() + Days::new(1));
        let yesterday = start_of_day(Local::now() - Days::new(1));

        assert_eq!(parse_str_to_date("today").unwrap(), today);
        assert_eq!(parse_str_to_date("tomorrow").unwrap(), tomorrow);
        assert_eq!(parse_str_to_date("yesterday").unwrap(), yesterday);
    }

    #[test]
    fn test_parse_absolute_dates() {
        let date = parse_str_to_date("2024-12-25").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 25);
    }

    #[test]
    fn test_parse_weekdays() {
        // These tests will pass as long as the weekday parsing works
        assert!(parse_str_to_date("monday").is_ok());
        assert!(parse_str_to_date("next-friday").is_ok());
        assert!(parse_str_to_date("this-wed").is_ok());
    }

    #[test]
    fn test_parse_due_date_arg() {
        let (filter, _date) = parse_due_date_arg("due:today").unwrap();
        assert_eq!(filter, "");

        let (filter, _date) = parse_due_date_arg("due.before:tomorrow").unwrap();
        assert_eq!(filter, "before");

        let (filter, _date) = parse_due_date_arg("due:overdue").unwrap();
        assert_eq!(filter, "before");
    }
}
