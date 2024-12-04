/// Tools for managing and identifying file types
use crate::config;
use crate::editor::Config;
use kaolinite::utils::get_file_name;
use kaolinite::Document;
use std::path::Path;
use synoptic::{from_extension, Highlighter, Regex};

/// A struct to store different file types and provide utilities for finding the correct one
#[derive(Default, Debug, Clone)]
pub struct FileTypes {
    /// The file types available
    pub types: Vec<FileType>,
}

impl FileTypes {
    pub fn identify(&self, doc: &mut Document) -> Option<FileType> {
        for t in &self.types {
            let mut extension = String::new();
            let mut file_name = String::new();
            if let Some(f) = &doc.file_name {
                file_name = get_file_name(f).unwrap_or_default();
                if let Some(e) = Path::new(&f).extension() {
                    extension = e.to_str().unwrap_or_default().to_string();
                }
            }
            doc.load_to(1);
            let first_line = doc.line(0).unwrap_or_default();
            if t.fits(&extension, &file_name, &first_line) {
                return Some(t.clone());
            }
        }
        None
    }

    pub fn identify_from_path(&self, path: &str) -> Option<FileType> {
        if let Some(e) = Path::new(&path).extension() {
            let file_name = get_file_name(path).unwrap_or_default();
            let extension = e.to_str().unwrap_or_default().to_string();
            for t in &self.types {
                if t.fits(&extension, &file_name, "") {
                    return Some(t.clone());
                }
            }
        }
        None
    }

    pub fn get_name(&self, name: &str) -> Option<FileType> {
        self.types.iter().find(|t| t.name == name).cloned()
    }
}

/// An struct to represent the characteristics of a file type
#[derive(Debug, Clone)]
pub struct FileType {
    /// The name of the file type
    pub name: String,
    /// The icon representing the file type
    pub icon: String,
    /// The file names that files of this type exhibit
    pub files: Vec<String>,
    /// The extensions that files of this type have
    pub extensions: Vec<String>,
    /// The modelines that files of this type have
    pub modelines: Vec<String>,
    /// The colour associated with this file type
    pub color: String,
}

impl Default for FileType {
    fn default() -> Self {
        FileType {
            name: "Unknown".to_string(),
            icon: "󰈙 ".to_string(),
            files: vec![],
            extensions: vec![],
            modelines: vec![],
            color: "grey".to_string(),
        }
    }
}

impl FileType {
    /// Determine whether a file fits with this file type
    pub fn fits(&self, extension: &String, file_name: &String, first_line: &str) -> bool {
        let mut modelines = false;
        for modeline in &self.modelines {
            if let Ok(re) = Regex::new(&format!("^{modeline}\\s*$")) {
                if re.is_match(first_line) {
                    modelines = true;
                    break;
                }
            }
        }
        self.extensions.contains(extension) || self.files.contains(file_name) || modelines
    }

    /// Identify the correct highlighter to use
    pub fn get_highlighter(&self, config: &Config, tab_width: usize) -> Highlighter {
        if let Some(highlighter) = config!(config, syntax).user_rules.get(&self.name) {
            // The user has defined their own syntax highlighter for this file type
            highlighter.clone()
        } else {
            // The user hasn't defined their own syntax highlighter, use synoptic builtins
            for ext in &self.extensions {
                if let Some(h) = from_extension(ext, tab_width) {
                    return h;
                }
            }
            Highlighter::new(tab_width)
        }
    }
}
