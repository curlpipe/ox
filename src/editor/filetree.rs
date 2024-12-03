/// Utilities for handling the file tree
use crate::config::{Color, FileTree as CfgFT};
use crate::editor::FileLayout;
use crate::{config, Editor, FileTypes, OxError, Result};
use kaolinite::utils::{file_or_dir, get_cwd, get_file_name};
use std::path::{Path, PathBuf};

/// How parts of a file tree are stored
/// (Padding, Icon, Icon Color, File Name)
pub type FTParts = Vec<(usize, String, Option<Color>, String)>;

/// The backend of a file tree - stores the structure of the files and directories
#[derive(Debug)]
pub enum FileTree {
    /// Represents a file
    File { path: String },
    /// Represents a directory
    Dir {
        path: String,
        /// NOTE: when files is None, it means it has been unexpanded
        /// directories lazily expand, only when the user requests them to be opened
        files: Option<Vec<FileTree>>,
    },
}

impl FileTree {
    /// Build a file tree from a directory
    pub fn build(dir: &str) -> Result<Self> {
        // Ensure we have the absolute path
        let root = std::fs::canonicalize(dir)?;
        let mut result = Self::build_shallow(&root)?;
        result.sort();
        Ok(result)
    }

    /// Expands into a directory
    fn build_shallow(path: &PathBuf) -> Result<Self> {
        if path.is_file() {
            Ok(Self::File {
                path: Self::path_to_string(path),
            })
        } else if path.is_dir() {
            let mut files = vec![];
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                if entry.path().is_file() {
                    files.push(Self::File {
                        path: Self::path_to_string(&entry.path()),
                    });
                } else if entry.path().is_dir() {
                    files.push(Self::Dir {
                        path: Self::path_to_string(&entry.path()),
                        files: None,
                    });
                }
            }
            Ok(Self::Dir {
                path: Self::path_to_string(path),
                files: Some(files),
            })
        } else {
            Err(OxError::InvalidPath)
        }
    }

    /// Takes a path and turns it into a string
    fn path_to_string(path: &Path) -> String {
        let mut path = path.to_string_lossy().to_string();
        if path.starts_with("\\\\?\\") {
            path = path[4..].to_string();
        }
        path
    }

    /// Search for and retrieve a mutable reference to a node
    pub fn get_mut(&mut self, needle: &str) -> Option<&mut Self> {
        match self {
            Self::File { path } => {
                if needle == path {
                    // Match found!
                    Some(self)
                } else {
                    // No match
                    None
                }
            }
            Self::Dir { path, .. } => {
                if needle == path {
                    // This directory is what we're searching for
                    Some(self)
                } else if let Self::Dir {
                    files: Some(files), ..
                } = self
                {
                    // Not directly what we're looking for, let's go deeper
                    for file in files {
                        if let Some(result) = file.get_mut(needle) {
                            // Found it! Return upwards
                            return Some(result);
                        }
                    }
                    // None of the files match up
                    None
                } else {
                    // Dead end
                    None
                }
            }
        }
    }

    /// Expand a directory downwards
    pub fn expand(&mut self) {
        if let Self::Dir { path, .. } = self {
            // Expand this directory
            if let Ok(root) = std::fs::canonicalize(path) {
                if let Ok(mut expanded) = Self::build_shallow(&root) {
                    expanded.sort();
                    *self = expanded;
                }
            }
        }
    }

    /// Sort a file tree to have directories and files separated and ordered alphabetically
    fn sort(&mut self) {
        match self {
            Self::File { .. } => (),
            Self::Dir { files, .. } => {
                // Sort child directories
                if let Some(files) = files {
                    for file in files.iter_mut() {
                        file.sort();
                    }

                    // Sort this directory
                    files.sort_by(|a, b| {
                        let a_is_hidden = a.is_hidden();
                        let b_is_hidden = b.is_hidden();
                        let a_is_dir = matches!(a, FileTree::Dir { .. });
                        let b_is_dir = matches!(b, FileTree::Dir { .. });

                        // Directories come first
                        match (a_is_hidden, b_is_hidden) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => {
                                // If both are the same hidden status, directories come first
                                match (a_is_dir, b_is_dir) {
                                    (true, false) => std::cmp::Ordering::Less,
                                    (false, true) => std::cmp::Ordering::Greater,
                                    _ => {
                                        // If both are the same type, compare by path
                                        let a_path = match a {
                                            FileTree::File { path }
                                            | FileTree::Dir { path, .. } => path,
                                        };
                                        let b_path = match b {
                                            FileTree::File { path }
                                            | FileTree::Dir { path, .. } => path,
                                        };
                                        a_path.cmp(b_path)
                                    }
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    /// Work out if this node is hidden or not
    fn is_hidden(&self) -> bool {
        let path = match self {
            Self::File { path } | Self::Dir { path, .. } => path,
        };
        get_file_name(path).is_some_and(|name| name.starts_with('.'))
    }

    /// Get a language-related icon for this node
    fn lang_icon(&self, fts: &FileTypes, config: &CfgFT) -> Option<(String, Color)> {
        if config.language_icons {
            let path = match self {
                Self::File { path } | Self::Dir { path, .. } => path,
            };
            if let Some(ft) = fts.identify_from_path(path) {
                return Some((ft.icon, ft.color));
            }
        }
        None
    }

    /// Get the appropriate icon
    fn icon(&self, fts: &FileTypes, config: &CfgFT) -> (String, Option<Color>) {
        let is_file = match self {
            Self::File { .. } => true,
            Self::Dir { .. } => false,
        };
        let is_expanded = match self {
            Self::File { .. } => false,
            Self::Dir { files, .. } => files.is_some(),
        };
        let is_hidden = self.is_hidden();
        match (self.lang_icon(fts, config), is_file, is_hidden, is_expanded) {
            // Language specific icons
            (Some((icon, colour)), _, _, _) => (icon + " ", Some(colour)),
            // Closed folders
            (_, false, false, false) => ("󰉖  ".to_string(), None),
            (_, false, true, false) => ("󱞞  ".to_string(), None),
            // Opened folders
            (_, false, _, true) => ("󰷏  ".to_string(), None),
            // Files
            (_, true, false, _) => ("󰈤  ".to_string(), None),
            (_, true, true, _) => ("󰘓  ".to_string(), None),
        }
    }

    /// Work out if this node is selected
    pub fn is_selected(&self, selection: &str) -> bool {
        match self {
            Self::File { path } | Self::Dir { path, .. } => path == selection,
        }
    }

    /// Display this file tree
    pub fn display(&self, sel: &str, fts: &FileTypes, cfg: &CfgFT) -> (FTParts, Option<usize>) {
        let icons = cfg.icons;
        match self {
            Self::File { path } => {
                let (icon, icon_color) = if icons {
                    self.icon(fts, cfg)
                } else {
                    (String::new(), None)
                };
                let file_name = get_file_name(path).unwrap_or(path.to_string());
                (
                    vec![(0, icon, icon_color, file_name)],
                    if self.is_selected(sel) { Some(0) } else { None },
                )
            }
            Self::Dir { path, files } => {
                let mut result = vec![];
                let mut at = None;
                // Write self
                let (icon, icon_color) = if icons {
                    self.icon(fts, cfg)
                } else {
                    (String::new(), None)
                };
                let file_name = get_file_name(path).unwrap_or(path.to_string());
                result.push((0, icon, icon_color, file_name));
                if self.is_selected(sel) {
                    at = Some(result.len().saturating_sub(1));
                }
                // Write child nodes
                if let Some(files) = files {
                    for file in files {
                        let (sub_display, sub_at) = file.display(sel, fts, cfg);
                        for (c, s) in sub_display.iter().enumerate() {
                            let mut s = s.clone();
                            s.0 += 1;
                            result.push(s);
                            if let Some(sub_at) = sub_at {
                                if sub_at == c {
                                    at = Some(result.len().saturating_sub(1));
                                }
                            }
                        }
                    }
                }
                (result, at)
            }
        }
    }

    /// Find the file path at a certain index
    pub fn flatten(&self) -> Vec<String> {
        match self {
            Self::File { path } => vec![path.to_string()],
            Self::Dir { path, files } => {
                let mut result = vec![];
                result.push(path.to_string());
                if let Some(files) = files {
                    for file in files {
                        result.append(&mut file.flatten());
                    }
                }
                result
            }
        }
    }
}

impl Editor {
    /// Open the file tree
    #[allow(clippy::cast_precision_loss)]
    pub fn open_file_tree(&mut self) {
        if !self.file_tree_is_open() {
            // Calculate display proportions
            let width = config!(self.config, file_tree).width;
            let other = if width <= 1.0 { 1.0 - width } else { 0.0 };
            // Set up file tree values
            self.old_ptr = self.ptr.clone();
            if let Some(cwd) = get_cwd() {
                if let Ok(ft) = FileTree::build(&cwd) {
                    self.file_tree = Some(ft);
                    self.file_tree_selection = Some(cwd);
                }
            }
            // Wrap existing file layout in new file layout
            if let FileLayout::SideBySide(ref mut layouts) = &mut self.files {
                // Shrink existing splits
                let redistribute = width / layouts.len() as f64;
                for (_, prop) in &mut *layouts {
                    if *prop >= redistribute {
                        *prop -= redistribute;
                    } else {
                        *prop = 0.0;
                    }
                }
                // Insert file tree
                layouts.insert(0, (FileLayout::FileTree, width));
            } else {
                self.files = FileLayout::SideBySide(vec![
                    (FileLayout::FileTree, width),
                    (self.files.clone(), other),
                ]);
            }
            self.ptr = vec![0];
        }
    }

    /// Close the file tree
    pub fn close_file_tree(&mut self) {
        if let Some(FileLayout::SideBySide(layouts)) = self.files.get_raw(vec![]) {
            let in_file_tree = matches!(
                self.files.get_raw(self.ptr.clone()),
                Some(FileLayout::FileTree)
            );
            // Locate where the file tree is
            let ftp = layouts
                .iter()
                .position(|(l, _)| matches!(l, FileLayout::FileTree));
            if let Some(at) = ftp {
                // Delete the file tree
                self.files.remove(vec![at]);
                // Clear up any leftovers sidebyside
                if let FileLayout::SideBySide(layouts) = &self.files {
                    if layouts.len() == 1 {
                        // Remove leftover
                        self.files = layouts[0].0.clone();
                    }
                }
                // Reset pointer back to what it used to be IF we're in the file tree
                if in_file_tree {
                    self.ptr = self.old_ptr.clone();
                } else if !self.ptr.is_empty() {
                    // If we're outside the file tree
                    // just take the existing pointer and remove file tree aspect
                    self.ptr.remove(0);
                }
            }
        }
    }

    /// Toggle the file tree
    pub fn toggle_file_tree(&mut self) {
        if self.file_tree_is_open() {
            self.close_file_tree();
        } else {
            self.open_file_tree();
        }
    }

    /// Determine whether the file tree is open
    pub fn file_tree_is_open(&self) -> bool {
        if let Some(FileLayout::SideBySide(layouts)) = self.files.get_raw(vec![]) {
            layouts
                .iter()
                .any(|(layout, _)| matches!(layout, FileLayout::FileTree))
        } else {
            false
        }
    }

    /// Move file tree selection upwards
    pub fn file_tree_select_up(&mut self) {
        if let Some(ref mut fts) = self.render_cache.file_tree_selection {
            // Move up a file (in the render cache)
            *fts = fts.saturating_sub(1);
            // Move up a file (in the backend)
            let flat = self
                .file_tree
                .as_ref()
                .map(FileTree::flatten)
                .unwrap_or_default();
            let new_path = flat.get(*fts);
            self.file_tree_selection = new_path.cloned();
        }
    }

    /// Move file tree selection upwards
    pub fn file_tree_select_down(&mut self) {
        if let Some(ref mut fts) = self.render_cache.file_tree_selection {
            let flat = self
                .file_tree
                .as_ref()
                .map(FileTree::flatten)
                .unwrap_or_default();
            if *fts + 1 < flat.len() {
                // Move up a file (in the render cache)
                *fts += 1;
                // Move up a file (in the backend)
                let new_path = flat.get(*fts);
                self.file_tree_selection = new_path.cloned();
            }
        }
    }

    /// Open a certain file / directory in a file tree
    pub fn file_tree_open_node(&mut self) -> Result<()> {
        if let Some(file_name) = &self.file_tree_selection.clone() {
            match file_or_dir(file_name) {
                "file" => self.file_tree_open_file()?,
                "directory" => self.file_tree_toggle_dir(),
                _ => (),
            }
        }
        Ok(())
    }

    /// Open a file from the file tree
    pub fn file_tree_open_file(&mut self) -> Result<()> {
        // Work out how to behave when opening files
        let move_focus = config!(self.config, file_tree).move_focus_to_file;
        if let Some(file_name) = &self.file_tree_selection.clone() {
            // Restore to old pointer to open
            let ptr_cache = self.ptr.clone();
            let mut temp = self.old_ptr.clone();
            temp.insert(0, 1);
            self.ptr = temp;
            // Perform open operation
            self.open(file_name)?;
            self.next();
            self.update_cwd();
            // If we don't want to move focus, then move focus back to the file tree
            if !move_focus {
                self.ptr = ptr_cache;
            }
        }
        Ok(())
    }

    pub fn file_tree_toggle_dir(&mut self) {
        if let Some(ref mut file_tree) = &mut self.file_tree {
            if let Some(file_name) = self.file_tree_selection.as_ref() {
                if let Some(node) = file_tree.get_mut(file_name) {
                    if let FileTree::Dir { files, .. } = node {
                        if files.is_some() {
                            // Clear expansion if already expanded
                            *files = None;
                        } else {
                            // Expand if not already expanded
                            node.expand();
                        }
                    }
                }
            }
        }
    }
}
