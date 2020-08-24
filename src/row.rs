// Row.rs - Handling the rows of a document
use unicode_segmentation::UnicodeSegmentation; // For splitting up unicode
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr}; // Getting width of unicode characters

// Ensure we can use the Clone trait to copy row structs for manipulation
#[derive(Clone)]
pub struct Row {
    pub string: String, // For holding the contents of the row
}

// Implement a trait (similar method to inheritance) into the row
impl From<&str> for Row {
    fn from(s: &str) -> Self {
        // Initialise a row from a string
        Self {
            string: s.to_string(),
        }
    }
}

// Add methods to the Row struct / class
impl Row {
    pub fn render(&self, start: usize, end: usize) -> String {
        // Render the row by trimming it to the correct size
        trim_end(&trim_start(&self.string, start), end)
    }
    pub fn length(&self) -> usize {
        // Get the current length of the row
        UnicodeWidthStr::width(&self.string[..])
    }
    pub fn chars(&self) -> Vec<&str> {
        // Get the characters of the line
        self.string.graphemes(true).collect()
    }
    pub fn ext_chars(&self) -> Vec<&str> {
        // Produce a special list of characters depending on the widths of characters
        let mut result = Vec::new();
        for i in self.chars() {
            for _ in 0..UnicodeWidthStr::width(i) {
                result.push(i);
            }
        }
        result
    }
    pub fn get_jumps(&self) -> Vec<usize> {
        // Get the intervals of the unicode widths
        let mut result = Vec::new();
        for i in self.chars() {
            result.push(UnicodeWidthStr::width(i));
        }
        result
    }
    pub fn boundaries(&self) -> Vec<usize> {
        // Get the boundaries of the unicode widths
        let mut result = Vec::new();
        let mut count = 0;
        for i in self.get_jumps() {
            result.push(count);
            count += i;
        }
        result
    }
    pub fn insert(&mut self, ch: char, pos: usize) {
        // Insert a character
        let mut before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(pos as usize).collect();
        before.push(ch);
        before.push_str(&after);
        self.string = before;
    }
    pub fn delete(&mut self, pos: usize) {
        // Remove a character
        let before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(1 + pos as usize).collect();
        self.string = before + &after;
    }
}

fn trim_start(text: &str, start: usize) -> String {
    // Create a special vector with spaces inserted for trimming
    let mut widths = Vec::new();
    for i in text.chars() {
        widths.push(UnicodeWidthChar::width(i).unwrap());
    }
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

fn trim_end(text: &str, end: usize) -> String {
    // Trim a string with unicode in it to fit into a specific length
    let mut widths = Vec::new();
    for i in text.chars() {
        widths.push(UnicodeWidthChar::width(i).unwrap());
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
