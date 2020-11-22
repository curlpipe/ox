// Row.rs - Handling the rows of a document and their appearance
use crate::config::{Reader, TokenType};
use crate::editor::{RESET_BG, RESET_FG};
use crate::highlight::{highlight, remove_nested_tokens, Token};
use crate::util::{safe_ansi_insert, Exp};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Ensure we can use the Clone trait to copy row structs for manipulation
#[derive(Debug, Clone)]
pub struct Row {
    pub string: String,                   // For holding the contents of the row
    pub syntax: HashMap<usize, Token>,    // Hashmap for syntax
    pub bg_syntax: HashMap<usize, Token>, // Hashmap for background syntax colour
    pub updated: bool,                    // Line needs to be redrawn
    regex: Exp,                           // For holding the regex expression
}

// Implement a trait (similar method to inheritance) into the row
impl From<&str> for Row {
    fn from(s: &str) -> Self {
        // Initialise a row from a string
        Self {
            string: s.to_string(),
            syntax: HashMap::new(),
            bg_syntax: HashMap::new(),
            regex: Exp::new(),
            updated: true,
        }
    }
}

// Add methods to the Row struct / class
impl Row {
    pub fn render_line_number(config: &Reader, offset: usize, index: usize) -> String {
        let post_padding = offset.saturating_sub(
            index.to_string().len() +         // Length of the number
            config.general.line_number_padding_right + // Length of the right padding
            config.general.line_number_padding_left, // Length of the left padding
        );
        format!(
            "{}{}{}{}{}{}{}{}",
            if config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(config.theme.line_number_bg)
            },
            Reader::rgb_fg(config.theme.line_number_fg),
            " ".repeat(config.general.line_number_padding_left),
            " ".repeat(post_padding),
            index,
            " ".repeat(config.general.line_number_padding_right),
            Reader::rgb_fg(config.theme.editor_fg),
            if config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(config.theme.editor_bg)
            },
        )
    }
    pub fn render(
        &self,
        mut start: usize,
        width: usize,
        index: usize,
        offset: usize,
        config: &Reader,
    ) -> String {
        // Render the row by trimming it to the correct size
        let index = index.saturating_add(1);
        // Padding to align line numbers to the right
        // Assemble the line number data
        let line_number = Row::render_line_number(config, offset, index);
        // Strip ANSI values from the line
        let line_number_len = self.regex.ansi_len(&line_number);
        let width = width.saturating_sub(line_number_len);
        let reset_foreground = RESET_FG.to_string();
        let reset_background = RESET_BG.to_string();
        let editor_bg = Reader::rgb_bg(config.theme.editor_bg).to_string();
        let mut initial = start;
        let mut result = vec![];
        // Ensure that the render isn't impossible
        if width != 0 && start < UnicodeWidthStr::width(&self.string[..]) {
            // Calculate the character positions
            let end = width + start;
            let mut dna = HashMap::new();
            let mut cumulative = 0;
            // Collect the DNA from the unicode characters
            for ch in self.string.graphemes(true) {
                dna.insert(cumulative, ch);
                cumulative += UnicodeWidthStr::width(ch);
            }
            // Repair dodgy start
            if !dna.contains_key(&start) {
                result.push(" ");
                start += 1;
            }
            // Push across characters
            'a: while start < end {
                if let Some(t) = self.syntax.get(&start) {
                    // There is a token here
                    result.push(&t.kind);
                    while start < end && start < t.span.1 {
                        if let Some(ch) = dna.get(&start) {
                            // The character overlaps with the edge
                            if start + UnicodeWidthStr::width(*ch) > end {
                                result.push(" ");
                                break 'a;
                            }
                            result.push(ch);
                            start += UnicodeWidthStr::width(*ch);
                        } else {
                            break 'a;
                        }
                    }
                    result.push(&reset_foreground);
                } else if let Some(ch) = dna.get(&start) {
                    // There is a character here
                    if start + UnicodeWidthStr::width(*ch) > end {
                        result.push(" ");
                        break 'a;
                    }
                    result.push(ch);
                    start += UnicodeWidthStr::width(*ch);
                } else {
                    // The quota has been used up
                    break 'a;
                }
            }
            // Correct colourization of tokens that are half off the screen and half on the screen
            let initial_initial = initial; // Terrible variable naming, I know
            if initial > 0 {
                // Calculate the last token start boundary
                while self.syntax.get(&initial).is_none() && initial > 0 {
                    initial -= 1;
                }
                // Verify that the token actually exists
                if let Some(t) = self.syntax.get(&initial) {
                    // Verify that the token isn't up against the far left side
                    if t.span.0 != initial_initial && t.span.1 >= initial_initial {
                        // Insert the correct colours
                        let mut real = 0;
                        let mut ch = 0;
                        for i in &result {
                            if ch == t.span.1 - initial_initial {
                                break;
                            }
                            real += i.len();
                            ch += UnicodeWidthStr::width(*i);
                        }
                        result.insert(real, &reset_foreground);
                        result.insert(0, &t.kind);
                    }
                }
            }
            // Insert background tokens
            for b in &self.bg_syntax {
                let bg = if config.theme.transparent_editor {
                    &reset_background
                } else {
                    &editor_bg
                };
                if let Some(a) = safe_ansi_insert(b.1.span.0, &result, &self.regex.ansi) {
                    if a < result.len() {
                        result.insert(a, &b.1.kind);
                    }
                };
                if let Some(a) = safe_ansi_insert(b.1.span.1, &result, &self.regex.ansi) {
                    if a < result.len() {
                        result.insert(a, bg);
                    }
                };
            }
        }
        // Return the full line string to be rendered
        line_number + &result.join("")
    }
    pub fn update_syntax(
        &mut self,
        config: &Reader,
        syntax: &[TokenType],
        doc: &str,
        index: usize,
        theme: &str,
    ) {
        // Update the syntax highlighting indices for this row
        self.syntax = remove_nested_tokens(
            &highlight(
                &self.string,
                &doc,
                index,
                &syntax,
                &config.highlights[theme],
            ),
            &self.string,
        );
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
        self.updated = true;
        let mut before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(pos as usize).collect();
        before.push(ch);
        before.push_str(&after);
        self.string = before;
    }
    pub fn delete(&mut self, pos: usize) -> Option<char> {
        // Remove a character
        self.updated = true;
        let before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(1 + pos as usize).collect();
        let result: Option<char>;
        if let Some(c) = self.chars().get(pos) {
            if let Ok(c) = c.parse() {
                result = Some(c);
            } else {
                result = None;
            }
        } else {
            result = None;
        }
        self.string = before + &after;
        result
    }
}
