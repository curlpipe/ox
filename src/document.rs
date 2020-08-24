// Document.rs - For managing external files
use crate::Row; // The Row struct
use std::fs; // For managing file reading and writing

// Document struct (class) to manage files and text
pub struct Document {
    pub rows: Vec<Row>, // For holding the contents of the document
    pub path: String,   // For holding the path to the document
    pub name: String,   // For holding the name of the document
}

// Add methods to the document struct
impl Document {
    pub fn from(path: Option<&str>) -> Self {
        // Create a new document from a path
        if let Some(path) = path {
            // Path was provided
            let mut import = Vec::new();
            if let Ok(file) = fs::read_to_string(path) {
                // File exists and can be accessed
                let mut lines = file.split('\n').collect::<Vec<&str>>();
                lines.pop();
                for row in lines {
                    import.push(Row::from(row));
                }
            } else {
                // File can't be accessed
                import = vec![Row::from("")];
            }
            Self {
                rows: import,
                name: path.to_string(),
                path: path.to_string(),
            }
        } else {
            // Empty path
            Self {
                rows: vec![Row::from("")],
                name: String::from("[No name]"),
                path: String::new(),
            }
        }
    }
    pub fn save(&self) -> std::io::Result<()> {
        // Save a file
        fs::write(&self.path, self.render())
    }
    pub fn save_as(&self, path: &str) -> std::io::Result<()> {
        // Save a file to a specific path
        fs::write(path, self.render())
    }
    fn render(&self) -> String {
        // Render the lines of a document for writing
        self.rows
            .iter()
            .map(|x| x.string.clone())
            .collect::<Vec<String>>()
            .join("\n")
            + "\n"
    }
    pub fn identify(&self) -> &str {
        // Identify which type of file the current buffer is
        let extension = self.name.split('.').last();
        match extension.unwrap() {
            "asm" => "Assembly",
            "b" => "B",
            "bf" => "Brainfuck",
            "bas" => "Basic",
            "bat" => "Batch file",
            "bash" => "Bash",
            "c" => "C",
            "cr" => "Crystal",
            "cs" => "C#",
            "cpp" => "C++",
            "css" => "CSS",
            "csv" => "CSV",
            "class" | "java" => "Java",
            "d" => "D",
            "db" => "Database",
            "erb" => "ERB",
            "fish" => "Fish shell",
            "go" => "Go",
            "gds" => "Godot Script",
            "gitignore" => "Gitignore",
            "hs" => "Haskell",
            "html" => "HTML",
            "js" => "JavaScript",
            "json" => "JSON",
            "lua" => "LUA",
            "log" => "Log file",
            "md" => "Markdown",
            "nim" => "Nim",
            "py" | "pyc" => "Python",
            "php" => "PHP",
            "r" => "R",
            "rs" => "Rust",
            "rb" => "Ruby",
            "sh" => "Shell",
            "sql" => "SQL",
            "swift" => "Swift",
            "sqlite" => "SQLite",
            "txt" => "Plain Text",
            "toml" => "Toml",
            "xml" => "XML",
            "vb" => "VB Script",
            "vim" => "VimScript",
            "yml" | "yaml" => "YAML",
            "zsh" => "Z Shell",
            _ => "Unknown",
        }
    }
}
