use crate::constants::*;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Default)]
pub struct RowStyle {
    pub mode: u8,
    pub fg: u8,
    pub bg: u8,
}

pub struct Table {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub row_styles: Vec<RowStyle>,
    pub width: usize,
}

impl Table {
    pub fn new(width: usize, header: Vec<String>) -> Self {
        let w = width.min(TABLE_MAX_WIDTH);

        Table {
            header,
            rows: Vec::new(),
            row_styles: vec![RowStyle {
                mode: MODE_HEADER,
                fg: 0,
                bg: 0,
            }],
            width: w,
        }
    }

    pub fn add_row(&mut self, row: Vec<String>, style: RowStyle) {
        if row.len() != self.header.len() {
            panic!(
                "Row length {} doesn't match header length {}",
                row.len(),
                self.header.len()
            );
        }
        self.rows.push(row);
        self.row_styles.push(style);
    }

    pub fn render(&self) {
        let mut original_widths = vec![0; self.header.len()];

        // Calculate widths from data rows
        for row in &self.rows {
            for (j, cell) in row.iter().enumerate() {
                let width = UnicodeWidthStr::width(cell.as_str());
                if original_widths[j] < width {
                    original_widths[j] = width;
                }
            }
        }

        // Account for header cells
        for (j, cell) in self.header.iter().enumerate() {
            let width = UnicodeWidthStr::width(cell.as_str());
            if original_widths[j] < width {
                original_widths[j] = width;
            }
        }

        // Initialize with original sizes
        let mut widths = original_widths.clone();

        // Account for gaps
        let width_budget = self
            .width
            .saturating_sub(TABLE_COL_GAP * (self.header.len() - 1));

        // Iteratively reduce widths to fit budget
        while widths.iter().sum::<usize>() > width_budget {
            // Find max width column
            let (max_idx, &max_width) = widths.iter().enumerate().max_by_key(|(_, w)| *w).unwrap();

            if max_width == 0 {
                break;
            }

            widths[max_idx] -= 1;
        }

        // Combine header and rows
        let mut all_rows = vec![self.header.clone()];
        all_rows.extend(self.rows.clone());

        // Render each row
        for (i, row) in all_rows.iter().enumerate() {
            let style = &self.row_styles[i];

            let mode = if style.mode == 0 {
                MODE_DEFAULT
            } else {
                style.mode
            };
            let fg = if style.fg == 0 { FG_DEFAULT } else { style.fg };
            let bg = if style.bg == 0 {
                if i % 2 != 0 {
                    BG_DEFAULT_1
                } else {
                    BG_DEFAULT_2
                }
            } else {
                style.bg
            };

            let mut cells = Vec::new();
            for (j, cell) in row.iter().enumerate() {
                let trimmed = fix_str(cell, widths[j]);

                // Support ' / ' markup for notes
                let final_cell = if trimmed.contains(&format!(" {} ", NOTE_MODE_KEYWORD)) {
                    let with_note_color = trimmed.replace(
                        &format!(" {} ", NOTE_MODE_KEYWORD),
                        &format!("\x1b[38;5;{}m ", FG_NOTE),
                    );
                    format!("{}\x1b[38;5;{}m", with_note_color, fg)
                } else {
                    trimmed
                };

                cells.push(final_cell);
            }

            let line = cells.join(&" ".repeat(TABLE_COL_GAP));
            println!("\x1b[{};38;5;{};48;5;{}m{}\x1b[0m", mode, fg, bg, line);
        }
    }
}

/// Fixes a string to a specific width, truncating or padding as needed
pub fn fix_str(text: &str, width: usize) -> String {
    // Remove anything after newline
    let text = text.split('\n').next().unwrap_or("");

    let current_width = UnicodeWidthStr::width(text);

    if current_width <= width {
        // Pad with spaces
        format!("{}{}", text, " ".repeat(width - current_width))
    } else {
        // Truncate with ellipsis
        truncate_with_ellipsis(text, width)
    }
}

/// Truncates a string to fit within width, adding " " at the end
fn truncate_with_ellipsis(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    if width == 1 {
        return " ".to_string();
    }

    let mut result = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let char_width = UnicodeWidthStr::width(ch.to_string().as_str());

        if current_width + char_width + 1 > width {
            // Need to add ellipsis
            result.push(' ');
            break;
        }

        result.push(ch);
        current_width += char_width;
    }

    // Pad to exact width
    while UnicodeWidthStr::width(result.as_str()) < width {
        result.push(' ');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_str_padding() {
        assert_eq!(fix_str("hello", 10), "hello     ");
    }

    #[test]
    fn test_fix_str_truncation() {
        let result = fix_str("hello world", 8);
        assert_eq!(UnicodeWidthStr::width(result.as_str()), 8);
        assert!(result.ends_with(' '));
    }

    #[test]
    fn test_fix_str_newline() {
        assert_eq!(fix_str("hello\nworld", 10), "hello     ");
    }
}
