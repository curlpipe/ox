/// utils.rs - utilities to assist in editing and keep code in document.rs readable
use unicode_width::UnicodeWidthStr;
use std::ops::{Bound, RangeBounds};

/// Utility for easily forming a regular expression from a string
#[macro_export]
macro_rules! regex {
    () => { regex::Regex::new("").unwrap() };
    ($ex:expr) => { regex::Regex::new($ex).unwrap() };
}

/// Represents a location
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Loc {
    pub x: usize,
    pub y: usize,
}

impl Loc {
    /// Shorthand to produce a location
    #[must_use]
    pub fn at(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

/// Represents a size
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub w: usize,
    pub h: usize,
}

impl Size {
    /// Shorthand to produce a size
    #[must_use]
    pub fn is(w: usize, h: usize) -> Self {
        Self { w, h }
    }
}

/// Takes a string and cuts it from a start point to a specified length.
/// Works with double width characters.
/// This allows x offset to work well with double width characters.
#[must_use]
pub fn trim(string: &str, start: usize, length: usize, tab_width: usize) -> String {
    let string = string.replace('\t', &" ".repeat(tab_width));
    if start >= string.width() {
        return "".to_string();
    }
    let desired_length = string.width() - start;
    let mut chars: String = string;
    while chars.width() > desired_length {
        chars = chars.chars().skip(1).collect();
    }
    if chars.width() < desired_length {
        chars = format!(" {}", chars);
    }
    while chars.width() > length {
        chars.pop();
    }
    if chars.width() < length && desired_length > length {
        chars = format!("{} ", chars);
    }
    chars
}

/// Extract range information
pub fn get_range<R>(range: &R, min: usize, max: usize) -> (usize, usize) where R: RangeBounds<usize> {
    let start = match range.start_bound() {
        Bound::Unbounded => 0,
        Bound::Excluded(_) => unreachable!(),
        Bound::Included(x) => *x,
    };
    let end = match range.end_bound() {
        Bound::Unbounded => max - min,
        Bound::Excluded(x) => x.saturating_sub(1),
        Bound::Included(x) => *x,
    };
    (start, end)
}

/// Utility function to determine the width of a string, with variable tab width
#[must_use]
pub fn width(st: &str, tab_width: usize) -> usize {
    let tabs = st.matches('\t').count();
    (st.width() + tabs * tab_width).saturating_sub(tabs)
}

/// Utility function to take a line and determine where spaces should be treated as tabs (forwards)
#[must_use]
pub fn tab_boundaries_forward(line: &str, tab_width: usize) -> Vec<usize> {
    let mut at = 0;
    let mut boundaries = vec![];
    while at < width(line, tab_width) {
        let tab_test = line.chars().skip(at).take(tab_width).collect::<String>();
        if tab_test == " ".repeat(tab_width) {
            // Should be treated as a tab
            boundaries.push(at);
            at += tab_width;
        } else {
            // Non-spaces here, don't treat as a tab
            break;
        }
    }
    boundaries
}

/// Utility function to take a line and determine where spaces should be treated as tabs (backwards)
#[must_use]
pub fn tab_boundaries_backward(line: &str, tab_width: usize) -> Vec<usize> {
    let mut at = 0;
    let mut boundaries = vec![];
    while at < width(line, tab_width) {
        let tab_test = line.chars().skip(at).take(tab_width).collect::<String>();
        if tab_test == " ".repeat(tab_width) {
            // Should be treated as a tab
            boundaries.push(at + tab_width);
            at += tab_width;
        } else {
            // Non-spaces here, don't treat as a tab
            break;
        }
    }
    boundaries
}

/// Determine the filetype from the extension
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn filetype(extension: &str) -> Option<String> {
    Some(
        match extension.to_ascii_lowercase().as_str() {
            "abap" => "ABAP",
            "ada" => "Ada",
            "ahk" | "ahkl" => "AutoHotkey",
            "applescript" | "scpt" => "AppleScript",
            "arc" => "Arc",
            "asp" | "asax" | "ascx" | "ashx" | "asmx" | "aspx" | "axd" => "ASP",
            "as" => "ActionScript",
            "asc" | "ash" => "AGS Script",
            "asm" | "nasm" => "Assembly",
            "awk" | "auk" | "gawk" | "mawk" | "nawk" => "Awk",
            "bat" | "cmd" => "Batch",
            "b" | "bf" => "Brainfuck",
            "c" => "C",
            "cmake" => "CMake",
            "cbl" | "cobol" | "cob" => "Cobol",
            "class" | "java" => "Java",
            "clj" | "cl2" | "cljs" | "cljx" | "cljc" => "Clojure",
            "coffee" => "CoffeeScript",
            "cr" => "Crystal",
            "cu" | "cuh" => "Cuda",
            "cpp" | "cxx" => "C++",
            "cs" | "cshtml" | "csx" => "C#",
            "css" => "CSS",
            "csv" => "CSV",
            "d" | "di" => "D",
            "dart" => "Dart",
            "diff" | "patch" => "Diff",
            "dockerfile" => "Dockerfile",
            "ex" | "exs" => "Elixr",
            "elm" => "Elm",
            "el" => "Emacs Lisp",
            "erb" => "ERB",
            "erl" | "es" => "Erlang",
            "fs" | "fsi" | "fsx" => "F#",
            "f" | "f90" | "fpp" | "for" => "FORTRAN",
            "fish" => "Fish",
            "fth" => "Forth",
            "g4" => "ANTLR",
            "gd" => "GDScript",
            "glsl" | "vert" | "shader" | "geo" | "fshader" | "vrx" | "vsh" | "vshader" | "frag" => {
                "GLSL"
            }
            "gnu" | "gp" | "plot" => "Gnuplot",
            "go" => "Go",
            "groovy" | "gvy" => "Groovy",
            "hlsl" => "HLSL",
            "h" => "C Header",
            "haml" => "Haml",
            "handlebars" | "hbs" => "Handlebars",
            "hs" => "Haskell",
            "hpp" => "C++ Header",
            "html" | "htm" | "xhtml" => "HTML",
            "ini" | "cfg" => "INI",
            "ino" => "Arduino",
            "ijs" => "J",
            "json" => "JSON",
            "jsx" => "JSX",
            "js" => "JavaScript",
            "jl" => "Julia",
            "kt" | "ktm" | "kts" => "Kotlin",
            "ll" => "LLVM",
            "l" | "lex" => "Lex",
            "lua" => "Lua",
            "ls" => "LiveScript",
            "lol" => "LOLCODE",
            "lisp" | "asd" | "lsp" => "Common Lisp",
            "log" => "Log file",
            "m4" => "M4",
            "man" | "roff" => "Groff",
            "matlab" => "Matlab",
            "m" => "Objective-C",
            "ml" => "OCaml",
            "mk" | "mak" => "Makefile",
            "md" | "markdown" => "Markdown",
            "nix" => "Nix",
            "numpy" => "NumPy",
            "opencl" | "cl" => "OpenCL",
            "php" => "PHP",
            "pas" => "Pascal",
            "pl" => "Perl",
            "psl" => "PowerShell",
            "pro" => "Prolog",
            "py" | "pyw" => "Python",
            "pyx" | "pxd" | "pxi" => "Cython",
            "r" => "R",
            "rst" => "reStructuredText",
            "rkt" => "Racket",
            "rb" | "ruby" => "Ruby",
            "rs" => "Rust",
            "sh" => "Shell",
            "scss" => "SCSS",
            "sql" => "SQL",
            "sass" => "Sass",
            "scala" => "Scala",
            "scm" => "Scheme",
            "st" => "Smalltalk",
            "swift" => "Swift",
            "toml" => "TOML",
            "tcl" => "Tcl",
            "tex" => "TeX",
            "ts" | "tsx" => "TypeScript",
            "txt" => "Plain Text",
            "vala" => "Vala",
            "vb" | "vbs" => "Visual Basic",
            "vue" => "Vue",
            "xm" | "x" | "xi" => "Logos",
            "xml" => "XML",
            "y" | "yacc" => "Yacc",
            "yaml" | "yml" => "Yaml",
            "yxx" => "Bison",
            "zsh" => "Zsh",
            _ => return None,
        }
        .to_string(),
    )
}
