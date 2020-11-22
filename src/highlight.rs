// Highlight.rs - For syntax highlighting
use crate::config::{Reader, TokenType};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

// Tokens for storing syntax highlighting info
#[derive(Debug, Clone)]
pub struct Token {
    pub span: (usize, usize),
    pub data: String,
    pub kind: String,
    pub priority: usize,
}

pub fn cine(token: &Token, hashmap: &mut HashMap<usize, Token>) {
    // Insert a token into a hashmap
    if let Some(t) = hashmap.get(&token.span.0) {
        if t.priority > token.priority {
            return;
        }
    }
    hashmap.insert(token.span.0, token.clone());
}

fn bounds(reg: &regex::Match, line: &str) -> (usize, usize) {
    // Work out the width of the capture
    let unicode_width = UnicodeWidthStr::width(reg.as_str());
    let pre_length = UnicodeWidthStr::width(&line[..reg.start()]);
    // Calculate the correct boundaries for syntax highlighting
    (pre_length, pre_length + unicode_width)
}

fn multi_to_single(doc: &str, m: &regex::Match) -> ((usize, usize), (usize, usize)) {
    // Multiline tokens to single line tokens
    let b = bounds(&m, &doc);
    let start_y = doc[..m.start()].matches('\n').count();
    let end_y = doc[..m.end()].matches('\n').count();
    let start_x = b.0
        - UnicodeWidthStr::width(&doc.split('\n').take(start_y).collect::<Vec<_>>().join("\n")[..]);
    let end_x = b.1
        - UnicodeWidthStr::width(&doc.split('\n').take(end_y).collect::<Vec<_>>().join("\n")[..]);
    ((start_x, start_y), (end_x, end_y))
}

pub fn highlight(
    row: &str,
    doc: &str,
    index: usize,
    regex: &[TokenType],
    highlights: &HashMap<String, (u8, u8, u8)>,
) -> HashMap<usize, Token> {
    // Generate syntax highlighting information
    let mut syntax: HashMap<usize, Token> = HashMap::new();
    if regex.is_empty() {
        // Language not found, return empty hashmap
        return syntax;
    }
    for exps in regex {
        match exps {
            TokenType::SingleLine(name, regex) => {
                if name == "keywords" {
                    for kw in regex {
                        // Locate keywords
                        for cap in kw.captures_iter(row) {
                            let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                            let boundaries = bounds(&cap, &row);
                            cine(
                                &Token {
                                    span: boundaries,
                                    data: cap.as_str().to_string(),
                                    kind: Reader::rgb_fg(highlights["keywords"]).to_string(),
                                    priority: 0,
                                },
                                &mut syntax,
                            );
                        }
                    }
                } else {
                    for exp in regex {
                        // Locate expressions
                        for cap in exp.captures_iter(row) {
                            let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                            let boundaries = bounds(&cap, &row);
                            cine(
                                &Token {
                                    span: boundaries,
                                    data: cap.as_str().to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 1,
                                },
                                &mut syntax,
                            );
                        }
                    }
                }
            }
            TokenType::MultiLine(name, regex) => {
                // Multiline token
                for exp in regex {
                    for cap in exp.captures_iter(doc) {
                        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                        let ((start_x, start_y), (end_x, end_y)) = multi_to_single(&doc, &cap);
                        if start_y == index {
                            cine(
                                &Token {
                                    span: (
                                        start_x,
                                        if start_y == end_y {
                                            end_x
                                        } else {
                                            UnicodeWidthStr::width(row)
                                        },
                                    ),
                                    data: row.to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 2,
                                },
                                &mut syntax,
                            )
                        } else if end_y == index {
                            cine(
                                &Token {
                                    span: (0, end_x),
                                    data: row.to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 2,
                                },
                                &mut syntax,
                            )
                        } else if (start_y..=end_y).contains(&index) {
                            cine(
                                &Token {
                                    span: (0, UnicodeWidthStr::width(row)),
                                    data: row.to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 2,
                                },
                                &mut syntax,
                            )
                        }
                    }
                }
            }
        }
    }
    syntax
}

pub fn remove_nested_tokens(tokens: &HashMap<usize, Token>, line: &str) -> HashMap<usize, Token> {
    // Remove tokens within tokens
    let mut result = HashMap::new();
    let mut c = 0;
    // While the line still isn't full
    while c < line.len() {
        // If the token at this position exists
        if let Some(t) = tokens.get(&c) {
            // Insert it and jump over everything
            result.insert(t.span.0, t.clone());
            c += t.span.1 - t.span.0;
        } else {
            // Shift forward
            c += 1;
        }
    }
    result
}
