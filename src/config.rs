// Config.rs - In charge of storing configuration information
use crossterm::style::{Color, SetBackgroundColor, SetForegroundColor};
use regex::Regex;
use ron::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

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
    Empty,
}

// Key binding type
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Deserialize)]
pub enum KeyBinding {
    Ctrl(RawKey),
    Alt(RawKey),
    Shift(RawKey),
    Raw(RawKey),
    F(u8),
    Unsupported,
}

// Keys without modifiers
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Deserialize)]
pub enum RawKey {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Enter,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
    BackTab,
    Delete,
    Insert,
    Null,
    Esc,
}

// Struct for storing and managing configuration
#[derive(Debug, Deserialize, Clone)]
pub struct Reader {
    pub general: General,
    pub theme: Theme,
    pub macros: HashMap<String, Vec<String>>,
    pub highlights: HashMap<String, HashMap<String, (u8, u8, u8)>>,
    pub keys: HashMap<KeyBinding, Vec<String>>,
    pub languages: Vec<Language>,
}

impl Reader {
    pub fn read(config: &str) -> (Self, Status) {
        // Read the config file, if it fails, use a hard-coded configuration
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
            } else if file.is_empty() {
                // When configuration file is empty
                (from_str(&default()).unwrap(), Status::Empty)
            } else {
                // There is a syntax issue with the config file
                let result: Result<Self, ron::Error> = from_str(&file);
                // Provide the syntax issue with the config file for debugging
                (
                    from_str(&default()).unwrap(),
                    Status::Parse(format!("{:?}", result)),
                )
            };
            result
        } else {
            // File wasn't able to be found
            (from_str(&default()).unwrap(), Status::File)
        }
    }
    pub fn get_syntax_regex(config: &Self, extension: &str) -> Vec<TokenType> {
        // Compile the regular expressions from their string format
        let mut result = vec![];
        for lang in &config.languages {
            // Locate the correct language for the extension
            if lang.extensions.contains(&extension.to_string()) {
                // Run through all the regex syntax definitions
                for (name, reg) in &lang.definitions {
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
    pub fn rgb_fg(colour: (u8, u8, u8)) -> SetForegroundColor {
        // Get the text ANSI code from an RGB value
        SetForegroundColor(Color::Rgb {
            r: colour.0,
            g: colour.1,
            b: colour.2,
        })
    }
    pub fn rgb_bg(colour: (u8, u8, u8)) -> SetBackgroundColor {
        // Get the background ANSI code from an RGB value
        SetBackgroundColor(Color::Rgb {
            r: colour.0,
            g: colour.1,
            b: colour.2,
        })
    }
}

// Struct for storing the general configuration
#[derive(Debug, Deserialize, Clone)]
pub struct General {
    pub line_number_padding_right: usize,
    pub line_number_padding_left: usize,
    pub tab_width: usize,
    pub undo_period: u64,
    pub status_left: String,
    pub status_right: String,
    pub tab: String,
    pub wrap_cursor: bool,
}

// Struct for storing theme information
#[derive(Debug, Deserialize, Clone)]
pub struct Theme {
    pub transparent_editor: bool,
    pub editor_bg: (u8, u8, u8),
    pub editor_fg: (u8, u8, u8),
    pub status_bg: (u8, u8, u8),
    pub status_fg: (u8, u8, u8),
    pub line_number_fg: (u8, u8, u8),
    pub line_number_bg: (u8, u8, u8),
    pub inactive_tab_fg: (u8, u8, u8),
    pub inactive_tab_bg: (u8, u8, u8),
    pub active_tab_fg: (u8, u8, u8),
    pub active_tab_bg: (u8, u8, u8),
    pub warning_fg: (u8, u8, u8),
    pub error_fg: (u8, u8, u8),
    pub info_fg: (u8, u8, u8),
    pub default_theme: String,
    pub fallback: bool,
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

// Default configuration format
// Minify using:
// (| )//[a-zA-Z0-9 ]+ on https://www.regextester.com/
// https://codebeautify.org/text-minifier
fn default() -> String {
"/*\n    My very own (awesome) Ox configuration file!\n    \n    Ox uses RON. RON is an object notation similar to JSON.\n    It makes it easy and quick for Ox to parse.\n\n    Config name: NAME\n    Author:      AUTHOR\n    YEAR:        YEAR\n*/\n\n// General settings for Ox\n(\n    general: General(\n        line_number_padding_right: 2, // Line number padding on the right\n        line_number_padding_left:  1, // Line number padding on the left\n        tab_width:                 4, // The amount of spaces for a tab\n        undo_period:               5, // Seconds of inactivity for undo\n        wrap_cursor:            true, // Determines wheter the cursor wraps around\n        // Values:\n        // %f - File name\n        // %F - File name with full path\n        // %I - Language specific icon with leading space\n        // %i - Language specific icon\n        // %n - Language name\n        // %l - Current line number in the document\n        // %L - Total number of lines in the document\n        // %x - X position of the cursor\n        // %y - Y position of the cursor\n        // %v - Version of the editor (e.g. 0.2.6)\n        // %d - Dirty file indicator text\n        // %D - Dirty file indicator icon\n        // %R - Read only file indicator\n        status_left:  \" %f%d %D \u{2502} %n %i\", // Left part of status line\n        status_right: \"\u{4e26} %l / %L \u{2502} \u{fae6}(%x, %y) \", // Right part of status line\n        tab: \"%I%f%d\", // Tab formatting\n    ),\n    // Custom defined macros\n    macros: {\n        // Macro to move a line up\n        \"move line up\": [\n            \"store line 1\", // Store current line in bank #1\n            \"delete 0\",     // Delete current line\n            \"move 1 up\",    // Move cursor up by 1\n            \"line above\",   // Insert an empty line above\n            \"move 1 up\",    // Move cursor up to the empty line\n            \"load line 1\",  // Load line in bank #1 over the empty line\n        ],\n        // Macro to move a line down\n        \"move line down\": [\n            \"store line 1\", // Store the current line in bank #1\n            \"delete 0\",     // Delete the current line\n            \"line below\",   // Create an empty line below\n            \"move 1 down\",  // Move cursor down to empty line\n            \"load line 1\",  // Overwrite empty line with line in bank #1\n        ],\n        // Macro to save with root permission\n        \"save #\": [\n            // SHCS: Shell with confirmation and substitution\n            // With substitution, `%C` becomes the current documents contents\n            // `%F` becomes the file path of the current document\n            \"shcs sudo cat > %F << EOF\\n%CEOF\", // \'%F\' is the current file name\n            \"is saved\", // Set the status of the file to saved\n        ],\n    },\n    // RGB values for the colours of Ox\n    theme: Theme(\n        transparent_editor: false,         // Makes editor background transparent\n        editor_bg:          (41, 41, 61), // The main background color\n        editor_fg:          (255, 255, 255), // The default text color\n        status_bg:          (59, 59, 84), // The background color of the status line\n        status_fg:          (35, 240, 144), // The text color of the status line\n        line_number_fg:     (73, 73, 110), // The text color of the line numbers\n        line_number_bg:     (49, 49, 73), // The background color of the line numbers\n        active_tab_fg:      (255, 255, 255), // The text color of the active tab\n        active_tab_bg:      (41, 41, 61), //  The background color of the active tab\n        inactive_tab_fg:    (255, 255, 255), // The text color of the inactive tab(s)\n        inactive_tab_bg:    (59, 59, 84), // The text color of the inactive tab(s)\n        warning_fg:         (208, 164, 79), // Text colour of the warning message\n        error_fg:           (224, 113, 113), // Text colour of the warning message\n        info_fg:            (255, 255, 255), // Text colour of the warning message\n        default_theme:    \"default\", // The default syntax highlights to use\n        fallback:         true, // Enables use of fallback themes (if detected)\n    ),\n    // Colours for the syntax highlighting\n    highlights: {\n        \"default\": {\n            \"comments\":   (113, 113, 169),\n            \"keywords\":   (134, 76, 232),\n            \"namespaces\": (134, 76, 232),\n            \"references\": (134, 76, 232),\n            \"strings\":    (39, 222, 145),\n            \"characters\": (40, 198, 232),\n            \"digits\":     (40, 198, 232),\n            \"booleans\":   (86, 217, 178),\n            \"functions\":  (47, 141, 252),\n            \"structs\":    (47, 141, 252),\n            \"macros\":     (223, 52, 249),\n            \"attributes\": (40, 198, 232),\n            \"headers\":    (47, 141, 252),\n            \"symbols\":    (47, 141, 252),\n            \"global\":     (86, 217, 178),\n            \"operators\":  (86, 217, 178),\n            \"regex\":      (40, 198, 232),\n            \"search_active\":   (41, 73, 131),\n            \"search_inactive\": (29, 52, 93),\n        },\n        \"alternative\": {\n            \"comments\":   (113, 113, 169),\n            \"keywords\":   (64, 86, 244),\n            \"namespaces\": (64, 86, 244),\n            \"references\": (64, 86, 244),\n            \"strings\":    (76, 224, 179),\n            \"characters\": (110, 94, 206),\n            \"digits\":     (4, 95, 204),\n            \"booleans\":   (76, 224, 179),\n            \"functions\":  (4, 95, 204),\n            \"structs\":    (4, 95, 204),\n            \"macros\":     (110, 94, 206),\n            \"attributes\": (4, 95, 204),\n            \"headers\":    (141, 129, 217),\n            \"symbols\":    (249, 233, 0),\n            \"global\":     (76, 224, 179),\n            \"operators\":  (76, 224, 179),\n            \"regex\":      (4, 95, 204),\n            \"search_active\":   (41, 73, 131),\n            \"search_inactive\": (29, 52, 93),\n        },\n    },\n    // Key bindings\n    keys: {\n        // Keybinding: [Oxa commands]\n        Ctrl(Char(\'q\')): [\"quit\"], // Quit current document\n        Ctrl(Char(\'s\')): [\"save\"], // Save current document\n        Alt(Char(\'s\')):  [\"save ?\"], // Save current document as\n        Ctrl(Char(\'w\')): [\"save *\"], // Save all open documents\n        Ctrl(Char(\'n\')): [\"new\"], // Create new document\n        Ctrl(Char(\'o\')): [\"open\"], // Open document\n        Ctrl(Left):      [\"prev\"], // Move to previous tab\n        Ctrl(Right):     [\"next\"], // Move to next tab\n        Ctrl(Char(\'z\')): [\"undo\"], // Undo last edit\n        Ctrl(Char(\'y\')): [\"redo\"], // Redo last edit\n        Ctrl(Char(\'f\')): [\"search\"], // Trigger search command\n        Ctrl(Char(\'r\')): [\"replace\"], // Trigger replace command\n        Ctrl(Char(\'a\')): [\"replace *\"], // Trigger replace all command\n        Ctrl(Up):        [\"move line up\"], // Move line up\n        Ctrl(Down):      [\"move line down\"], // Move line down\n        Ctrl(Delete):    [\"delete word left\"], // Delete word\n        Alt(Char(\'a\')):  [\"cmd\"], // Open the command line\n        // Show help message URL\n        F(1):   [\n            \"sh echo You can get help here:\",\n            \"shc echo https://github.com/curlpipe/ox/wiki\",\n        ]\n    },\n    // Language specific settings\n    languages: [\n        Language(\n            name: \"Rust\", // Name of the language\n            icon: \"\u{e7a8} \", // Icon for the language\n            extensions: [\"rs\"], // Extensions of the language\n            // Keywords of the language\n            keywords: [\n                \"as\", \"break\", \"const\", \"continue\", \"crate\", \"else\", \n                \"enum\", \"extern\", \"fn\", \"for\", \"if\", \"impl\", \"in\", \n                \"let\", \"loop\", \"match\", \"mod\", \"move\", \"mut\", \"pub\", \n                \"ref\", \"return\", \"self\", \"static\", \"struct\", \"super\", \n                \"trait\", \"type\", \"unsafe\", \"use\", \"where\", \"while\", \n                \"async\", \"await\", \"dyn\", \"abstract\", \"become\", \"box\", \n                \"do\", \"final\", \"macro\", \"override\", \"priv\", \"typeof\", \n                \"unsized\", \"virtual\", \"yield\", \"try\", \"\'static\",\n                \"u8\", \"u16\", \"u32\", \"u64\", \"u128\", \"usize\",\n                \"i8\", \"i16\", \"i32\", \"i64\", \"i128\", \"isize\",\n                \"f32\", \"f64\", \"String\", \"Vec\", \"str\", \"Some\", \"bool\",\n                \"None\", \"Box\", \"Result\", \"Option\", \"Ok\", \"Err\", \"Self\",\n                \"std\"\n            ],\n            // Syntax definitions\n            definitions: {\n                \"operators\":  [\n                    r\"(=)\",\n                    r\"(\\+)\",\n                    r\"(\\-)\",\n                    r\"(\\*)\",\n                    r\"[^/](/)[^/]\",\n                    r\"(\\+=)\",\n                    r\"(\\-=)\",\n                    r\"(\\*=)\",\n                    r\"(\\\\=)\",\n                    r\"(==)\",\n                    r\"(!=)\",\n                    r\"(\\?)\",\n                    r\"(>=)\",\n                    r\"(<=)\",\n                    r\"(<)\",\n                    r\"(>)\",\n                ],\n                \"namespaces\": [\n                    r\"([a-z_][A-Za-z0-9_]*)::\",\n                ],\n                \"comments\":   [\n                    \"(?m)(//.*)$\", \n                    \"(?ms)(/\\\\*.*?\\\\*/)\",\n                ],\n                \"strings\":    [\n                    \"\\\"(?:[^\\\"\\\\\\\\]*(?:\\\\\\\\.[^\\\"\\\\\\\\]*)*)\\\"\",\n                    \"(r\\\".*?\\\")\",\n                    \"(?ms)(r#\\\".*?\\\"#)\",\n                    \"(?ms)(#\\\".*?\\\"#)\",\n                ],\n                \"characters\": [\n                    \"(\'.\')\", \n                    \"(\'\\\\\\\\.\')\",\n                ],\n                \"digits\":     [\n                    \"\\\\b(\\\\d+.\\\\d+|\\\\d+)\",\n                    \"\\\\b(\\\\d+.\\\\d+(?:f32|f64))\",\n                ],\n                \"booleans\":   [\n                    \"\\\\b(true)\\\\b\", \n                    \"\\\\b(false)\\\\b\",\n                ],\n                \"functions\":  [\n                    \"fn\\\\s+([a-z_][A-Za-z0-9_]*)\\\\s*\\\\(\",\n                    r\"\\.([a-z_][A-Za-z0-9_]*)\\s*\\(\",\n                    r\"([a-z_][A-Za-z0-9_]*)\\s*\\(\",\n                ],\n                \"structs\":    [\n                    \"(?:trait|enum|struct|impl)\\\\s+([A-Z][A-Za-z0-9_]*)\\\\s*\", \n                    \"impl(?:<.*?>|)\\\\s+([A-Z][A-Za-z0-9_]*)\",\n                    \"([A-Z][A-Za-z0-9_]*)::\",\n                    r\"([A-Z][A-Za-z0-9_]*)\\s*\\(\",\n                    \"impl.*for\\\\s+([A-Z][A-Za-z0-9_]*)\",\n                    r\"::\\s*([a-z_][A-Za-z0-9_]*)\\s*\\(\",\n                ],\n                \"macros\":     [\n                    \"\\\\b([a-z_][a-zA-Z0-9_]*!)\",\n                    r\"(\\$[a-z_][A-Za-z0-9_]*)\",\n                ],\n                \"attributes\": [\n                    \"(?ms)^\\\\s*(#(?:!|)\\\\[.*?\\\\])\",\n                ],\n                \"references\": [\n                    \"(&)\",\n                    \"&str\", \"&mut\", \"&self\", \n                    \"&i8\", \"&i16\", \"&i32\", \"&i64\", \"&i128\", \"&isize\",\n                    \"&u8\", \"&u16\", \"&u32\", \"&u64\", \"&u128\", \"&usize\",\n                    \"&f32\", \"&f64\",\n                ]\n            }\n        ),\n        Language(\n            name: \"Ruby\", // Name of the language\n            icon: \"\u{e739} \", // Icon for the language\n            extensions: [\"rb\"], // Extensions of the language\n            // Keywords of the language\n            keywords: [\n                \"__ENCODING__\", \"__LINE__\", \"__FILE__\", \"BEGIN\", \"END\", \n                \"alias\", \"and\", \"begin\", \"break\", \"case\", \"class\", \"def\", \n                \"defined?\", \"do\", \"else\", \"elsif\", \"end\", \"ensure\", \"print\",\n                \"for\", \"if\", \"in\", \"module\", \"next\", \"nil\", \"not\", \"or\", \"puts\",\n                \"redo\", \"rescue\", \"retry\", \"return\", \"self\", \"super\", \"then\", \n                \"undef\", \"unless\", \"until\", \"when\", \"while\", \"yield\", \"raise\",\n                \"include\", \"extend\", \"require\" \n            ],\n            // Syntax definitions\n            definitions: {\n                \"comments\":   [\n                    \"(?m)(#.*)$\", \n                    \"(?ms)(=begin.*=end)\", \n                ],\n                \"strings\":    [\n                    \"(?:f|r|)\\\"(?:[^\\\"\\\\\\\\]*(?:\\\\\\\\.[^\\\"\\\\\\\\]*)*)\\\"\",\n                    \"(?:f|r|)\\\'(?:[^\\\'\\\\\\\\]*(?:\\\\\\\\.[^\\\'\\\\\\\\]*)*)\\\'\",\n                ],\n                \"digits\":     [\n                    r\"\\b(\\d+.\\d+|\\d+)\",\n                ],\n                \"booleans\":   [\n                    r\"\\b(true)\\b\", \n                    r\"\\b(false)\\b\",\n                ],\n                \"structs\":    [\n                    r\"class(\\s+[A-Za-z0-9_]*)\",\n                ],\n                \"functions\":  [\n                    r\"def\\s+([a-z_][A-Za-z0-9_\\\\?!]*)\",\n                    \"\\\\.([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\",\n                    \"\\\\b([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\\\\(\",\n                ],\n                \"symbols\":    [\n                    r\"(:[^,\\)\\.\\s=]+)\",\n                ],\n                \"global\":     [\n                    r\"(\\$[a-z_][A-Za-z0-9_]*)\\s\",\n                ],\n                \"regex\": [\n                    r\"/.+/\"\n                ],\n                \"operators\":  [\n                    r\"(=)\",\n                    r\"(\\+)\",\n                    r\"(\\-)\",\n                    r\"(\\*)\",\n                    r\"(\\s/\\s)\",\n                    r\"(\\+=)\",\n                    r\"(\\-=)\",\n                    r\"(\\*=)\",\n                    r\"(\\\\=)\",\n                    r\"(==)\",\n                    r\"(!=)\",\n                    r\"(\\&\\&)\",\n                    r\"(\\|\\|)\",\n                    r\"(!)\\S\",\n                    r\"(>=)\",\n                    r\"(<=)\",\n                    r\"(<)\",\n                    r\"(>)\",\n                ],\n            }\n        ),\n        Language(\n            name: \"Crystal\", // Name of the language\n            icon: \"\u{e7a3} \", // Icon for the language\n            extensions: [\"cr\"], // Extensions of the language\n            // Keywords of the language\n            keywords: [\n                \"__ENCODING__\", \"__LINE__\", \"__FILE__\", \"BEGIN\", \"END\", \n                \"alias\", \"and\", \"begin\", \"break\", \"case\", \"class\", \"def\", \n                \"defined?\", \"do\", \"else\", \"elsif\", \"end\", \"ensure\", \"print\",\n                \"for\", \"if\", \"in\", \"module\", \"next\", \"nil\", \"not\", \"or\", \"puts\",\n                \"redo\", \"rescue\", \"retry\", \"return\", \"self\", \"super\", \"then\", \n                \"undef\", \"unless\", \"until\", \"when\", \"while\", \"yield\", \"raise\",\n                \"include\", \"extend\", \"Int32\", \"String\", \"getter\", \"setter\",\n                \"property\", \"Array\", \"Set\", \"Hash\", \"Range\", \"Proc\", \"typeof\",\n            ],\n            // Syntax definitions\n            definitions: {\n                \"comments\":   [\n                    \"(?m)(#.*)$\", \n                    \"(?ms)(=begin.*=end)\", \n                ],\n                \"strings\":    [\n                    \"(?ms)(\\\".*?\\\")\",\n                    \"(?:f|r|)\\\"(?:[^\\\"\\\\\\\\]*(?:\\\\\\\\.[^\\\"\\\\\\\\]*)*)\\\"\",\n                    \"(\\\'.*?\\\')\",\n                ],\n                \"digits\":     [\n                    r\"\\b(\\d+.\\d+|\\d+)\",\n                    r\"(_i(?:8|16|32|64|128))\",\n                    r\"(_u(?:8|16|32|64|128))\",\n                    r\"(_f(?:8|16|32|64|128))\",\n                    \"0x[A-Fa-f0-9]{6}\"\n                ],\n                \"booleans\":   [\n                    r\"\\b(true)\\b\", \n                    r\"\\b(false)\\b\",\n                ],\n                \"structs\":    [\n                    r\"class(\\s+[A-Za-z0-9_]*)\",\n                ],\n                \"functions\":  [\n                    r\"def\\s+([a-z_][A-Za-z0-9_\\\\?!]*)\",\n                    \"\\\\.([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\",\n                    \"\\\\b([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\\\\(\",\n                ],\n                \"symbols\":    [\n                    r\"(:[^,\\}\\)\\.\\s=]+)\",\n                ],\n                \"global\":     [\n                    r\"(\\$[a-z_][A-Za-z0-9_]*)\\s\",\n                ],\n                \"operators\":  [\n                    r\"(=)\",\n                    r\"(\\+)\",\n                    r\"(\\-)\",\n                    r\"(\\*)\",\n                    r\"(\\s/\\s)\",\n                    r\"(\\+=)\",\n                    r\"(\\-=)\",\n                    r\"(\\*=)\",\n                    r\"(\\\\=)\",\n                    r\"(==)\",\n                    r\"(!=)\",\n                    r\"(\\&\\&)\",\n                    r\"(\\|\\|)\",\n                    r\"(!)\\S\",\n                    r\"(>=)\",\n                    r\"(<=)\",\n                    r\"(<)\",\n                    r\"(>)\",\n                    r\"(\\?)\",\n                ],\n            }\n        ),\n        Language(\n            name: \"Python\", // Name of the language\n            icon: \"\u{e73c} \", // Icon for the language\n            extensions: [\"py\", \"pyw\"], // Extensions of the language\n            // Keywords of the language\n            keywords: [\n                \"and\", \"as\", \"assert\", \"break\", \"class\", \"continue\", \n                \"def\", \"del\", \"elif\", \"else\", \"except\", \"exec\", \n                \"finally\", \"for\", \"from\", \"global\", \"if\", \"import\", \n                \"in\", \"is\", \"lambda\", \"not\", \"or\", \"pass\", \"print\", \n                \"raise\", \"return\", \"try\", \"while\", \"with\", \"yield\",\n                \"str\", \"bool\", \"int\", \"tuple\", \"list\", \"dict\", \"tuple\",\n                \"len\", \"None\", \"input\", \"type\", \"set\", \"range\", \"enumerate\",\n                \"open\", \"iter\", \"min\", \"max\", \"dir\", \"self\", \"isinstance\", \n                \"help\", \"next\", \"super\",\n            ],\n            // Syntax definitions\n            definitions: {\n                \"comments\":   [\n                    \"(?m)(#.*)$\", \n                ],\n                \"strings\":    [\n                    \"(?ms)(\\\"\\\"\\\".*?\\\"\\\"\\\")\",\n                    \"(?ms)(\\\'\\\'\\\'.*?\\\'\\\'\\\')\",\n                    \"(?:f|r|)\\\"(?:[^\\\"\\\\\\\\]*(?:\\\\\\\\.[^\\\"\\\\\\\\]*)*)\\\"\",\n                    \"(?:f|r|)\\\'(?:[^\\\'\\\\\\\\]*(?:\\\\\\\\.[^\\\'\\\\\\\\]*)*)\\\'\",\n                ],\n                \"digits\":     [\n                    \"\\\\b(\\\\d+.\\\\d+|\\\\d+)\",\n                ],\n                \"booleans\":   [\n                    \"\\\\b(True)\\\\b\", \n                    \"\\\\b(False)\\\\b\",\n                ],\n                \"structs\":    [\n                    \"class\\\\s+([A-Za-z0-9_]*)\",\n                ],\n                \"functions\":  [\n                    \"def\\\\s+([a-z_][A-Za-z0-9_]*)\",\n                    \"\\\\.([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\",\n                    \"\\\\b([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\\\\(\",\n                ],\n                \"attributes\": [\n                    \"@.*$\",\n                ],\n                \"operators\":  [\n                    r\"(=)\",\n                    r\"(\\+)\",\n                    r\"(\\-)\",\n                    r\"(\\*)\",\n                    r\"(\\s/\\s)\",\n                    r\"(\\s//\\s)\",\n                    r\"(%)\",\n                    r\"(\\+=)\",\n                    r\"(\\-=)\",\n                    r\"(\\*=)\",\n                    r\"(\\\\=)\",\n                    r\"(==)\",\n                    r\"(!=)\",\n                    r\"(>=)\",\n                    r\"(<=)\",\n                    r\"(<)\",\n                    r\"(>)\",\n                ],\n            }\n        ),\n        Language(\n            name: \"Javascript\", // Name of the language\n            icon: \"\u{e74e} \", // Icon for the language\n            extensions: [\"js\"], // Extensions of the language\n            // Keywords of the language\n            keywords: [\n                \"abstract\", \"arguments\", \"await\", \"boolean\", \"break\", \"byte\", \n                \"case\", \"catch\", \"char\", \"class\", \"const\", \"continue\", \"debugger\", \n                \"default\", \"delete\", \"do\", \"double\", \"else\", \"enum\", \"eval\", \n                \"export\", \"extends\", \"final\", \"finally\", \"float\", \"for\", \"of\",\n                \"function\", \"goto\", \"if\", \"implements\", \"import\", \"in\", \"instanceof\", \n                \"int\", \"interface\", \"let\", \"long\", \"native\", \"new\", \"null\", \"package\", \n                \"private\", \"protected\", \"public\", \"return\", \"short\", \"static\", \n                \"super\", \"switch\", \"synchronized\", \"this\", \"throw\", \"throws\", \n                \"transient\", \"try\", \"typeof\", \"var\", \"void\", \"volatile\", \"console\",\n                \"while\", \"with\", \"yield\", \"undefined\", \"NaN\", \"-Infinity\", \"Infinity\",\n            ],\n            // Syntax definitions\n            definitions: {\n                \"comments\":   [\n                    \"(?m)(//.*)$\", \n                    \"(?ms)(/\\\\*.*\\\\*/)$\", \n                ],\n                \"strings\":    [\n                    \"(?ms)(\\\"\\\"\\\".*?\\\"\\\"\\\")\",\n                    \"(?ms)(\\\'\\\'\\\'.*?\\\'\\\'\\\')\",\n                    \"(?:f|r|)\\\"(?:[^\\\"\\\\\\\\]*(?:\\\\\\\\.[^\\\"\\\\\\\\]*)*)\\\"\",\n                    \"(?:f|r|)\\\'(?:[^\\\'\\\\\\\\]*(?:\\\\\\\\.[^\\\'\\\\\\\\]*)*)\\\'\",\n                ],\n                \"digits\":     [\n                    \"\\\\b(\\\\d+.\\\\d+|\\\\d+)\",\n                ],\n                \"booleans\":   [\n                    \"\\\\b(true)\\\\b\", \n                    \"\\\\b(false)\\\\b\",\n                ],\n                \"structs\":    [\n                    \"class\\\\s+([A-Za-z0-9_]*)\",\n                ],\n                \"functions\":  [\n                    \"function\\\\s+([a-z_][A-Za-z0-9_]*)\",\n                    \"\\\\b([a-z_][A-Za-z0-9_]*)\\\\s*\\\\(\",\n                    \"\\\\.([a-z_][A-Za-z0-9_\\\\?!]*)\\\\s*\",\n                ],\n                \"operators\":  [\n                    r\"(=)\",\n                    r\"(\\+)\",\n                    r\"(\\-)\",\n                    r\"(\\*)\",\n                    r\"(\\s/\\s)\",\n                    r\"(%)\",\n                    r\"(\\+=)\",\n                    r\"(\\-=)\",\n                    r\"(\\*=)\",\n                    r\"(\\\\=)\",\n                    r\"(==)\",\n                    r\"(!=)\",\n                    r\"(>=)\",\n                    r\"(<=)\",\n                    r\"(<)\",\n                    r\"(>)\",\n                    r\"(<<)\",\n                    r\"(>>)\",\n                    r\"(\\&\\&)\",\n                    r\"(\\|\\|)\",\n                    r\"(!)\\S\",\n                ],\n            }\n        ),\n        Language(\n            name: \"C\", // Name of the language\n            icon: \"\u{e61e} \", // Icon for the language\n            extensions: [\"c\", \"h\"], // Extensions of the language\n            // Keywords of the language\n            keywords: [\n                \"auto\", \"break\", \"case\", \"char\", \"const\", \"continue\", \"default\", \n                \"do\", \"double\", \"else\", \"enum\", \"extern\", \"float\", \"for\", \"goto\", \n                \"if\", \"int\", \"long\", \"register\", \"return\", \"short\", \"signed\", \n                \"sizeof\", \"static\", \"struct\", \"switch\", \"typedef\", \"union\", \n                \"unsigned\", \"void\", \"volatile\", \"while\", \"printf\", \"fscanf\", \n                \"scanf\", \"fputsf\", \"exit\", \"stderr\", \"malloc\", \"calloc\", \"bool\",\n                \"realloc\", \"free\", \"strlen\", \"size_t\",\n            ],\n            // Syntax definitions\n            definitions: {\n                \"comments\":   [\n                    \"(?m)(//.*)$\", \n                    \"(?ms)(/\\\\*.*?\\\\*/)\",\n                ],\n                \"strings\":    [\n                    \"\\\"(?:[^\\\"\\\\\\\\]*(?:\\\\\\\\.[^\\\"\\\\\\\\]*)*)\\\"\",\n                ],\n                \"characters\": [\n                    \"(\'.\')\", \n                    \"(\'\\\\\\\\.\')\",\n                ],\n                \"digits\":     [\n                    \"\\\\b(\\\\d+.\\\\d+|\\\\d+)\",\n                    \"\\\\b(\\\\d+.\\\\d+(?:f|))\",\n                ],\n                \"booleans\":   [\n                    \"\\\\b(true)\\\\b\", \n                    \"\\\\b(false)\\\\b\",\n                ],\n                \"functions\":  [\n                    \"(int|bool|void|char|double|long|short|size_t)\\\\s+([a-z_][A-Za-z0-9_]*)\\\\s*\\\\(\",\n                    \"\\\\b([a-z_][A-Za-z0-9_]*)\\\\s*\\\\(\",\n                ],\n                \"structs\":    [\n                    \"struct\\\\s+([A-Za-z0-9_]*)\\\\s*\", \n                ],\n                \"attributes\": [\n                    \"^\\\\s*(#.*?)\\\\s\",\n                ],\n                \"headers\":    [\n                    \"(<.*?>)\",\n                ],\n                \"operators\":  [\n                    r\"(=)\",\n                    r\"(\\+)\",\n                    r\"(\\-)\",\n                    r\"(\\*)\",\n                    r\"(\\s/\\s)\",\n                    r\"(%)\",\n                    r\"(\\+=)\",\n                    r\"(\\-=)\",\n                    r\"(\\*=)\",\n                    r\"(\\\\=)\",\n                    r\"(==)\",\n                    r\"(!=)\",\n                    r\"(>=)\",\n                    r\"(<=)\",\n                    r\"(<)\",\n                    r\"(>)\",\n                    r\"(<<)\",\n                    r\"(>>)\",\n                    r\"(\\&\\&)\",\n                    r\"(\\|\\|)\",\n                    r\"(!)\\S\",\n                ],\n            }\n        ),\n    ],\n)\n".to_string()
}