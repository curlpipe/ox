use crate::document::Cursor;
use crate::event::{Error, Result, UndoMgmt};
use crate::map::{form_map, CharMap};
use crate::utils::get_absolute_path;
use crate::{Document, Loc, Size};
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter};

/// A document info struct to store information about the file it represents
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DocumentInfo {
    /// Whether or not the document can be edited
    pub read_only: bool,
    /// Flag for an EOL
    pub eol: bool,
    /// true if the file has been modified since saving, false otherwise
    pub modified: bool,
    /// Contains the number of lines buffered into the document
    pub loaded_to: usize,
}

impl Document {
    /// Creates a new, empty document with no file name.
    #[cfg(not(tarpaulin_include))]
    #[must_use]
    pub fn new(size: Size) -> Self {
        let mut this = Self {
            file: Rope::from_str("\n"),
            lines: vec![String::new()],
            dbl_map: CharMap::default(),
            tab_map: CharMap::default(),
            file_name: None,
            cursor: Cursor::default(),
            offset: Loc::default(),
            size,
            char_ptr: 0,
            undo_mgmt: UndoMgmt::default(),
            tab_width: 4,
            old_cursor: 0,
            in_redo: false,
            info: DocumentInfo {
                loaded_to: 1,
                eol: false,
                read_only: false,
                modified: false,
            },
        };
        this.undo_mgmt.undo.push(this.take_snapshot());
        this.undo_mgmt.saved();
        this
    }

    /// Open a document from a file name.
    /// # Errors
    /// Returns an error when file doesn't exist, or has incorrect permissions.
    /// Also returns an error if the rope fails to initialise due to character set issues or
    /// disk errors.
    #[cfg(not(tarpaulin_include))]
    pub fn open<S: Into<String>>(size: Size, file_name: S) -> Result<Self> {
        let file_name = file_name.into();
        let file = Rope::from_reader(BufReader::new(File::open(&file_name)?))?;
        let file_name = get_absolute_path(&file_name);
        let mut this = Self {
            info: DocumentInfo {
                loaded_to: 0,
                eol: !file
                    .line(file.len_lines().saturating_sub(1))
                    .to_string()
                    .is_empty(),
                read_only: false,
                modified: false,
            },
            file,
            lines: vec![],
            dbl_map: CharMap::default(),
            tab_map: CharMap::default(),
            file_name,
            cursor: Cursor::default(),
            offset: Loc::default(),
            size,
            char_ptr: 0,
            undo_mgmt: UndoMgmt::default(),
            tab_width: 4,
            old_cursor: 0,
            in_redo: false,
        };
        this.undo_mgmt.undo.push(this.take_snapshot());
        this.undo_mgmt.saved();
        Ok(this)
    }

    /// Save back to the file the document was opened from.
    /// # Errors
    /// Returns an error if the file fails to write, due to permissions
    /// or character set issues.
    pub fn save(&mut self) -> Result<()> {
        if self.info.read_only {
            Err(Error::ReadOnlyFile)
        } else if let Some(file_name) = &self.file_name {
            self.file
                .write_to(BufWriter::new(File::create(file_name)?))?;
            self.undo_mgmt.saved();
            self.info.modified = false;
            Ok(())
        } else {
            Err(Error::NoFileName)
        }
    }

    /// Save to a specified file.
    /// # Errors
    /// Returns an error if the file fails to write, due to permissions
    /// or character set issues.
    pub fn save_as(&self, file_name: &str) -> Result<()> {
        if self.info.read_only {
            Err(Error::ReadOnlyFile)
        } else {
            self.file
                .write_to(BufWriter::new(File::create(file_name)?))?;
            Ok(())
        }
    }

    /// Load lines in this document up to a specified index.
    /// This must be called before starting to edit the document as
    /// this is the function that actually load and processes the text.
    pub fn load_to(&mut self, mut to: usize) {
        // Make sure to doesn't go over the number of lines in the buffer
        let len_lines = self.file.len_lines();
        if to >= len_lines {
            to = len_lines;
        }
        // Only act if there are lines we haven't loaded yet
        if to > self.info.loaded_to {
            // For each line, run through each character and make note of any double width characters
            for i in self.info.loaded_to..to {
                let line: String = self.file.line(i).chars().collect();
                // Add to char maps
                let (dbl_map, tab_map) = form_map(&line, self.tab_width);
                self.dbl_map.insert(i, dbl_map);
                self.tab_map.insert(i, tab_map);
                // Cache this line
                self.lines
                    .push(line.trim_end_matches(['\n', '\r']).to_string());
            }
            // Store new loaded point
            self.info.loaded_to = to;
        }
    }
}
