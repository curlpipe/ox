/// utils.rs - utilities to assist in editing and keep code in document.rs readable
use std::ops::{Bound, RangeBounds};
use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Utility for easily forming a regular expression from a string
#[macro_export]
macro_rules! regex {
    () => {
        regex::Regex::new("").unwrap()
    };
    ($ex:expr) => {
        if let Ok(reg) = regex::Regex::new($ex) {
            reg
        } else {
            // Pattern that will not match anything
            regex::Regex::new("a^").unwrap()
        }
    };
}

/// Represents a location
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Loc {
    pub y: usize,
    pub x: usize,
}

impl Loc {
    /// Shorthand to produce a location
    #[must_use]
    pub fn at(x: usize, y: usize) -> Self {
        Self { y, x }
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
        return String::new();
    }
    let desired_length = string.width().saturating_sub(start);
    let mut chars: String = string;
    while chars.width() > desired_length {
        chars = chars.chars().skip(1).collect();
    }
    if chars.width() < desired_length {
        chars = format!(" {chars}");
    }
    while chars.width() > length {
        chars.pop();
    }
    if chars.width() < length && desired_length > length {
        chars = format!("{chars} ");
    }
    chars
}

/// Extract range information
pub fn get_range<R>(range: &R, min: usize, max: usize) -> (usize, usize)
where
    R: RangeBounds<usize>,
{
    let start = match range.start_bound() {
        Bound::Unbounded => 0,
        Bound::Excluded(_) => unreachable!(),
        Bound::Included(x) => *x,
    };
    let end = match range.end_bound() {
        Bound::Unbounded => max.saturating_sub(min),
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

/// Utility function to determine the width of a character, with variable tab width
#[must_use]
pub fn width_char(ch: &char, tab_width: usize) -> usize {
    match ch {
        '\t' => tab_width,
        _ => ch.width().unwrap_or(0),
    }
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

/// Will get the absolute path to a file
#[must_use]
pub fn get_absolute_path(path: &str) -> Option<String> {
    let abs = std::fs::canonicalize(path).ok()?;
    let mut abs = abs.to_string_lossy().to_string();
    if abs.starts_with("\\\\?\\") {
        abs = abs[4..].to_string();
    }
    Some(abs)
}

/// Will get the file name from a file
#[must_use]
pub fn get_file_name(path: &str) -> Option<String> {
    let p = Path::new(path);
    p.file_name()
        .and_then(|name| name.to_str())
        .map(std::string::ToString::to_string)
}

/// Will get the file name from a file
#[must_use]
pub fn get_file_ext(path: &str) -> Option<String> {
    let p = Path::new(path);
    p.extension()
        .and_then(|name| name.to_str())
        .map(std::string::ToString::to_string)
}

/// Will get the current working directory
#[must_use]
#[cfg(not(tarpaulin_include))]
pub fn get_cwd() -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let mut cwd = cwd.display().to_string();
    // Strip away annoying verbatim component
    if cwd.starts_with("\\\\?\\") {
        cwd = cwd[4..].to_string();
    }
    Some(cwd)
}

/// Will list a directory
#[must_use]
#[cfg(not(tarpaulin_include))]
pub fn list_dir(path: &str) -> Option<Vec<String>> {
    Some(
        std::fs::read_dir(path)
            .ok()?
            .filter_map(std::result::Result::ok)
            .filter_map(|e| e.path().to_str().map(std::string::ToString::to_string))
            .collect(),
    )
}

/// Get the parent directory
#[must_use]
#[cfg(not(tarpaulin_include))]
pub fn get_parent(path: &str) -> Option<String> {
    Path::new(path).parent().map(|p| p.display().to_string())
}

/// Determine if something is a directory or a file
#[must_use]
#[cfg(not(tarpaulin_include))]
pub fn file_or_dir(path: &str) -> &str {
    let path = Path::new(path);
    let metadata = std::fs::metadata(path);
    if let Ok(metadata) = metadata {
        if metadata.is_file() {
            "file"
        } else if metadata.is_dir() {
            "directory"
        } else {
            "neither"
        }
    } else {
        "neither"
    }
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

/// Determine the icon for the file type
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn icon(language: &str) -> String {
    match language {
        "Ada" => "",
        "AutoHotkey" => " ",
        "AppleScript" => "",
        "ActionScript" => "󰑷 ",
        "Assembly" => " ",
        "Batch" => "󰆍 ",
        "Brainfuck" => " ",
        "C" | "C Header" => " ",
        "CMake" | "Makefile" => " ",
        "Java" => " ",
        "Clojure" => " ",
        "CoffeeScript" => " ",
        "Crystal" => " ",
        "Cuda" => " ",
        "C++" | "C++ Header" => " ",
        "C#" => " ",
        "CSS" => " ",
        "CSV" => " ",
        "D" => " ",
        "Dart" => " ",
        "Diff" => " ",
        "Dockerfile" => " ",
        "Elixr" => " ",
        "Elm" => " ",
        "Emacs Lisp" => " ",
        "Erlang" => " ",
        "F#" => " ",
        "FORTRAN" => "󱈚 ",
        "Fish" => " ",
        "GDScript" => " ",
        "GLSL" => " ",
        "Gnuplot" => " ",
        "Go" => "",
        "Groovy" => " ",
        "Haml" => "",
        "Handlebars" => "󰅩 ",
        "Haskell" => " ",
        "HTML" => " ",
        "INI" => " ",
        "Arduino" => " ",
        "J" => " ",
        "JSON" => " ",
        "JSX" => " ",
        "JavaScript" => " ",
        "Julia" => " ",
        "Kotlin" => " ",
        "Lua" => " ",
        "LiveScript" => " ",
        "Common Lisp" => " ",
        "Log file" => " ",
        "Matlab" => " ",
        "Objective-C" => " ",
        "OCaml" => " ",
        "Markdown" => " ",
        "Nix" => " ",
        "NumPy" => "󰘨 ",
        "PHP" => "󰌟 ",
        "Perl" => " ",
        "PowerShell" => "󰨊 ",
        "Prolog" => " ",
        "Python" | "Cython" => " ",
        "R" => " ",
        "reStructuredText" => "󰊄",
        "Ruby" => " ",
        "Rust" => " ",
        "Shell" | "Zsh" => " ",
        "SCSS" | "Sass" => " ",
        "SQL" => " ",
        "Scala" => "",
        "Scheme" => "",
        "Swift" => " ",
        "TOML" => " ",
        "TeX" => " ",
        "TypeScript" => " ",
        "Plain Text" => " ",
        "Vala" => " ",
        "Visual Basic" => "󰯁 ",
        "Vue" => " ",
        "XML" => "󰗀 ",
        _ => "󰈙 ",
    }
    .to_string()
}

/// Determine the file extension based off the magic modeline (if present)
#[must_use]
pub fn modeline(first_line: &str) -> Option<&str> {
    // Create a regex to handle leading/trailing whitespaces and spaces between '#!' and path
    let re = regex!(r"^#!\s*/\s*(\S+)(\s+\S+)?");

    // Match the cleaned-up shebang
    if let Some(caps) = re.captures(first_line) {
        let shebang = caps
            .get(0)
            .map(|m| m.as_str().replace("#! ", "#!"))
            .unwrap_or_default();
        match shebang.as_str() {
            "#!/bin/sh" | "#!/usr/bin/env bash" | "#!/bin/bash" => Some("sh"),
            "#!/usr/bin/python"
            | "#!/usr/bin/python3"
            | "#!/usr/bin/env python"
            | "#!/usr/bin/env python3" => Some("py"),
            "#!/usr/bin/env ruby" | "#!/usr/bin/ruby" => Some("rb"),
            "#!/usr/bin/perl" | "#!/usr/bin/env perl" => Some("pl"),
            "#!/usr/bin/env node" | "#!/usr/bin/node" => Some("js"),
            "#!/usr/bin/env lua" | "#!/usr/bin/lua" => Some("lua"),
            "#!/usr/bin/env php" | "#!/usr/bin/php" => Some("php"),
            "#!/usr/bin/env rust" => Some("rs"),
            "#!/usr/bin/env tcl" => Some("tcl"),
            "#!/bin/awk" | "#!/usr/bin/env awk" => Some("awk"),
            "#!/bin/sed" | "#!/usr/bin/env sed" => Some("sed"),
            "#!/usr/bin/env fish" => Some("fish"),
            _ => None,
        }
    } else {
        None
    }
}
