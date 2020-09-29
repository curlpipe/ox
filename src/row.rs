// Row.rs - Handling the rows of a document and their appearance
use crate::config::Reader; // For configuration
use crate::editor::RESET_FG; // Reset colours
use crate::util::{trim_end, trim_start, Exp}; // Utilities
use regex::Regex;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation; // For splitting up unicode
use unicode_width::UnicodeWidthStr; // Getting width of unicode characters

// Enum for border type of token
enum Token {
    Start,
    Stop,
}

// Ensure we can use the Clone trait to copy row structs for manipulation
#[derive(Debug, Clone)]
pub struct Row {
    pub string: String,     // For holding the contents of the row
    pub syn_string: String, // String to be rendered
    regex: Exp,             // For holding the regex expression
}

// Implement a trait (similar method to inheritance) into the row
impl From<&str> for Row {
    fn from(s: &str) -> Self {
        // Initialise a row from a string
        Self {
            string: s.to_string(),
            syn_string: s.to_string(),
            regex: Exp::new(),
        }
    }
}

// Add methods to the Row struct / class
impl Row {
    pub fn render(
        &self,
        start: usize,
        end: usize,
        index: usize,
        offset: usize,
        config: &Reader,
        syntax: &Option<HashMap<String, Vec<Regex>>>,
    ) -> String {
        // Render the row by trimming it to the correct size
        let index = index.saturating_add(1);
        // Padding to align line numbers to the right
        let post_padding = offset.saturating_sub(
            index.to_string().len() +         // Length of the number
            config.general.line_number_padding_right + // Length of the right padding
            config.general.line_number_padding_left, // Length of the left padding
        );
        // Assemble the line number data
        let line_number = format!(
            "{}{}{}{}{}{}",
            Reader::rgb_fg(config.theme.line_number_fg),
            " ".repeat(config.general.line_number_padding_left),
            " ".repeat(post_padding),
            index,
            " ".repeat(config.general.line_number_padding_right),
            RESET_FG,
        );
        // Strip ANSI values from the line
        let line_number_len = self.regex.ansi_len(&line_number);
        // Trim the line to fit into the terminal width
        let mut body = trim_end(
            &trim_start(&self.string, start),
            end.saturating_sub(line_number_len),
        );
        // Unpack the syntax highlighting information
        if let Some(syntax) = syntax {
            body = self.highlight(body, syntax, &config.syntax.highlights);
        }
        // Return the full line string to be rendered
        line_number + &body
    }
    pub fn highlight(
        &self,
        body: String,
        syntax: &HashMap<String, Vec<Regex>>,
        highlights: &HashMap<String, (u8, u8, u8)>,
    ) -> String {
        let bounds = self.tokenize(&body, &syntax);
        let mut result = String::new();
        let mut active = false;
        let mut level = 0;
        let mut pushed = false;
        for (i, c) in body.chars().enumerate() {
            if let Some(token) = bounds.get(&i) {
                for t in token {
                    match t.0 {
                        Token::Start => {
                            if active {
                                level += 1;
                                if !pushed {
                                    result += &c.to_string();
                                    pushed = true;
                                }
                            } else {
                                active = true;
                                result += &format!("{}", Reader::rgb_fg(highlights[&t.1]));
                                if !pushed {
                                    result += &c.to_string();
                                    pushed = true;
                                }
                            }
                        }
                        Token::Stop => {
                            if active && level == 0 {
                                result += &format!("{}", RESET_FG);
                                if token.len() == 1 && !pushed {
                                    result += &c.to_string();
                                    pushed = true;
                                }
                                active = false;
                            } else {
                                level -= 1;
                                if !pushed {
                                    result += &c.to_string();
                                    pushed = true;
                                }
                            }
                        }
                    }
                }
                pushed = false;
            } else {
                result += &c.to_string();
            }
        }
        result
    }
    fn tokenize(
        &self,
        line: &str,
        syntax: &HashMap<String, Vec<Regex>>,
    ) -> HashMap<usize, Vec<(Token, String)>> {
        // Find the token boundaries using the regex
        let mut token_bounds = HashMap::new();
        for i in 0..=line.len() {
            token_bounds.insert(i, vec![]);
        }
        for (name, regex) in syntax {
            for ex in regex {
                for m in ex.captures_iter(&line) {
                    let cap = m.get(m.len().saturating_sub(1)).unwrap();
                    token_bounds
                        .get_mut(&cap.start())
                        .unwrap()
                        .push((Token::Start, (*name).to_string()));
                    token_bounds
                        .get_mut(&cap.end())
                        .unwrap()
                        .push((Token::Stop, (*name).to_string()));
                }
            }
        }
        token_bounds
            .into_iter()
            .filter(|x| !x.1.is_empty())
            .collect::<HashMap<usize, Vec<(Token, String)>>>()
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
