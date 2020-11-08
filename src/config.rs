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

// Key binding type
#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
pub enum KeyBinding {
    Ctrl(char),
    Alt(char),
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
            } else {
                // There is a syntax issue with the config file
                let result: Result<Self, ron::Error> = from_str(&file);
                // Provide the syntax issue with the config file for debugging
                (
                    from_str(DEFAULT).unwrap(),
                    Status::Parse(format!("{:?}", result)),
                )
            };
            result
        } else {
            // File wasn't able to be found
            (from_str(DEFAULT).unwrap(), Status::File)
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
    pub status_left: String,
    pub status_right: String,
    pub tab: String,
}

// Struct for storing theme information
#[derive(Debug, Deserialize, Clone)]
pub struct Theme {
    pub editor_bg: (u8, u8, u8),
    pub editor_fg: (u8, u8, u8),
    pub status_bg: (u8, u8, u8),
    pub status_fg: (u8, u8, u8),
    pub line_number_fg: (u8, u8, u8),
    pub inactive_tab_fg: (u8, u8, u8),
    pub inactive_tab_bg: (u8, u8, u8),
    pub active_tab_fg: (u8, u8, u8),
    pub active_tab_bg: (u8, u8, u8),
    pub default_theme: String,
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
const DEFAULT: &str = r#"
// General settings for Ox
(
	general: General(
		line_number_padding_right: 2, // Line number padding on the right
		line_number_padding_left:  1, // Line number padding on the left
		tab_width:                 4, // The amount of spaces for a tab
		undo_period:               5, // Seconds of inactivity for undo
		status_left:  " %f%d %D \u{2502} %n %i", // Left part of status line
		status_right: "\u{4e26} %l / %L \u{2502} \u{fae6}(%x, %y) ", // Right part of status line
		tab: "%I%f%d", // Tab formatting
	),
	// Custom defined macros
	macros: {
		// Macro to move a line up
		"move line up": [
			"store line 1", // Store current line in bank #1
			"delete 0",     // Delete current line
			"move 1 up",    // Move cursor up by 1
			"line above",   // Insert an empty line above
			"move 1 up",    // Move cursor up to the empty line
			"load line 1",  // Load line in bank #1 over the empty line
		],
		// Macro to move a line down
		"move line down": [
			"store line 1", // Store the current line in bank #1
			"delete 0",     // Delete the current line
			"line below",   // Create an empty line below
			"move 1 down",  // Move cursor down to empty line
			"load line 1",  // Overwrite empty line with line in bank #1
		],
	},
	// RGB values for the colours of Ox
	theme: Theme(
		editor_bg:        (41, 41, 61), // The main background color
		editor_fg:        (255, 255, 255), // The default text color
		status_bg:        (59, 59, 84), // The background color of the status line
		status_fg:        (35, 240, 144), // The text color of the status line
		line_number_fg:   (65, 65, 98), // The text color of the line numbers
		active_tab_fg:    (255, 255, 255), // The text color of the active tab
		active_tab_bg:    (41, 41, 61), //  The background color of the active tab
		inactive_tab_fg:  (255, 255, 255), // The text color of the inactive tab(s)
		inactive_tab_bg:  (59, 59, 84), // The text color of the inactive tab(s)
		default_theme:    "default", // The default syntax highlights to use
	),
	// Colours for the syntax highlighting
	highlights: {
		"default": {
			"comments":   (113, 113, 169),
			"keywords":   (134, 76, 232),
			"references": (134, 76, 232),
			"strings":    (39, 222, 145),
			"characters": (40, 198, 232),
			"digits":     (40, 198, 232),
			"booleans":   (86, 217, 178),
			"functions":  (47, 141, 252),
			"structs":    (47, 141, 252),
			"macros":     (223, 52, 249),
			"attributes": (40, 198, 232),
			"headers":    (47, 141, 252),
			"symbols":    (47, 141, 252),
			"global":     (86, 217, 178),
		},
		"alternative": {
			"comments":   (113, 113, 169),
			"keywords":   (64, 86, 244),
			"references": (64, 86, 244),
			"strings":    (76, 224, 179),
			"characters": (110, 94, 206),
			"digits":     (4, 95, 204),
			"booleans":   (76, 224, 179),
			"functions":  (4, 95, 204),
			"structs":    (4, 95, 204),
			"macros":     (110, 94, 206),
			"attributes": (4, 95, 204),
			"headers":    (141, 129, 217),
			"symbols":    (249, 233, 0),
			"global":     (76, 224, 179),
		},
	},
	// Key bindings
	keys: {
		// Keybinding: [Oxa commands]
		Ctrl('q'): ["quit"], // Quit current document
		Ctrl('s'): ["save"], // Save current document
		Ctrl('w'): ["save ?"], // Save current document as
		Ctrl('p'): ["save *"], // Save all open documents
		Ctrl('n'): ["new"], // Create new document
		Ctrl('o'): ["open"], // Open document
		Ctrl('d'): ["prev"], // Move to previous tab
		Ctrl('h'): ["next"], // Move to next tab
		Ctrl('z'): ["undo"], // Undo last edit
		Ctrl('y'): ["redo"], // Redo last edit
		Ctrl('f'): ["search"], // Trigger search command
		Ctrl('r'): ["replace"], // Trigger replace command
		Ctrl('a'): ["replace *"], // Trigger replace all command
		Ctrl('v'): ["move line up"], // Move line up
		Ctrl('k'): ["move line down"], // Move line down
		Alt('a'):  ["cmd"], // Open the command line
	},
	// Language specific settings
	languages: [
		Language(
			name: "Rust", // Name of the language
			icon: "\u{e7a8} ", // Icon for the language
			extensions: ["rs"], // Extensions of the language
			// Keywords of the language
			keywords: [
				"as", "break", "const", "continue", "crate", "else", 
				"enum", "extern", "fn", "for", "if", "impl", "in", 
				"let", "loop", "match", "mod", "move", "mut", "pub", 
				"ref", "return", "self", "static", "struct", "super", 
				"trait", "type", "unsafe", "use", "where", "while", 
				"async", "await", "dyn", "abstract", "become", "box", 
				"do", "final", "macro", "override", "priv", "typeof", 
				"unsized", "virtual", "yield", "try", "'static",
				"u8", "u16", "u32", "u64", "u128", "usize",
				"i8", "i16", "i32", "i64", "i128", "isize",
				"f32", "f64", "String", "Vec", "str", "Some", "bool",
				"None", "Box", "Result", "Option", "Ok", "Err",
			],
			// Syntax definitions
			definitions: {
				"comments":   [
					"(?m)(//.*)$", 
					"(?ms)(/\\*.*?\\*/)",
				],
				"strings":    [
					"(\".*?\")",
				],
				"characters": [
					"('.')", 
					"('\\\\.')",
				],
				"digits":     [
					"\\b(\\d+.\\d+|\\d+)",
					"\\b(\\d+.\\d+(?:f32|f64))",
				],
				"booleans":   [
					"\\b(true)\\b", 
					"\\b(false)\\b",
				],
				"functions":  [
					"fn\\s+([a-z_][A-Za-z0-9_]*)\\s*\\(",
				],
				"structs":    [
					"(?:trait|enum|struct|impl)\\s+([A-Z][A-Za-z0-9_]*)\\s*", 
					"impl(?:<.*?>|)\\s+([A-Z][A-Za-z0-9_]*)",
					"([A-Z][A-Za-z0-9_]*)::",
					"impl.*for\\s+([A-Z][A-Za-z0-9_]*)",
				],
				"macros":     [
					"\\b([a-z_][a-zA-Z0-9_]*!)",
				],
				"attributes": [
					"(?ms)^\\s*(#(?:!|)\\[.*?\\])",
				],
				"references": [
					"&str", "&mut", "&self", 
					"&i8", "&i16", "&i32", "&i64", "&i128", "&isize",
					"&u8", "&u16", "&u32", "&u64", "&u128", "&usize",
					"&f32", "&f64",
				]
			}
		),
		Language(
			name: "Ruby", // Name of the language
			icon: "\u{e739} ", // Icon for the language
			extensions: ["rb"], // Extensions of the language
			// Keywords of the language
			keywords: [
				"__ENCODING__", "__LINE__", "__FILE__", "BEGIN", "END", 
				"alias", "and", "begin", "break", "case", "class", "def", 
				"defined?", "do", "else", "elsif", "end", "ensure", "print",
				"for", "if", "in", "module", "next", "nil", "not", "or", "puts",
				"redo", "rescue", "retry", "return", "self", "super", "then", 
				"undef", "unless", "until", "when", "while", "yield", "raise",
				"include", "extend", 
			],
			// Syntax definitions
			definitions: {
				"comments":   [
					"(?m)(#.*)$", 
					"(?ms)(=begin.*=end)", 
				],
				"strings":    [
					"((?:f|r|)\".*?\")",
					"(\'.*?\')",
				],
				"digits":     [
					r"\b(\d+.\d+|\d+)",
				],
				"booleans":   [
					r"\b(true)\b", 
					r"\b(false)\b",
				],
				"structs":    [
					r"class(\s+[A-Za-z0-9_]*)",
				],
				"functions":  [
					r"def\s+([a-z_][A-Za-z0-9_]*)",
				],
				"symbols":    [
					r"(:[^,\)\.\s=]+)",
				],
				"global":     [
					r"(\$[a-z_][A-Za-z0-9_]*)\s",
				]
			}
		),
		Language(
			name: "Crystal", // Name of the language
			icon: "\u{e7a3} ", // Icon for the language
			extensions: ["cr"], // Extensions of the language
			// Keywords of the language
			keywords: [
				"__ENCODING__", "__LINE__", "__FILE__", "BEGIN", "END", 
				"alias", "and", "begin", "break", "case", "class", "def", 
				"defined?", "do", "else", "elsif", "end", "ensure", "print",
				"for", "if", "in", "module", "next", "nil", "not", "or", "puts",
				"redo", "rescue", "retry", "return", "self", "super", "then", 
				"undef", "unless", "until", "when", "while", "yield", "raise",
				"include", "extend", "Int32", "String", "getter", "setter",
				"property",
			],
			// Syntax definitions
			definitions: {
				"comments":   [
					"(?m)(#.*)$", 
					"(?ms)(=begin.*=end)", 
				],
				"strings":    [
					"(?ms)(\".*?\")",
					"((?:f|r|)\".*?\")",
					"(\'.*?\')",
				],
				"digits":     [
					r"\b(\d+.\d+|\d+)",
				],
				"booleans":   [
					r"\b(true)\b", 
					r"\b(false)\b",
				],
				"structs":    [
					r"class(\s+[A-Za-z0-9_]*)",
				],
				"functions":  [
					r"def\s+([a-z_][A-Za-z0-9_]*)",
				],
				"symbols":    [
					r"(:[^,\}\)\.\s=]+)",
				],
				"global":     [
					r"(\$[a-z_][A-Za-z0-9_]*)\s",
				]
			}
		),
		Language(
			name: "Python", // Name of the language
			icon: "\u{e73c} ", // Icon for the language
			extensions: ["py", "pyw"], // Extensions of the language
			// Keywords of the language
			keywords: [
				"and", "as", "assert", "break", "class", "continue", 
				"def", "del", "elif", "else", "except", "exec", 
				"finally", "for", "from", "global", "if", "import", 
				"in", "is", "lambda", "not", "or", "pass", "print", 
				"raise", "return", "try", "while", "with", "yield",
				"str", "bool", "int", "tuple", "list", "dict", "tuple",
				"len", "None", "input", "type", "set", "range", "enumerate",
				"open", "iter", "min", "max", "dir", "self", "isinstance", 
				"help", "next", "super",
			],
			// Syntax definitions
			definitions: {
				"comments":   [
					"(?m)(#.*)$", 
				],
				"strings":    [
					"(?ms)(\"\"\".*?\"\"\")",
					"(?ms)(\'\'\'.*?\'\'\')",
					"((?:f|r|)\".*?\")",
					"(\'.*?\')",
				],
				"digits":     [
					"\\b(\\d+.\\d+|\\d+)",
				],
				"booleans":   [
					"\\b(True)\\b", 
					"\\b(False)\\b",
				],
				"structs":    [
					"class\\s+([A-Za-z0-9_]*)",
				],
				"functions":  [
					"def\\s+([a-z_][A-Za-z0-9_]*)",
				],
				"attributes": [
					"@.*$",
				]
			}
		),
		Language(
			name: "Javascript", // Name of the language
			icon: "\u{e74e} ", // Icon for the language
			extensions: ["js"], // Extensions of the language
			// Keywords of the language
			keywords: [
				"abstract", "arguments", "await", "boolean", "break", "byte", 
				"case", "catch", "char", "class", "const", "continue", "debugger", 
				"default", "delete", "do", "double", "else", "enum", "eval", 
				"export", "extends", "final", "finally", "float", "for", "of",
				"function", "goto", "if", "implements", "import", "in", "instanceof", 
				"int", "interface", "let", "long", "native", "new", "null", "package", 
				"private", "protected", "public", "return", "short", "static", 
				"super", "switch", "synchronized", "this", "throw", "throws", 
				"transient", "try", "typeof", "var", "void", "volatile", "console",
				"while", "with", "yield", "undefined", "NaN", "-Infinity", "Infinity",
			],
			// Syntax definitions
			definitions: {
				"comments":   [
					"(?m)(//.*)$", 
					"(?ms)(/\\*.*\\*/)$", 
				],
				"strings":    [
					"(?ms)(\"\"\".*?\"\"\")",
					"(?ms)(\'\'\'.*?\'\'\')",
					"((?:f|r|)\".*?\")",
					"(\'.*?\')",
				],
				"digits":     [
					"\\b(\\d+.\\d+|\\d+)",
				],
				"booleans":   [
					"\\b(true)\\b", 
					"\\b(false)\\b",
				],
				"structs":    [
					"class\\s+([A-Za-z0-9_]*)",
				],
				"functions":  [
					"function\\s+([a-z_][A-Za-z0-9_]*)",
					"\\b([a-z_][A-Za-z0-9_]*)\\s*\\("
				],
			}
		),
		Language(
			name: "C", // Name of the language
			icon: "\u{e61e} ", // Icon for the language
			extensions: ["c", "h"], // Extensions of the language
			// Keywords of the language
			keywords: [
				"auto", "break", "case", "char", "const", "continue", "default", 
				"do", "double", "else", "enum", "extern", "float", "for", "goto", 
				"if", "int", "long", "register", "return", "short", "signed", 
				"sizeof", "static", "struct", "switch", "typedef", "union", 
				"unsigned", "void", "volatile", "while", "printf", "fscanf", 
				"scanf", "fputsf", "exit", "stderr", "malloc", "calloc", "bool",
				"realloc", "free", "strlen", "size_t",
			],
			// Syntax definitions
			definitions: {
				"comments":   [
					"(?m)(//.*)$", 
					"(?ms)(/\\*.*?\\*/)",
				],
				"strings":    [
					"(\".*?\")",
				],
				"characters": [
					"('.')", 
					"('\\\\.')",
				],
				"digits":     [
					"\\b(\\d+.\\d+|\\d+)",
					"\\b(\\d+.\\d+(?:f|))",
				],
				"booleans":   [
					"\\b(true)\\b", 
					"\\b(false)\\b",
				],
				"functions":  [
					"(int|bool|void|char|double|long|short|size_t)\\s+([a-z_][A-Za-z0-9_]*)\\s*\\(",
				],
				"structs":    [
					"struct\\s+([A-Za-z0-9_]*)\\s*", 
				],
				"attributes": [
					"^\\s*(#.*?)\\s",
				],
				"headers":    [
					"(<.*?>)",
				],
			}
		),
	],
)
"#;
