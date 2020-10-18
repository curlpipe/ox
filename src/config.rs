// Config.rs - In charge of storing configuration information
use regex::Regex;
use ron::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use termion::color;

// Enum for determining what type of token it is
#[derive(Clone)]
pub enum TokenType {
    MultiLine(String, Vec<Regex>),
    SingleLine(String, Vec<Regex>),
}

// Error enum for config reading
#[derive(Debug)]
pub enum Status {
    Parse(String),
    File,
    Success,
}

// Struct for storing and managing configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Reader {
    pub general: General,
    pub theme: Theme,
    pub highlights: HashMap<String, (u8, u8, u8)>,
    pub languages: Vec<Language>,
}

impl Reader {
    pub fn read(config: &str) -> (Self, Status) {
        // Read the config file, if it fails, use a hard-coded configuration
        let rust_kw = vec![
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "fn", "for",
            "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "static", "struct", "super", "trait", "type", "unsafe", "use",
            "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final",
            "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "'static",
        ];
        let default = Self {
            general: General {
                line_number_padding_right: 2,
                line_number_padding_left: 1,
                tab_width: 4,
                undo_period: 5,
            },
            theme: Theme {
                editor_bg: (41, 41, 61),
                editor_fg: (255, 255, 255),
                status_bg: (59, 59, 84),
                status_fg: (35, 240, 144),
                line_number_fg: (65, 65, 98),
            },
            highlights: [
                ("comments".to_string(), (113, 113, 169)),
                ("keywords".to_string(), (134, 76, 232)),
                ("strings".to_string(), (39, 222, 145)),
                ("characters".to_string(), (40, 198, 232)),
                ("digits".to_string(), (40, 198, 232)),
                ("booleans".to_string(), (86, 217, 178)),
                ("functions".to_string(), (47, 141, 252)),
                ("structs".to_string(), (47, 141, 252)),
                ("macros".to_string(), (223, 52, 249)),
                ("attributes".to_string(), (40, 198, 232)),
            ]
            .iter()
            .cloned()
            .collect(),
            languages: vec![Language {
                name: "Rust".to_string(),           // Name of the language
                icon: "\u{e7a8} ".to_string(),      // Icon for the language
                extensions: vec!["rs".to_string()], // Extensions of the language
                // Keywords of the language
                keywords: rust_kw.iter().map(|x| (*x).to_string()).collect(),
                // Syntax definitions
                definitions: [
                    (
                        "comments".to_string(),
                        vec!["(?m)(//.*)$".to_string(), "(?ms)(/\\*.*?\\*/)".to_string()],
                    ),
                    ("strings".to_string(), vec!["(\".*?\")".to_string()]),
                    (
                        "characters".to_string(),
                        vec!["('.')".to_string(), "('\\\\.')".to_string()],
                    ),
                    ("digits".to_string(), vec!["(\\d+.\\d+|\\d+)".to_string()]),
                    (
                        "booleans".to_string(),
                        vec!["\\b(true|false)\\b".to_string()],
                    ),
                    (
                        "functions".to_string(),
                        vec!["\\b\\s+([a-z_]*)\\b\\(".to_string()],
                    ),
                    (
                        "structs".to_string(),
                        vec!["\\b([A-Z][A-Za-z_]*)\\b\\s*\\{".to_string()],
                    ),
                    (
                        "macros".to_string(),
                        vec!["\\b([a-z_][a-zA-Z_]*!)".to_string()],
                    ),
                    (
                        "attributes".to_string(),
                        vec!["^\\s*(#(?:!|)\\[.*?\\])".to_string()],
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            }],
        };
        // Expand the path to get rid of any filepath issues
        let config = if let Ok(config) = shellexpand::full(config) {
            (*config).to_string()
        } else {
            config.to_string()
        };
        // Attempt to read and parse the configuration file
        if let Ok(file) = fs::read_to_string(config) {
            let result: (Self, Status) = if let Ok(contents) = from_str(&file) {
                (contents, Status::Success)
            } else {
                // There is a syntax issue with the config file
                let result: Result<Self, ron::Error> = from_str(&file);
                // Provide the syntax issue with the config file for debugging
                (default, Status::Parse(format!("{:?}", result)))
            };
            result
        } else {
            // File wasn't able to be found
            (default, Status::File)
        }
    }
    pub fn get_syntax_regex(config: &Self, extension: &str) -> Vec<TokenType> {
        // Compile the regular expressions from their string format
        let mut result = vec![];
        for lang in &config.languages {
            // Locate the correct language for the extension
            if lang.extensions.contains(&extension.to_string()) {
                // Run through all the regex syntax definitions
                for (name, reg) in &config.languages[0].definitions {
                    let mut single = vec![];
                    let mut multi = vec![];
                    for expr in reg {
                        if expr.starts_with("(?ms)") || expr.starts_with("(?sm)") {
                            // Multiline regular expression
                            if let Ok(regx) = Regex::new(&expr) {
                                multi.push(regx);
                            }
                        } else {
                            // Single line regular expression
                            if let Ok(regx) = Regex::new(&expr) {
                                single.push(regx);
                            }
                        }
                    }
                    if !single.is_empty() {
                        result.push(TokenType::SingleLine(name.clone(), single));
                    }
                    if !multi.is_empty() {
                        result.push(TokenType::MultiLine(name.clone(), multi));
                    }
                }
                // Process all the keywords
                result.push(TokenType::SingleLine(
                    "keywords".to_string(),
                    lang.keywords
                        .iter()
                        .map(|x| Regex::new(&format!(r"\b({})\b", x)).unwrap())
                        .collect(),
                ));
            }
        }
        result
    }
    pub fn rgb_fg(colour: (u8, u8, u8)) -> color::Fg<color::Rgb> {
        // Get the text ANSI code from an RGB value
        color::Fg(color::Rgb(colour.0, colour.1, colour.2))
    }
    pub fn rgb_bg(colour: (u8, u8, u8)) -> color::Bg<color::Rgb> {
        // Get the background ANSI code from an RGB value
        color::Bg(color::Rgb(colour.0, colour.1, colour.2))
    }
}

// Struct for storing the general configuration
#[derive(Debug, Deserialize, Clone)]
pub struct General {
    pub line_number_padding_right: usize,
    pub line_number_padding_left: usize,
    pub tab_width: usize,
    pub undo_period: u64,
}

// Struct for storing theme information
#[derive(Debug, Deserialize, Clone)]
pub struct Theme {
    pub editor_bg: (u8, u8, u8),
    pub editor_fg: (u8, u8, u8),
    pub status_bg: (u8, u8, u8),
    pub status_fg: (u8, u8, u8),
    pub line_number_fg: (u8, u8, u8),
}

// Struct for storing language information
#[derive(Debug, Deserialize, Clone)]
pub struct Language {
    pub name: String,
    pub icon: String,
    pub extensions: Vec<String>,
    pub keywords: Vec<String>,
    pub definitions: HashMap<String, Vec<String>>,
}
