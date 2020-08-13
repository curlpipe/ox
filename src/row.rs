/*
    Row.rs - For managing the individual rows of the editor

    This includes a struct (which is a class) which handles
    unicode character widths
*/

use std::str::Chars; // Trait to get characters
use unicode_segmentation::UnicodeSegmentation; // For splitting unicode characters
use unicode_width::UnicodeWidthStr; // For getting the width of unicode characters

// Our row struct
#[derive(Clone)]
pub struct Row {
    pub string: String,    // To hold the contents of the row
    pub jumps: Vec<usize>, // To hold the grapheme boundaries
}

// Add methods to this struct
impl Row {
    pub fn new(row: String) -> Self {
        // Create a new row instance
        let mut seg = Vec::new();
        let graphemes = UnicodeSegmentation::graphemes(&row[..], true).collect::<Vec<&str>>();
        for g in graphemes {
            seg.push(UnicodeWidthStr::width(g));
        }
        Self {
            string: row,
            jumps: seg,
        }
    }
    pub fn update_jumps(&mut self) {
        // Update the grapheme jumps after editing
        let mut seg = Vec::new();
        let graphemes =
            UnicodeSegmentation::graphemes(&self.string[..], true).collect::<Vec<&str>>();
        for g in graphemes {
            seg.push(UnicodeWidthStr::width(g));
        }
        self.jumps = seg;
    }
    pub fn raw_length(&self) -> usize {
        // Return the raw length of the string
        UnicodeWidthStr::width(&self.string[..])
    }
    pub fn length(&self) -> usize {
        // Return the number of unicode characters
        self.jumps.len()
    }
    pub fn chars(&self) -> Chars {
        // Obtain the individual characters from the row
        self.string.chars()
    }
}
