use crate::{Position, Row};
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone)]
pub struct Exp {
    ansi: Regex,
    pub keywords: Vec<Regex>,
    pub digits: Regex,
    pub strings: Regex,
    pub characters: Regex,
    pub single_comments: Regex,
    pub macros: Regex,
    pub functions: Regex,
    pub structs: Regex,
    pub attributes: Regex,
    pub booleans: Regex,
}

impl Exp {
    pub fn new() -> Self {
        let kw = ["as", "break", "const", "continue", "cra
te", "else", "enum", "extern", "fn", "for", "if", "impl", "in"
, "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
"return", "self", "static", "struct", "super", "trait"
, "type", "unsafe", "use", "where", "while", "async", "await",
 "dyn", "abstract", "become", "box", "do", "final", "macro", "
override", "priv", "typeof", "unsized", "virtual", "yield", "t
ry", "'static"];
        Self {
            ansi: Regex::new(r"\u{1b}\[[0-?]*[ -/]*[@-~]").unwrap(),
            keywords: kw.iter().map(|x| Regex::new(&format!(r"\b({})\b", x)).unwrap()).collect(),
            digits: Regex::new(r"(\d+\.\d+|\d+)").unwrap(),
            strings: Regex::new("(\".*?\")").unwrap(),
            characters: Regex::new("('.')").unwrap(),
            single_comments: Regex::new("(//.*)").unwrap(),
            macros: Regex::new("\\b([a-z_][a-zA-Z_]*!)").unwrap(),
            functions: Regex::new("\\b\\s+([a-z_]*)\\b\\(").unwrap(),
            structs: Regex::new("\\b([A-Z][A-Za-z_]*)\\b\\s*\\{").unwrap(),
            attributes: Regex::new("^\\s*(#(?:!|)\\[.*?\\])").unwrap(),
            booleans: Regex::new("\\b(true|false)\\b").unwrap(),
        }
    }
    pub fn ansi_len(&self, string: &str) -> usize {
        // Find the length of a string without ANSI values
        UnicodeWidthStr::width(&*self.ansi.replace_all(string, ""))
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
    let chars: Vec<&str> = text.graphemes(true).collect();
    let mut result = vec![];
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
    let chars: Vec<&str> = text.graphemes(true).collect();
    let mut result = vec![];
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
