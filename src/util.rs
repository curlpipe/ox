use crate::{Position, Row};
use regex::Regex;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Clone)]
pub struct Exp {
    ansi: Regex,
}

impl Exp {
    pub fn new() -> Self {
        Self {
            ansi: Regex::new(r"\u{1b}\[[0-?]*[ -/]*[@-~]").unwrap(),
        }
    }
    pub fn ansi(&self, string: &str) -> String {
        // Find the length of a string without ANSI values
        (*self.ansi.replace(string, "")).to_string()
    }
    pub fn ansi_len(&self, string: &str) -> usize {
        // Find the length of a string without ANSI values
        UnicodeWidthStr::width(&*self.ansi.replacen(string, 2, ""))
    }
}

pub fn title(c: &str) -> String {
    c.chars().next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + &c[1..]
    })
}

pub fn trim_start(text: &str, start: usize) -> String {
    // Create a special vector with spaces inserted for trimming
    let widths: Vec<usize> = text
        .chars()
        .map(|i| UnicodeWidthChar::width(i).map_or(0, |i| i))
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
    result
        .get(start..)
        .map_or(String::new(), |result| result.join(""))
}

pub fn trim_end(text: &str, end: usize) -> String {
    // Trim a string with unicode in it to fit into a specific length
    let mut widths = Vec::new();
    for i in text.chars() {
        widths.push(UnicodeWidthChar::width(i).map_or(0, |i| i));
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

pub fn raw_to_grapheme(x: usize, string: &str) -> usize {
    let mut graphemes = 0;
    let current = Row::from(string);
    let jumps = current.get_jumps();
    let mut counter = 0;
    for (mut counter2, i) in jumps.into_iter().enumerate() {
        if counter == x {
            break;
        }
        counter2 += 1;
        graphemes = counter2;
        counter += i;
    }
    graphemes
}
