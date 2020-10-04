// Row.rs - Handling the rows of a document and their appearance
use crate::config::Reader; // For configuration
use crate::editor::RESET_FG; // Reset colours
use crate::util::{trim_end, trim_start, Exp}; // Utilities
use regex::Regex;
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation; // For splitting up unicode
use unicode_width::UnicodeWidthStr; // Getting width of unicode characters

// Enum for border type of token
pub enum Token {
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
            Self::digits_in_number(index) +         // Digits of the number
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
        let mut body;
        // Strip ANSI values from the line
        let line_number_len = self.regex.ansi_len(&line_number);
        // Unpack the syntax highlighting information
        if let Some(syntax) = syntax {
            let tokens = Row::tokenize(&self.string, &syntax);
            // Trim the line to fit into the terminal width
            body = trim_end(
                &trim_start(&self.string, start),
                end.saturating_sub(line_number_len),
            );
            body = Row::highlight(&body, &tokens, &config.syntax.highlights);
        } else {
            body = trim_end(
                &trim_start(&self.string, start),
                end.saturating_sub(line_number_len),
            );
        }
        // Return the full line string to be rendered
        line_number + &body
    }
    pub fn highlight(
        body: &str,
        bounds: &HashMap<usize, Vec<(Token, String)>>,
        highlights: &HashMap<String, (u8, u8, u8)>,
    ) -> String {
        let mut result = String::new();
        let mut active = false;
        let mut level = 0;
        let mut pushed = false;
        for (i, c) in body.graphemes(true).enumerate() {
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
                    let cap = Row::bounds(&cap, &line);
                    token_bounds
                        .get_mut(&cap.0)
                        .unwrap()
                        .push((Token::Start, (*name).to_string()));
                    token_bounds
                        .get_mut(&cap.1)
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
    fn bounds(reg: &regex::Match, line: &str) -> (usize, usize) {
        // Work out the width of the capture
        let unicode_wid = reg.as_str().graphemes(true).count();
        let pre_start = &line[..reg.start()];
        let pre_length = pre_start.graphemes(true).count();
        // Calculate the correct boundaries for syntax highlighting
        (pre_length, pre_length + unicode_wid)
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
    /*
     We make the parameter mutable so that we can modify it in place instead
     of copying it in a local variable
    */
    const fn digits_in_number(mut index: usize)->usize {
        /*
         If the number is 0, then there's still a digit and we return 1 (which is the number of digits still)
         and that would have the same behavior as converting the number to a string
          and we return early so that we skip looping in that special case
        */
        if index == 0 {
            return 1;
        }
        let mut digit_count = 0;
        while index != 0 {
            /*
             Each time we succesfully divde by 10 we have another digit to add
             to the count; of course we divide in place so that we reduce
             our number by 10% each time we loop https://www.geeksforgeeks.org/program-count-digits-integer-3-different-methods/
            */
            index /= 10;
            digit_count += 1;
        }
        digit_count
    }
}
