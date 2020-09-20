// Row.rs - Handling the rows of a document
use crate::config::{LINE_NUMBER_FG, LINE_NUMBER_PADDING, RESET_FG}; // Config stuff
use crate::util::{no_ansi_len, trim_end, trim_start}; // Utilities
use unicode_segmentation::UnicodeSegmentation; // For splitting up unicode
use unicode_width::UnicodeWidthStr; // Getting width of unicode strings // Bring in the utilities

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
    pub fn render(&self, start: usize, end: usize, index: usize, offset: usize) -> String {
        // Render the row by trimming it to the correct size
        let index = index.saturating_add(1);
        let post_padding = offset.saturating_sub(index.to_string().len() + LINE_NUMBER_PADDING);
        let line_number = format!(
            "{}{}{}{}{}",
            LINE_NUMBER_FG,
            " ".repeat(post_padding),
            index,
            " ".repeat(LINE_NUMBER_PADDING.saturating_add(1)),
            RESET_FG,
        );
        let line_number_len = no_ansi_len(&line_number);
        let body = trim_end(
            &trim_start(&self.string, start),
            end.saturating_sub(line_number_len),
        );
        line_number + &body
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
            result.resize(result.len() + UnicodeWidthStr::width(i), i);
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
    pub fn delete(&mut self, pos: usize) -> Option<char> {
        // Remove a character
        let before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(1 + pos as usize).collect();
        let result: Option<char>;
        if let Some(c) = self.chars().get(pos) {
            result = Some(c.parse().unwrap());
        } else {
            result = None;
        }
        self.string = before + &after;
        result
    }
}
