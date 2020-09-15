use crate::Position;
use regex::Regex; // Regex engine
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr}; // Getting width of unicode characters

pub fn no_ansi_len(data: &str) -> usize {
    // Find the length of a string without ANSI values
    let ansi_scanner = Regex::new(r"\u{1b}\[[0-?]*[ -/]*[@-~]").unwrap();
    let data = ansi_scanner.replacen(data, 2, "");
    UnicodeWidthStr::width(&*data)
}

pub fn title(c: &str) -> String {
    if let Some(f) = c.chars().next() {
        f.to_uppercase().collect::<String>() + &c[1..]
    } else {
        String::new()
    }
}

pub fn trim_start(text: &str, start: usize) -> String {
    // Create a special vector with spaces inserted for trimming
    let widths: Vec<usize> = text
        .chars()
        .map(|i| {
            if let Some(i) = UnicodeWidthChar::width(i) {
                i
            } else {
                0
            }
        })
        .collect();
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut count = 0;
    for i in 0..chars.len() {
        for c in 0..widths[i] {
            if c == 0 {
                result.push(chars[i].to_string());
            } else if count <= start {
                result.push(" ".to_string());
            }
            count += 1;
        }
    }
    if let Some(result) = result.get(start..) {
        result.join("")
    } else {
        String::new()
    }
}

pub fn trim_end(text: &str, end: usize) -> String {
    // Trim a string with unicode in it to fit into a specific length
    let mut widths = Vec::new();
    for i in text.chars() {
        widths.push(if let Some(i) = UnicodeWidthChar::width(i) {
            i
        } else {
            0
        });
    }
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut length = 0;
    for i in 0..chars.len() {
        let chr = chars[i];
        let wid = widths[i];
        if length == end {
            return result.join("");
        } else if length + wid <= end {
            result.push(chr.to_string());
            length += wid;
        } else if length + wid > end {
            result.push(" ".to_string());
            return result.join("");
        }
    }
    result.join("")
}

pub fn is_behind(cursor: &Position, offset: &Position, position: &Position) -> bool {
    if position.y > cursor.y + offset.y {
        false
    } else {
        !(position.y == cursor.y + offset.y && cursor.x + offset.x <= position.x)
    }
}

pub fn is_ahead(cursor: &Position, offset: &Position, position: &Position) -> bool {
    if position.y < cursor.y + offset.y {
        false
    } else {
        !(position.y == cursor.y + offset.y && cursor.x + offset.x >= position.x)
    }
}
