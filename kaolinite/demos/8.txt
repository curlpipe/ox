//! document: Tools for opening and saving files
//!
//! Here is where you'll find the most important struct: [Document]
//! Please see the documentation over at [kaolinite](crate) for more information
//!
//! This module also contains the [`FileInfo`] struct, which contains information
//! about the opened file, which holds things like the file name, file ending and tab width.
//!
//! See the structs section below to find out more about each struct

use crate::event::EditStack;
use crate::event::{Error, Event, Result, Status};
use crate::row::Row;
use crate::utils::{filetype, width_char, Loc, Size};
use crate::{regex, st};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A struct that stores information about a file
#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    /// The file name of the document
    pub file: Option<String>,
    /// True if \r\n is used, false if \n is used
    pub is_dos: bool,
    /// Tab width of the file in spaces (default is 4, you can overwrite if need be)
    /// There is a slight quirk, however. You must edit this field *directly after*
    /// defining a Document, otherwise, the configuration may not apply.
    pub tab_width: usize,
}

impl Default for FileInfo {
    /// Create a `FileInfo` struct with default data
    fn default() -> Self {
        Self {
            file: None,
            is_dos: false,
            tab_width: 4,
        }
    }
}

/// A struct that contains all the basic tools necessary to manage documents
#[derive(Debug, Default)]
pub struct Document {
    /// The information for the current file
    pub info: FileInfo,
    /// All the rows within the document
    pub rows: Vec<Row>,
    /// Boolean that changes when the file is edited via the event executor
    pub modified: bool,
    /// The size holds how much space the document has to render
    pub size: Size,
    /// A pointer to the character at the current cursor position
    pub char_ptr: usize,
    /// The position within the terminal
    pub cursor: Loc,
    /// Stores information about scrolling
    pub offset: Loc,
    /// Render cache space for optimisation purposes
    pub render: String,
    /// Toggle that determines if the document requires rerendering
    pub needs_rerender: bool,
    /// An undo / redo stack
    pub event_stack: EditStack,
}

impl Document {
    /// Create a new document
    ///
    /// The argument `size` takes in a [Size](crate::utils::Size) struct. This should
    /// store information about the terminal size.
    ///e
    /// If you plan to implement things
    /// like status lines or tabs, you should subtract them from the size height, as this
    /// size is purely for the file viewport size.
    #[cfg(not(tarpaulin_include))]
    pub fn new<S: Into<Size>>(size: S) -> Self {
        Self {
            info: FileInfo::default(),
            rows: vec![],
            modified: false,
            cursor: Loc::default(),
            offset: Loc::default(),
            size: size.into(),
            char_ptr: 0,
            render: st!(""),
            needs_rerender: true,
            event_stack: EditStack::default(),
        }
    }

    /// Open a file at a specified path into this document.
    ///
    /// This will also reset the cursor position, offset position,
    /// file name, contents and line ending information
    /// # Errors
    /// Will return `Err` if `path` does not exist or the user does not have
    /// permission to read from it.
    #[cfg(not(tarpaulin_include))]
    pub fn open<P: Into<String>>(&mut self, path: P) -> Result<()> {
        // Read in information
        let path = path.into();
        let raw = fs::read_to_string(&path)?;
        // Reset to default values
        self.info = FileInfo {
            file: Some(path),
            is_dos: raw.contains("\\r\\n"),
            tab_width: self.info.tab_width,
        };
        self.cursor = Loc::default();
        self.offset = Loc::default();
        self.char_ptr = 0;
        self.modified = false;
        // Load in the rows
        self.rows = self.raw_to_rows(&raw);
        Ok(())
    }

    /// Save a file
    ///
    /// This will reset `modified` to `false`, as it has been saved back to it's original file.
    /// # Errors
    /// Will return `Err` if the file path the document came from wasn't able to be written
    /// to, potentially because of file permission errors.
    pub fn save(&mut self) -> Result<()> {
        let data = self.render();
        let file = self.info.file.as_ref().ok_or(Error::NoFileName)?;
        fs::write(file, data)?;
        self.modified = false;
        Ok(())
    }

    /// Save a file to a specified path
    ///
    /// Similar to [save](Document::save) but takes a file argument, and saves it there.
    /// This method also doesn't change `modified`.
    /// # Errors
    /// Will return `Err` if the provided file path wasn't able to be written to,
    /// potentially because fo file permission errors.
    pub fn save_as(&self, file: &str) -> Result<()> {
        let data = self.render();
        fs::write(file, data)?;
        Ok(())
    }

    /// Execute an event in this document
    ///
    /// This method is the main method that should be used to modify the document.
    /// It takes in an [Event](crate::event::Event) enum.
    ///
    /// This method also takes advantage of undo & redo functionality, efficient syntax
    /// highlighting and the document modificatior indicator and moves your cursor automatically.
    /// If you change the rows in the document directly, you will not gain access
    /// to these benefits, but you can always manually handle these features if need be.
    /// # Errors
    /// Will return `Err` if the event tried to modifiy data outside the scope of the
    /// document.
    pub fn execute(&mut self, event: Event) -> Result<Status> {
        let r = self.forth(event.clone());
        self.event_stack.exe(event);
        r
    }

    /// Execute an event, without the undo / redo tracking
    /// # Errors
    /// Will error if the location is out of range
    pub fn forth(&mut self, event: Event) -> Result<Status> {
        let tab_width = self.info.tab_width;
        match event {
            Event::Insert(loc, ch) => {
                self.goto(loc)?;
                self.row_mut(loc.y)?.insert(loc.x, ch, tab_width)?;
                self.modified = true;
                self.needs_rerender = true;
                self.move_right()
            }
            Event::Remove(mut loc, _) => {
                if loc.x == 0 {
                    return Ok(Status::StartOfRow);
                }
                loc.x -= 1;
                self.goto(loc)?;
                self.row_mut(loc.y)?.remove(loc.x..=loc.x)?;
                self.modified = true;
                self.needs_rerender = true;
                Ok(Status::None)
            }
            Event::InsertRow(loc, st) => {
                self.rows.insert(loc, Row::new(st, self.info.tab_width));
                self.modified = true;
                self.needs_rerender = true;
                self.goto_y(loc)?;
                Ok(Status::None)
            }
            Event::RemoveRow(loc, _) => {
                self.goto_y(loc)?;
                self.rows.remove(loc);
                self.modified = true;
                self.needs_rerender = true;
                Ok(Status::None)
            }
            Event::SpliceUp(loc) => {
                let mut upper = self.row(loc.y)?.clone();
                let lower = self.row(loc.y + 1)?.clone();
                self.rows[loc.y] = upper.splice(lower);
                self.modified = true;
                self.needs_rerender = true;
                self.rows.remove(loc.y + 1);
                self.goto(loc)?;
                Ok(Status::None)
            }
            Event::SplitDown(loc) => {
                let (left, right) = self.row(loc.y)?.split(loc.x)?;
                self.rows[loc.y] = left;
                self.modified = true;
                self.needs_rerender = true;
                self.rows.insert(loc.y + 1, right);
                self.goto((0, loc.y + 1))?;
                Ok(Status::None)
            }
        }
    }

    /// Take in an event and perform the opposite
    /// # Errors
    /// Will error if the location is out of range
    pub fn back(&mut self, event: Event) -> Result<Status> {
        let tab_width = self.info.tab_width;
        match event {
            Event::Insert(loc, _) => {
                self.goto(loc)?;
                let r = self.row_mut(loc.y)?.remove(loc.x..=loc.x);
                self.modified = true;
                self.needs_rerender = true;
                r
            }
            Event::Remove(loc, ch) => {
                let r = self.row_mut(loc.y)?.insert(loc.x - 1, ch, tab_width);
                self.modified = true;
                self.needs_rerender = true;
                self.goto(loc)?;
                r
            }
            Event::InsertRow(loc, _) => {
                self.rows.remove(loc);
                self.goto_y(loc - 1)?;
                self.modified = true;
                self.needs_rerender = true;
                Ok(Status::None)
            }
            Event::RemoveRow(loc, st) => {
                self.goto_y(loc)?;
                self.rows.insert(loc, Row::new(st, tab_width));
                self.modified = true;
                self.needs_rerender = true;
                Ok(Status::None)
            }
            Event::SpliceUp(loc) => {
                let (left, right) = self.row(loc.y)?.split(loc.x)?;
                self.rows[loc.y] = left;
                self.modified = true;
                self.needs_rerender = true;
                self.rows.insert(loc.y + 1, right);
                self.goto((0, loc.y + 1))?;
                Ok(Status::None)
            }
            Event::SplitDown(loc) => {
                let mut upper = self.row(loc.y)?.clone();
                let lower = self.row(loc.y + 1)?.clone();
                self.rows[loc.y] = upper.splice(lower);
                self.modified = true;
                self.needs_rerender = true;
                self.rows.remove(loc.y + 1);
                self.goto(loc)?;
                Ok(Status::None)
            }
        }
    }

    /// Move the cursor to a specific x and y coordinate
    /// # Errors
    /// Will return `Err` if the location provided is out of scope of the document.
    pub fn goto<L: Into<Loc>>(&mut self, loc: L) -> Result<()> {
        let loc = loc.into();
        self.goto_y(loc.y)?;
        self.goto_x(loc.x)?;
        Ok(())
    }

    /// Move the cursor to a specific x coordinate
    /// # Errors
    /// Will return `Err` if the location provided is out of scope of the document.
    pub fn goto_x(&mut self, x: usize) -> Result<()> {
        // Bounds checking
        if self.char_ptr == x {
            return Ok(());
        } else if x > self.current_row()?.len() {
            return Err(Error::OutOfRange);
        }
        // Gather and update information
        let viewport = self.offset.x..self.offset.x + self.size.w;
        self.char_ptr = x;
        let x = *self
            .current_row()?
            .indices
            .get(x)
            .ok_or(Error::OutOfRange)?;
        // Start movement
        if x < self.size.w {
            // Cursor is in view when offset is 0
            self.offset.x = 0;
            self.cursor.x = x;
        } else if viewport.contains(&x) {
            // If the point is in viewport already, only move cursor
            self.cursor.x = x - self.offset.x;
        } else {
            // If the point is out of viewport, set cursor to 0, and adjust offset
            self.cursor.x = 0;
            self.offset.x = x;
        }
        Ok(())
    }

    /// Move the cursor to a specific y coordinate
    /// # Errors
    /// Will return `Err` if the location provided is out of scope of the document.
    pub fn goto_y(&mut self, y: usize) -> Result<()> {
        // Bounds checking
        if self.raw_loc().y == y {
            return Ok(());
        } else if y > self.rows.len() {
            return Err(Error::OutOfRange);
        }
        let viewport = self.offset.y..self.offset.y + self.size.h;
        if y < self.size.h {
            // Cursor is in view when offset is 0
            self.offset.y = 0;
            self.cursor.y = y;
        } else if viewport.contains(&y) {
            // If the point is in viewport already, only move cursor
            self.cursor.y = y - self.offset.y;
        } else {
            // If the point is out of viewport, move cursor to bottom, and adjust offset
            self.cursor.y = self.size.h - 1;
            self.offset.y = y - (self.size.h - 1);
        }
        // Snap to grapheme boundary
        self.snap_grapheme()?;
        // Correct char pointer
        self.char_ptr = self.current_row()?.get_char_ptr(self.raw_loc().x);
        Ok(())
    }

    /// Move the cursor to the left
    /// # Errors
    /// Will return `Err` if the cursor is out of scope of the document
    pub fn move_left(&mut self) -> Result<Status> {
        // Check to see if the cursor is already as far left as possible
        if self.char_ptr == 0 {
            return Ok(Status::StartOfRow);
        }
        // Traverse the grapheme
        for _ in 0..self.get_width(-1)? {
            // Determine whether to change offset or cursor
            if self.cursor.x == 0 {
                self.offset.x -= 1;
            } else {
                self.cursor.x -= 1;
            }
        }
        self.char_ptr -= 1;
        Ok(Status::None)
    }

    /// Move the cursor to the right
    /// # Errors
    /// Will return `Err` if the cursor is out of scope of the document
    pub fn move_right(&mut self) -> Result<Status> {
        // Check to see if the cursor is already as far right as possible
        if self.char_ptr == self.current_row()?.len() {
            return Ok(Status::EndOfRow);
        }
        // Traverse the grapheme
        for _ in 0..self.get_width(0)? {
            // Determine whether to change offset or cursor
            if self.cursor.x == self.size.w - 1 {
                self.offset.x += 1;
            } else {
                self.cursor.x += 1;
            }
        }
        self.char_ptr += 1;
        Ok(Status::None)
    }

    /// Move the cursor upwards
    /// # Errors
    /// Will return `Err` if the cursor is out of scope of the document
    pub fn move_up(&mut self) -> Result<Status> {
        // Check to see if the cursor is already as far up as possible
        if self.raw_loc().y == 0 {
            return Ok(Status::StartOfDocument);
        }
        // Determine whether to change offset or cursor
        if self.cursor.y == 0 {
            self.offset.y -= 1;
        } else {
            self.cursor.y -= 1;
        }
        // Snap to grapheme boundary
        self.snap_grapheme()?;
        // Correct char pointer
        self.char_ptr = self.current_row()?.get_char_ptr(self.raw_loc().x);
        Ok(Status::None)
    }

    /// Move the cursor downwards
    /// # Errors
    /// Will return `Err` if the cursor is out of scope of the document
    pub fn move_down(&mut self) -> Result<Status> {
        // Check to see if the cursor is already as far up as possible
        if self.raw_loc().y == self.rows.len() {
            return Ok(Status::EndOfDocument);
        }
        // Determine whether to change offset or cursor
        if self.cursor.y == self.size.h - 1 {
            self.offset.y += 1;
        } else {
            self.cursor.y += 1;
        }
        // Snap to grapheme boundary
        std::mem::drop(self.snap_grapheme());
        // Correct char pointer
        self.char_ptr = if let Ok(row) = self.current_row() {
            row.get_char_ptr(self.raw_loc().x)
        } else {
            // Move to 0 when entering row below document
            self.cursor.x = 0;
            self.offset.x = 0;
            0
        };
        Ok(Status::None)
    }

    /// Work out the line number text to use
    #[must_use]
    pub fn line_number(&self, row: usize) -> String {
        let total = self.rows.len().to_string().len();
        let num = (row + 1).to_string();
        format!("{}{}", " ".repeat(total - num.len()), num)
    }

    /// A helper function that returns info about the document
    /// in a [`HashMap`] type.
    ///
    /// It will return (with keys for the hashmap):
    /// - Row number: `row`
    /// - Column number: `column`
    /// - Total rows: `total`
    /// - File name (no path): `file`
    /// - File name (full path): `full_file`
    /// - File type: `type`
    /// - Modifed indicator: `modified`
    /// - File extension: `extension`
    #[must_use]
    pub fn status_line_info(&self) -> HashMap<&str, String> {
        let row = self.loc().y + 1;
        let total = self.rows.len();
        let column = self.loc().x;
        let modified = if self.modified { "[+]" } else { "" };
        let (full_file, file, ext) = if let Some(name) = &self.info.file {
            let f = Path::new(&name)
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or(name)
                .to_string();
            let e = Path::new(&name)
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("")
                .to_string();
            (name.clone(), f, e)
        } else {
            (st!("[No Name]"), st!("[No Name]"), st!(""))
        };
        let mut info = HashMap::new();
        info.insert("row", st!(row));
        info.insert("column", st!(column));
        info.insert("total", st!(total));
        info.insert("file", st!(file));
        info.insert("full_file", st!(full_file));
        info.insert("type", filetype(&ext).unwrap_or_else(|| st!("Unknown")));
        info.insert("modified", st!(modified));
        info.insert("extension", ext);
        info
    }

    /// Render the document into the correct form
    #[must_use]
    pub fn render(&self) -> String {
        let line_ending = if self.info.is_dos { "\r\n" } else { "\n" };
        self.rows
            .iter()
            .map(Row::render_raw)
            .collect::<Vec<_>>()
            .join(line_ending)
            + line_ending
    }

    /// Render the document into the correct form
    #[must_use]
    pub fn render_full(&self) -> String {
        let line_ending = if self.info.is_dos { "\r\n" } else { "\n" };
        let mut result = st!("");
        for r in &self.rows {
            result.push_str(&r.render_full(self.info.tab_width));
            result.push_str(line_ending);
        }
        result
    }

    /// Shift the cursor back to the nearest grapheme boundary
    fn snap_grapheme(&mut self) -> Result<()> {
        // Collect information
        let row = self.current_row()?;
        let start = self.raw_loc().x;
        let mut ptr = self.raw_loc().x;
        // Shift back until on boundary
        while !row.indices.contains(&ptr) {
            ptr -= 1;
        }
        // Work out required adjustment
        let adjustment = start - ptr;
        // Perform adjustment
        for _ in 0..adjustment {
            if self.cursor.x == 0 {
                self.offset.x -= 1;
            } else {
                self.cursor.x -= 1;
            }
        }
        Ok(())
    }

    /// Take raw text and convert it into Row structs
    #[must_use]
    pub fn raw_to_rows(&self, text: &str) -> Vec<Row> {
        let text = regex!("(\\r\\n|\\n)$").replace(text, "").to_string();
        let rows: Vec<&str> = regex!("(\\r\\n|\\n)").split(&text).collect();
        rows.iter()
            .map(|s| Row::new(*s, self.info.tab_width))
            .collect()
    }

    /// Return a reference to a row in the document
    /// # Errors
    /// This will error if the index is out of range
    pub fn row(&self, index: usize) -> Result<&Row> {
        self.rows.get(index).ok_or(Error::OutOfRange)
    }

    /// Return a mutable reference to a row in the document
    /// # Errors
    /// This will error if the index is out of range
    pub fn row_mut(&mut self, index: usize) -> Result<&mut Row> {
        self.rows.get_mut(index).ok_or(Error::OutOfRange)
    }

    /// Get the current row
    /// # Errors
    /// This will error if the cursor position isn't on a existing row
    pub fn current_row(&self) -> Result<&Row> {
        self.row(self.raw_loc().y)
    }

    /// Get the width of a character
    fn get_width(&self, offset: i128) -> Result<usize> {
        // TODO: Optimise using arithmetic rather than width calculation
        let idx = (self.char_ptr as i128 + offset) as usize;
        let ch = self.current_row()?.text[idx];
        Ok(width_char(ch, self.info.tab_width))
    }

    /// Get the current position in the document
    ///
    /// This ought to be used by the document only as it returns the display indices
    /// Use the `Document::loc` function instead.
    #[must_use]
    pub const fn raw_loc(&self) -> Loc {
        Loc {
            x: self.cursor.x + self.offset.x,
            y: self.cursor.y + self.offset.y,
        }
    }

    /// Get the current position in the document
    ///
    /// This will return the character and row indices
    #[must_use]
    pub const fn loc(&self) -> Loc {
        Loc {
            x: self.char_ptr,
            y: self.cursor.y + self.offset.y,
        }
    }
}

//! event: Enums that represent the status of the editor and events
//!
//! This contains the Error types, as well as the possible events you can use

use crate::utils::Loc;

/// Neater error type
pub type Result<T> = std::result::Result<T, Error>;

/// Event represents all the document events that could occur
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Insert a character at a position.
    /// Takes a location and a character to insert
    Insert(Loc, char),
    /// Remove a character at a position.
    /// Takes a location and the character that has been removed.
    Remove(Loc, char),
    /// Insert a row.
    /// Takes a row index and a string for the row.
    InsertRow(usize, String),
    /// Remove a row.
    /// Takes a row index and a string for the row.
    RemoveRow(usize, String),
    /// Cut a line in half and drop the last half down a line.
    /// This is for times when the enter key is pressed in the middle of a line.
    SplitDown(Loc),
    /// Splice a line with the line above.
    /// This is for times when the backspace key is pressed at the start of a line.
    SpliceUp(Loc),
}

/// Status contains the states the document can be in after an event execution
#[derive(Debug, PartialEq)]
pub enum Status {
    /// Cursor reaches the end of a row.
    /// Useful for if you want to wrap the cursor around when it hits the end of the row.
    EndOfRow,
    /// Cursor reaches the start of a row.
    /// Useful for if you want to wrap the cursor around when it hits the start of the row.
    StartOfRow,
    /// Cursor reaches the start of the document.
    EndOfDocument,
    /// Cursor reaches the start of the document.
    StartOfDocument,
    /// Nothing of note.
    None,
}

/// Error represents the potential failures in function calls when using this API
#[derive(Debug)]
pub enum Error {
    /// Returned when you provide an index that is out of range
    OutOfRange,
    /// When the program is unable to open a file e.g. doesn't exist or file permissions
    FileError(std::io::Error),
    /// Saving an unnamed file
    NoFileName,
}

impl std::fmt::Display for Error {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    #[cfg(not(tarpaulin_include))]
    fn from(e: std::io::Error) -> Error {
        Error::FileError(e)
    }
}

/// Event stack is a struct that handles events
#[derive(Debug, Default)]
pub struct EditStack {
    /// Where the current smaller editing events are stored
    pub patch: Vec<Event>,
    /// This is where events that have been done are
    pub done: Vec<Vec<Event>>,
    /// This is where events that have been undone are
    pub undone: Vec<Vec<Event>>,
}

impl EditStack {
    /// Adds an event to the current patch
    pub fn exe(&mut self, event: Event) {
        self.undone.clear();
        self.patch.push(event);
    }

    /// Commit the patch to the done stack
    pub fn commit(&mut self) {
        if !self.patch.is_empty() {
            let patch = std::mem::take(&mut self.patch);
            self.done.push(patch);
        }
    }

    /// Returns the last performed event and moves it around
    pub fn undo(&mut self) -> Option<&Vec<Event>> {
        self.commit();
        let mut done = self.done.pop()?;
        done.reverse();
        self.undone.push(done);
        self.undone.last()
    }

    /// Returns the last undone event and moves it around
    pub fn redo(&mut self) -> Option<&Vec<Event>> {
        let mut undone = self.undone.pop()?;
        undone.reverse();
        self.done.push(undone);
        self.done.last()
    }
}

//! Welcome to the documentatoin for Kaolinite
//! ## What is Kaolinite?
//!
//! At first, it seems like buliding a text editor is easy,
//! some pepole have made ones in fewer than 1000 lines!
//! But when you try opening files with unicode or large files
//! or implement your own configuration system that allows
//! the user to create custom themes and add their own syntax highlighting
//! it becomes very disorientating very quickly, and when using crates
//! like `syntect` you start seeing the crates your editor depends on
//! stack up and it compiles slower and slower.
//!
//! Kaolinite is a library that has most of the features you'll need
//! in order to create a TUI text editor, like vim or nano. It's lightweight
//! and tries to implement text editing in the most efficient way possible.
//!
//! It doesn't force you to use any TUI library in the Rust ecosystem,
//! so you can choose how to implement your UI. Nor does it force you to use
//! any style of editor, your editor could be modal if you wanted it to be.
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml` file:
//! ```toml
//! [dependencies]
//! kaolinite = "0"
//! ```
//!
//! Or you can use `cargo-edit`:
//!
//! ```sh
//! $ cargo add kaolinite
//! ```
//!
//! The main struct that you'll want to use is [Document](document::Document).
//! This struct handles the insertion and deletion of characters, splitting rows,
//! splicing rows, reading files, saving files, cursor position and scrolling,
//! searcing, syntax highlighting, undo and redo, and unicode grapheme handling.
//!
//! There is also a [Row](row::Row) struct that provides more row-specific
//! operations and information such as finding word boundaries, rendering
//! themselves in certain ways, and determining if the row has been modified.
//! You won't really need to use many of the methods here, as
//! [Document](document::Document) handles most of the row operations you'd need.
//!
//! Here are a few examples of how it would look:
//!
//! ```
//! // Opening a file,
//! use kaolinite::document::Document;
//! let mut doc = Document::new((10, 10));
//! // Imagine if test.txt were `The quick brown fox`
//! doc.open("examples/test.txt").expect("Failed to open file");
//! // This would get the word boundaries of the first row: [0, 4, 10, 16, 19]
//! println!("{:?}", doc.row(0).unwrap().words());
//! ```
//!
//! Because this library is quite a large collection of tools, it's hard to demonstrate it
//! here. You can find a examples directory on github
//! with many different examples, including a barebones text editor using the
//! [crossterm](https://docs.rs/crossterm) library, in 400 SLOC (excluding comments and blank lines),
//! with syntax highlighting, undo & redo, and full unicode support.
//! You can use that as a starting point, if you wish.

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_sign_loss)]

pub mod document;
pub mod event;
pub mod row;
pub mod utils;

//! row: Tools for inserting and removing characters
//!
//! This contains the [Row] struct. Occasionally, you might
//! require some row-specific information such as how it looks when rendered,
//! or where the word boundaries in it are.

use crate::event::{Error, Result, Status};
use crate::st;
use crate::utils::{raw_indices, width, width_char, BoundedRange};
use synoptic::Token;

/// A struct that contains all the basic tools necessary to manage rows in a document
#[derive(Debug, PartialEq, Clone)]
pub struct Row {
    /// All the characters within the row
    pub text: Vec<char>,
    /// Corresponding display widths for each character
    pub indices: Vec<usize>,
    /// A tool for determining if the row has been edited
    /// ```
    /// use kaolinite::row::Row;
    /// let tab_width = 4;
    /// let mut row = Row::new("Hello", tab_width);
    /// assert_eq!(row.modified, false);
    /// row.insert(5, ", world!", tab_width);
    /// assert_eq!(row.modified, true);
    /// row.modified = false;
    /// assert_eq!(row.modified, false);
    /// ```
    /// This is ideal for optimisation
    pub modified: bool,
    /// Tokens for this row
    pub tokens: Vec<Token>,
    pub needs_rerender: bool,
}

impl Row {
    /// Create a new row from raw text
    #[cfg(not(tarpaulin_include))]
    pub fn new<S: Into<String>>(raw: S, tab_width: usize) -> Self {
        let raw = raw.into();
        let text: Vec<char> = raw.chars().collect();
        Self {
            indices: Row::raw_to_indices(&text, tab_width),
            text,
            modified: false,
            tokens: vec![],
            needs_rerender: true,
        }
    }

    /// Insert text at a position
    /// # Errors
    /// Will return `Err` if `start` is out of range of the row
    pub fn insert<S: Into<String>>(
        &mut self,
        start: usize,
        text: S,
        tabs: usize,
    ) -> Result<Status> {
        if start > self.width() {
            return Err(Error::OutOfRange);
        }
        let text = text.into();
        self.text.splice(start..start, text.chars());
        self.indices = Row::raw_to_indices(&self.text, tabs);
        self.modified = true;
        self.needs_rerender = true;
        Ok(Status::None)
    }

    /// Remove text in a range
    ///
    /// This takes in an inclusive or exclusive range: `..` and `..=` only.
    /// # Errors
    /// Will return `Err` if `range` is out of range of the row
    #[allow(clippy::needless_pass_by_value)]
    pub fn remove<R>(&mut self, range: R) -> Result<Status>
    where
        R: BoundedRange,
    {
        let (start, end) = (range.first(), range.last());
        if start > self.width() {
            return Err(Error::OutOfRange);
        }
        let shift = self.indices[end] - self.indices[start];
        self.text.splice(start..end, []);
        self.indices.splice(start..end, []);
        self.indices
            .iter_mut()
            .skip(start)
            .for_each(|i| *i -= shift);
        self.modified = true;
        self.needs_rerender = true;
        Ok(Status::None)
    }

    /// Splits this row into two separate rows
    /// # Errors
    /// Will return `Err` if `idx` is out of range of the row
    pub fn split(&self, idx: usize) -> Result<(Row, Row)> {
        let left = Row {
            text: self.text.get(..idx).ok_or(Error::OutOfRange)?.to_vec(),
            indices: self.indices.get(..=idx).ok_or(Error::OutOfRange)?.to_vec(),
            modified: true,
            tokens: vec![],
            needs_rerender: true,
        };
        let mut right = Row {
            text: self.text.get(idx..).ok_or(Error::OutOfRange)?.to_vec(),
            indices: self.indices.get(idx..).ok_or(Error::OutOfRange)?.to_vec(),
            modified: false,
            tokens: vec![],
            needs_rerender: true,
        };
        // Shift down
        let shift = *right.indices.first().unwrap_or(&0);
        right.indices.iter_mut().for_each(|i| *i -= shift);
        Ok((left, right))
    }

    /// Joins this row with another row
    pub fn splice(&mut self, mut row: Row) -> Row {
        let mut indices = self.indices.clone();
        let shift = *self.indices.last().unwrap_or(&0);
        row.indices.remove(0);
        row.indices.iter_mut().for_each(|i| *i += shift);
        indices.append(&mut row.indices);
        let mut text = self.text.clone();
        text.append(&mut row.text);
        Row {
            indices,
            text,
            modified: true,
            tokens: vec![],
            needs_rerender: true,
        }
    }

    /// Retrieve the indices of word boundaries
    /// ```
    /// // Opening a file,
    /// use kaolinite::document::Document;
    /// let mut doc = Document::new((10, 10));
    /// // Imagine if test.txt were `The quick brown fox`
    /// doc.open("examples/test.txt").expect("Failed to open file");
    /// // This would get the word boundaries of the first row: [0, 4, 10, 16, 19]
    /// println!("{:?}", doc.row(0).unwrap().words());
    /// ```
    #[must_use]
    pub fn words(&self) -> Vec<usize> {
        crate::utils::words(self)
    }

    /// Find the next word in this row from the character index
    #[must_use]
    pub fn next_word_forth(&self, loc: usize) -> usize {
        let bounds = self.words();
        let mut last = *bounds.last().unwrap_or(&0);
        for bound in bounds.iter().rev() {
            if bound <= &loc {
                return last;
            }
            last = *bound;
        }
        *bounds.first().unwrap_or(&0)
    }

    /// Find the previous word in this row from the character index
    #[must_use]
    pub fn next_word_back(&self, loc: usize) -> usize {
        let bounds = self.words();
        let mut last = 0;
        for bound in &bounds {
            if bound >= &loc {
                return last;
            }
            last = *bound;
        }
        *bounds.last().unwrap_or(&0)
    }

    /// Render part of the row
    /// When trying to render X axis offset, this is the ideal function to use
    /// ```ignore
    /// "He好llo好" // 0..
    /// "e好llo好"  // 1..
    /// "好llo好"   // 2..
    /// " llo好"    // 3..
    /// "llo好"     // 4..
    /// "lo好"      // 5..
    /// "o好"       // 6..
    /// "好"        // 7..
    /// " "         // 8..
    /// ""          // 9..
    /// ```
    /// This also handles double width characters by inserting whitespace when half
    /// of the character is off the screen
    #[must_use]
    pub fn render(&self, range: std::ops::RangeFrom<usize>, tabs: usize) -> String {
        let mut start = range.start;
        // Render the row
        let text = self.render_raw();
        // Return an empty string if start is out of range
        if start >= width(&text, tabs) {
            return st!("");
        }
        // Obtain the character indices
        let ind = raw_indices(&text, &self.indices, tabs);
        // Shift the cut point forward until on a character boundary
        let space = !ind.contains_key(&start);
        while !ind.contains_key(&start) {
            start += 1;
        }
        // Perform cut and format
        let text = text.replace("\t", &" ".repeat(tabs));
        format!("{}{}", if space { " " } else { "" }, &text[ind[&start]..])
    }

    /// Render the entire row, with tabs converted into spaces
    #[must_use]
    pub fn render_full(&self, tabs: usize) -> String {
        // Retrieve tab width
        self.text.iter().fold(st!(""), |a, x| {
            format!(
                "{}{}",
                a,
                if x == &'\t' {
                    " ".repeat(tabs)
                } else {
                    x.to_string()
                }
            )
        })
    }

    /// Render this row as is, with no tab interference
    #[must_use]
    pub fn render_raw(&self) -> String {
        self.text.iter().fold(st!(""), |a, x| format!("{}{}", a, x))
    }

    /// Find the character length of this row
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Determine if the row is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Find the display width of this row
    #[must_use]
    pub fn width(&self) -> usize {
        *self.indices.last().unwrap_or(&0)
    }

    /// Calculate the character pointer from a display index
    #[must_use]
    pub fn get_char_ptr(&self, x: usize) -> usize {
        // Handle large values of x
        if x >= self.width() {
            return self.len();
        }
        // Calculate the character width
        self.indices.iter().position(|i| &x == i).unwrap_or(0)
    }

    /// Find the widths of the characters in raw text
    fn raw_to_indices(text: &[char], tab_width: usize) -> Vec<usize> {
        let mut data = vec![&'\x00'];
        data.splice(1.., text);
        data.iter()
            .map(|c| width_char(**c, tab_width))
            .scan(0, |a, x| {
                *a += x;
                Some(*a)
            })
            .collect()
    }
}

use crate::row::Row;
use std::collections::HashMap;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Whitespace character array
const WHITESPACE: [char; 2] = [' ', '\t'];

/// String helper macro
#[macro_export]
macro_rules! st {
    ($value:expr) => {
        $value.to_string()
    };
}

/// Lazy regex creation
#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

/// A struct that holds positions
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Loc {
    pub x: usize,
    pub y: usize,
}

impl From<(usize, usize)> for Loc {
    fn from(loc: (usize, usize)) -> Loc {
        let (x, y) = loc;
        Loc { x, y }
    }
}

/// A struct that holds size
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub w: usize,
    pub h: usize,
}

impl From<(usize, usize)> for Size {
    fn from(size: (usize, usize)) -> Size {
        let (w, h) = size;
        Size { w, h }
    }
}

pub trait BoundedRange {
    fn first(&self) -> usize;
    fn last(&self) -> usize;
}

impl BoundedRange for std::ops::Range<usize> {
    fn first(&self) -> usize {
        self.start
    }

    fn last(&self) -> usize {
        self.end
    }
}

impl BoundedRange for std::ops::RangeInclusive<usize> {
    fn first(&self) -> usize {
        *self.start()
    }

    fn last(&self) -> usize {
        *self.end() + 1
    }
}

/// Generate a look up table between the raw and display indices
#[must_use]
pub fn raw_indices(s: &str, i: &[usize], tab_width: usize) -> HashMap<usize, usize> {
    let mut raw = 0;
    let mut indices = HashMap::new();
    indices.insert(0, 0);
    for (c, ch) in s.chars().enumerate() {
        if ch == '\t' {
            for i in 1..=tab_width {
                indices.insert(c + i, raw + i);
            }
            raw += 4;
        } else {
            raw += ch.len_utf8();
            indices.insert(i[c + 1], raw);
        }
    }
    indices
}

/// Retrieve the indices of word boundaries
#[must_use]
pub fn words(row: &Row) -> Vec<usize> {
    // Gather information and set up algorithm
    let mut result = vec![];
    let mut chr = 0;
    let mut pad = true;
    // While still inside the row
    while chr < row.text.len() {
        let c = row.text[chr];
        match c {
            // Move forward through all the spaces
            ' ' => (),
            '\t' => {
                // If we haven't encountered text yet
                if pad {
                    // Make this a word boundary
                    result.push(chr);
                }
            }
            _ => {
                // Set the marker to false, as we're encountering text
                pad = false;
                // Set this as a word boundary
                result.push(chr);
                // Skip through text, end when we find whitespace or the end of the row
                while chr < row.text.len() && !WHITESPACE.contains(&row.text[chr]) {
                    chr += 1;
                }
                // Deal with next lot of whitespace or exit if at the end of the row
                continue;
            }
        }
        // Advance and continue
        chr += 1;
    }
    // Add on the last point on the row as a word boundary
    result.push(row.len());
    result
}

/// Determine the display width of a string
#[must_use]
pub fn width(s: &str, tab: usize) -> usize {
    let s = s.replace('\t', &" ".repeat(tab));
    s.width()
}

/// Determine the display width of a character
#[must_use]
pub fn width_char(c: char, tab: usize) -> usize {
    if c == '\t' {
        tab
    } else {
        c.width().unwrap_or(0)
    }
}

/// This will take text, and align it to the middle of the screen
#[must_use]
pub fn align_middle(s: &str, space: usize, tab_width: usize) -> Option<String> {
    let len = width(s, tab_width) / 2;
    let half = space / 2;
    let pad = " ".repeat(half.saturating_sub(len));
    if len * 2 + pad.len() > space {
        None
    } else {
        Some(format!("{}{}{}", pad, s, pad))
    }
}

/// This will take text, and align it to the left and right hand sides
#[must_use]
pub fn align_sides(lhs: &str, rhs: &str, space: usize, tab_width: usize) -> Option<String> {
    let total = width(lhs, tab_width) + width(rhs, tab_width);
    if total > space {
        None
    } else {
        Some(format!(
            "{}{}{}",
            lhs,
            " ".repeat(space.saturating_sub(total)),
            rhs
        ))
    }
}

/// Determine the filetype from the extension
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn filetype(extension: &str) -> Option<String> {
    Some(st!(match extension.to_ascii_lowercase().as_str() {
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
        "glsl" | "vert" | "shader" | "geo" | "fshader" | "vrx" | "vsh" | "vshader" | "frag" =>
            "GLSL",
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
    }))
}

use crate::tokens::{Bounded, FullToken, TokOpt, Token};
use crate::{gidx, glen};
use regex::{Error as ReError, Regex};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Write;

/// For performing highlighting operations
/// You can create a new Highlighter instance using the `new` method
/// ```rust
/// let mut h = Highlighter::new();
/// ```
#[derive(Debug, Clone)]
pub struct Highlighter {
    pub regex: HashMap<String, Vec<Regex>>,
    pub multiline_regex: HashMap<String, Vec<Regex>>,
    pub bounded: Vec<Bounded>,
}

impl Highlighter {
    /// This will create a new, blank highlighter instance
    #[must_use]
    pub fn new() -> Self {
        // Create a new highlighter
        Self {
            regex: HashMap::new(),
            multiline_regex: HashMap::new(),
            bounded: Vec::new(),
        }
    }

    /// This method allows you to add multiple definitions to the highlighter
    /// The first argument is for your list of definitions and the second is for the name
    /// This is useful for adding lists of keywords, for example:
    /// ```rust
    /// let mut python = Highlighter::new();
    /// python.join(&["def", "return", "import"], "keyword");
    /// ```
    /// For multiline tokens, you can add (?ms) or (?sm) to the beginning
    ///
    /// # Errors
    /// This will return an error if one or more of your regex expressions are invalid
    pub fn join(&mut self, regex: &[&str], token: &str) -> Result<(), ReError> {
        // Add a regex that will match on a single line
        for i in regex {
            self.add(i, token)?;
        }
        Ok(())
    }

    /// This method allows you to add a single definition to the highlighter
    /// The first argument is for your definition and the second is for the name
    /// This is useful for adding things like regular expressions, for example:
    /// ```rust
    /// let mut python = Highlighter::new();
    /// python.add("[0-9]+", "number");
    /// ```
    /// For multiline tokens, you can add (?ms) or (?sm) to the beginning.
    /// (See the `add_bounded` method for a better way of doing multiline tokens
    /// if you plan on doing file buffering.)
    ///
    /// # Errors
    /// This will return an error if your regex is invalid
    pub fn add(&mut self, regex: &str, token: &str) -> Result<(), ReError> {
        // Add a regex that will match on a single line
        let re = Regex::new(regex)?;
        if regex.starts_with("(?ms)") || regex.starts_with("(?sm)") {
            insert_regex(&mut self.multiline_regex, re, token);
        } else {
            insert_regex(&mut self.regex, re, token);
        }
        Ok(())
    }

    /// This method allows you to add a special, non-regex definition to the highlighter
    /// This not only makes it clearer to use for multiline tokens, but it will also allow you
    /// to buffer files from memory, and still be able to highlight multiline tokens, without
    /// having to have the end part visible in order to create a token.
    /// The first argument is for the text that starts the token
    /// The second argument is for the text that ends the token
    /// The third argument is true if you want to allow for escaping of the end token, false if
    /// not (for example, you might want to allow string escaping in strings).
    /// The forth argument is for the token name.
    /// ```rust
    /// let mut rust = Highlighter::new();
    /// rust.add_bounded("/*", "*/", false, "comment");
    /// ```
    /// You can still use regex to create a multiline token, but doing that won't guarantee that
    /// your highlighting will survive file buffering.
    pub fn add_bounded(&mut self, start: &str, end: &str, escaping: bool, token: &str) {
        let bounded = Bounded {
            kind: token.to_string(),
            start: start.to_string(),
            end: end.to_string(),
            escaping,
        };
        // Insert it into the bounded hashmap
        self.bounded.push(bounded);
    }

    /// A utility function to scan for just single line tokens
    fn run_singleline(&self, context: &str, result: &mut HashMap<usize, Vec<FullToken>>) {
        for (name, expressions) in &self.regex {
            for expr in expressions {
                let captures = expr.captures_iter(context);
                for captures in captures {
                    if let Some(m) = captures.get(captures.len().saturating_sub(1)) {
                        insert_token(
                            result,
                            m.start(),
                            FullToken {
                                text: m.as_str().to_string(),
                                kind: name.clone(),
                                start: m.start(),
                                end: m.end(),
                                multi: false,
                            },
                        );
                    }
                }
            }
        }
    }

    /// A utility function to scan for just multi line tokens
    fn run_multiline(&self, context: &str, result: &mut HashMap<usize, Vec<FullToken>>) {
        for (name, expressions) in &self.multiline_regex {
            for expr in expressions {
                let captures = expr.captures_iter(context);
                for captures in captures {
                    if let Some(m) = captures.get(captures.len().saturating_sub(1)) {
                        insert_token(
                            result,
                            m.start(),
                            FullToken {
                                text: m.as_str().to_string(),
                                kind: name.to_string(),
                                start: m.start(),
                                end: m.end(),
                                multi: true,
                            },
                        );
                    }
                }
            }
        }
    }

    #[allow(clippy::missing_panics_doc)]
    /// A utility function to scan for just bounded tokens
    pub fn run_bounded(&self, context: &str, result: &mut HashMap<usize, Vec<FullToken>>) {
        for tok in &self.bounded {
            // Init
            let mut start_index = 0;
            let mut grapheme_index = 0;
            // Iterate over each character
            while start_index < context.len() {
                // Get and check for potential start token match
                let potential_token: String = context
                    .chars()
                    .skip(grapheme_index)
                    .take(glen!(tok.start))
                    .collect();

                // If there is a start token, keep incrementing until end token is found
                if potential_token == tok.start {
                    let tok_start_index = start_index;
                    let mut tok_grapheme_index = grapheme_index;

                    // Start creating token
                    let mut current_token = FullToken {
                        kind: tok.kind.to_string(),
                        text: tok.start.to_string(),
                        start: tok_start_index,
                        end: tok_start_index + tok.start.len(),
                        multi: false,
                    };
                    tok_grapheme_index += glen!(tok.start);
                    let mut potential_end: String = "".to_string();
                    while potential_end != tok.end && current_token.end != context.len() {
                        potential_end = context
                            .chars()
                            .skip(tok_grapheme_index)
                            .take(glen!(tok.end))
                            .collect();
                        // Check for potential escaped end character to skip over
                        if tok.escaping {
                            if let Some(lookahead) =
                                context.chars().nth(tok_grapheme_index + glen!(tok.end))
                            {
                                if format!("{}{}", potential_end, lookahead)
                                    == format!("\\{}", tok.end)
                                {
                                    current_token.end += 1 + tok.end.len();
                                    write!(current_token.text, "\\{}", tok.end).unwrap();
                                    tok_grapheme_index += 1 + glen!(tok.end);
                                    continue;
                                }
                            }
                        }
                        if potential_end == tok.end {
                            current_token.end += tok.end.len();
                            current_token.text.push_str(&tok.end);
                            break;
                        }
                        // Part of the token, append on
                        current_token
                            .text
                            .push(context.chars().nth(tok_grapheme_index).unwrap());
                        current_token.end += gidx!(context, tok_grapheme_index);
                        tok_grapheme_index += 1;
                    }
                    // Update and add the token to the end result
                    current_token.multi = current_token.text.contains('\n');
                    insert_token(result, current_token.start, current_token);
                }
                // Update the indices
                if start_index < context.len() {
                    start_index += gidx!(context, grapheme_index);
                    grapheme_index += 1;
                }
            }
        }
    }

    /// This is the method that you call to get the stream of tokens for a specific line.
    /// The first argument is the string with the code that you wish to highlight.  
    /// the second argument is the line number that you wish to highlight.
    /// It returns a vector of tokens which can be used to highlight the individual line
    /// ```rust
    /// let mut lua = Highlighter::new();
    /// lua.add("(?ms)[[.*?]]", "string");
    /// lua.add("print", "keyword");
    /// lua.run_line(r#"
    /// print ([[ Hello World!
    /// ]])
    /// "#, 2);
    /// ```
    /// This example will return the second line, with the `]]` marked as a string
    /// The advantage of using this over the `run` method is that it is a lot faster
    /// This is because it only has to render one line rather than all of them, saving time
    ///
    /// This won't work with bounded tokens due to problems with determining what is a start
    /// token and what isn't. Bounded tokens require all lines above to be loaded, which
    /// run line doesn't assume.
    #[must_use]
    pub fn run_line(&self, context: &str, line: usize) -> Option<Vec<Token>> {
        // Locate multiline stuff
        let mut result: HashMap<usize, Vec<FullToken>> = HashMap::new();
        // Locate multiline regular expressions
        self.run_multiline(context, &mut result);
        // Calculate start and end indices (raw) of the line
        let (mut start, mut end) = (0, 0);
        let mut current_line = 0;
        let mut raw: usize = 0;
        for i in context.chars() {
            raw += i.to_string().len();
            if i == '\n' {
                current_line += 1;
                match current_line.cmp(&line) {
                    Ordering::Equal => start = raw,
                    Ordering::Greater => {
                        end = raw.saturating_sub(1);
                        break;
                    }
                    #[cfg(not(tarpaulin_include))]
                    Ordering::Less => (),
                }
            }
        }
        // Prune multiline tokens
        for (s, tok) in result.clone() {
            let tok = find_longest_token(&tok);
            if tok.start > end || tok.end < start {
                // This token is before or after this line
                result.remove(&s);
            } else {
                // This token is outside this line
                result.insert(s, vec![tok]);
            }
        }
        // Get then line contents
        let line_text = &context.get(start..end)?;
        // Locate single line tokens within the line (not the context - hence saving time)
        self.run_singleline(line_text, &mut result);
        // Split multiline tokens to ensure all data in result is relevant
        for (s, tok) in result.clone() {
            let tok = tok[0].clone();
            if tok.multi {
                // Check if line starts in token
                let tok_start = if start > tok.start && start < tok.end {
                    start - tok.start
                } else {
                    0
                };
                let tok_end = if end > tok.start && end < tok.end {
                    end - tok.start
                } else {
                    tok.len()
                };
                let tok_text = &tok.text[tok_start..tok_end];
                let true_start = if start > tok.start {
                    0
                } else {
                    tok.start - start
                };
                let true_end = true_start + tok_text.len();
                result.remove(&s);
                let tok = FullToken {
                    text: tok_text.to_string(),
                    kind: tok.kind,
                    start: true_start,
                    end: true_end,
                    multi: true,
                };
                result.insert(true_start, vec![tok]);
            }
        }
        // Assemble the line
        let mut stream = vec![];
        let mut eat = String::new();
        let mut c = 0;
        let mut g = 0;
        let chars: Vec<char> = line_text.chars().collect();
        while c != line_text.len() {
            if let Some(v) = result.get(&c) {
                // There are tokens here
                if !eat.is_empty() {
                    stream.push(Token::Text(eat.to_string()));
                    eat = String::new();
                }
                // Get token
                let tok = find_longest_token(v);
                stream.push(Token::Start(tok.kind.clone()));
                // Iterate over each character in the token text
                let mut token_eat = String::new();
                for ch in tok.text.chars() {
                    token_eat.push(ch);
                }
                if !token_eat.is_empty() {
                    stream.push(Token::Text(token_eat));
                }
                stream.push(Token::End(tok.kind.clone()));
                c += tok.len();
                g += tok.text.chars().count();
            } else {
                // There are no tokens here
                eat.push(chars[g]);
                c += chars[g].to_string().len();
                g += 1;
            }
        }
        if !eat.is_empty() {
            stream.push(Token::Text(eat));
        }
        Some(stream)
    }

    /// This is the method that you call to get the stream of tokens
    /// The argument is the string with the code that you wish to highlight
    /// Return a vector of a vector of tokens, representing the lines and the tokens in them
    /// ```rust
    /// let mut python = Highlighter::new();
    /// python.add("[0-9]+", "number");
    /// python.run("some numbers: 123");
    /// ```
    /// This example will highlight the numbers `123` in the string
    #[must_use]
    pub fn run(&self, code: &str) -> Vec<Vec<Token>> {
        // Do the highlighting on the code
        let mut result: HashMap<usize, Vec<FullToken>> = HashMap::new();
        // Locate regular expressions
        self.run_singleline(code, &mut result);
        // Locate multiline regular expressions
        self.run_multiline(code, &mut result);
        // Locate bounded tokens
        self.run_bounded(code, &mut result);
        // Use the hashmap into a vector
        let mut lines = vec![];
        let mut stream = vec![];
        let mut eat = String::new();
        let mut c = 0;
        let mut g = 0;
        let chars: Vec<char> = code.chars().collect();
        while c < code.len() {
            if let Some(v) = result.get(&c) {
                // There are tokens here
                if !eat.is_empty() {
                    stream.push(Token::Text(eat.to_string()));
                    eat = String::new();
                }
                // Get token
                let tok = find_longest_token(v);
                stream.push(Token::Start(tok.kind.clone()));
                // Iterate over each character in the token text
                let mut token_eat = String::new();
                for ch in tok.text.chars() {
                    if ch == '\n' {
                        stream.push(Token::Text(token_eat));
                        token_eat = String::new();
                        stream.push(Token::End(tok.kind.clone()));
                        lines.push(stream);
                        stream = vec![Token::Start(tok.kind.clone())];
                    } else {
                        token_eat.push(ch);
                    }
                }
                if !token_eat.is_empty() {
                    stream.push(Token::Text(token_eat));
                }
                stream.push(Token::End(tok.kind.clone()));
                c += tok.len();
                g += tok.text.chars().count();
            } else {
                // There are no tokens here
                if chars[g] == '\n' {
                    if !eat.is_empty() {
                        stream.push(Token::Text(eat.to_string()));
                    }
                    lines.push(stream);
                    stream = vec![];
                    eat = String::new();
                } else {
                    eat.push(chars[g]);
                }
                c += chars[g].to_string().len();
                g += 1;
            }
        }
        if !eat.is_empty() {
            stream.push(Token::Text(eat));
        }
        lines.push(stream);
        lines
    }

    /// This is a function that will convert from a stream of tokens into a token option type
    /// A token option type is nicer to work with for certain formats such as HTML
    #[must_use]
    pub fn from_stream(input: &[Token]) -> Vec<TokOpt> {
        let mut result = vec![];
        let mut current = String::new();
        let mut toggle = false;
        for i in input {
            match i {
                Token::Start(_) => {
                    toggle = true;
                }
                Token::Text(t) => {
                    if toggle {
                        current.push_str(t);
                    } else {
                        result.push(TokOpt::None(t.clone()));
                    }
                }
                Token::End(k) => {
                    toggle = false;
                    result.push(TokOpt::Some(current, k.clone()));
                    current = String::new();
                }
            }
        }
        result
    }

    /// This is a function that will convert from a tokopt slice to a token stream
    /// A token stream is easier to render for certain formats such as the command line
    #[must_use]
    pub fn from_opt(input: &[TokOpt]) -> Vec<Token> {
        let mut result = vec![];
        for i in input {
            match i {
                TokOpt::Some(text, kind) => {
                    result.push(Token::Start(kind.to_string()));
                    result.push(Token::Text(text.clone()));
                    result.push(Token::End(kind.to_string()));
                }
                TokOpt::None(text) => result.push(Token::Text(text.clone())),
            }
        }
        result
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// This is a method to find the token that occupies the most space
/// The argument is for the list of tokens to compare
fn find_longest_token(tokens: &[FullToken]) -> FullToken {
    let mut longest = FullToken {
        text: "".to_string(),
        kind: "".to_string(),
        start: 0,
        end: 0,
        multi: false,
    };
    for tok in tokens {
        if longest.len() < tok.len() {
            longest = tok.clone();
        }
    }
    longest
}

/// This is a method to insert regex into a hashmap
/// It takes the hashmap to add to, the regex to add and the name of the token
fn insert_regex(hash: &mut HashMap<String, Vec<Regex>>, regex: Regex, token: &str) {
    // Insert regex into hashmap of vectors
    if let Some(v) = hash.get_mut(token) {
        v.push(regex);
    } else {
        hash.insert(token.to_string(), vec![regex]);
    }
}

/// This is a method to insert a token into a hashmap
/// It takes the hashmap to add to, the token to add and the start position of the token
fn insert_token(map: &mut HashMap<usize, Vec<FullToken>>, key: usize, token: FullToken) {
    // Insert token into hashmap of vectors
    if let Some(v) = map.get_mut(&key) {
        v.push(token);
    } else {
        map.insert(key, vec![token]);
    }
}

//! These highlighters will return the following tokens names:
//!
//! keyword - a keyword for that language
//! boolean - a boolean
//! comment - a comment (both multiline and single line)
//! string - a string data type
//! number - a number
//! function - a function identifier
//! macro - a macro identifier
//! struct - a class / struct / enum / trait name
//! operator - operators within that language e.g. == or != or >= or +
//! namespace - a namespace for modules
//! character - a character data type
//! attribute - an attribute within the language
//! reference - for references within the language e.g. &self or &mut
//! symbol - a symbol data type (mainly for the Ruby language)
//! global - for global variable identifiers
//! regex - for regex datatypes in languages
//! header - for headers (mainly for the C language)
//!
//! These syntax highlighters are quite advanced and tend to do a decent job of syntax highlighting
//! with detail of which wouldn't be out of place in a popular text editor.
//! there may be an edge case where something goes a bit wrong, in that case, please open an issue

use crate::Highlighter;

/// Obtain the rust syntax highlighter
#[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
#[must_use]
pub fn rust() -> Highlighter {
    let mut h = Highlighter::new();
    let keywords: Vec<&str> = vec![
        r"\b(as)\b",
        r"\b(break)\b",
        r"\b(char)\b",
        r"\b(const)\b",
        r"\b(continue)\b",
        r"\b(crate)\b",
        r"\b(else)\b",
        r"\b(enum)\b",
        r"\b(extern)\b",
        r"\b(fn)\b",
        r"\b(for)\b",
        r"\b(if)\b",
        r"\b(impl)\b",
        r"\b(in)\b",
        r"\b(let)\b",
        r"\b(loop)\b",
        r"\b(match)\b",
        r"\b(mod)\b",
        r"\b(move)\b",
        r"\b(mut)\b",
        r"\b(pub)\b",
        r"\b(ref)\b",
        r"\b(return)\b",
        r"\b(self)\b",
        r"\b(static)\b",
        r"\b(struct)\b",
        r"\b(super)\b",
        r"\b(trait)\b",
        r"\b(type)\b",
        r"\b(unsafe)\b",
        r"\b(use)\b",
        r"\b(where)\b",
        r"\b(while)\b",
        r"\b(async)\b",
        r"\b(await)\b",
        r"\b(dyn)\b",
        r"\b(abstract)\b",
        r"\b(become)\b",
        r"\b(box)\b",
        r"\b(do)\b",
        r"\b(final)\b",
        r"\b(macro)\b",
        r"\b(override)\b",
        r"\b(priv)\b",
        r"\b(typeof)\b",
        r"\b(unsized)\b",
        r"\b(virtual)\b",
        r"\b(yield)\b",
        r"\b(try)\b",
        r"\b('static)\b",
        r"\b(u8)\b",
        r"\b(u16)\b",
        r"\b(u32)\b",
        r"\b(u64)\b",
        r"\b(u128)\b",
        r"\b(usize)\b",
        r"\b(i8)\b",
        r"\b(i16)\b",
        r"\b(i32)\b",
        r"\b(i64)\b",
        r"\b(i128)\b",
        r"\b(isize)\b",
        r"\b(f32)\b",
        r"\b(f64)\b",
        r"\b(String)\b",
        r"\b(Vec)\b",
        r"\b(str)\b",
        r"\b(Some)\b",
        r"\b(bool)\b",
        r"\b(None)\b",
        r"\b(Box)\b",
        r"\b(Result)\b",
        r"\b(Option)\b",
        r"\b(Ok)\b",
        r"\b(Err)\b",
        r"\b(Self)\b",
        r"\b(std)\b",
    ];
    // Keywords
    h.join(keywords.as_slice(), "keyword").unwrap();
    h.join(&[r"\b(true)\b", r"\b(false)\b"], "boolean").unwrap();
    // Add comment definitions
    h.add(r"(?m)(//.*)$", "comment").unwrap();
    h.add_bounded("/*", "*/", false, "comment");
    // Add numbers definition
    h.join(&[r"\b(\d+.\d+|\d+)", r"\b(\d+.\d+(?:f32|f64))"], "number")
        .unwrap();
    // Add string definition
    h.add_bounded("\"", "\"", true, "string");
    // Add identifier definition
    h.add(r"([a-z_][A-Za-z0-9_]*)\s*\(", "function").unwrap();
    // Add macro definition
    h.add(r"([a-z_][A-Za-z0-9_]*!)\s*", "macro").unwrap();
    // Structs
    h.join(
        &[
            "(?:trait|enum|struct|impl)\\s+([A-Z][A-Za-z0-9_]*)\\s*",
            "impl(?:<.*?>|)\\s+([A-Z][A-Za-z0-9_]*)",
            "([A-Z][A-Za-z0-9_]*)::",
            r"([A-Z][A-Za-z0-9_]*)\s*\(",
            "impl.*for\\s+([A-Z][A-Za-z0-9_]*)",
            r"::\s*([A-Z_][A-Za-z0-9_]*)\s*\(",
        ],
        "struct",
    )
    .unwrap();
    // Operators
    h.join(
        &[
            r"(=)",
            r"(\+)",
            r"(\-)",
            r"(\*)",
            r"[^/](/)[^/]",
            r"(\+=)",
            r"(\-=)",
            r"(\*=)",
            r"(\\=)",
            r"(\?)",
            r"(==)",
            r"(!=)",
            r"(>=)",
            r"(<=)",
            r"(<)",
            r"(>)",
        ],
        "operator",
    )
    .unwrap();
    // Namespaces
    h.add(r"([a-z_][A-Za-z0-9_]*)::", "namespace").unwrap();
    // Characters
    h.join(&["('.')", r"('\\.')"], "character").unwrap();
    // Attributes
    h.add("(?ms)^\\s*(#(?:!|)\\[.*?\\])", "attribute").unwrap();
    // References
    h.join(
        &[
            "(&)", "&str", "&mut", "&self", "&i8", "&i16", "&i32", "&i64", "&i128", "&isize",
            "&u8", "&u16", "&u32", "&u64", "&u128", "&usize", "&f32", "&f64",
        ],
        "reference",
    )
    .unwrap();
    h
}

/// Obtain the python syntax highlighter
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn python() -> Highlighter {
    let mut h = Highlighter::new();
    let keywords: Vec<&str> = vec![
        r"\b(and)\b",
        r"\b(as)\b",
        r"\b(assert)\b",
        r"\b(break)\b",
        r"\b(class)\b",
        r"\b(continue)\b",
        r"\b(def)\b",
        r"\b(del)\b",
        r"\b(elif)\b",
        r"\b(else)\b",
        r"\b(except)\b",
        r"\b(exec)\b",
        r"\b(finally)\b",
        r"\b(for)\b",
        r"\b(from)\b",
        r"\b(global)\b",
        r"\b(if)\b",
        r"\b(import)\b",
        r"\b(in)\b",
        r"\b(is)\b",
        r"\b(lambda)\b",
        r"\b(not)\b",
        r"\b(or)\b",
        r"\b(pass)\b",
        r"\b(raise)\b",
        r"\b(return)\b",
        r"\b(try)\b",
        r"\b(while)\b",
        r"\b(with)\b",
        r"\b(yield)\b",
        r"\b(str)\b",
        r"\b(bool)\b",
        r"\b(int)\b",
        r"\b(tuple)\b",
        r"\b(list)\b",
        r"\b(dict)\b",
        r"\b(tuple)\b",
        r"\b(len)\b",
        r"\b(None)\b",
        r"\b(input)\b",
        r"\b(type)\b",
        r"\b(set)\b",
        r"\b(range)\b",
        r"\b(enumerate)\b",
        r"\b(open)\b",
        r"\b(iter)\b",
        r"\b(min)\b",
        r"\b(max)\b",
        r"\b(dir)\b",
        r"\b(self)\b",
        r"\b(isinstance)\b",
        r"\b(help)\b",
        r"\b(next)\b",
        r"\b(super)\b",
    ];
    // Keywords
    h.join(keywords.as_slice(), "keyword").unwrap();
    h.join(&[r"\b(True)\b", r"\b(False)\b"], "boolean").unwrap();
    // Add comment definitions
    h.add(r"(?m)(#.*)$", "comment").unwrap();
    // Add numbers definition
    h.add(r"\b(\d+.\d+|\d+)", "number").unwrap();
    // Add string definition
    h.add_bounded("\"\"\"", "\"\"\"", false, "string");
    h.add_bounded("\"", "\"", true, "string");
    // Add identifier definition
    h.add(r"([a-z_][A-Za-z0-9_]*)\s*\(", "function").unwrap();
    // Struct definition
    h.add(r"class\s+([A-Za-z0-9_]*)", "struct").unwrap();
    // Operators
    h.join(
        &[
            r"(=)",
            r"(\+)",
            r"(\-)",
            r"(\*)",
            r"(\s/\s)",
            r"(\s//\s)",
            r"(%)",
            r"(\+=)",
            r"(\-=)",
            r"(\*=)",
            r"(\\=)",
            r"(==)",
            r"(!=)",
            r"(>=)",
            r"(<=)",
            r"(<)",
            r"(>)",
        ],
        "operator",
    )
    .unwrap();
    // Attributes
    h.add("@.*$", "attribute").unwrap();
    h
}

#![warn(clippy::all, clippy::pedantic)]

//! # Synoptic
//! ## A simple rust syntax highlighting crate
//!
//! Here's an example of it in action (using the `termion` crate)
//!
//! ```rust
//! use synoptic::{Token, Highlighter};
//! use termion::color;
//!
//! const DEMO: &str = r#"/*
//! Multiline comments
//! Work great
//! */
//!
//! pub fn main() -> bool {
//!     // Demonstrate syntax highlighting in Rust!
//!     println!("Full Unicode Support: 你好！Pretty cool");
//!     return true;
//! }
//! "#;
//!
//! fn main() {
//!     // Build the rust syntax highlighter
//!     let mut rust = Highlighter::new();
//!     // Add keywords
//!     rust.join(&["fn", "return", "pub"], "keyword").unwrap();
//!     rust.join(&["bool"], "type").unwrap();
//!     rust.join(&["true", "false"], "boolean").unwrap();
//!     // Add comment definitions
//!     rust.add(r"(?m)(//.*)$", "comment").unwrap();
//!     rust.add(r"(?ms)/\*.*?\*/", "comment").unwrap();
//!     // Add string definition
//!     rust.add("\".*?\"", "string").unwrap();
//!     // Add identifier definition
//!     rust.add(r"([a-z_][A-Za-z0-9_]*)\s*\(", "identifier").unwrap();
//!     // Add macro definition
//!     rust.add(r"([a-z_][A-Za-z0-9_]*!)\s*", "macro").unwrap();
//!
//!     // Run highlighter
//!     let highlighting = rust.run(DEMO);
//!     
//!     // For each row
//!     for (c, row) in highlighting.iter().enumerate() {
//!         // Print line number (with padding)
//!         print!("{: >3} |", c);
//!         // For each token within each row
//!         for tok in row {
//!         // Handle the tokens
//!             match tok {
//!                 // Handle the start token (start foreground colour)
//!                 Token::Start(kind) => match kind.as_str() {
//!                     "comment" => print!("{}", color::Fg(color::Black)),
//!                     "string" => print!("{}", color::Fg(color::Green)),
//!                     "keyword" => print!("{}", color::Fg(color::Blue)),
//!                     "type" => print!("{}", color::Fg(color::LightMagenta)),
//!                     "boolean" => print!("{}", color::Fg(color::LightGreen)),
//!                     "identifier" => print!("{}", color::Fg(color::Yellow)),
//!                     "macro" => print!("{}", color::Fg(color::Magenta)),
//!                     _ => (),
//!                 }
//!                 // Handle a text token (print out the contents)
//!                 Token::Text(txt) => print!("{}", txt),
//!                 // Handle an end token (reset foreground colour)
//!                 Token::End(_) => print!("{}", color::Fg(color::Reset)),
//!             }
//!         }
//!         // Prevent text being cut off without a newline
//!         println!("");
//!     }
//! }
//! ```

/// This provides the main Highlighter class you will need to make your own
/// syntax rules, or if you wish to modify the existing rules from the set of provided highlighters
pub mod highlighter;
/// This provides a set of prebuilt highlighters for various languages
/// You can always build on top of them, as they just return highlighter classes
pub mod languages;
/// This provides the types of tokens which you can use to apply your syntax highlighting into
/// whichever format you please
pub mod tokens;
/// This provides utilities to help with formatting tokens on the screen
pub mod util;

/// Highlighter is the highlighter struct that does the highlighting
/// This is what you'll want to use
pub use highlighter::Highlighter;

/// This contains enums and structs that represent tokens
pub use tokens::{TokOpt, Token};

/// This contains utilitiues for trimming lines
pub use util::trim;

use unicode_width::UnicodeWidthChar;

/// For storing tokens to put into a string
/// It has a start token, to mark the start of a token
/// It has a text token, for the text inbetween and inside tokens
/// It also has an end token, to mark the end of a token
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    Start(String),
    Text(String),
    End(String),
}

/// An alternative way to store tokens, makes it easy to trim them
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokOpt {
    Some(String, String),
    None(String),
}

impl TokOpt {
    /// Determines if this token is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let (TokOpt::Some(text, _) | TokOpt::None(text)) = self;
        text.len() == 0
    }

    /// Takes a single character off the front of a token
    pub fn nibble(&mut self) -> Option<char> {
        let (TokOpt::Some(ref mut text, _) | TokOpt::None(ref mut text)) = self;
        let ch = *text.chars().collect::<Vec<_>>().get(0)?;
        text.remove(0);
        if UnicodeWidthChar::width(ch)? > 1 {
            text.insert(0, ' ');
        }
        Some(ch)
    }
}

/// For storing all the data in a token to prevent overwriting
/// This contains the contents, type, start and end of the token
/// This is used to compare tokens to each other to prevent tokens inside tokens
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FullToken {
    pub text: String,
    pub kind: String,
    pub start: usize,
    pub end: usize,
    pub multi: bool,
}

impl FullToken {
    /// Returns the length of the token
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Determines if the token is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// For representing a bounded token definition
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Bounded {
    pub kind: String,
    pub start: String,
    pub end: String,
    pub escaping: bool,
}

use crate::highlighter::Highlighter;
use crate::tokens::{TokOpt, Token};

#[macro_export]
macro_rules! glen {
    ($e:expr) => {
        $e.chars().collect::<Vec<_>>().len()
    };
}

// Converts grapheme index into byte index
#[macro_export]
macro_rules! gidx {
    ($e:expr, $i:expr) => {
        $e.chars().nth($i).unwrap().len_utf8()
    };
}

/// This will trim tokens to adjust to an offset
/// This is really useful if you are building a text editor on the command line
/// The first argument is a stream of tokens, the second is the start point
/// ```rust
/// let mut rust = Highlighter::new();
/// rust.add("fn", "keyword");
/// let result = rust.run("fn");
/// trim(&result, 1); // <- This will return [Start("keyword"), Text("n"), End("keyword")]
/// ```
/// This will cut off the beginning of the token and keep the token's colour intact
#[must_use]
pub fn trim(input: &[Token], start: usize) -> Vec<Token> {
    let mut opt = Highlighter::from_stream(input);
    let mut total_width = 0;
    for i in &opt {
        let (TokOpt::Some(txt, _) | TokOpt::None(txt)) = i;
        total_width += txt.len();
    }
    let width = total_width.saturating_sub(start);
    while total_width != width {
        if let Some(token) = opt.get_mut(0) {
            token.nibble();
            total_width -= 1;
            if token.is_empty() {
                opt.remove(0);
            }
        } else {
            break;
        }
    }
    Highlighter::from_opt(&opt)
}

// Document.rs - For managing external files
use crate::config::{Reader, Status, TokenType};
use crate::editor::OFFSET;
use crate::util::{line_offset, spaces_to_tabs, tabs_to_spaces};
use crate::{log, Editor, Event, EventStack, Position, Row, Size, Variable, VERSION};
use crossterm::event::KeyCode as Key;
use regex::Regex;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{cmp, fs};
use unicode_width::UnicodeWidthStr;

// For holding the info in the command line
pub struct CommandLine {
    pub msg: Type,
    pub text: String,
}

// Enum for the kinds of status messages
pub enum Type {
    Error,
    Warning,
    Info,
}

// Enum to determine which tab type
#[derive(Debug, Copy, Clone)]
pub enum TabType {
    Spaces,
    Tabs,
}

// Document struct (class) to manage files and text
pub struct Document {
    pub rows: Vec<Row>,         // For holding the contents of the document
    pub path: String,           // For holding the path to the document
    pub name: String,           // For holding the name of the document
    pub dirty: bool,            // True if the current document has been edited
    pub cmd_line: CommandLine,  // For holding the command line
    pub line_offset: usize,     // For holding a line number offset
    pub undo_stack: EventStack, // For holding the undo event stack
    pub redo_stack: EventStack, // For holding the redo event stack
    pub regex: Vec<TokenType>,  // For holding regular expressions
    pub icon: String,           // For holding the icon of the document
    pub kind: String,           // For holding the icon of the document
    pub show_welcome: bool,     // Whether to show welcome in the document
    pub cursor: Position,       // For holding the raw cursor location
    pub offset: Position,       // For holding the offset on the X and Y axes
    pub graphemes: usize,       // For holding the special grapheme cursor
    pub tabs: TabType,          // For detecting if tabs are used over spaces
    pub last_save_index: usize, // For holding the last save index
    pub true_path: String,      // For holding the path that was provided as argument
    pub read_only: bool,        // Boolean to determine if the document is read only
}

// Add methods to the document struct
impl Document {
    pub fn new(config: &Reader, status: &Status, read_only: bool) -> Self {
        // Create a new, empty document
        Self {
            rows: vec![Row::from("")],
            name: String::from("[No name]"),
            dirty: false,
            cmd_line: Document::config_to_commandline(&status),
            path: String::new(),
            line_offset: config.general.line_number_padding_right
                + config.general.line_number_padding_left,
            undo_stack: EventStack::new(),
            redo_stack: EventStack::new(),
            regex: Reader::get_syntax_regex(&config, ""),
            icon: String::new(),
            kind: String::new(),
            show_welcome: true,
            graphemes: 0,
            cursor: Position { x: 0, y: OFFSET },
            offset: Position { x: 0, y: 0 },
            tabs: TabType::Spaces,
            last_save_index: 0,
            true_path: String::new(),
            read_only,
        }
    }
    pub fn open(config: &Reader, status: &Status, path: &str, read_only: bool) -> Option<Self> {
        // Create a new document from a path
        let true_path = path.to_string();
        let path = path.split(':').next().unwrap();
        if let Ok(file) = fs::read_to_string(path) {
            // File exists
            let tabs = file.contains("\n\t");
            let file = tabs_to_spaces(&file, config.general.tab_width);
            let mut file = Document::split_file(&file);
            // Handle newline on last line
            if let Some(line) = file.iter().last() {
                if line.is_empty() {
                    let _ = file.pop();
                }
            }
            // Handle empty document by automatically inserting a row
            if file.is_empty() {
                file.push("");
            }
            let ext = path.split('.').last().unwrap_or(&"");
            Some(Self {
                rows: file.iter().map(|row| Row::from(*row)).collect(),
                name: Path::new(path)
                    .file_name()
                    .unwrap_or_else(|| OsStr::new(path))
                    .to_str()
                    .unwrap_or(&path)
                    .to_string(),
                dirty: false,
                cmd_line: Document::config_to_commandline(&status),
                path: path.to_string(),
                line_offset: config.general.line_number_padding_right
                    + config.general.line_number_padding_left,
                undo_stack: EventStack::new(),
                redo_stack: EventStack::new(),
                regex: Reader::get_syntax_regex(&config, ext),
                kind: Self::identify(path).0.to_string(),
                icon: Self::identify(path).1.to_string(),
                show_welcome: false,
                graphemes: 0,
                cursor: Position { x: 0, y: OFFSET },
                offset: Position { x: 0, y: 0 },
                tabs: if tabs { TabType::Tabs } else { TabType::Spaces },
                last_save_index: 0,
                true_path,
                read_only,
            })
        } else {
            // File doesn't exist
            None
        }
    }
    pub fn from(config: &Reader, status: &Status, path: &str, read_only: bool) -> Self {
        // Create a new document from a path with empty document on error
        let true_path = path.to_string();
        let path = path.split(':').next().unwrap();
        if let Some(doc) = Document::open(&config, &status, &true_path, read_only) {
            log!("Opening file", "File was found");
            doc
        } else {
            // Create blank document
            log!("Opening file", "File not found");
            let ext = path.split('.').last().unwrap_or(&"");
            Self {
                rows: vec![Row::from("")],
                name: path.to_string(),
                path: path.to_string(),
                dirty: false,
                cmd_line: Document::config_to_commandline(&status),
                line_offset: config.general.line_number_padding_right
                    + config.general.line_number_padding_left,
                undo_stack: EventStack::new(),
                redo_stack: EventStack::new(),
                regex: Reader::get_syntax_regex(&config, ext),
                kind: Self::identify(path).0.to_string(),
                icon: Self::identify(path).1.to_string(),
                show_welcome: false,
                graphemes: 0,
                cursor: Position { x: 0, y: OFFSET },
                offset: Position { x: 0, y: 0 },
                tabs: TabType::Spaces,
                last_save_index: 0,
                true_path,
                read_only,
            }
        }
    }
    pub fn split_file(contents: &str) -> Vec<&str> {
        // Detect DOS line ending
        let splitter = Regex::new("(?ms)(\r\n|\n)").unwrap();
        splitter.split(contents).collect()
    }
    pub fn correct_path(&mut self, term: &Size) {
        if self.true_path.contains(':') {
            let split: Vec<&str> = self.true_path.split(':').collect();
            let mut y = split.get(1).unwrap_or(&"").parse().unwrap_or(0);
            let mut x = split.get(2).unwrap_or(&"").parse().unwrap_or(0);
            if y >= self.rows.len() {
                self.set_command_line(format!("Row {} out of scope", y), Type::Warning);
                y = self.rows.len();
            }
            if x >= self.rows[y.saturating_sub(1)].length() {
                self.set_command_line(format!("Column {} out of scope", x), Type::Warning);
                x = self.rows[y.saturating_sub(1)].length();
            }
            self.goto(
                Position {
                    x,
                    y: y.saturating_sub(1),
                },
                term,
            );
        }
    }
    pub fn set_command_line(&mut self, text: String, msg: Type) {
        // Function to update the command line
        self.cmd_line = CommandLine { text, msg };
    }
    pub fn mass_redraw(&mut self) {
        for i in &mut self.rows {
            i.updated = true;
        }
    }
    pub fn config_to_commandline(status: &Status) -> CommandLine {
        CommandLine {
            text: match status {
                Status::Success => "Welcome to Ox".to_string(),
                Status::File => "Config file not found, using default values".to_string(),
                Status::Parse(error) => format!("Failed to parse: {:?}", error),
                Status::Empty => "Config file is empty, using defaults".to_string(),
            },
            msg: match status {
                Status::Success | Status::Empty => Type::Info,
                Status::File => Type::Warning,
                Status::Parse(_) => Type::Error,
            },
        }
    }
    pub fn format(&self, template: &str) -> String {
        // Form data from a template
        template
            .replace("%f", &self.name)
            .replace("%F", &self.path)
            .replace("%i", &self.icon)
            .replace(
                "%I",
                &if self.icon.is_empty() {
                    String::new()
                } else {
                    format!("{} ", self.icon)
                },
            )
            .replace("%n", &self.kind)
            .replace(
                "%l",
                &format!("{}", self.cursor.y + self.offset.y.saturating_sub(OFFSET)),
            )
            .replace("%L", &format!("{}", self.rows.len()))
            .replace("%x", &format!("{}", self.cursor.x + self.offset.x))
            .replace("%y", &format!("{}", self.cursor.y + self.offset.y))
            .replace("%v", VERSION)
            .replace("%d", if self.dirty { "[+]" } else { "" })
            .replace("%D", if self.dirty { "\u{fb12} " } else { "\u{f723} " })
    }
    pub fn move_cursor(&mut self, direction: Key, term: &Size, wrap: bool) {
        // Move the cursor around the editor
        match direction {
            Key::Down => {
                // Move the cursor down
                if self.cursor.y + self.offset.y + 1 - (OFFSET) < self.rows.len() {
                    // If the proposed move is within the length of the document
                    if self.cursor.y == term.height.saturating_sub(3) {
                        self.offset.y = self.offset.y.saturating_add(1);
                    } else {
                        self.cursor.y = self.cursor.y.saturating_add(1);
                    }
                    self.snap_cursor(term);
                    self.prevent_unicode_hell();
                    self.recalculate_graphemes();
                }
            }
            Key::Up => {
                // Move the cursor up
                if self.cursor.y - OFFSET == 0 {
                    self.offset.y = self.offset.y.saturating_sub(1);
                } else if self.cursor.y != OFFSET {
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                }
                self.snap_cursor(term);
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Right => {
                // Move the cursor right
                let line = &self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
                // Check for line wrapping
                if line.length() == self.cursor.x + self.offset.x
                    && self.cursor.y + self.offset.y - OFFSET != self.rows.len().saturating_sub(1)
                    && wrap
                {
                    self.move_cursor(Key::Down, term, wrap);
                    self.leap_cursor(Key::Home, term);
                    return;
                }
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line.ext_chars().get(self.cursor.x + self.offset.x) {
                    jump = UnicodeWidthStr::width(*chr);
                }
                // Check the proposed move is within the current line length
                if line.length() > self.cursor.x + self.offset.x {
                    // Check for normal width character
                    let indicator1 =
                        self.cursor.x == term.width.saturating_sub(self.line_offset + jump + 1);
                    // Check for half broken unicode character
                    let indicator2 =
                        self.cursor.x == term.width.saturating_sub(self.line_offset + jump);
                    if indicator1 || indicator2 {
                        self.offset.x = self.offset.x.saturating_add(jump);
                    } else {
                        self.cursor.x = self.cursor.x.saturating_add(jump);
                    }
                    self.graphemes = self.graphemes.saturating_add(1);
                }
            }
            Key::Left => {
                // Move the cursor left
                let line = &self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
                if self.cursor.x + self.offset.x == 0
                    && self.cursor.y + self.offset.y - OFFSET != 0
                    && wrap
                {
                    self.move_cursor(Key::Up, term, wrap);
                    self.leap_cursor(Key::End, term);
                    return;
                }
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line
                    .ext_chars()
                    .get((self.cursor.x + self.offset.x).saturating_sub(1))
                {
                    jump = UnicodeWidthStr::width(*chr);
                }
                if self.cursor.x == 0 {
                    self.offset.x = self.offset.x.saturating_sub(jump);
                } else {
                    self.cursor.x = self.cursor.x.saturating_sub(jump);
                }
                self.graphemes = self.graphemes.saturating_sub(1);
            }
            _ => (),
        }
    }
    pub fn leap_cursor(&mut self, action: Key, term: &Size) {
        // Handle large cursor movements
        match action {
            Key::PageUp => {
                // Move cursor to the top of the screen
                self.cursor.y = OFFSET;
                self.snap_cursor(term);
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::PageDown => {
                // Move cursor to the bottom of the screen
                self.cursor.y = cmp::min(
                    self.rows.len().saturating_sub(1).saturating_add(OFFSET),
                    term.height.saturating_sub(3) as usize,
                );
                self.snap_cursor(term);
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Home => {
                // Move cursor to the start of the line
                self.offset.x = 0;
                self.cursor.x = 0;
                self.graphemes = 0;
            }
            Key::End => {
                // Move cursor to the end of the line
                let cursor = self.cursor;
                let offset = self.offset;
                let line = self.rows[cursor.y + offset.y - OFFSET].clone();
                if line.length() >= term.width.saturating_sub(self.line_offset) {
                    // Work out the width of the character to traverse
                    let mut jump = 1;
                    if let Some(chr) = line.ext_chars().get(line.length()) {
                        jump = UnicodeWidthStr::width(*chr);
                    }
                    self.offset.x = line
                        .length()
                        .saturating_add(jump + self.line_offset + 1)
                        .saturating_sub(term.width as usize);
                    self.cursor.x = term.width.saturating_sub(jump + self.line_offset + 1);
                } else {
                    self.cursor.x = line.length();
                }
                self.graphemes = line.chars().len();
            }
            _ => (),
        }
    }
    pub fn snap_cursor(&mut self, term: &Size) {
        // Snap the cursor to the end of the row when outside
        let current = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        if current.length() <= self.cursor.x + self.offset.x {
            // If the cursor is out of bounds
            self.leap_cursor(Key::Home, term);
            self.leap_cursor(Key::End, term);
        }
    }
    pub fn prevent_unicode_hell(&mut self) {
        // Make sure that the cursor isn't inbetween a unicode character
        let line = &self.rows[self.cursor.y + self.offset.y - OFFSET];
        if line.length() > self.cursor.x + self.offset.x {
            // As long as the cursor is within range
            let boundaries = line.boundaries();
            let mut index = self.cursor.x + self.offset.x;
            if !boundaries.contains(&index) && index != 0 {}
            while !boundaries.contains(&index) && index != 0 {
                self.cursor.x = self.cursor.x.saturating_sub(1);
                self.graphemes = self.graphemes.saturating_sub(1);
                index = index.saturating_sub(1);
            }
        }
    }
    pub fn recalculate_graphemes(&mut self) {
        // Recalculate the grapheme cursor after moving up and down
        let current = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        let jumps = current.get_jumps();
        let mut counter = 0;
        for (mut counter2, i) in jumps.into_iter().enumerate() {
            if counter == self.cursor.x + self.offset.x {
                break;
            }
            counter2 += 1;
            self.graphemes = counter2;
            counter += i;
        }
    }
    pub fn recalculate_offset(&mut self, config: &Reader) {
        // Calculate the offset for the line numbers
        self.line_offset = self.rows.len().to_string().len()
            + config.general.line_number_padding_right
            + config.general.line_number_padding_left;
    }
    pub fn tab(&mut self, pos: &Position, config: &Reader, term: &Size) {
        // Insert a tab
        for _ in 0..config.general.tab_width {
            self.rows[pos.y].insert(' ', pos.x);
            self.move_cursor(Key::Right, term, config.general.wrap_cursor);
        }
    }
    fn overwrite(&mut self, after: &[Row]) {
        // Override the entire contents of the document
        self.dirty = true;
        self.rows = after.to_vec();
    }
    fn update_line(&mut self, pos: &Position, after: Row, offset: i128) -> usize {
        // Update a line in the document
        self.dirty = true;
        let ind = line_offset(pos.y, offset, self.rows.len());
        self.rows[ind] = after;
        ind
    }
    fn delete_line(&mut self, pos: &Position, offset: i128) {
        // Delete a line in the document
        self.dirty = true;
        let ind = line_offset(pos.y, offset, self.rows.len());
        if self.rows.len() > 1 {
            self.rows.remove(ind);
        }
    }
    fn splice_up(&mut self, pos: &Position, reversed: bool, term: &Size, other: &Position) {
        // Splice the line up to the next
        self.dirty = true;
        let above = self.rows[pos.y.saturating_sub(1)].clone();
        let current = self.rows[pos.y].clone();
        let new = format!("{}{}", above.string, current.string);
        self.rows[pos.y.saturating_sub(1)] = Row::from(&new[..]);
        self.rows.remove(pos.y);
        if reversed {
            self.goto(*other, term);
        } else {
            let other = Position {
                x: above.length(),
                y: pos.y.saturating_sub(1),
            };
            self.goto(
                Position {
                    x: other.x,
                    y: other.y,
                },
                term,
            );
            self.undo_stack.push(Event::SpliceUp(*pos, other));
            self.undo_stack.commit();
        }
    }
    fn split_down(&mut self, pos: &Position, reversed: bool, term: &Size, other: &Position) {
        // Split the line in half
        self.dirty = true;
        let current = self.rows[pos.y].clone();
        let left: String = current.string.chars().take(pos.x).collect();
        let right: String = current.string.chars().skip(pos.x).collect();
        self.rows[pos.y] = Row::from(&left[..]);
        self.rows
            .insert(pos.y.saturating_add(1), Row::from(&right[..]));
        if reversed {
            self.goto(*other, term);
        } else {
            let other = Position {
                x: 0,
                y: pos.y.saturating_add(1),
            };
            self.goto(other, term);
            self.undo_stack.push(Event::SplitDown(*pos, other));
            self.undo_stack.commit();
        }
    }
    pub fn execute(&mut self, event: Event, reversed: bool, term: &Size, config: &Reader) {
        // Document edit event executor
        if self.read_only && Editor::will_edit(&event) {
            return;
        }
        match event {
            Event::Set(variable, value) => match variable {
                Variable::Saved => self.dirty = !value,
            },
            Event::Overwrite(_, ref after) => {
                self.overwrite(after);
                self.goto(Position { x: 0, y: 0 }, term);
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::UpdateLine(pos, offset, _, ref after) => {
                let ind = self.update_line(&pos, *after.clone(), offset);
                self.goto(Position { x: pos.x, y: ind }, term);
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::DeleteLine(pos, offset, _) => {
                self.delete_line(&pos, offset);
                self.goto(pos, term);
                if self.cursor.y + self.offset.y - OFFSET >= self.rows.len() {
                    self.move_cursor(Key::Up, term, config.general.wrap_cursor);
                }
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::Insertion(pos, ch) => {
                self.dirty = true;
                self.rows[pos.y].insert(ch, pos.x);
                self.move_cursor(Key::Right, term, config.general.wrap_cursor);
                self.goto(pos, term);
                self.move_cursor(Key::Right, term, config.general.wrap_cursor);
                if !reversed {
                    self.undo_stack.push(event);
                    if ch == ' ' {
                        self.undo_stack.commit();
                    }
                }
            }
            Event::Deletion(pos, _) => {
                self.dirty = true;
                self.show_welcome = false;
                self.recalculate_graphemes();
                self.goto(pos, term);
                if reversed {
                    self.move_cursor(Key::Left, term, config.general.wrap_cursor);
                } else {
                    self.undo_stack.push(event);
                }
                self.rows[pos.y].delete(self.graphemes.saturating_sub(1));
            }
            Event::InsertLineAbove(pos) => {
                self.dirty = true;
                self.rows.insert(pos.y, Row::from(""));
                self.goto(pos, term);
                self.move_cursor(Key::Down, term, config.general.wrap_cursor);
                if !reversed {
                    self.undo_stack.push(event);
                    self.undo_stack.commit();
                }
            }
            Event::InsertLineBelow(pos) => {
                self.dirty = true;
                self.rows.insert(pos.y.saturating_add(1), Row::from(""));
                self.goto(pos, term);
                if !reversed {
                    self.undo_stack.push(event);
                    self.undo_stack.commit();
                }
            }
            Event::SpliceUp(pos, other) => self.splice_up(&pos, reversed, term, &other),
            Event::SplitDown(pos, other) => self.split_down(&pos, reversed, term, &other),
            Event::InsertTab(pos) => {
                self.dirty = true;
                self.goto(pos, term);
                self.tab(&pos, &config, term);
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::DeleteTab(pos) => {
                self.dirty = true;
                self.goto(pos, term);
                for _ in 0..config.general.tab_width {
                    self.rows[pos.y].delete(pos.x);
                }
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::DeleteWord(pos, _) => self.delete_word(&pos, term),
            _ => (),
        }
        self.recalculate_graphemes();
    }
    pub fn word_left(&mut self, term: &Size) {
        self.move_cursor(Key::Left, term, false);
        let row = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        while self.cursor.x + self.offset.x != 0
            && row.chars()[self.graphemes.saturating_sub(1)] != " "
        {
            self.move_cursor(Key::Left, term, false);
        }
    }
    pub fn word_right(&mut self, term: &Size) {
        let row = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        while self.cursor.x + self.offset.x != row.length() && row.chars()[self.graphemes] != " " {
            self.move_cursor(Key::Right, term, false);
        }
        self.move_cursor(Key::Right, term, false);
    }
    pub fn find_word_boundary_left(&self, pos: &Position) -> Option<Position> {
        self.find_prev(" ", pos)
    }
    pub fn find_word_boundary_right(&self, pos: &Position) -> Option<Position> {
        self.find_next(" ", pos)
    }
    pub fn delete_word(&mut self, pos: &Position, term: &Size) {
        let mut right = if let Some(&" ") = self.rows[pos.y].ext_chars().get(pos.x) {
            *pos
        } else {
            self.find_word_boundary_right(pos)
                .unwrap_or(Position { x: 0, y: pos.y })
        };
        let mut left = self.find_word_boundary_left(pos).unwrap_or(Position {
            x: self.rows[pos.y].length(),
            y: pos.y,
        });
        if right.y != pos.y {
            right = Position {
                x: self.rows[pos.y].length(),
                y: pos.y,
            };
        }
        if left.y != pos.y {
            left = Position { x: 0, y: pos.y };
        }
        self.goto(left, term);
        for _ in left.x..right.x {
            self.rows[pos.y].delete(left.x);
        }
    }
    pub fn goto(&mut self, mut pos: Position, term: &Size) {
        // Move the cursor to a specific location
        let on_y = pos.y >= self.offset.y
            && pos.y <= self.offset.y.saturating_add(term.height.saturating_sub(4));
        let on_x = pos.x >= self.offset.x
            && pos.x
                <= self
                    .offset
                    .x
                    .saturating_add(term.width.saturating_sub(self.line_offset));
        // Verify that the goto is necessary
        if on_y && on_x {
            // No need to adjust offset
            self.cursor.y = pos.y - self.offset.y + OFFSET;
            self.cursor.x = pos.x - self.offset.x;
            return;
        }
        // Move to that position
        let max_y = term.height.saturating_sub(3);
        let max_x = (term.width).saturating_sub(self.line_offset);
        let halfway_y = max_y / 2;
        let halfway_x = max_x / 2;
        pos.y = pos.y.saturating_add(1);
        if self.offset.x == 0 && pos.y < max_y && pos.x < max_x {
            // Cursor is on the screen
            self.offset = Position { x: 0, y: 0 };
            self.cursor = pos;
        } else {
            // Cursor is off the screen, move to the Y position
            self.offset.y = pos.y.saturating_sub(halfway_y);
            self.cursor.y = halfway_y;
            // Change the X
            if pos.x >= max_x {
                // Move to the center
                self.offset.x = pos.x.saturating_sub(halfway_x);
                self.cursor.x = halfway_x;
            } else {
                // No offset
                self.offset.x = 0;
                self.cursor.x = pos.x;
            }
            if self.offset.y + self.cursor.y != pos.y {
                // Fix cursor misplacement
                self.offset.y = 0;
                self.cursor.y = pos.y;
            }
        }
    }
    pub fn save(&self, path: &str, tab: usize) -> std::io::Result<()> {
        // Save a file
        let contents = self.render(self.tabs, tab);
        log!("Saved file", format!("File tab status is {:?}", self.tabs));
        fs::write(path, contents)
    }
    pub fn find_prev(&self, needle: &str, current: &Position) -> Option<Position> {
        // Find all the points where "needle" occurs before the current position
        let re = Regex::new(needle).ok()?;
        for (c, r) in self
            .rows
            .iter()
            .take(current.y.saturating_add(1))
            .map(|x| x.string.as_str())
            .enumerate()
            .rev()
        {
            let mut xs = vec![];
            for i in re.captures_iter(r) {
                for j in 0..i.len() {
                    let j = i.get(j).unwrap();
                    xs.push(j.start());
                }
            }
            while let Some(i) = xs.pop() {
                if i < current.x || c != current.y {
                    return Some(Position { x: i, y: c });
                }
            }
        }
        None
    }
    pub fn find_next(&self, needle: &str, current: &Position) -> Option<Position> {
        // Find all the points where "needle" occurs after the current position
        let re = Regex::new(needle).ok()?;
        for (c, r) in self
            .rows
            .iter()
            .skip(current.y)
            .map(|x| x.string.as_str())
            .enumerate()
        {
            for i in re.captures_iter(r) {
                for cap in 0..i.len() {
                    let cap = i.get(cap).unwrap();
                    if c != 0 || cap.start() > current.x {
                        return Some(Position {
                            x: cap.start(),
                            y: current.y + c,
                        });
                    }
                }
            }
        }
        None
    }
    pub fn find_all(&self, needle: &str) -> Option<Vec<Position>> {
        // Find all the places where the needle is
        let mut result = vec![];
        let re = Regex::new(needle).ok()?;
        for (c, r) in self.rows.iter().map(|x| x.string.to_string()).enumerate() {
            for i in re.captures_iter(&r) {
                for cap in 0..i.len() {
                    let cap = i.get(cap).unwrap();
                    result.push(Position {
                        x: cap.start(),
                        y: c,
                    });
                }
            }
        }
        Some(result)
    }
    pub fn render(&self, tab_type: TabType, tab_width: usize) -> String {
        // Render the lines of a document for writing
        let render = self
            .rows
            .iter()
            .map(|x| x.string.clone())
            .collect::<Vec<String>>()
            .join("\n")
            + "\n";
        if let TabType::Tabs = tab_type {
            spaces_to_tabs(&render, tab_width)
        } else {
            render
        }
    }
    pub fn identify(path: &str) -> (&str, &str) {
        // Identify which type of file the current buffer is
        match path.split('.').last() {
            Some(ext) => match ext {
                "asm" => ("Assembly ", "\u{f471} "),
                "b" => ("B", "\u{e7a3} "),
                "bf" => ("Brainfuck", "\u{e28c} "),
                "bas" => ("Basic", "\u{e7a3} "),
                "bat" => ("Batch file", "\u{e795} "),
                "bash" => ("Bash", "\u{e795} "),
                "c" => ("C", "\u{e61e} "),
                "cr" => ("Crystal", "\u{e7a3} "),
                "cs" => ("C#", "\u{f81a} "),
                "cpp" => ("C++", "\u{e61d} "),
                "css" => ("CSS", "\u{e749} "),
                "csv" => ("CSV", "\u{f1c0} "),
                "class" | "java" => ("Java", "\u{e738} "),
                "d" => ("D", "\u{e7af} "),
                "db" => ("Database", "\u{f1c0} "),
                "erb" => ("ERB", "\u{e739} "),
                "fish" => ("Fish shell", "\u{f739} "),
                "go" => ("Go", "\u{e724} "),
                "gds" => ("Godot Script", "\u{fba7} "),
                "gitignore" => ("Gitignore", "\u{e702} "),
                "hs" => ("Haskell", "\u{e777} "),
                "html" => ("HTML", "\u{e736} "),
                "js" => ("JavaScript", "\u{e74e} "),
                "json" => ("JSON", "\u{e60b} "),
                "lua" => ("LUA", "\u{e620} "),
                "log" => ("Log file", "\u{f15c} "),
                "md" => ("Markdown", "\u{e73e} "),
                "nim" => ("Nim", "\u{e26e} "),
                "py" | "pyc" | "pyw" => ("Python", "\u{e73c} "),
                "php" => ("PHP", "\u{f81e} "),
                "r" => ("R", "\u{f1c0} "),
                "rs" => ("Rust", "\u{e7a8} "),
                "rb" => ("Ruby", "\u{e739} "),
                "sh" => ("Shell", "\u{e795} "),
                "sql" => ("SQL", "\u{f1c0} "),
                "swift" => ("Swift", "\u{e755} "),
                "sqlite" => ("SQLite", "\u{f1c0} "),
                "txt" => ("Plain Text", "\u{f15c} "),
                "toml" => ("Toml", "\u{f669} "),
                "xml" => ("XML", "\u{f72d} "),
                "vb" => ("VB Script", "\u{4eae}"),
                "vim" => ("VimScript", "\u{e7c5} "),
                "yml" | "yaml" => ("YAML", "\u{e7a3} "),
                "zsh" => ("Z Shell", "\u{e795} "),
                _ => ("Unknown", "\u{f128}"),
            },
            None => ("Unknown", "\u{f128}"),
        }
    }
}

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

// Editor.rs - Controls the editor and brings everything together
use crate::config::{KeyBinding, RawKey, Reader, Status, Theme};
use crate::document::{TabType, Type};
use crate::highlight::Token;
use crate::oxa::interpret_line;
use crate::undo::{reverse, BankType};
use crate::util::{title, trim_end, Exp};
use crate::{log, Document, Event, Row, Size, Terminal, VERSION};
use clap::App;
use crossterm::event::{Event as InputEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Attribute, Color, SetBackgroundColor, SetForegroundColor};
use crossterm::ErrorKind;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Error, ErrorKind as Iek, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

// Set up color resets
pub const RESET_BG: SetBackgroundColor = SetBackgroundColor(Color::Reset);
pub const RESET_FG: SetForegroundColor = SetForegroundColor(Color::Reset);

// Set up offset rules
pub const OFFSET: usize = 1;

// Macro for running shell commands within the editor
macro_rules! shell {
    ($command:expr, $confirm:expr, $root:expr) => {
        // Execute a shell command
        let command = if $root {
            Command::new("sudo")
                .arg("bash")
                .arg("-c")
                .arg($command)
                .stdout(Stdio::piped())
                .spawn()
        } else {
            Command::new("bash")
                .arg("-c")
                .arg($command)
                .stdout(Stdio::piped())
                .spawn()
        };
        if let Ok(s) = command {
            log!("Shell", "Command requested");
            if let Ok(s) = s
                .stdout
                .ok_or_else(|| Error::new(Iek::Other, "Could not capture standard output."))
            {
                // Go back into canonical mode to restore normal operation
                Terminal::exit();
                log!("Shell", "Ready to go");
                // Stream the input and output of the command to the current stdout
                BufReader::new(s)
                    .lines()
                    .filter_map(std::result::Result::ok)
                    .for_each(|line| println!("{}", line));
                // Wait for user to press enter, then reenter raw mode
                log!("Shell", "Exited");
                if $confirm {
                    println!("Shell command exited. Press [Return] to continue");
                    let mut output = String::new();
                    let _ = std::io::stdin().read_line(&mut output);
                }
                Terminal::enter();
            } else {
                log!("Failure to open standard output", "");
            }
        } else {
            log!(
                "Failure to run command",
                format!(
                    "{} {:?}",
                    $command,
                    Command::new($command).stdout(Stdio::piped()).spawn()
                )
            );
        }
    };
}

// Enum for holding prompt events
enum PromptEvent {
    Update,
    CharPress(bool),
    KeyPress(KeyCode),
}

// For representing positions
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

// Enum for direction
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// The main editor struct
pub struct Editor {
    pub config: Reader,                      // Storage for configuration
    pub status: Status,                      // Holding the status of the config
    config_path: String,                     // Holds the file path of the config file
    quit: bool,                              // Toggle for cleanly quitting the editor
    term: Terminal,                          // For the handling of the terminal
    doc: Vec<Document>,                      // For holding our document
    tab: usize,                              // Holds the number of the current tab
    last_keypress: Option<Instant>,          // For holding the time of the last input event
    keypress: KeyBinding,                    // For holding the last keypress event
    exp: Exp,                                // For holding expressions
    position_bank: HashMap<usize, Position>, // Bank for cursor positions
    row_bank: HashMap<usize, Row>,           // Bank for lines
    theme: String,                           // Currently used theme
}

// Implementing methods for our editor struct / class
impl Editor {
    pub fn new(args: App) -> Result<Self, ErrorKind> {
        // Create a new editor instance
        let args = args.get_matches();
        // Set up terminal
        let term = Terminal::new()?;
        // Set up the arguments
        let files: Vec<&str> = args.values_of("files").unwrap_or_default().collect();
        let config_path = args.value_of("config").unwrap_or_default();
        let mut config = Reader::read(config_path);
        // Check for fallback colours
        if config.0.theme.fallback {
            let max = Terminal::availablility();
            log!("Available Colours", max);
            if max != 24 {
                // Fallback to 16 bit colours
                config.0.highlights.insert(
                    "16fallback".to_string(),
                    [
                        ("comments".to_string(), (128, 128, 128)),
                        ("keywords".to_string(), (0, 0, 255)),
                        ("namespaces".to_string(), (0, 0, 255)),
                        ("references".to_string(), (0, 0, 128)),
                        ("strings".to_string(), (0, 128, 0)),
                        ("characters".to_string(), (0, 128, 128)),
                        ("digits".to_string(), (0, 128, 128)),
                        ("booleans".to_string(), (0, 255, 0)),
                        ("functions".to_string(), (0, 128, 128)),
                        ("structs".to_string(), (0, 128, 128)),
                        ("macros".to_string(), (128, 0, 128)),
                        ("attributes".to_string(), (0, 128, 128)),
                        ("headers".to_string(), (0, 128, 128)),
                        ("symbols".to_string(), (128, 128, 0)),
                        ("global".to_string(), (0, 255, 0)),
                        ("operators".to_string(), (0, 128, 128)),
                        ("regex".to_string(), (0, 255, 0)),
                        ("search_inactive".to_string(), (128, 128, 128)),
                        ("search_active".to_string(), (0, 128, 128)),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                );
                config.0.theme = Theme {
                    transparent_editor: false,
                    editor_bg: (0, 0, 0),
                    editor_fg: (255, 255, 255),
                    status_bg: (128, 128, 128),
                    status_fg: (255, 255, 255),
                    line_number_fg: (255, 255, 255),
                    line_number_bg: (0, 0, 0),
                    active_tab_fg: (255, 255, 255),
                    inactive_tab_fg: (255, 255, 255),
                    active_tab_bg: (128, 128, 128),
                    inactive_tab_bg: (0, 0, 0),
                    warning_fg: (255, 255, 0),
                    error_fg: (255, 0, 0),
                    info_fg: (255, 255, 255),
                    default_theme: "16fallback".to_string(),
                    fallback: true,
                };
            }
        }
        // Read in documents
        let mut documents = vec![];
        if files.is_empty() {
            documents.push(Document::new(
                &config.0,
                &config.1,
                args.is_present("readonly"),
            ));
        } else {
            for file in &files {
                documents.push(Document::from(
                    &config.0,
                    &config.1,
                    file,
                    args.is_present("readonly"),
                ));
            }
        }
        // Calculate neater paths
        for d in &mut documents {
            d.correct_path(&term.size);
        }
        // Create the new editor instance
        Ok(Self {
            quit: false,
            // Display information about the config file into text for the status line
            term,
            tab: 0,
            doc: documents,
            last_keypress: None,
            keypress: KeyBinding::Unsupported,
            config: config.0.clone(),
            config_path: config_path.to_string(),
            status: config.1,
            exp: Exp::new(),
            position_bank: HashMap::new(),
            row_bank: HashMap::new(),
            theme: config.0.theme.default_theme,
        })
    }
    pub fn run(&mut self) {
        // Run the editor instance
        log!("Ox opened", "Ox was opened successfully");
        while !self.quit {
            self.update();
            self.process_input();
        }
        // Leave alternative screen and disable raw mode
        Terminal::exit();
    }
    fn read_event(&mut self) -> InputEvent {
        // Wait until a key, mouse or terminal resize event
        loop {
            if let Ok(true) = crossterm::event::poll(Duration::from_millis(16)) {
                if let Ok(key) = crossterm::event::read() {
                    // When a keypress was detected
                    self.last_keypress = Some(Instant::now());
                    return key;
                }
            } else {
                // Check for a period of inactivity
                if let Some(time) = self.last_keypress {
                    // Check to see if it's over the config undo period
                    if time.elapsed().as_secs() >= self.config.general.undo_period {
                        // Commit the undo changes to the stack
                        self.doc[self.tab].undo_stack.commit();
                        self.last_keypress = None;
                    }
                }
            }
        }
    }
    fn key_event_to_ox_key(key: KeyCode, modifiers: KeyModifiers) -> KeyBinding {
        // Convert crossterm's complicated key structure into Ox's simpler one
        let inner = match key {
            KeyCode::Char(c) => RawKey::Char(c),
            KeyCode::BackTab => RawKey::BackTab,
            KeyCode::Insert => RawKey::Insert,
            KeyCode::Esc => RawKey::Esc,
            KeyCode::Backspace => RawKey::Backspace,
            KeyCode::Tab => RawKey::Tab,
            KeyCode::Enter => RawKey::Enter,
            KeyCode::Delete => RawKey::Delete,
            KeyCode::Null => RawKey::Null,
            KeyCode::PageUp => RawKey::PageUp,
            KeyCode::PageDown => RawKey::PageDown,
            KeyCode::Home => RawKey::Home,
            KeyCode::End => RawKey::End,
            KeyCode::Up => RawKey::Up,
            KeyCode::Down => RawKey::Down,
            KeyCode::Left => RawKey::Left,
            KeyCode::Right => RawKey::Right,
            KeyCode::F(i) => return KeyBinding::F(i),
        };
        match modifiers {
            KeyModifiers::CONTROL => KeyBinding::Ctrl(inner),
            KeyModifiers::ALT => KeyBinding::Alt(inner),
            KeyModifiers::SHIFT => KeyBinding::Shift(inner),
            KeyModifiers::NONE => KeyBinding::Raw(inner),
            _ => KeyBinding::Unsupported,
        }
    }
    fn process_key(&mut self, key: KeyEvent) {
        self.doc[self.tab].show_welcome = false;
        let cursor = self.doc[self.tab].cursor;
        let offset = self.doc[self.tab].offset;
        let current = Position {
            x: cursor.x + offset.x,
            y: cursor.y + offset.y - OFFSET,
        };
        let ox_key = Editor::key_event_to_ox_key(key.code, key.modifiers);
        self.keypress = ox_key;
        match ox_key {
            KeyBinding::Raw(RawKey::Enter) => {
                self.doc[self.tab].redo_stack.empty();
                if current.x == 0 {
                    // Return key pressed at the start of the line
                    self.execute(Event::InsertLineAbove(current), false);
                } else if current.x == self.doc[self.tab].rows[current.y].length() {
                    // Return key pressed at the end of the line
                    self.execute(Event::InsertLineBelow(current), false);
                    self.execute(Event::MoveCursor(1, Direction::Down), false);
                } else {
                    // Return key pressed in the middle of the line
                    self.execute(Event::SplitDown(current, current), false);
                }
            }
            KeyBinding::Raw(RawKey::Tab) => {
                self.doc[self.tab].redo_stack.empty();
                self.execute(Event::InsertTab(current), false);
            }
            KeyBinding::Raw(RawKey::Backspace) => {
                self.doc[self.tab].redo_stack.empty();
                self.execute(
                    if current.x == 0 && current.y != 0 {
                        // Backspace at the start of a line
                        Event::SpliceUp(current, current)
                    } else if current.x == 0 {
                        return;
                    } else {
                        // Backspace in the middle of a line
                        let row = self.doc[self.tab].rows[current.y].clone();
                        let chr = row
                            .ext_chars()
                            .get(current.x.saturating_add(1))
                            .map_or(" ", |chr| *chr);
                        let current = Position {
                            x: current.x.saturating_sub(UnicodeWidthStr::width(chr)),
                            y: current.y,
                        };
                        Event::Deletion(current, chr.parse().unwrap_or(' '))
                    },
                    false,
                );
            }
            // Detect control and alt and function key bindings
            KeyBinding::Ctrl(_) | KeyBinding::Alt(_) | KeyBinding::F(_) => {
                if let Some(commands) = self.config.keys.get(&ox_key) {
                    for i in commands.clone() {
                        self.text_to_event(&i);
                    }
                }
            }
            KeyBinding::Raw(RawKey::Char(c)) | KeyBinding::Shift(RawKey::Char(c)) => {
                self.doc[self.tab].redo_stack.empty();
                self.execute(Event::Insertion(current, c), false);
            }
            KeyBinding::Raw(RawKey::Up) => self.execute(Event::MoveCursor(1, Direction::Up), false),
            KeyBinding::Raw(RawKey::Down) => {
                self.execute(Event::MoveCursor(1, Direction::Down), false)
            }
            KeyBinding::Raw(RawKey::Left) => {
                self.execute(Event::MoveCursor(1, Direction::Left), false)
            }
            KeyBinding::Raw(RawKey::Right) => {
                self.execute(Event::MoveCursor(1, Direction::Right), false)
            }
            KeyBinding::Raw(RawKey::PageDown) => self.execute(Event::PageDown, false),
            KeyBinding::Raw(RawKey::PageUp) => self.execute(Event::PageUp, false),
            KeyBinding::Raw(RawKey::Home) => self.execute(Event::Home, false),
            KeyBinding::Raw(RawKey::End) => self.execute(Event::End, false),
            _ => (),
        }
    }
    fn process_input(&mut self) {
        // Read a key and act on it
        match self.read_event() {
            InputEvent::Key(key) => self.process_key(key),
            InputEvent::Resize(width, height) => {
                // Terminal resize event
                self.term.size = Size {
                    width: width as usize,
                    height: height as usize,
                };
                // Move cursor if needed
                let size = self.term.size.height.saturating_sub(3);
                if self.doc[self.tab].cursor.y > size && size != 0 {
                    // Prevent cursor going off the screen and breaking everything
                    self.doc[self.tab].cursor.y = size;
                }
                // Re-render everything to the new size
                self.update();
            }
            InputEvent::Mouse(_) => (),
        }
    }
    fn new_document(&mut self) {
        // Create a new document
        self.doc
            .push(Document::new(&self.config, &self.status, false));
        self.tab = self.doc.len().saturating_sub(1);
    }
    fn open_document(&mut self, file: Option<String>) {
        // Open a document
        let to_open = if let Some(path) = file {
            // File was specified
            path
        } else if let Some(path) = self.prompt("Open", ": ", &|_, _, _| {}) {
            // Ask for a file and open it
            path
        } else {
            // User cancelled
            return;
        };
        if let Some(doc) = Document::open(&self.config, &self.status, &to_open, false) {
            // Overwrite the current document
            self.doc.push(doc);
            self.tab = self.doc.len().saturating_sub(1);
        } else {
            self.doc[self.tab].set_command_line("File couldn't be opened".to_string(), Type::Error);
        }
    }
    fn save_document(&mut self, file: Option<String>, prompt: bool) {
        // Save the document
        let save = if let Some(file) = file {
            // File was specified
            file
        } else {
            // File not specified
            if prompt {
                // Save as
                if let Some(path) = self.prompt("Save as", ": ", &|_, _, _| {}) {
                    path
                } else {
                    // User cancelled
                    return;
                }
            } else {
                // Use current document
                self.doc[self.tab].path.clone()
            }
        };
        if self.doc[self.tab].path != save && Path::new(&save).exists() {
            // File already exists, possible loss of data
            self.doc[self.tab]
                .set_command_line(format!("File {} already exists", save), Type::Error);
            return;
        }
        // Attempt document save
        let tab_width = self.config.general.tab_width;
        if self.doc[self.tab].save(&save, tab_width).is_ok() {
            // The document saved successfully
            let ext = save.split('.').last().unwrap_or(&"");
            self.doc[self.tab].dirty = false;
            self.doc[self.tab].set_command_line(
                format!("File saved to \"{}\" successfully", save),
                Type::Info,
            );
            // Update the current documents details in case of filetype change
            self.doc[self.tab].last_save_index = self.doc[self.tab].undo_stack.len();
            self.doc[self.tab].kind = Document::identify(&save).0.to_string();
            self.doc[self.tab].icon = Document::identify(&save).1.to_string();
            self.doc[self.tab].name = Path::new(&save)
                .file_name()
                .unwrap_or_else(|| OsStr::new(&save))
                .to_str()
                .unwrap_or(&save)
                .to_string();
            self.doc[self.tab].path = save.clone();
            self.doc[self.tab].regex = Reader::get_syntax_regex(&self.config, ext);
        } else if save.is_empty() {
            // The document couldn't save due to an empty name
            self.doc[self.tab].set_command_line(
                "Filename is blank, please specify file name".to_string(),
                Type::Error,
            );
        } else {
            // The document couldn't save due to permission errors / invalid name
            self.doc[self.tab]
                .set_command_line(format!("Failed to save file to \"{}\"", save), Type::Error);
        }
        // Commit to undo stack on document save
        self.execute(Event::Commit, false);
    }
    fn save_every_document(&mut self) {
        // Save every document in the editor
        let tab_width = self.config.general.tab_width;
        let mut successes = 0;
        let mut failiures = 0;
        for i in 0..self.doc.len() {
            let path = self.doc[i].path.clone();
            if self.doc[i].save(&path, tab_width).is_ok() {
                // The document saved successfully
                self.doc[i].dirty = false;
                successes += 1;
            } else {
                // The document couldn't save due to permission errors
                failiures += 1;
            }
            self.doc[i].set_command_line(
                format!("Saved {} documents, {} failed", successes, failiures),
                Type::Info,
            );
            // Commit to undo stack on document save
            self.execute(Event::Commit, false);
        }
    }
    fn quit_document(&mut self, force: bool) {
        // For handling a quit event
        if let KeyBinding::Ctrl(_) | KeyBinding::Alt(_) = self.keypress {
            if force || self.dirty_prompt(self.keypress, "quit") {
                if self.doc.len() <= 1 {
                    // Quit Ox
                    self.quit = true;
                    return;
                } else if self.tab == self.doc.len().saturating_sub(1) {
                    // Close current tab and move right
                    self.doc.remove(self.tab);
                    self.tab -= 1;
                } else {
                    // Close current tab and move left
                    self.doc.remove(self.tab);
                }
                self.doc[self.tab].set_command_line("Closed tab".to_string(), Type::Info);
            }
        }
    }
    fn quit_all(&mut self, force: bool) {
        // Quit all the documents in the editor
        self.tab = 0;
        while !self.quit {
            self.execute(Event::Quit(force), false);
        }
    }
    fn next_tab(&mut self) {
        // Move to the next tab
        if self.tab.saturating_add(1) < self.doc.len() {
            self.tab = self.tab.saturating_add(1);
        }
    }
    fn prev_tab(&mut self) {
        // Move to the previous tab
        self.tab = self.tab.saturating_sub(1);
    }
    pub fn shell(&mut self, mut command: String, substitution: bool, root: bool, confirm: bool) {
        if substitution {
            let file =
                self.doc[self.tab].render(self.doc[self.tab].tabs, self.config.general.tab_width);
            command = command.replacen("%F", &self.doc[self.tab].path, 1);
            command = command.replacen("%C", &file, 1);
        }
        shell!(&command, confirm, root);
    }
    pub fn execute(&mut self, event: Event, reversed: bool) {
        // Event executor
        if self.doc[self.tab].read_only && Editor::will_edit(&event) {
            return;
        }
        match event {
            Event::New => self.new_document(),
            Event::Open(file) => self.open_document(file),
            Event::Save(file, prompt) => self.save_document(file, prompt),
            Event::SaveAll => self.save_every_document(),
            Event::Quit(force) => self.quit_document(force),
            Event::QuitAll(force) => self.quit_all(force),
            Event::NextTab => self.next_tab(),
            Event::PrevTab => self.prev_tab(),
            Event::Search => self.search(),
            Event::Replace => self.replace(),
            Event::ReplaceAll => self.replace_all(),
            Event::Cmd => self.cmd(),
            Event::Shell(command, confirm, substitution, root) => {
                self.shell(command, confirm, substitution, root)
            }
            Event::ReloadConfig => {
                let config = Reader::read(&self.config_path);
                self.config = config.0;
                self.doc[self.tab].cmd_line = Document::config_to_commandline(&config.1);
            }
            Event::Theme(name) => {
                self.theme = name;
                self.doc[self.tab].mass_redraw();
                self.update();
            }
            Event::MoveWord(direction) => match direction {
                Direction::Left => self.doc[self.tab].word_left(&self.term.size),
                Direction::Right => self.doc[self.tab].word_right(&self.term.size),
                _ => {},
            },
            Event::GotoCursor(pos) => {
                let rows = &self.doc[self.tab].rows;
                if rows.len() > pos.y && rows[pos.y].length() >= pos.x {
                    self.doc[self.tab].goto(pos, &self.term.size);
                }
            }
            Event::MoveCursor(magnitude, direction) => {
                for _ in 0..magnitude {
                    self.doc[self.tab].move_cursor(
                        match direction {
                            Direction::Up => KeyCode::Up,
                            Direction::Down => KeyCode::Down,
                            Direction::Left => KeyCode::Left,
                            Direction::Right => KeyCode::Right,
                        },
                        &self.term.size,
                        self.config.general.wrap_cursor,
                    );
                }
            }
            Event::Commit => self.doc[self.tab].undo_stack.commit(),
            Event::Store(kind, bank) => {
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let current = Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                };
                match kind {
                    BankType::Cursor => {
                        self.position_bank.insert(bank, current);
                    }
                    BankType::Line => {
                        self.row_bank
                            .insert(bank, self.doc[self.tab].rows[current.y].clone());
                    }
                }
            }
            Event::Load(kind, bank) => {
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let current = Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                };
                match kind {
                    BankType::Cursor => {
                        let cursor = *self.position_bank.get(&bank).unwrap_or(&current);
                        self.doc[self.tab].goto(cursor, &self.term.size);
                    }
                    BankType::Line => {
                        if let Some(row) = self.row_bank.get(&bank) {
                            self.doc[self.tab].rows[current.y] = row.clone();
                        }
                    }
                }
            }
            Event::Home => self.doc[self.tab].leap_cursor(KeyCode::Home, &self.term.size),
            Event::End => self.doc[self.tab].leap_cursor(KeyCode::End, &self.term.size),
            Event::PageUp => self.doc[self.tab].leap_cursor(KeyCode::PageUp, &self.term.size),
            Event::PageDown => self.doc[self.tab].leap_cursor(KeyCode::PageDown, &self.term.size),
            Event::Undo => self.undo(),
            Event::Redo => self.redo(),
            // Event is a document event, send to current document
            _ => self.doc[self.tab].execute(event, reversed, &self.term.size, &self.config),
        }
        self.doc[self.tab].recalculate_graphemes();
    }
    fn cmd(&mut self) {
        // Recieve macro command
        if let Some(command) = self.prompt(":", "", &|s, e, _| {
            if let PromptEvent::KeyPress(KeyCode::Esc) = e {
                s.doc[s.tab].set_command_line("".to_string(), Type::Info);
            }
        }) {
            // Parse and Lex instruction
            for command in command.split('|') {
                self.text_to_event(command);
            }
        }
    }
    fn execute_macro(&mut self, command: &str) {
        // Work out number of times to execute it
        let mut command = command.to_string();
        let times = if let Ok(times) = command.split(' ').next().unwrap_or("").parse::<usize>() {
            command = command.split(' ').skip(1).collect::<Vec<_>>().join(" ");
            times
        } else {
            1
        };
        // Build and execute the macro
        for _ in 0..times {
            for i in self.config.macros[&command].clone() {
                self.text_to_event(&i);
            }
        }
    }
    fn text_to_event(&mut self, command: &str) {
        let command = command.trim_start_matches(' ');
        let mut cmd = command.split(' ');
        let actual_command;
        let times = if let Ok(repeat) = cmd.next().unwrap_or_default().parse::<usize>() {
            actual_command = cmd.collect::<Vec<_>>().join(" ");
            repeat
        } else {
            actual_command = command.to_string();
            1
        };
        for _ in 0..times {
            if self.config.macros.contains_key(&actual_command) {
                self.execute_macro(&actual_command);
            } else {
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let instruction = interpret_line(
                    &actual_command,
                    &Position {
                        x: cursor.x + offset.x,
                        y: cursor.y + offset.y - OFFSET,
                    },
                    self.doc[self.tab].graphemes,
                    &self.doc[self.tab].rows,
                );
                // Execute the instruction
                if let Some(instruct) = instruction {
                    for i in instruct {
                        if Editor::will_edit(&i) {
                            self.doc[self.tab].redo_stack.empty();
                        }
                        self.execute(i, false);
                    }
                    self.doc[self.tab].undo_stack.commit();
                }
            }
        }
    }
    pub fn will_edit(event: &Event) -> bool {
        matches!(event, Event::SpliceUp(_, _)
            | Event::SplitDown(_, _)
            | Event::InsertLineAbove(_)
            | Event::InsertLineBelow(_)
            | Event::Deletion(_, _)
            | Event::Insertion(_, _)
            | Event::InsertTab(_)
            | Event::DeleteTab(_)
            | Event::DeleteLine(_, _, _)
            | Event::UpdateLine(_, _, _, _)
            | Event::ReplaceAll
            | Event::Replace
            | Event::Overwrite(_, _))
    }
    pub fn undo(&mut self) {
        self.doc[self.tab].undo_stack.commit();
        if let Some(events) = self.doc[self.tab].undo_stack.pop() {
            for event in events.clone() {
                if let Some(reversed) = reverse(event, self.doc[self.tab].rows.len()) {
                    for i in reversed {
                        self.execute(i, true);
                    }
                    self.update();
                }
            }
            self.doc[self.tab]
                .redo_stack
                .append(events.into_iter().rev().collect());
        } else {
            self.doc[self.tab].set_command_line("Empty Undo Stack".to_string(), Type::Error);
        }
        if self.doc[self.tab].undo_stack.len() == self.doc[self.tab].last_save_index {
            self.doc[self.tab].dirty = false;
        }
    }
    pub fn redo(&mut self) {
        if let Some(events) = self.doc[self.tab].redo_stack.pop() {
            for event in events {
                self.execute(event, false);
                self.update();
            }
        } else {
            self.doc[self.tab].set_command_line("Empty Redo Stack".to_string(), Type::Error);
        }
    }
    fn refresh_view(&mut self) {
        let offset = self.doc[self.tab].offset.y;
        for o in self.doc[self.tab]
            .rows
            .iter_mut()
            .skip(offset)
            .take(self.term.size.width)
        {
            o.updated = true;
        }
    }
    fn highlight_bg_tokens(&mut self, t: &str, current: Position) -> Option<()> {
        let occurances = self.doc[self.tab].find_all(t)?;
        for i in &mut self.doc[self.tab].rows {
            i.bg_syntax.clear();
        }
        if !t.is_empty() {
            for o in occurances {
                self.doc[self.tab].rows[o.y].bg_syntax.insert(
                    o.x,
                    Token {
                        span: (o.x, o.x + UnicodeWidthStr::width(t)),
                        data: t.to_string(),
                        kind: Reader::rgb_bg(
                            self.config.highlights[&self.theme][if o == current {
                                "search_active"
                            } else {
                                "search_inactive"
                            }],
                        )
                        .to_string(),
                        priority: 10,
                    },
                );
            }
        }
        None
    }
    fn search(&mut self) {
        // For searching the file
        let initial_cursor = self.doc[self.tab].cursor;
        let initial_offset = self.doc[self.tab].offset;
        let initial = Position {
            x: initial_cursor.x + initial_offset.x,
            y: initial_cursor.y + initial_offset.y - OFFSET,
        };
        // Ask for a search term after saving the current cursor position
        self.prompt("Search", ": ", &|s, e, t| {
            // Find all occurances in the document
            let current = Position {
                x: s.doc[s.tab].cursor.x + s.doc[s.tab].offset.x,
                y: s.doc[s.tab].cursor.y + s.doc[s.tab].offset.y - OFFSET,
            };
            match e {
                PromptEvent::KeyPress(c) => match c {
                    KeyCode::Up | KeyCode::Left => {
                        if let Some(p) = s.doc[s.tab].find_prev(t, &current) {
                            s.doc[s.tab].goto(p, &s.term.size);
                            s.refresh_view();
                            s.highlight_bg_tokens(&t, p);
                        }
                    }
                    KeyCode::Down | KeyCode::Right => {
                        if let Some(p) = s.doc[s.tab].find_next(t, &current) {
                            s.doc[s.tab].goto(p, &s.term.size);
                            s.refresh_view();
                            s.highlight_bg_tokens(&t, p);
                        }
                    }
                    KeyCode::Esc => {
                        s.doc[s.tab].cursor = initial_cursor;
                        s.doc[s.tab].offset = initial_offset;
                    }
                    _ => (),
                },
                PromptEvent::CharPress(backspace) => {
                    // Highlight the tokens
                    if backspace {
                        s.highlight_bg_tokens(&t, initial);
                    }
                    if let Some(p) = s.doc[s.tab].find_next(t, &initial) {
                        s.doc[s.tab].goto(p, &s.term.size);
                        s.refresh_view();
                        s.highlight_bg_tokens(&t, p);
                    } else {
                        s.doc[s.tab].goto(initial, &s.term.size);
                        s.highlight_bg_tokens(&t, initial);
                    }
                }
                PromptEvent::Update => (),
            }
        });
        // User cancelled or found what they were looking for
        for i in &mut self.doc[self.tab].rows {
            i.bg_syntax.clear();
        }
        self.doc[self.tab].set_command_line("Search exited".to_string(), Type::Info);
    }
    fn replace(&mut self) {
        // Replace text within the document
        let initial_cursor = self.doc[self.tab].cursor;
        let initial_offset = self.doc[self.tab].offset;
        let current = Position {
            x: self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x,
            y: self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET,
        };
        // After saving the cursor position, ask the user for the information
        if let Some(target) = self.prompt("Replace", ": ", &|_, _, _| {}) {
            let re = if let Ok(re) = Regex::new(&target) {
                re
            } else {
                self.doc[self.tab].set_command_line("Invalid Regex".to_string(), Type::Error);
                return;
            };
            self.highlight_bg_tokens(&target, current);
            if let Some(arrow) = self.prompt("With", ": ", &|_, _, _| {}) {
                if let Some(p) = self.doc[self.tab].find_next(&target, &current) {
                    self.doc[self.tab].goto(p, &self.term.size);
                    self.highlight_bg_tokens(&target, p);
                    self.update();
                }
                loop {
                    // Read an event
                    let key = if let InputEvent::Key(key) = self.read_event() {
                        key
                    } else {
                        continue;
                    };
                    let current = Position {
                        x: self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x,
                        y: self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET,
                    };
                    match key.code {
                        KeyCode::Up | KeyCode::Left => {
                            if let Some(p) = self.doc[self.tab].find_prev(&target, &current) {
                                self.doc[self.tab].goto(p, &self.term.size);
                                self.highlight_bg_tokens(&target, p);
                            }
                        }
                        KeyCode::Down | KeyCode::Right => {
                            if let Some(p) = self.doc[self.tab].find_next(&target, &current) {
                                self.doc[self.tab].goto(p, &self.term.size);
                                self.highlight_bg_tokens(&target, p);
                            }
                        }
                        KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => {
                            self.doc[self.tab].undo_stack.commit();
                            let before = self.doc[self.tab].rows[current.y].clone();
                            let after = Row::from(&*re.replace_all(&before.string, &arrow[..]));
                            self.doc[self.tab].rows[current.y] = after.clone();
                            self.highlight_bg_tokens(&target, current);
                            if before.string != after.string {
                                self.doc[self.tab].undo_stack.push(Event::UpdateLine(
                                    current,
                                    0,
                                    Box::new(before),
                                    Box::new(after),
                                ));
                            }
                        }
                        KeyCode::Esc => {
                            self.doc[self.tab].cursor = initial_cursor;
                            self.doc[self.tab].offset = initial_offset;
                            self.doc[self.tab]
                                .set_command_line("Replace finished".to_string(), Type::Info);
                            break;
                        }
                        _ => (),
                    }
                    self.update();
                }
            }
            for i in &mut self.doc[self.tab].rows {
                i.bg_syntax.clear();
            }
        }
    }
    fn replace_all(&mut self) {
        // Replace all occurances of a substring
        if let Some(target) = self.prompt("Replace all", ": ", &|_, _, _| {}) {
            let re = if let Ok(re) = Regex::new(&target) {
                re
            } else {
                return;
            };
            if let Some(arrow) = self.prompt("With", ": ", &|_, _, _| {}) {
                // Find all occurances
                let search_points = if let Some(t) = self.doc[self.tab].find_all(&target) {
                    t
                } else {
                    vec![]
                };
                for p in search_points {
                    let before = self.doc[self.tab].rows[p.y].clone();
                    let after = Row::from(&*re.replace_all(&before.string, &arrow[..]));
                    self.doc[self.tab].rows[p.y] = after.clone();
                    if before.string != after.string {
                        self.doc[self.tab].undo_stack.push(Event::UpdateLine(
                            Position { x: 0, y: p.y },
                            0,
                            Box::new(before),
                            Box::new(after),
                        ));
                    }
                }
            }
        }
        // Exit message
        self.doc[self.tab].set_command_line("Replace finished".to_string(), Type::Info);
    }
    fn dirty_prompt(&mut self, key: KeyBinding, subject: &str) -> bool {
        // For events that require changes to the document
        if self.doc[self.tab].dirty {
            // Handle unsaved changes
            self.doc[self.tab].set_command_line(
                format!("Unsaved Changes! {:?} to force {}", key, subject),
                Type::Warning,
            );
            self.update();
            if let InputEvent::Key(KeyEvent {
                code: c,
                modifiers: m,
            }) = self.read_event()
            {
                let ox_key = Editor::key_event_to_ox_key(c, m);
                match ox_key {
                    KeyBinding::Raw(RawKey::Enter) => return true,
                    KeyBinding::Ctrl(_) | KeyBinding::Alt(_) => {
                        if ox_key == key {
                            return true;
                        } else {
                            self.doc[self.tab].set_command_line(
                                format!("{} cancelled", title(subject)),
                                Type::Info,
                            );
                        }
                    }
                    _ => self.doc[self.tab]
                        .set_command_line(format!("{} cancelled", title(subject)), Type::Info),
                }
            }
        } else {
            return true;
        }
        false
    }
    fn prompt(
        &mut self,
        prompt: &str,
        ending: &str,
        func: &dyn Fn(&mut Self, PromptEvent, &str),
    ) -> Option<String> {
        // Create a new prompt
        self.doc[self.tab].set_command_line(format!("{}{}", prompt, ending), Type::Info);
        self.update();
        let mut result = String::new();
        'p: loop {
            if let InputEvent::Key(KeyEvent {
                code: c,
                modifiers: m,
            }) = self.read_event()
            {
                match Editor::key_event_to_ox_key(c, m) {
                    KeyBinding::Raw(RawKey::Enter) => {
                        // Exit on enter key
                        break 'p;
                    }
                    KeyBinding::Raw(RawKey::Char(c)) | KeyBinding::Shift(RawKey::Char(c)) => {
                        // Update the prompt contents
                        result.push(c);
                        func(self, PromptEvent::CharPress(false), &result)
                    }
                    KeyBinding::Raw(RawKey::Backspace) => {
                        // Handle backspace event
                        result.pop();
                        func(self, PromptEvent::CharPress(true), &result)
                    }
                    KeyBinding::Raw(RawKey::Esc) => {
                        // Handle escape key
                        func(self, PromptEvent::KeyPress(c), &result);
                        return None;
                    }
                    _ => func(self, PromptEvent::KeyPress(c), &result),
                }
            }
            self.doc[self.tab]
                .set_command_line(format!("{}{}{}", prompt, ending, result), Type::Info);
            func(self, PromptEvent::Update, &result);
            self.update();
        }
        Some(result)
    }
    fn update(&mut self) {
        // Move the cursor and render the screen
        Terminal::hide_cursor();
        Terminal::goto(&Position { x: 0, y: 0 });
        self.doc[self.tab].recalculate_offset(&self.config);
        self.render();
        Terminal::goto(&Position {
            x: self.doc[self.tab]
                .cursor
                .x
                .saturating_add(self.doc[self.tab].line_offset),
            y: self.doc[self.tab].cursor.y,
        });
        Terminal::show_cursor();
        Terminal::flush();
    }
    fn welcome_message(&self, text: &str, colour: SetForegroundColor) -> String {
        // Render the welcome message
        let pad = " ".repeat(
            (self.term.size.width / 2)
                .saturating_sub(text.len() / 2)
                .saturating_sub(self.config.general.line_number_padding_right)
                .saturating_sub(self.config.general.line_number_padding_left)
                .saturating_sub(1),
        );
        let pad_right = " ".repeat(
            (self.term.size.width.saturating_sub(1))
                .saturating_sub(text.len() + pad.len())
                .saturating_sub(self.config.general.line_number_padding_left)
                .saturating_sub(self.config.general.line_number_padding_right),
        );
        format!(
            "{}{}{}~{}{}{}{}{}{}{}{}",
            if self.config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(self.config.theme.line_number_bg)
            },
            Reader::rgb_fg(self.config.theme.line_number_fg),
            " ".repeat(self.config.general.line_number_padding_left),
            RESET_FG,
            colour,
            " ".repeat(self.config.general.line_number_padding_right),
            if self.config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(self.config.theme.editor_bg)
            },
            trim_end(
                &format!("{}{}", pad, text),
                self.term.size.width.saturating_sub(4)
            ),
            pad_right,
            RESET_FG,
            RESET_BG,
        )
    }
    fn status_line(&mut self) -> String {
        // Produce the status line
        // Create the left part of the status line
        let left = self.doc[self.tab].format(&self.config.general.status_left);
        // Create the right part of the status line
        let right = self.doc[self.tab].format(&self.config.general.status_right);
        // Get the padding value
        let padding = self.term.align_break(&left, &right);
        // Generate it
        format!(
            "{}{}{}{}{}{}{}",
            Attribute::Bold,
            Reader::rgb_fg(self.config.theme.status_fg),
            Reader::rgb_bg(self.config.theme.status_bg),
            trim_end(
                &format!("{}{}{}", left, padding, right),
                self.term.size.width
            ),
            RESET_BG,
            RESET_FG,
            Attribute::Reset,
        )
    }
    fn add_background(&self, text: &str) -> String {
        // Add a background colour to a line
        if self.config.theme.transparent_editor {
            text.to_string()
        } else {
            format!(
                "{}{}{}{}",
                Reader::rgb_bg(self.config.theme.editor_bg),
                text,
                self.term.align_left(&text),
                RESET_BG
            )
        }
    }
    fn command_line(&self) -> String {
        // Render the command line
        let line = &self.doc[self.tab].cmd_line.text;
        // Add the correct styling
        match self.doc[self.tab].cmd_line.msg {
            Type::Error => self.add_background(&format!(
                "{}{}{}{}{}",
                Attribute::Bold,
                Reader::rgb_fg(self.config.theme.error_fg),
                self.add_background(&trim_end(&line, self.term.size.width)),
                RESET_FG,
                Attribute::Reset
            )),
            Type::Warning => self.add_background(&format!(
                "{}{}{}{}{}",
                Attribute::Bold,
                Reader::rgb_fg(self.config.theme.warning_fg),
                self.add_background(&trim_end(&line, self.term.size.width)),
                RESET_FG,
                Attribute::Reset
            )),
            Type::Info => self.add_background(&format!(
                "{}{}{}",
                Reader::rgb_fg(self.config.theme.info_fg),
                self.add_background(&trim_end(&line, self.term.size.width)),
                RESET_FG,
            )),
        }
    }
    fn tab_line(&mut self) -> String {
        // Render the tab line
        let mut result = vec![];
        let mut widths = vec![];
        let active_background = Reader::rgb_bg(self.config.theme.active_tab_bg);
        let inactive_background = Reader::rgb_bg(self.config.theme.inactive_tab_bg);
        let active_foreground = Reader::rgb_fg(self.config.theme.active_tab_fg);
        let inactive_foreground = Reader::rgb_fg(self.config.theme.inactive_tab_fg);
        // Iterate through documents and create their tab text
        for num in 0..self.doc.len() {
            let this = format!(
                "{} {} {}{}{}\u{2502}",
                if num == self.tab {
                    format!(
                        "{}{}{}",
                        Attribute::Bold,
                        active_background,
                        active_foreground,
                    )
                } else {
                    format!("{}{}", inactive_background, inactive_foreground,)
                },
                self.doc[num].format(&self.config.general.tab),
                Attribute::Reset,
                inactive_background,
                inactive_foreground,
            );
            widths.push(self.exp.ansi_len(this.as_str()));
            result.push(this);
        }
        // Determine if the tab can be rendered on screen
        let mut more_right = true;
        while widths.iter().sum::<usize>() > self.term.size.width {
            if self.tab == 0 || self.tab == 1 {
                result.pop();
                widths.pop();
                more_right = false;
            } else {
                result.remove(0);
                widths.remove(0);
            }
        }
        if widths.iter().sum::<usize>() < self.term.size.width.saturating_sub(3) && !more_right {
            result.push("...".to_string());
        }
        let result = result.join("");
        format!(
            "{}{}{}{}{}{}",
            inactive_background,
            inactive_foreground,
            result,
            self.term.align_left(&result),
            RESET_FG,
            RESET_BG,
        )
    }
    fn render(&mut self) {
        // Draw the screen to the terminal
        let offset = self.doc[self.tab].offset;
        let mut frame = vec![self.tab_line()];
        let rendered = self.doc[self.tab].render(TabType::Spaces, 0);
        let reg = self.doc[self.tab].regex.clone();
        if self.config.theme.transparent_editor {
            // Prevent garbage characters spamming the screen
            Terminal::clear();
        }
        for row in OFFSET..self.term.size.height {
            // Clear the current line
            let row = row.saturating_sub(OFFSET);
            if let Some(r) = self.doc[self.tab].rows.get_mut(offset.y + row) {
                if r.updated {
                    r.update_syntax(&self.config, &reg, &rendered, offset.y + row, &self.theme);
                    r.updated = false;
                }
            }
            if row == self.term.size.height - 1 - OFFSET {
                // Render command line
                frame.push(self.command_line());
            } else if row == self.term.size.height - 2 - OFFSET {
                // Render status line
                frame.push(self.status_line());
            } else if row == self.term.size.height / 4 - OFFSET && self.doc[self.tab].show_welcome {
                frame.push(self.welcome_message(
                    &format!("Ox editor  v{}", VERSION),
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(1) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "A Rust powered editor by Luke",
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(3) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "To access the wiki: Press F1",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(5) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Start typing to begin",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if let Some(line) = self.doc[self.tab]
                .rows
                .get(self.doc[self.tab].offset.y + row)
            {
                // Render lines of code
                frame.push(self.add_background(&line.render(
                    self.doc[self.tab].offset.x,
                    self.term.size.width,
                    self.doc[self.tab].offset.y + row,
                    self.doc[self.tab].line_offset,
                    &self.config,
                )));
            } else {
                // Render empty lines
                let o = self.doc[self.tab].line_offset.saturating_sub(
                    1 + self.config.general.line_number_padding_right
                        + self.config.general.line_number_padding_left,
                );
                frame.push(format!(
                    "{}{}{}",
                    Reader::rgb_fg(self.config.theme.line_number_fg),
                    self.add_background(&format!(
                        "{}{}~{}{}{}",
                        if self.config.theme.transparent_editor {
                            RESET_BG
                        } else {
                            Reader::rgb_bg(self.config.theme.line_number_bg)
                        },
                        " ".repeat(self.config.general.line_number_padding_left),
                        " ".repeat(self.config.general.line_number_padding_right),
                        " ".repeat(o),
                        Reader::rgb_bg(self.config.theme.editor_bg),
                    )),
                    RESET_FG
                ));
            }
        }
        print!("{}", frame.join("\r\n"));
    }
}

// Highlight.rs - For syntax highlighting
use crate::config::{Reader, TokenType};
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

// Tokens for storing syntax highlighting info
#[derive(Debug, Clone)]
pub struct Token {
    pub span: (usize, usize),
    pub data: String,
    pub kind: String,
    pub priority: usize,
}

pub fn cine(token: &Token, hashmap: &mut HashMap<usize, Token>) {
    // Insert a token into a hashmap
    if let Some(t) = hashmap.get(&token.span.0) {
        if t.priority > token.priority {
            return;
        }
    }
    hashmap.insert(token.span.0, token.clone());
}

fn bounds(reg: &regex::Match, line: &str) -> (usize, usize) {
    // Work out the width of the capture
    let unicode_width = UnicodeWidthStr::width(reg.as_str());
    let pre_length = UnicodeWidthStr::width(&line[..reg.start()]);
    // Calculate the correct boundaries for syntax highlighting
    (pre_length, pre_length + unicode_width)
}

fn multi_to_single(doc: &str, m: &regex::Match) -> ((usize, usize), (usize, usize)) {
    // Multiline tokens to single line tokens
    let b = bounds(&m, &doc);
    let start_y = doc[..m.start()].matches('\n').count();
    let end_y = doc[..m.end()].matches('\n').count();
    let start_x = b.0
        - UnicodeWidthStr::width(&doc.split('\n').take(start_y).collect::<Vec<_>>().join("\n")[..]);
    let end_x = b.1
        - UnicodeWidthStr::width(&doc.split('\n').take(end_y).collect::<Vec<_>>().join("\n")[..]);
    ((start_x, start_y), (end_x, end_y))
}

pub fn highlight(
    row: &str,
    doc: &str,
    index: usize,
    regex: &[TokenType],
    highlights: &HashMap<String, (u8, u8, u8)>,
) -> HashMap<usize, Token> {
    // Generate syntax highlighting information
    let mut syntax: HashMap<usize, Token> = HashMap::new();
    if regex.is_empty() {
        // Language not found, return empty hashmap
        return syntax;
    }
    for exps in regex {
        match exps {
            TokenType::SingleLine(name, regex) => {
                if name == "keywords" {
                    for kw in regex {
                        // Locate keywords
                        for cap in kw.captures_iter(row) {
                            let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                            let boundaries = bounds(&cap, &row);
                            cine(
                                &Token {
                                    span: boundaries,
                                    data: cap.as_str().to_string(),
                                    kind: Reader::rgb_fg(highlights["keywords"]).to_string(),
                                    priority: 0,
                                },
                                &mut syntax,
                            );
                        }
                    }
                } else {
                    for exp in regex {
                        // Locate expressions
                        for cap in exp.captures_iter(row) {
                            let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                            let boundaries = bounds(&cap, &row);
                            cine(
                                &Token {
                                    span: boundaries,
                                    data: cap.as_str().to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 1,
                                },
                                &mut syntax,
                            );
                        }
                    }
                }
            }
            TokenType::MultiLine(name, regex) => {
                // Multiline token
                for exp in regex {
                    for cap in exp.captures_iter(doc) {
                        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                        let ((start_x, start_y), (end_x, end_y)) = multi_to_single(&doc, &cap);
                        if start_y == index {
                            cine(
                                &Token {
                                    span: (
                                        start_x,
                                        if start_y == end_y {
                                            end_x
                                        } else {
                                            UnicodeWidthStr::width(row)
                                        },
                                    ),
                                    data: row.to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 2,
                                },
                                &mut syntax,
                            )
                        } else if end_y == index {
                            cine(
                                &Token {
                                    span: (0, end_x),
                                    data: row.to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 2,
                                },
                                &mut syntax,
                            )
                        } else if (start_y..=end_y).contains(&index) {
                            cine(
                                &Token {
                                    span: (0, UnicodeWidthStr::width(row)),
                                    data: row.to_string(),
                                    kind: Reader::rgb_fg(highlights[name]).to_string(),
                                    priority: 2,
                                },
                                &mut syntax,
                            )
                        }
                    }
                }
            }
        }
    }
    syntax
}

pub fn remove_nested_tokens(tokens: &HashMap<usize, Token>, line: &str) -> HashMap<usize, Token> {
    // Remove tokens within tokens
    let mut result = HashMap::new();
    let mut c = 0;
    // While the line still isn't full
    while c < line.len() {
        // If the token at this position exists
        if let Some(t) = tokens.get(&c) {
            // Insert it and jump over everything
            result.insert(t.span.0, t.clone());
            c += t.span.1 - t.span.0;
        } else {
            // Shift forward
            c += 1;
        }
    }
    result
}

#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::used_underscore_binding,
    clippy::cast_sign_loss
)]

/*
    Ox editor is a text editor written in the Rust programming language.

    It runs in the terminal and provides keyboard shortcuts to interact.
    This removes the need for a mouse which can slow down editing files.
    I have documented this code where necessary and it has been formatted
    with Rustfmt to ensure clean and consistent style throughout.

    More information here:
    https://rust-lang.org
    https://github.com/rust-lang/rustfmt
    https://github.com/curlpipe/ox
*/

// Bring in the external modules
mod config;
mod document;
mod editor;
mod highlight;
mod oxa;
mod row;
mod terminal;
mod undo;
mod util;

use clap::{App, Arg};
use directories::BaseDirs;
use document::Document;
use editor::{Direction, Editor, Position};
use oxa::Variable;
use row::Row;
use std::fs::OpenOptions;
use std::io::Write;
use std::{env, panic};
use terminal::{Size, Terminal};
use undo::{Event, EventStack};

// Create log macro
#[macro_export]
macro_rules! log {
    ($type:literal, $msg:expr) => {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/ox.log");
        if let Ok(mut log) = file {
            writeln!(log, "{}: {}", $type, $msg).unwrap();
        } else {
            panic!("{:?}", file);
        }
    };
}

// Get the current version of Ox
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    log!("Ox started", "Ox has just been started");
    // Set up panic hook in case of unexpected crash
    panic::set_hook(Box::new(|e| {
        // Reenter canonical mode
        Terminal::exit();
        // Set hook to log crash reason
        log!("Unexpected panic", e);
        // Print panic info
        eprintln!("{}", e);
    }));
    // Attempt to start an editor instance
    let config_dir = load_config().unwrap_or_else(|| " ~/.config/ox/ox.ron".to_string());
    // Gather the command line arguments
    let cli = App::new("Ox")
        .version(VERSION)
        .author("Author: Luke <https://github.com/curlpipe>")
        .about("An independent Rust powered text editor")
        .arg(
            Arg::with_name("files")
                .multiple(true)
                .takes_value(true)
                .help(
                    r#"The files you wish to edit
You can also provide the line number to jump to by doing this:
file.txt:100 (This will go to line 100 in file.txt)"#,
                ),
        )
        .arg(
            Arg::with_name("readonly")
                .long("readonly")
                .short("r")
                .takes_value(false)
                .required(false)
                .help("Enable read only mode"),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .takes_value(true)
                .default_value(&config_dir)
                .help("The directory of the config file"),
        );
    // Fire up the editor, ensuring that no start up problems occured
    if let Ok(mut editor) = Editor::new(cli) {
        editor.run();
    }
}

fn load_config() -> Option<String> {
    // Load the configuration file
    let base_dirs = BaseDirs::new()?;
    Some(format!(
        "{}/ox/ox.ron",
        base_dirs.config_dir().to_str()?.to_string()
    ))
}

/*
    Oxa.rs - Tools for parsing and lexing the Ox Assembly format

    Oxa is an interpreted specific purpose language inspired by x86 assembly
    It is used to write macros for the editor that make editing text painless
    It is also used for writing commands in the "macro mode" when editing

    An example usage could be writing a macro to delete the current line
*/
use crate::undo::BankType;
use crate::util::line_offset;
use crate::{Direction, Event, Position, Row};

#[derive(Debug, Copy, Clone)]
pub enum Variable {
    Saved,
}

pub fn interpret_line(
    line: &str,
    cursor: &Position,
    graphemes: usize,
    rows: &[Row],
) -> Option<Vec<Event>> {
    // Take an instruction of Oxa and interpret it
    let mut events = vec![];
    let mut line = line.split(' ');
    if let Some(instruction) = line.next() {
        let mut args: Vec<&str> = line.collect();
        let root = if let Some(&"sudo") = args.get(0) {
            args.remove(0);
            true
        } else {
            false
        };
        match instruction {
            "new" => events.push(Event::New),
            "open" => events.push(open_command(&args)),
            "undo" => events.push(Event::Undo),
            "commit" => events.push(Event::Commit),
            "redo" => events.push(Event::Redo),
            "quit" => events.push(quit_command(&args)),
            "prev" => events.push(Event::PrevTab),
            "next" => events.push(Event::NextTab),
            "set" => events.push(set_command(&args, &cursor, &rows)),
            "split" => events.push(Event::SplitDown(*cursor, *cursor)),
            "splice" => events.push(Event::SpliceUp(*cursor, *cursor)),
            "search" => events.push(Event::Search),
            "reload" => events.push(Event::ReloadConfig),
            "cmd" => events.push(Event::Cmd),
            "replace" => events.push(replace_command(&args)),
            // Shell with substitution and no confirm
            "shs" => events.push(Event::Shell(args.join(" "), false, true, root)),
            // Shell with substitution and confirm
            "shcs" => events.push(Event::Shell(args.join(" "), true, true, root)),
            // Shell with confirm and no substitution
            "shc" => events.push(Event::Shell(args.join(" "), true, false, root)),
            // Shell with no confirm nor substitution
            "sh" => events.push(Event::Shell(args.join(" "), false, false, root)),
            "is" => {
                if let Some(set) = is_command(&args) {
                    events.push(set)
                } else {
                    return None;
                }
            }
            "theme" => {
                if let Some(theme) = theme_command(&args) {
                    events.push(theme)
                } else {
                    return None;
                }
            }
            "line" => {
                if let Some(line) = line_command(&args, &cursor) {
                    events.push(line);
                } else {
                    return None;
                }
            }
            _ => {
                let i = match instruction {
                    "save" => save_command(&args),
                    "goto" => goto_command(&args),
                    "move" => move_command(&args),
                    "put" => put_command(&args, &cursor),
                    "delete" => delete_command(&args, &cursor, graphemes, &rows),
                    "load" => load_command(&args),
                    "store" => store_command(&args),
                    "overwrite" => overwrite_command(&args, &rows),
                    _ => return None,
                };
                if let Some(mut command) = i {
                    events.append(&mut command);
                } else {
                    return None;
                }
            }
        }
    }
    Some(events)
}

fn is_command(args: &[&str]) -> Option<Event> {
    Some(Event::Set(
        match &args[0][..] {
            "saved" => Variable::Saved,
            _ => return None,
        },
        true,
    ))
}

fn theme_command(args: &[&str]) -> Option<Event> {
    if args.is_empty() {
        None
    } else {
        Some(Event::Theme(args[0].to_string()))
    }
}

fn replace_command(args: &[&str]) -> Event {
    if !args.is_empty() && args[0] == "*" {
        Event::ReplaceAll
    } else {
        Event::Replace
    }
}

fn open_command(args: &[&str]) -> Event {
    Event::Open(if args.is_empty() {
        None
    } else {
        Some(args[0].to_string())
    })
}

fn quit_command(args: &[&str]) -> Event {
    if args.contains(&"*") {
        Event::QuitAll(args.contains(&"!"))
    } else {
        Event::Quit(args.contains(&"!"))
    }
}

fn line_command(args: &[&str], cursor: &Position) -> Option<Event> {
    if args.is_empty() {
        return None;
    } else if let Some(dir) = args.get(0) {
        return match *dir {
            "below" => Some(Event::InsertLineBelow(*cursor)),
            "above" => Some(Event::InsertLineAbove(*cursor)),
            _ => None,
        };
    }
    None
}

fn set_command(args: &[&str], cursor: &Position, rows: &[Row]) -> Event {
    if args.is_empty() {
        Event::UpdateLine(
            *cursor,
            0,
            Box::new(rows[cursor.y].clone()),
            Box::new(Row::from("")),
        )
    } else {
        Event::UpdateLine(
            *cursor,
            0,
            Box::new(rows[cursor.y].clone()),
            Box::new(Row::from(args.join(" ").as_str())),
        )
    }
}

fn overwrite_command(args: &[&str], rows: &[Row]) -> Option<Vec<Event>> {
    Some(vec![if args.is_empty() {
        Event::Overwrite(rows.to_vec(), vec![Row::from("")])
    } else {
        Event::Overwrite(
            rows.to_vec(),
            args.join(" ")
                .split("\\n")
                .map(Row::from)
                .collect::<Vec<_>>(),
        )
    }])
}

fn save_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.is_empty() {
        events.push(Event::Save(None, false))
    } else {
        events.push(if args[0] == "*" {
            Event::SaveAll
        } else if args[0] == "?" {
            Event::Save(None, true)
        } else {
            Event::Save(Some(args[0].to_string()), false)
        })
    }
    Some(events)
}

fn store_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.len() == 2 {
        if let Ok(bank) = args[1].parse::<usize>() {
            if let Some(kind) = args.get(0) {
                match *kind {
                    "cursor" => events.push(Event::Store(BankType::Cursor, bank)),
                    "line" => events.push(Event::Store(BankType::Line, bank)),
                    _ => return None,
                }
            }
        } else {
            return None;
        }
    } else {
        return None;
    }
    Some(events)
}

fn load_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.len() == 2 {
        if let Ok(bank) = args[1].parse::<usize>() {
            if let Some(kind) = args.get(0) {
                match *kind {
                    "cursor" => events.push(Event::Load(BankType::Cursor, bank)),
                    "line" => events.push(Event::Load(BankType::Line, bank)),
                    _ => return None,
                }
            }
        } else {
            return None;
        }
    } else {
        return None;
    }
    Some(events)
}

fn goto_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    match args.len() {
        0 => events.push(Event::GotoCursor(Position { x: 0, y: 0 })),
        1 => {
            if let Ok(y) = args[0].parse::<usize>() {
                events.push(Event::GotoCursor(Position {
                    x: 0,
                    y: y.saturating_sub(1),
                }));
            } else {
                return None;
            }
        }
        2 => {
            if let (Ok(x), Ok(y)) = (args[0].parse::<usize>(), args[1].parse::<usize>()) {
                events.push(Event::GotoCursor(Position {
                    x: x.saturating_sub(1),
                    y: y.saturating_sub(1),
                }));
            } else {
                return None;
            }
        }
        _ => return None,
    }
    Some(events)
}

fn put_command(args: &[&str], cursor: &Position) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args[0] == "\\t" {
        events.push(Event::InsertTab(*cursor));
    } else {
        for (c, ch) in args.join(" ").chars().enumerate() {
            events.push(Event::Insertion(
                Position {
                    x: cursor.x.saturating_add(c),
                    y: cursor.y,
                },
                ch,
            ))
        }
    }
    Some(events)
}

fn move_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.len() == 2 {
        if let Ok(magnitude) = args[0].parse::<usize>() {
            let direction = args[1];
            events.push(Event::MoveCursor(
                magnitude as i128,
                match direction {
                    "up" => Direction::Up,
                    "down" => Direction::Down,
                    "left" => Direction::Left,
                    "right" => Direction::Right,
                    _ => return None,
                },
            ));
        } else if args[0] == "word" {
            events.push(Event::MoveWord(match args[1] {
                "left" => Direction::Left,
                "right" => Direction::Right,
                _ => return None,
            }));
        } else {
            return None;
        }
    } else if let Some(direction) = args.get(0) {
        events.push(match *direction {
            "home" => Event::Home,
            "end" => Event::End,
            "pageup" => Event::PageUp,
            "pagedown" => Event::PageDown,
            _ => return None,
        });
    } else {
        return None;
    }
    Some(events)
}

fn delete_command(
    args: &[&str],
    cursor: &Position,
    graphemes: usize,
    rows: &[Row],
) -> Option<Vec<Event>> {
    // Handle the delete command (complicated)
    let mut events = vec![];
    if args.is_empty() {
        if let Some(ch) = rows[cursor.y]
            .string
            .chars()
            .collect::<Vec<_>>()
            .get(graphemes)
        {
            events.push(Event::Deletion(*cursor, *ch));
        }
    } else if args[0] == "word" {
        events.push(Event::DeleteWord(*cursor, "egg".to_string()));
    } else if let Ok(line) = args[0].parse::<i128>() {
        events.push(Event::DeleteLine(
            *cursor,
            line,
            Box::new(rows[line_offset(cursor.y, line, rows.len())].clone()),
        ));
    } else {
        return None;
    }
    Some(events)
}

// Row.rs - Handling the rows of a document and their appearance
use crate::config::{Reader, TokenType};
use crate::editor::{RESET_BG, RESET_FG};
use crate::highlight::{highlight, remove_nested_tokens, Token};
use crate::util::{safe_ansi_insert, Exp};
use std::collections::HashMap;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// Ensure we can use the Clone trait to copy row structs for manipulation
#[derive(Debug, Clone)]
pub struct Row {
    pub string: String,                   // For holding the contents of the row
    pub syntax: HashMap<usize, Token>,    // Hashmap for syntax
    pub bg_syntax: HashMap<usize, Token>, // Hashmap for background syntax colour
    pub updated: bool,                    // Line needs to be redrawn
    regex: Exp,                           // For holding the regex expression
}

// Implement a trait (similar method to inheritance) into the row
impl From<&str> for Row {
    fn from(s: &str) -> Self {
        // Initialise a row from a string
        Self {
            string: s.to_string(),
            syntax: HashMap::new(),
            bg_syntax: HashMap::new(),
            regex: Exp::new(),
            updated: true,
        }
    }
}

// Add methods to the Row struct / class
impl Row {
    pub fn render_line_number(config: &Reader, offset: usize, index: usize) -> String {
        let post_padding = offset.saturating_sub(
            index.to_string().len() +         // Length of the number
            config.general.line_number_padding_right + // Length of the right padding
            config.general.line_number_padding_left, // Length of the left padding
        );
        format!(
            "{}{}{}{}{}{}{}{}",
            if config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(config.theme.line_number_bg)
            },
            Reader::rgb_fg(config.theme.line_number_fg),
            " ".repeat(config.general.line_number_padding_left),
            " ".repeat(post_padding),
            index,
            " ".repeat(config.general.line_number_padding_right),
            Reader::rgb_fg(config.theme.editor_fg),
            if config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(config.theme.editor_bg)
            },
        )
    }
    pub fn render(
        &self,
        mut start: usize,
        width: usize,
        index: usize,
        offset: usize,
        config: &Reader,
    ) -> String {
        // Render the row by trimming it to the correct size
        let index = index.saturating_add(1);
        // Padding to align line numbers to the right
        // Assemble the line number data
        let line_number = Row::render_line_number(config, offset, index);
        // Strip ANSI values from the line
        let line_number_len = self.regex.ansi_len(&line_number);
        let width = width.saturating_sub(line_number_len);
        let reset_foreground = RESET_FG.to_string();
        let reset_background = RESET_BG.to_string();
        let editor_bg = Reader::rgb_bg(config.theme.editor_bg).to_string();
        let mut initial = start;
        let mut result = vec![];
        // Ensure that the render isn't impossible
        if width != 0 && start < UnicodeWidthStr::width(&self.string[..]) {
            // Calculate the character positions
            let end = width + start;
            let mut dna = HashMap::new();
            let mut cumulative = 0;
            // Collect the DNA from the unicode characters
            for ch in self.string.graphemes(true) {
                dna.insert(cumulative, ch);
                cumulative += UnicodeWidthStr::width(ch);
            }
            // Repair dodgy start
            if !dna.contains_key(&start) {
                result.push(" ");
                start += 1;
            }
            // Push across characters
            'a: while start < end {
                if let Some(t) = self.syntax.get(&start) {
                    // There is a token here
                    result.push(&t.kind);
                    while start < end && start < t.span.1 {
                        if let Some(ch) = dna.get(&start) {
                            // The character overlaps with the edge
                            if start + UnicodeWidthStr::width(*ch) > end {
                                result.push(" ");
                                break 'a;
                            }
                            result.push(ch);
                            start += UnicodeWidthStr::width(*ch);
                        } else {
                            break 'a;
                        }
                    }
                    result.push(&reset_foreground);
                } else if let Some(ch) = dna.get(&start) {
                    // There is a character here
                    if start + UnicodeWidthStr::width(*ch) > end {
                        result.push(" ");
                        break 'a;
                    }
                    result.push(ch);
                    start += UnicodeWidthStr::width(*ch);
                } else {
                    // The quota has been used up
                    break 'a;
                }
            }
            // Correct colourization of tokens that are half off the screen and half on the screen
            let initial_initial = initial; // Terrible variable naming, I know
            if initial > 0 {
                // Calculate the last token start boundary
                while self.syntax.get(&initial).is_none() && initial > 0 {
                    initial -= 1;
                }
                // Verify that the token actually exists
                if let Some(t) = self.syntax.get(&initial) {
                    // Verify that the token isn't up against the far left side
                    if t.span.0 != initial_initial && t.span.1 >= initial_initial {
                        // Insert the correct colours
                        let mut real = 0;
                        let mut ch = 0;
                        for i in &result {
                            if ch == t.span.1 - initial_initial {
                                break;
                            }
                            real += i.len();
                            ch += UnicodeWidthStr::width(*i);
                        }
                        result.insert(real, &reset_foreground);
                        result.insert(0, &t.kind);
                    }
                }
            }
            // Insert background tokens
            for b in &self.bg_syntax {
                let bg = if config.theme.transparent_editor {
                    &reset_background
                } else {
                    &editor_bg
                };
                if let Some(a) = safe_ansi_insert(b.1.span.0, &result, &self.regex.ansi) {
                    if a < result.len() {
                        result.insert(a, &b.1.kind);
                    }
                };
                if let Some(a) = safe_ansi_insert(b.1.span.1, &result, &self.regex.ansi) {
                    if a < result.len() {
                        result.insert(a, bg);
                    }
                };
            }
        }
        // Return the full line string to be rendered
        line_number + &result.join("")
    }
    pub fn update_syntax(
        &mut self,
        config: &Reader,
        syntax: &[TokenType],
        doc: &str,
        index: usize,
        theme: &str,
    ) {
        // Update the syntax highlighting indices for this row
        self.syntax = remove_nested_tokens(
            &highlight(
                &self.string,
                &doc,
                index,
                &syntax,
                &config.highlights[theme],
            ),
            &self.string,
        );
    }
    pub fn length(&self) -> usize {
        // Get the current length of the row
        UnicodeWidthStr::width(&self.string[..])
    }
    pub fn chars(&self) -> Vec<&str> {
        // Get the characters of the line
        self.string.graphemes(true).collect()
    }
    pub fn ext_chars(&self) -> Vec<&str> {
        // Produce a special list of characters depending on the widths of characters
        let mut result = Vec::new();
        for i in self.chars() {
            result.resize(result.len() + UnicodeWidthStr::width(i), i);
        }
        result
    }
    pub fn get_jumps(&self) -> Vec<usize> {
        // Get the intervals of the unicode widths
        let mut result = Vec::new();
        for i in self.chars() {
            result.push(UnicodeWidthStr::width(i));
        }
        result
    }
    pub fn boundaries(&self) -> Vec<usize> {
        // Get the boundaries of the unicode widths
        let mut result = Vec::new();
        let mut count = 0;
        for i in self.get_jumps() {
            result.push(count);
            count += i;
        }
        result
    }
    pub fn insert(&mut self, ch: char, pos: usize) {
        // Insert a character
        self.updated = true;
        let mut before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(pos as usize).collect();
        before.push(ch);
        before.push_str(&after);
        self.string = before;
    }
    pub fn delete(&mut self, pos: usize) -> Option<char> {
        // Remove a character
        self.updated = true;
        let before: String = self.string.graphemes(true).take(pos as usize).collect();
        let after: String = self.string.graphemes(true).skip(1 + pos as usize).collect();
        let result: Option<char>;
        if let Some(c) = self.chars().get(pos) {
            if let Ok(c) = c.parse() {
                result = Some(c);
            } else {
                result = None;
            }
        } else {
            result = None;
        }
        self.string = before + &after;
        result
    }
}

// Terminal.rs - Handling low level terminal operations
use crate::util::Exp;
use crate::Position;
use crossterm::terminal;
use crossterm::{execute, ErrorKind};
use std::env;
use std::io::{stdout, Write};
use term::terminfo::TermInfo;
use unicode_width::UnicodeWidthStr;

// Struct to hold size
pub struct Size {
    pub width: usize,
    pub height: usize,
}

// The terminal struct
pub struct Terminal {
    pub size: Size, // For holding the size of the terminal
    regex: Exp,     // For holding the regex
}

// Implement methods into the terminal struct / class
impl Terminal {
    pub fn new() -> Result<Self, ErrorKind> {
        // Create a new terminal and switch into raw mode
        let size = terminal::size()?;
        Terminal::enter();
        Ok(Self {
            size: Size {
                width: size.0 as usize,
                height: size.1 as usize,
            },
            regex: Exp::new(),
        })
    }
    pub fn enter() {
        // Enter the current terminal
        terminal::enable_raw_mode().unwrap();
        execute!(stdout(), terminal::EnterAlternateScreen).unwrap();
    }
    pub fn exit() {
        // Exit the terminal
        execute!(stdout(), terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
    pub fn goto(p: &Position) {
        // Move the cursor to a position
        execute!(stdout(), crossterm::cursor::MoveTo(p.x as u16, p.y as u16)).unwrap();
    }
    pub fn flush() {
        // Flush the screen to prevent weird behaviour
        stdout().flush().unwrap();
    }
    pub fn hide_cursor() {
        // Hide the text cursor
        execute!(stdout(), crossterm::cursor::Hide).unwrap();
    }
    pub fn show_cursor() {
        execute!(stdout(), crossterm::cursor::Show).unwrap();
    }
    pub fn clear() {
        execute!(stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
    }
    pub fn align_break(&self, l: &str, r: &str) -> String {
        // Align two items to the left and right
        let left_length = UnicodeWidthStr::width(l);
        let right_length = UnicodeWidthStr::width(r);
        let padding = (self.size.width as usize).saturating_sub(left_length + right_length);
        " ".repeat(padding as usize)
    }
    pub fn align_left(&self, text: &str) -> String {
        // Align items to the left
        let length = self.regex.ansi_len(text);
        let padding = (self.size.width as usize).saturating_sub(length);
        " ".repeat(padding as usize)
    }
    pub fn availablility() -> usize {
        let colour = env::var("COLORTERM");
        if colour.unwrap_or_else(|_| "".to_string()) == "truecolor" {
            24
        } else if let Ok(info) = TermInfo::from_env() {
            if info.numbers.get("colors").unwrap() == &256 {
                256
            } else {
                16
            }
        } else {
            16
        }
    }
}

// Undo.rs - Utilities for undoing, redoing and storing events
use crate::util::line_offset;
use crate::{Direction, Position, Row, Variable};

// Enum for the the types of banks
#[derive(Debug, Clone)]
pub enum BankType {
    Line,   // For holding lines from the document
    Cursor, // For holding cursor positions
}

// Event enum to store the types of events that occur
#[derive(Debug, Clone)]
pub enum Event {
    Store(BankType, usize),                         // Store an item in a bank
    Load(BankType, usize),                          // Load an item from a bank
    SpliceUp(Position, Position),                   // Delete from start
    SplitDown(Position, Position),                  // Return from middle of the line
    InsertLineAbove(Position),                      // Return key in the middle of line
    InsertLineBelow(Position),                      // Return on the end of line
    Deletion(Position, char),                       // Delete from middle
    Insertion(Position, char),                      // Insert character
    InsertTab(Position),                            // Insert a tab character
    DeleteTab(Position),                            // Delete a tab character
    DeleteLine(Position, i128, Box<Row>),           // For deleting a line
    UpdateLine(Position, i128, Box<Row>, Box<Row>), // For holding entire line updates
    MoveCursor(i128, Direction),                    // For moving the cursor
    GotoCursor(Position),                           // For setting the cursor position
    MoveWord(Direction),                            // Move cursor through words
    DeleteWord(Position, String),                   // Delete word
    Theme(String),                                  // Theme change event
    Search,                                         // Search the document
    Replace,                                        // Replace certain occurances
    ReplaceAll,                                     // Replace everything
    Cmd,                                            // Trigger command mode
    Home,                                           // Moving cursor to the start of line
    End,                                            // Moving cursor to the end of line
    PageUp,                                         // Moving cursor one page up
    PageDown,                                       // Moving cursor one page down
    Overwrite(Vec<Row>, Vec<Row>),                  // Overwrite document
    New,                                            // New document
    Open(Option<String>),                           // Open document
    Save(Option<String>, bool),                     // Save document
    SaveAll,                                        // Save all documents
    Undo,                                           // Undo event
    Redo,                                           // Redo event
    Commit,                                         // Commit undo event
    Quit(bool),                                     // Quit document
    QuitAll(bool),                                  // Quit all
    NextTab,                                        // Next tab
    PrevTab,                                        // Previous tab
    ReloadConfig,                                   // Reload the configuration file
    Shell(String, bool, bool, bool),                // Running a shell command
    Set(Variable, bool),                            // For updating variables of the document
}

// A struct for holding all the events taken by the user
#[derive(Debug)]
pub struct EventStack {
    history: Vec<Vec<Event>>,  // For storing the history of events
    current_patch: Vec<Event>, // For storing the current group
}

// Methods for the EventStack
impl EventStack {
    pub fn new() -> Self {
        // Initialise an Event stack
        Self {
            history: vec![],
            current_patch: vec![],
        }
    }
    pub fn push(&mut self, event: Event) {
        // Add an event to the event stack
        self.current_patch.insert(0, event);
    }
    pub fn pop(&mut self) -> Option<Vec<Event>> {
        // Take a patch off the event stack
        self.history.pop()
    }
    pub fn append(&mut self, patch: Vec<Event>) {
        // Append a patch to the stack
        self.history.push(patch);
    }
    pub fn empty(&mut self) {
        // Empty the stack
        self.history.clear();
    }
    pub fn commit(&mut self) {
        // Commit patch to history
        if !self.current_patch.is_empty() {
            self.history.push(self.current_patch.clone());
            self.current_patch.clear();
        }
    }
    pub fn len(&self) -> usize {
        // Find the length of the undo stack
        self.history.len()
    }
}

pub fn reverse(before: Event, limit: usize) -> Option<Vec<Event>> {
    // Turn an event into the opposite of itself
    // Used for undo
    Some(match before {
        Event::SpliceUp(before, after) => vec![Event::SplitDown(after, before)],
        Event::SplitDown(before, after) => vec![Event::SpliceUp(after, before)],
        Event::InsertLineAbove(pos) => vec![Event::DeleteLine(pos, 0, Box::new(Row::from("")))],
        Event::InsertLineBelow(pos) => vec![Event::DeleteLine(pos, 1, Box::new(Row::from("")))],
        Event::Deletion(pos, ch) => vec![Event::Insertion(pos, ch)],
        Event::Insertion(pos, ch) => vec![Event::Deletion(
            Position {
                x: pos.x.saturating_add(1),
                y: pos.y,
            },
            ch,
        )],
        Event::DeleteLine(pos, offset, before) => vec![
            Event::InsertLineAbove(Position {
                x: pos.x,
                y: line_offset(pos.y, offset, limit),
            }),
            Event::UpdateLine(pos, offset, Box::new(Row::from("")), before),
        ],
        Event::UpdateLine(pos, offset, before, after) => {
            vec![Event::UpdateLine(pos, offset, after, before)]
        }
        Event::Overwrite(before, after) => vec![Event::Overwrite(after, before)],
        Event::InsertTab(pos) => vec![Event::DeleteTab(pos)],
        Event::DeleteTab(pos) => vec![Event::InsertTab(pos)],
        _ => return None,
    })
}

// Util.rs - Utilities for the rest of the program
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// For holding general purpose regular expressions
#[derive(Debug, Clone)]
pub struct Exp {
    pub ansi: Regex,
}

impl Exp {
    pub fn new() -> Self {
        // Create the regular expressions
        Self {
            ansi: Regex::new(r"\u{1b}\[[0-?]*[ -/]*[@-~]").unwrap(),
        }
    }
    pub fn ansi_len(&self, string: &str) -> usize {
        // Find the length of a string without ANSI values
        UnicodeWidthStr::width(&*self.ansi.replace_all(string, ""))
    }
}

pub fn title(c: &str) -> String {
    // Title-ize the string
    c.chars().next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + &c[1..]
    })
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

pub fn line_offset(point: usize, offset: i128, limit: usize) -> usize {
    if offset.is_negative() {
        if point as i128 + offset >= 0 {
            (point as i128 + offset) as usize
        } else {
            0
        }
    } else if point as i128 + offset < limit as i128 {
        (point as i128 + offset) as usize
    } else {
        limit.saturating_sub(1)
    }
}

pub fn spaces_to_tabs(code: &str, tab_width: usize) -> String {
    // Convert spaces to tabs
    let mut result = vec![];
    for mut line in code.split('\n') {
        // Count the number of spaces
        let mut spaces = 0;
        for c in line.chars() {
            if c == ' ' {
                spaces += 1;
            } else {
                break;
            }
        }
        // Divide by tab width
        let tabs = spaces / tab_width;
        // Remove spaces
        line = &line[spaces..];
        // Add tabs
        result.push(format!("{}{}", "\t".repeat(tabs), line));
    }
    result.join("\n")
}

pub fn tabs_to_spaces(code: &str, tab_width: usize) -> String {
    // Convert tabs to spaces
    let mut result = vec![];
    for mut line in code.split('\n') {
        // Count the number of spaces
        let mut tabs = 0;
        for c in line.chars() {
            if c == '\t' {
                tabs += 1;
            } else {
                break;
            }
        }
        // Divide by tab width
        let spaces = tabs * tab_width;
        // Remove spaces
        line = &line[tabs..];
        // Add tabs
        result.push(format!("{}{}", " ".repeat(spaces), line));
    }
    result.join("\n")
}

pub fn is_ansi(s: &str, chk: &Regex) -> bool {
    chk.is_match(s)
}

pub fn safe_ansi_insert(index: usize, list: &[&str], chk: &Regex) -> Option<usize> {
    let mut c = 0;
    for (ac, i) in list.iter().enumerate() {
        if !is_ansi(i, chk) {
            c += 1;
        }
        if c == index {
            return Some(ac.saturating_add(1));
        }
    }
    None
}

/*
    StarWM is an attempt at a window manager.
    It's written in Rust for stability and speed.
    It was written to be manually edited, if need be.
    The code is commented throughout,
    feel free to modify it if you dislike any part of StarWM.
*/

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::unreadable_literal)]

mod config;
#[macro_use]
mod utils;
mod key;
mod mouse;
mod window;
mod wm;

use key::{META, META_SHIFT, NONE};
use wm::StarMan;

// List of commands to run within the WM
const ROFI: &str = "rofi -show run";
const ALACRITTY: &str = "alacritty";
const MAIM: &str = "maim -suB --delay=0.1 | xclip -selection clipboard -t image/png";

fn main() {
    // Initialise and run StarWM
    let mut starman = StarMan::new();

    // Exit on [Meta] + [Shift] + [BackSpace]
    starman.bind((META_SHIFT, "BackSpace"), |_| std::process::exit(0));
    // Close window on [Meta] + [Q]
    starman.bind((META, "q"), StarMan::destroy_focus);
    // Move window to workspace on [Meta] + [Shift] + [WORKSPACE]
    starman.bind((META_SHIFT, "1"), |s| s.move_window_to_workspace(0));
    starman.bind((META_SHIFT, "2"), |s| s.move_window_to_workspace(1));
    starman.bind((META_SHIFT, "3"), |s| s.move_window_to_workspace(2));
    starman.bind((META_SHIFT, "4"), |s| s.move_window_to_workspace(3));
    starman.bind((META_SHIFT, "5"), |s| s.move_window_to_workspace(4));
    starman.bind((META_SHIFT, "6"), |s| s.move_window_to_workspace(5));
    starman.bind((META_SHIFT, "7"), |s| s.move_window_to_workspace(6));
    starman.bind((META_SHIFT, "8"), |s| s.move_window_to_workspace(7));
    starman.bind((META_SHIFT, "9"), |s| s.move_window_to_workspace(8));
    starman.bind((META_SHIFT, "0"), |s| s.move_window_to_workspace(9));
    // Toggle monocle mode on [Meta] + [M]
    starman.bind((META, "m"), |s| {
        if s.workspace().get_monocle().is_none() {
            s.monocle_focus();
        } else {
            s.monocle_clear();
        }
    });

    // Start application launcher on [Meta] + [Space]
    starman.bind((META, "space"), |_| cmd!(ROFI));
    // Start terminal on [Meta] + [Return]
    starman.bind((META, "Return"), |_| cmd!(ALACRITTY));
    // Screenshot on [Meta] + [S]
    starman.bind((META, "s"), |_| cmd!(MAIM));
    // Open rofi on search key
    starman.bind((NONE, "XF86Search"), |_| cmd!(ROFI));

    // Run the window manager
    starman.run();
}

// Config.rs - Handles configuration of the editor
use crate::key::Key;
use crate::StarMan;
use std::collections::HashMap;

// This is a function or closure that is run on a key press event
pub type Handler = fn(&mut StarMan) -> ();

// Configuration that holds the key bindings within the window manager
pub struct Config {
    pub key_bindings: HashMap<Key, Handler>,
    pub unfocused_border: WindowBorder,
    pub focused_border: WindowBorder,
}

impl Config {
    pub fn new() -> Self {
        // Start a fresh configuration struct
        Self {
            key_bindings: HashMap::new(),
            unfocused_border: WindowBorder {
                size: 2,
                colour: 0x383838,
            },
            focused_border: WindowBorder {
                size: 2,
                colour: 0x006755,
            },
        }
    }

    pub fn bind_handler(&mut self, key: Key, handler: Handler) {
        // Add a key binding and a handler function to the configuration
        self.key_bindings.insert(key, handler);
    }

    pub fn key(&self, key: &Key) -> Option<&Handler> {
        // Get a handler function when a key binding occurs
        self.key_bindings.get(key)
    }
}

// Struct to hold window border information
pub struct WindowBorder {
    pub size: u32,
    pub colour: u32,
}

// Key.rs - Handles key reading and processing
use std::collections::HashMap;
use std::ffi::CStr;
use xcb::get_keyboard_mapping;
pub use xcb::{
    ModMask, MOD_MASK_1 as ALT, MOD_MASK_4 as META, MOD_MASK_CONTROL as CONTROL,
    MOD_MASK_SHIFT as SHIFT, NONE,
};

// Key table shorthand
pub type SymTable = HashMap<u8, Vec<String>>;

// Common combinations
pub const META_SHIFT: ModMask = META | SHIFT;
/*
pub const CONTROL_SHIFT: ModMask = CONTROL | SHIFT;
pub const CONTROL_ALT_SHIFT: ModMask = CONTROL | ALT | SHIFT;
pub const CONTROL_ALT: ModMask = CONTROL | ALT;
pub const META_ALT: ModMask = META | ALT;
pub const META_ALT_SHIFT: ModMask = META | ALT | SHIFT;
*/

// Representation of a key, with modifiers
#[derive(PartialEq, Eq, Hash)]
pub struct Key {
    pub code: String,
    pub mods: ModMask,
}

impl Key {
    pub fn new(mods: ModMask, code: &str) -> Self {
        // Create a new key, from X key input data
        Self {
            code: st!(code),
            mods,
        }
    }

    pub fn xcode(&self, table: &SymTable) -> Vec<u8> {
        // This gives out the X code for the key
        let mut result = vec![];
        for (k, v) in table {
            if v.contains(&self.code) {
                result.push(*k);
            }
        }
        result
    }
}

// Helpful into trait for short arguments
impl From<(ModMask, String)> for Key {
    fn from(f: (ModMask, String)) -> Key {
        Key::new(f.0, &f.1)
    }
}

// Helpful into trait for short arguments
impl From<(ModMask, &str)> for Key {
    fn from(f: (ModMask, &str)) -> Key {
        Key::new(f.0, f.1)
    }
}

pub fn get_lookup(conn: &xcb::Connection) -> HashMap<u8, Vec<String>> {
    // Retrieve the lookup table for keypresses
    let setup = conn.get_setup();
    // Work out range of keycodes
    let start = setup.min_keycode();
    let width = setup.max_keycode() - start + 1;
    // Get the keyboard mapping
    let keyboard_mapping = get_keyboard_mapping(conn, start, width)
        .get_reply()
        .unwrap();
    // Retrieve the key symbols and how many there are per keycode
    let keysyms = keyboard_mapping.keysyms();
    let keysyms_per_keycode = keyboard_mapping.keysyms_per_keycode() as usize;
    let ptr_value = unsafe { &*(keyboard_mapping.ptr) };
    // Work out how many keycodes there are in total
    let keycode_count = ptr_value.length as usize / keysyms_per_keycode as usize;
    // Prepare final table
    let mut result = HashMap::new();
    for keycode in 0..keycode_count {
        // Prepare list of symbols
        let mut syms = vec![];
        for keysym in 0..keysyms_per_keycode {
            // Retrieve each symbol
            let sym = keysyms[keysym + keycode * keysyms_per_keycode];
            if sym == 0 {
                continue;
            }
            let string_ptr = unsafe { x11::xlib::XKeysymToString(u64::from(sym)) };
            syms.push(if string_ptr.is_null() {
                st!("None")
            } else {
                unsafe { CStr::from_ptr(string_ptr) }
                    .to_str()
                    .unwrap()
                    .to_owned()
            });
        }
        // Insert into result table
        #[allow(clippy::cast_possible_truncation)]
        result.insert(start + keycode as u8, syms);
    }
    result
}

// Mouse.rs - Handling mouse events
use xcb::{ffi, Event, Reply};

// Mouse move event struct
#[derive(Default)]
#[allow(clippy::module_name_repetitions)]
pub struct MouseInfo {
    pub root_x: i16,
    pub root_y: i16,
    pub child: u32,
    pub detail: u8,
    pub geo: Option<(i64, i64, u32, u32)>,
}

impl MouseInfo {
    pub fn new(
        event: &Event<ffi::xcb_button_press_event_t>,
        geo: Option<Reply<ffi::xcb_get_geometry_reply_t>>,
    ) -> Self {
        // Take in a mouse press event, and convert into a friendly struct
        Self {
            root_x: event.root_x(),
            root_y: event.root_y(),
            child: event.child(),
            detail: event.detail(),
            geo: geo.map(|geo| {
                (
                    i64::from(geo.x()),
                    i64::from(geo.y()),
                    u32::from(geo.width()),
                    u32::from(geo.height()),
                )
            }),
        }
    }

    pub fn motion(event: &Event<ffi::xcb_motion_notify_event_t>) -> Self {
        // Take in a mouse movement event, and convert to a friendly struct
        Self {
            root_x: event.root_x(),
            root_y: event.root_y(),
            child: event.child(),
            detail: event.detail(),
            geo: None,
        }
    }
}

// Utils.rs - Contains useful tools that help make code concise throughout.

// Helper macro for creating strings
#[macro_export]
macro_rules! st {
    ($value:expr) => {
        $value.to_string()
    };
}

// Helper macro for running commands
#[macro_export]
macro_rules! cmd {
    ($cmd:expr) => {{
        std::thread::spawn(move || {
            std::mem::drop(
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg($cmd)
                    .status(),
            );
        });
    }};
}

// Window.rs - Handles window arrangement and management
use crate::key::Key;

pub const BLACKLIST: [&str; 14] = [
    "_NET_WM_WINDOW_TYPE_DESKTOP",
    "_NET_WM_WINDOW_TYPE_COMBO",
    "_NET_WM_WINDOW_TYPE_MENU",
    "_NET_WM_WINDOW_TYPE_POPUP_MENU",
    "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU",
    "_NET_WM_WINDOW_TYPE_TOOLTIP",
    "_NET_WM_WINDOW_TYPE_UTILITY",
    "_NET_WM_WINDOW_TYPE_NOTIFICATION",
    "_NET_WM_WINDOW_TYPE_TOOLBAR",
    "_NET_WM_WINDOW_TYPE_SPLASH",
    "_NET_WM_WINDOW_TYPE_DIALOG",
    "_NET_WM_WINDOW_TYPE_DOCK",
    "_NET_WM_WINDOW_TYPE_DND",
    "WM_ZOOM_HINTS",
];

// Workspace struct that holds information about a specific workspace
pub struct Workspace {
    pub trigger: Key,
    floating: Vec<u32>,
    monocle: Option<u32>,
    pub previous_geometry: Option<(i64, i64, u32, u32)>,
    focus: usize,
}

impl Workspace {
    pub fn new<K: Into<Key>>(trigger: K) -> Self {
        // Create a new workspace
        Self {
            trigger: trigger.into(),
            floating: vec![],
            monocle: None,
            previous_geometry: None,
            focus: 0,
        }
    }

    pub fn add(&mut self, window: u32) {
        // Add window to this workspace
        self.floating.push(window);
        self.focus = self.floating.len().saturating_sub(1);
    }

    pub fn remove(&mut self, window: u32) {
        // Remove a window from this workspace
        self.floating.retain(|&w| w != window);
        // Fix focus if need be
        if self.focus >= self.floating.len() {
            self.focus = self.floating.len().saturating_sub(1);
        }
    }

    pub fn get_focus(&self) -> Option<u32> {
        // Get the currently focused window
        Some(*self.floating.get(self.focus)?)
    }

    pub fn set_focus(&mut self, window: u32) {
        // Set the currently focused window
        self.focus = self.find(window).unwrap();
    }

    pub fn set_monocle(&mut self) -> Option<u32> {
        // Set focused to monocle window
        let focus = self.get_focus()?;
        self.floating.retain(|&w| w != focus);
        self.monocle = Some(focus);
        return self.monocle;
    }

    pub fn get_monocle(&self) -> Option<u32> {
        // Get the current monocle
        self.monocle
    }

    pub fn clear_monocle(&mut self) -> Option<u32> {
        // Clear the monocle
        let monocle = self.monocle?;
        self.floating.insert(self.focus, monocle);
        self.monocle = None;
        Some(monocle)
    }

    pub fn show(&self, conn: &xcb::Connection) {
        // Show all windows within this workspace
        for window in &self.floating {
            xcb::map_window(conn, *window);
        }
        // Show monocled window if need be
        if let Some(monocle) = self.monocle {
            xcb::map_window(conn, monocle);
        }
    }

    pub fn hide(&self, conn: &xcb::Connection) {
        // Hide all windows within this workspace
        for window in &self.floating {
            xcb::unmap_window(conn, *window);
        }
        // Hide monocled window if need be
        if let Some(monocle) = self.monocle {
            xcb::unmap_window(conn, monocle);
        }
    }

    pub fn contains(&self, window: u32) -> bool {
        // Check if this workspace contains a window
        self.floating.contains(&window) || self.monocle == Some(window)
    }

    pub fn find(&self, window: u32) -> Option<usize> {
        // Find this window, returns None if not found, or if in monocle mode
        self.floating.iter().position(|w| w == &window)
    }
}

// Wm.rs - This is where all the magic happens
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
use crate::config::{Config, Handler};
use crate::key::{get_lookup, Key, SymTable, META, META_SHIFT};
use crate::mouse::MouseInfo;
use crate::window::{Workspace, BLACKLIST};
use xcb::{xproto, Connection};

// Shorthand for an X events
pub type XMapEvent<'a> = &'a xcb::MapNotifyEvent;
pub type XDestroyEvent<'a> = &'a xcb::DestroyNotifyEvent;
pub type XKeyEvent<'a> = &'a xcb::KeyPressEvent;
pub type XEnterEvent<'a> = &'a xcb::EnterNotifyEvent;
pub type XLeaveEvent<'a> = &'a xcb::LeaveNotifyEvent;
pub type XButtonPressEvent<'a> = &'a xcb::ButtonPressEvent;
pub type XMotionEvent<'a> = &'a xcb::MotionNotifyEvent;

// Ignore the David Bowie reference, this is the struct that controls X
pub struct StarMan {
    conn: Connection,
    conf: Config,
    keymap: SymTable,
    workspaces: Vec<Workspace>,
    workspace: usize,
    mouse: Option<MouseInfo>,
}

impl StarMan {
    pub fn new() -> Self {
        // Establish connection with X
        let (conn, _) = Connection::connect(None).expect("Failed to connect to X");
        let setup = conn.get_setup();
        let screen = setup.roots().next().unwrap();
        // Set up workspaces
        let workspaces = vec![
            // New workspace, triggered on [Meta] + [WORKSPACE NUMBER]
            Workspace::new((META, "1")),
            Workspace::new((META, "2")),
            Workspace::new((META, "3")),
            Workspace::new((META, "4")),
            Workspace::new((META, "5")),
            Workspace::new((META, "6")),
            Workspace::new((META, "7")),
            Workspace::new((META, "8")),
            Workspace::new((META, "9")),
            Workspace::new((META, "0")),
        ];
        // Call XInitThreads to.. well.. init threads
        unsafe {
            x11::xlib::XInitThreads();
        }

        // Establish grab for workspace trigger events
        let keymap = get_lookup(&conn);
        for trigger in workspaces.iter().map(|w| &w.trigger) {
            StarMan::grab_key(&conn, &screen, trigger, &keymap);
        }
        // Establish a grab for mouse events
        StarMan::grab_button(&conn, &screen, 1, META as u16);
        StarMan::grab_button(&conn, &screen, 1, META_SHIFT as u16);
        // Set root cursor as normal left pointer
        StarMan::set_cursor(&conn, &screen, 68);
        // Establish a grab for notification events
        StarMan::grab_notify_events(&conn, &screen);
        // Write buffer to server
        conn.flush();
        // Instantiate and return
        Self {
            keymap,
            workspaces,
            workspace: 0,
            conf: Config::new(),
            conn,
            mouse: None,
        }
    }

    pub fn run(&mut self) {
        // Start event loop
        loop {
            // Wait for event
            let event = self.conn.wait_for_event().unwrap();
            match event.response_type() {
                // On window map (window appears)
                xcb::MAP_NOTIFY => {
                    let map_notify: XMapEvent = unsafe { xcb::cast_event(&event) };
                    self.map_event(map_notify);
                }
                // On window destroy (window closes)
                xcb::DESTROY_NOTIFY => {
                    let destroy_notify: XDestroyEvent = unsafe { xcb::cast_event(&event) };
                    self.destroy_event(destroy_notify);
                }
                // On mouse entering a window
                xcb::ENTER_NOTIFY => {
                    let enter_notify: XEnterEvent = unsafe { xcb::cast_event(&event) };
                    self.enter_event(enter_notify);
                }
                // On mouse leaving a window
                xcb::LEAVE_NOTIFY => {
                    let leave_notify: XLeaveEvent = unsafe { xcb::cast_event(&event) };
                    self.leave_event(leave_notify);
                }
                // On mouse button press
                xcb::BUTTON_PRESS => {
                    let button_press: XButtonPressEvent = unsafe { xcb::cast_event(&event) };
                    self.button_press_event(button_press);
                }
                // On mouse movement
                xcb::MOTION_NOTIFY => {
                    let motion_event: XMotionEvent = unsafe { xcb::cast_event(&event) };
                    self.motion_event(motion_event);
                }
                // On mouse button release
                xcb::BUTTON_RELEASE => {
                    self.mouse = None;
                }
                // On key press
                xcb::KEY_PRESS => {
                    // Retrieve key code
                    let key_press: XKeyEvent = unsafe { xcb::cast_event(&event) };
                    self.key_event(key_press);
                }
                // Otherwise
                _ => (),
            }
            // Write buffer to server
            self.conn.flush();
        }
    }

    fn map_event(&mut self, map_notify: XMapEvent) {
        // Handle window map event
        let window = map_notify.window();
        // Ensure window type isn't on the blacklist
        let kind = self.get_atom_property(window, "_NET_WM_WINDOW_TYPE");
        let kind = xcb::get_atom_name(&self.conn, kind).get_reply().unwrap();
        if BLACKLIST.contains(&kind.name()) {
            return;
        }
        // Ensure that this window isn't already assigned to a workspace
        if self.workspaces.iter().any(|w| w.contains(window)) {
            return;
        }
        // Add to the workspace
        self.workspace_mut().add(window);
        // Grab the events where the cursor leaves and enters the window
        self.grab_enter_leave(window);
        // If in monocle, restore layer position
        if let Some(monocle) = self.workspace().get_monocle() {
            xcb::configure_window(
                &self.conn,
                monocle,
                &[(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)],
            );
            self.focus_window(monocle);
        } else {
            // Focus on this window
            self.focus_window(window);
        }
        // Give window a border
        self.border_unfocused(window);
        self.set_border_width(window, self.conf.unfocused_border.size);
    }

    fn destroy_event(&mut self, destroy_notify: XDestroyEvent) {
        // Handle window destroy event
        let window = destroy_notify.window();
        if self.is_monocle(window) {
            // Is monocle, clear monocle
            self.monocle_clear();
        }
        // Remove from workspace
        self.workspace_mut().remove(window);
        // Refocus
        if let Some(monocle) = self.workspace().get_monocle() {
            self.focus_window(monocle);
        } else {
            if let Some(target) = self.workspace().get_focus() {
                self.focus_window(target);
            }
        }
    }

    fn enter_event(&mut self, enter_notify: XEnterEvent) {
        // Handle window enter event
        let window = enter_notify.event();
        // Focus window
        xcb::configure_window(
            &self.conn,
            window,
            &[(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)],
        );

        self.border_focused(window);
        if !self.is_monocle(window) {
            self.focus_window(window);
            self.workspace_mut().set_focus(window);
        }
    }

    fn leave_event(&mut self, leave_notify: XLeaveEvent) {
        // Handle window leave event
        let window = leave_notify.event();
        self.border_unfocused(window);
    }

    fn border_unfocused(&mut self, window: u32) {
        // Change the border of a window to an unfocused border style
        xcb::change_window_attributes(
            &self.conn,
            window,
            &[(xcb::CW_BORDER_PIXEL, self.conf.unfocused_border.colour)],
        );
    }

    fn border_focused(&mut self, window: u32) {
        // Change the border of a window to a focused border style
        xcb::change_window_attributes(
            &self.conn,
            window,
            &[(xcb::CW_BORDER_PIXEL, self.conf.focused_border.colour)],
        );
    }

    fn button_press_event(&mut self, button_press: XButtonPressEvent) {
        // Handle mouse button click event
        if !self.is_monocle(button_press.child()) {
            // Window isn't in monocle mode
            let geo = xcb::get_geometry(&self.conn, button_press.child())
                .get_reply()
                .ok();
            self.mouse = Some(MouseInfo::new(button_press, geo));
        }
    }

    fn motion_event(&mut self, motion_event: XMotionEvent) {
        // Handle mouse motion event
        let resize = motion_event.state() == 321;
        if let Some(start) = self.mouse.as_ref() {
            let end = MouseInfo::motion(motion_event);
            // Calculate deltas
            let delta_x = i64::from(end.root_x - start.root_x);
            let delta_y = i64::from(end.root_y - start.root_y);
            if (delta_x == 0 && delta_y == 0) || start.detail != 1 {
                // Exit if only a click, or not using the left mouse button
                return;
            }
            // Move window if drag was performed
            if let Some(geo) = start.geo {
                if resize {
                    let w = i64::from(geo.2) + delta_x;
                    let h = i64::from(geo.3) + delta_y;
                    if w > 0 && h > 0 {
                        self.resize_window(start.child, w, h);
                    }
                } else {
                    let x = geo.0 as i64 + delta_x;
                    let y = geo.1 as i64 + delta_y;
                    self.move_window(start.child, x, y);
                }
            }
        }
    }

    fn key_event(&mut self, key_press: XKeyEvent) {
        // Handle key press events
        let code = st!(self.keymap[&key_press.detail()][0]);
        let modifiers = key_press.state();
        // Create key
        let key = Key::new(modifiers.into(), &code);
        // Check if user defined handler
        if let Some(handler) = self.conf.key(&key) {
            handler(self);
            return;
        }
        // Check for workspace trigger
        if let Some(idx) = self.workspaces.iter().position(|w| w.trigger == key) {
            // Exit if already focused
            if idx == self.workspace {
                return;
            }
            // Hide previous workspace windows
            self.workspace().hide(&self.conn);
            // Update index
            self.workspace = idx;
            // Show new workspace windows
            self.workspace().show(&self.conn);
            // Refocus monocle if need be
            if let Some(monocle) = self.workspace().get_monocle() {
                self.focus_window(monocle);
            }
        }
    }

    pub fn bind<K: Into<Key>>(&mut self, key: K, handler: Handler) {
        // Bind a key to a handler
        let key = key.into();
        let setup = self.conn.get_setup();
        let screen = setup.roots().next().unwrap();
        // Establish a grab on this shortcut
        StarMan::grab_key(&self.conn, &screen, &key, &self.keymap);
        // Perform the bind
        self.conf.bind_handler(key, handler);
    }

    pub fn destroy(&mut self, target: u32) {
        // Set up a destroy event
        let protocols = xcb::intern_atom(&self.conn, false, "WM_PROTOCOLS")
            .get_reply()
            .unwrap()
            .atom();
        let delete = xcb::intern_atom(&self.conn, false, "WM_DELETE_WINDOW")
            .get_reply()
            .unwrap()
            .atom();
        let data = xcb::ClientMessageData::from_data32([delete, xcb::CURRENT_TIME, 0, 0, 0]);
        let event = xcb::ClientMessageEvent::new(32, target, protocols, data);
        // Clear monocle if target is monocle
        if self.is_monocle(target) {
            self.monocle_clear();
        }
        // Send the event
        xcb::send_event(&self.conn, false, target, xcb::EVENT_MASK_NO_EVENT, &event);
    }

    pub fn destroy_focus(&mut self) {
        // Check that focus isn't monocle
        if self.workspace().get_monocle().is_some() {
            self.monocle_clear();
        }
        // Destroy the window that is currently focused on
        if let Some(target) = self.workspace().get_focus() {
            self.destroy(target);
        }
    }

    pub fn move_window_to_workspace(&mut self, workspace: usize) {
        // Move a window to a specific workspace
        if workspace == self.workspace {
            return;
        }
        // Get the focused window
        if let Some(focus) = self.workspace().get_focus() {
            // Remove from current workspace
            self.workspace_mut().remove(focus);
            // Unmap the window
            xcb::unmap_window(&self.conn, focus);
            // Add into new workspace and set focus
            self.workspaces[workspace].add(focus);
            self.workspaces[workspace].set_focus(focus);
        }
    }

    pub fn monocle_focus(&mut self) {
        // Set the monocle to the focused window
        if let Some(monocle) = self.workspace_mut().set_monocle() {
            // Get current window geometry
            let geo = xcb::get_geometry(&self.conn, monocle).get_reply().unwrap();
            self.workspace_mut().previous_geometry = Some((
                i64::from(geo.x()),
                i64::from(geo.y()),
                u32::from(geo.width()),
                u32::from(geo.height()),
            ));
            // Get window and border size
            let window = self.conn.get_setup().roots().next().unwrap();
            let (w, h) = (window.width_in_pixels(), window.height_in_pixels());
            let border = (self.conf.focused_border.size * 2) as i64;
            // Move and Resize
            self.reshape_window(monocle, 0, 0, w as i64 - border, h as i64 - border);
        }
    }

    pub fn monocle_clear(&mut self) {
        // Clear the monocle
        if let Some(monocle) = self.workspace_mut().clear_monocle() {
            let geo = std::mem::take(&mut self.workspace_mut().previous_geometry).unwrap();
            self.reshape_window(monocle, geo.0, geo.1, geo.2 as i64, geo.3 as i64);
        }
    }
    
    pub fn is_monocle(&mut self, window: u32) -> bool {
        // Returns true if the window provided is in monocle mode
        self.workspace().get_monocle() == Some(window)
    }

    fn move_window(&self, window: u32, x: i64, y: i64) {
        // Move a window to a specific X and Y coordinate
        xcb::configure_window(
            &self.conn,
            window,
            &[
                (xcb::CONFIG_WINDOW_X as u16, x as u32),
                (xcb::CONFIG_WINDOW_Y as u16, y as u32),
            ],
        );
    }

    fn resize_window(&self, window: u32, w: i64, h: i64) {
        // Resize a window to a specific W and H size
        xcb::configure_window(
            &self.conn,
            window,
            &[
                (xcb::CONFIG_WINDOW_WIDTH as u16, w as u32),
                (xcb::CONFIG_WINDOW_HEIGHT as u16, h as u32),
            ],
        );
    }

    fn reshape_window(&self, window: u32, x: i64, y: i64, w: i64, h: i64) {
        // Reshape a window to a specific position and size all in one
        xcb::configure_window(
            &self.conn,
            window,
            &[
                (xcb::CONFIG_WINDOW_X as u16, x as u32),
                (xcb::CONFIG_WINDOW_Y as u16, y as u32),
                (xcb::CONFIG_WINDOW_WIDTH as u16, w as u32),
                (xcb::CONFIG_WINDOW_HEIGHT as u16, h as u32),
            ],
        );
    }

    fn set_border_width(&self, window: u32, width: u32) {
        // Set the border width of a window
        xcb::configure_window(
            &self.conn,
            window,
            &[(xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, width)],
        );
    }

    fn set_cursor(conn: &xcb::Connection, screen: &xcb::Screen, k: u16) {
        // Set the cursor on the screen
        let f = conn.generate_id();
        xcb::open_font(conn, f, "cursor");
        let c = conn.generate_id();
        xcb::create_glyph_cursor(conn, c, f, f, k, k + 1, 0, 0, 0, 0xffff, 0xffff, 0xffff);
        xcb::change_window_attributes(conn, screen.root(), &[(xcb::CW_CURSOR, c)]);
    }

    #[rustfmt::skip]
    fn get_atom_property(&self, window: u32, property: &str) -> u32 {
        // Get a property from an atom
        let a = xcb::intern_atom(&self.conn, true, property)
            .get_reply()
            .unwrap()
            .atom();
        let prop = xproto::get_property(&self.conn, false, window, a, xproto::ATOM_ATOM, 0, 1024)
            .get_reply()
            .unwrap();
        if prop.value_len() == 0 { 42 } else { prop.value()[0] }
    }

    fn grab_button(conn: &xcb::Connection, screen: &xcb::Screen, button: u8, mods: u16) {
        // Tell X to grab all mouse events with specific modifiers and buttons
        xcb::grab_button(
            conn,
            false,
            screen.root(),
            (xcb::EVENT_MASK_BUTTON_PRESS
                | xcb::EVENT_MASK_BUTTON_RELEASE
                | xcb::EVENT_MASK_POINTER_MOTION) as u16,
            xcb::GRAB_MODE_ASYNC as u8,
            xcb::GRAB_MODE_ASYNC as u8,
            xcb::NONE,
            xcb::NONE,
            button,
            mods,
        );
    }

    fn grab_key(conn: &xcb::Connection, screen: &xcb::Screen, key: &Key, keymap: &SymTable) {
        // Tell X to grab all key events from a specific key
        for code in key.xcode(keymap) {
            xcb::grab_key(
                conn,
                false,
                screen.root(),
                key.mods as u16,
                code,
                xcb::GRAB_MODE_ASYNC as u8,
                xcb::GRAB_MODE_ASYNC as u8,
            );
        }
    }

    fn grab_notify_events(conn: &xcb::Connection, screen: &xcb::Screen) {
        // Tell X to grab all notify events on a screen
        StarMan::grab(
            conn,
            screen.root(),
            xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32,
        );
    }

    fn grab_enter_leave(&self, window: u32) {
        // Tell X to grab all enter and level events on screen
        StarMan::grab(
            &self.conn,
            window,
            xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW,
        );
    }

    fn focus_window(&self, window: u32) {
        // Tell X to set focus on a specific window
        xcb::set_input_focus(&self.conn, xcb::INPUT_FOCUS_PARENT as u8, window, 0);
    }

    fn grab(conn: &xcb::Connection, window: u32, events: u32) {
        // Generic helper function to set up an event grab on a window
        xcb::change_window_attributes(conn, window, &[(xcb::CW_EVENT_MASK, events)]);
    }

    pub fn workspace(&self) -> &Workspace {
        // Get the current workspace (immutable operations)
        &self.workspaces[self.workspace]
    }

    pub fn workspace_mut(&mut self) -> &mut Workspace {
        // Get the current workspace (mutable operations)
        &mut self.workspaces[self.workspace]
    }
}

// audio.rs - handling music playback
use crate::config::{Config, Database};
use crate::playlist::PlayList;
use crate::track::{Tag, Track};
use crate::util::form_library_tree;
use gstreamer::prelude::*;
use gstreamer::ClockTime;
use gstreamer_player::{Player, PlayerGMainContextSignalDispatcher, PlayerSignalDispatcher};
use std::collections::BTreeMap;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::time::Duration;

// Represents playback status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

// Represents loop status
#[derive(Debug, Clone, Copy)]
pub enum LoopStatus {
    None,
    Track,
    Playlist,
}

// Stores metadata to be transmitted between threads
#[derive(Debug, Clone)]
pub struct Metadata {
    pub playback_status: PlaybackStatus,
    pub loop_status: LoopStatus,
    pub shuffle_status: bool,
    pub volume: f64,
    pub position: (u64, u64, f64),
    pub tag: Tag,
}

// Main manager struct that handles everything
pub struct Manager {
    pub player: Player,
    pub playlist: PlayList,
    pub metadata: Arc<Mutex<Metadata>>,
    pub update_transmit: Sender<()>,
    pub mpris: Receiver<crate::mpris::Event>,
    pub config: Config,
    pub database: Database,
    pub library_tree: BTreeMap<String, BTreeMap<String, Vec<usize>>>,
    // TODO: Replace use of channels with mutexes on this variable.
    pub updated: bool,
}

impl Manager {
    pub fn new() -> Self {
        // Initiate gstreamer player
        gstreamer::init().unwrap();
        let dispatcher = PlayerGMainContextSignalDispatcher::new(None);
        let player = Player::new(None, Some(&dispatcher.upcast::<PlayerSignalDispatcher>()));
        // Set up channel to recieve and send events
        let (_, rx) = mpsc::sync_channel(32);
        // Placeholder channel
        let (tx2, _) = mpsc::channel();
        // Get config and generate library tree
        let database = Database::open();
        let library_tree = form_library_tree(&database.tracks);
        // Initiate player
        Self {
            // Create player
            player,
            // Initialise an empty playlist
            playlist: PlayList::default(),
            // Default placeholder values
            metadata: Arc::new(Mutex::new(Metadata {
                playback_status: PlaybackStatus::Stopped,
                loop_status: LoopStatus::None,
                shuffle_status: false,
                volume: 1.0,
                position: (0, 0, 0.0),
                tag: Tag::default(),
            })),
            // Add in mpris information channels
            mpris: rx,
            update_transmit: tx2,
            // Load in config file and library database
            config: Config::open(),
            database,
            library_tree,
            // Updated flag for UI
            updated: false,
        }
    }

    pub fn init(&mut self) {
        // Initialise this manager
        self.player.set_volume(1.0);
        // Set up channels
        let (tx, rx) = mpsc::sync_channel(32);
        let (tx2, rx2) = mpsc::channel();
        self.update_transmit = tx2;
        self.mpris = rx;
        // Event handler
        let ev = Arc::new(Mutex::new(move |event: crate::mpris::Event| {
            tx.send(event).ok();
        }));
        // Spawn mpris thread
        let md = self.metadata.clone();
        std::thread::spawn(move || crate::mpris::connect(ev, &md, &rx2));
    }

    pub fn open(&mut self, track: Track) {
        // If the track is already in the library, load it, otherwise, add it and then load it
        let mut found = None;
        for (id, value) in &self.database.tracks {
            if value == &track {
                found = Some(*id);
                break;
            }
        }
        if let Some(id) = found {
            self.load(id);
        } else {
            let idx = self.add_library(track);
            self.load(idx);
        }
    }

    pub fn load(&mut self, id: usize) {
        // Load a track into this player
        if let Some(track) = self.database.tracks.get(&id) {
            let mut md = self.metadata.lock().unwrap();
            md.playback_status = PlaybackStatus::Stopped;
            md.tag = track.tag.clone();
            self.playlist.play(track.clone(), id);
            self.player
                .set_uri(self.playlist.current().unwrap().path.as_str());
            std::mem::drop(md);
            self.update();
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn load_playlist(&mut self, playlist: &str) {
        // Load a playlist in
        let mut md = self.metadata.lock().unwrap();
        if let Some(load) = self.database.playlists.get(playlist) {
            let mut playlist = vec![];
            for id in load {
                playlist.push(self.database.tracks[id].clone());
            }
            self.playlist.set(0, playlist, load.clone());
            md.playback_status = PlaybackStatus::Stopped;
            if self.playlist.is_empty() {
                md.tag = Tag::default();
                self.player.set_uri("");
            } else {
                md.tag = self.playlist.current().unwrap().tag;
                self.player
                    .set_uri(self.playlist.current().unwrap().path.as_str());
            }
            std::mem::drop(md);
            self.update();
        } else {
            println!("ERROR: Couldn't find playlist: {}", playlist);
        }
    }

    pub fn new_playlist(&mut self, name: &str) {
        // Create a new playlist
        self.database.playlists.insert(name.to_string(), vec![]);
        self.database.display.playlists.push(name.to_string());
    }

    pub fn list_playlist(&mut self, name: &str) -> String {
        // List a playlist
        let mut result = format!("{}:\n", name);
        if let Some(load) = self.database.playlists.get(name) {
            for id in load {
                result.push_str(&format!("{}\n", self.database.tracks[id].format()));
            }
        } else {
            result = format!("ERROR: Couldn't find playlist: {}", name);
        }
        result
    }

    pub fn list_playlists(&self) -> String {
        // List all the playlists
        let mut result = String::new();
        for i in self.database.playlists.keys() {
            result.push_str(&format!("{}\n", i));
        }
        result
    }

    pub fn rename_playlist(&mut self, old: &str, new: &str) {
        // Rename a playlist to something else
        if let Some(val) = self.database.playlists.remove(old) {
            self.database.playlists.insert(new.to_string(), val);
            // Update playlist display
            let idx = self
                .database
                .display
                .playlists
                .iter()
                .position(|x| x == old)
                .unwrap();
            *self.database.display.playlists.get_mut(idx).unwrap() = new.to_string();
        } else {
            println!("ERROR: Couldn't find playlist: {}", old);
        }
    }

    pub fn delete_playlist(&mut self, name: &str) {
        // Delete a playlist
        if self.database.playlists.remove(name).is_none() {
            println!("ERROR: Couldn't find playlist: {}", name);
        } else if let Some(idx) = self
            .database
            .display
            .playlists
            .iter()
            .position(|x| x == name)
        {
            self.database.display.playlists.remove(idx);
        }
    }

    pub fn add_to_playlist(&mut self, playlist: &str, track: usize) {
        if let Some(load) = self.database.playlists.get_mut(playlist) {
            if self.database.tracks.len() > track {
                load.push(track);
            } else {
                println!("ERROR: Track ID out of range: {}", track);
            }
        } else {
            println!("ERROR: Couldn't find playlist: {}", playlist);
        }
    }

    pub fn remove_from_playlist(&mut self, playlist: &str, idx: usize) {
        if let Some(load) = self.database.playlists.get_mut(playlist) {
            load.remove(idx);
        } else {
            println!("ERROR: Couldn't find playlist: {}", playlist);
        }
    }

    pub fn queue(&mut self, id: usize) {
        // Queue a track
        if let Some(track) = self.database.tracks.get(&id) {
            self.playlist.queue(track.clone(), id);
        }
    }

    pub fn clear_queue(&mut self) {
        // Clear the queue and stop playback
        self.playlist.clear();
        self.stop();
    }

    pub fn play(&mut self) {
        // Play the current track
        if !self.playlist.is_empty() {
            let mut md = self.metadata.lock().unwrap();
            if md.playback_status == PlaybackStatus::Stopped {
                self.player.stop();
            }
            md.playback_status = PlaybackStatus::Playing;
            self.player.play();
            std::mem::drop(md);
            self.update();
        }
    }

    pub fn pause(&mut self) {
        // Pause the current track
        let mut md = self.metadata.lock().unwrap();
        md.playback_status = PlaybackStatus::Paused;
        self.player.pause();
        std::mem::drop(md);
        self.update();
    }

    pub fn play_pause(&mut self) {
        // Toggle play or pause on the track
        let status = self.metadata.lock().unwrap().playback_status;
        match status {
            PlaybackStatus::Paused | PlaybackStatus::Stopped => self.play(),
            PlaybackStatus::Playing => self.pause(),
        }
    }

    pub fn stop(&mut self) {
        // Stop the currently playing track
        let mut md = self.metadata.lock().unwrap();
        md.playback_status = PlaybackStatus::Stopped;
        self.player.stop();
        std::mem::drop(md);
        self.update();
    }

    pub fn next(&mut self) -> Option<()> {
        // Move to the next track
        let next = self.playlist.next()?;
        self.player.set_uri(&next.path);
        self.metadata.lock().unwrap().tag = next.tag;
        self.play();
        self.update();
        Some(())
    }

    pub fn previous(&mut self) -> Option<()> {
        // Move to the previous track
        let previous = self.playlist.previous()?;
        self.player.set_uri(&previous.path);
        self.metadata.lock().unwrap().tag = previous.tag;
        self.play();
        self.update();
        Some(())
    }

    pub fn set_loop(&mut self, s: LoopStatus) {
        // Set the loop status
        let mut md = self.metadata.lock().unwrap();
        md.loop_status = s;
        std::mem::drop(md);
        self.update();
    }

    pub fn cycle_loop(&mut self) {
        // Cycle through the loop statuses
        let mut md = self.metadata.lock().unwrap();
        md.loop_status = match md.loop_status {
            LoopStatus::None => LoopStatus::Track,
            LoopStatus::Track => LoopStatus::Playlist,
            LoopStatus::Playlist => LoopStatus::None,
        };
        std::mem::drop(md);
        self.update();
    }

    pub fn set_shuffle(&mut self, s: bool) {
        // Set the shuffle status
        let mut md = self.metadata.lock().unwrap();
        md.shuffle_status = s;
        std::mem::drop(md);
        self.update();
    }

    pub fn cycle_shuffle(&mut self) {
        // Toggle the shuffle option
        let mut md = self.metadata.lock().unwrap();
        md.shuffle_status = !md.shuffle_status;
        std::mem::drop(md);
        self.update();
    }

    pub fn seek(&mut self, forwards: bool, s: Duration) {
        // Perform a seek operation
        if self.metadata.lock().unwrap().playback_status != PlaybackStatus::Stopped {
            // Player is not stopped and ready to be seeked
            if let Some((mut position, duration, _)) = self.get_position() {
                // Update position
                position = if forwards {
                    position + s.as_secs()
                } else {
                    position.saturating_sub(s.as_secs())
                };
                if position > duration {
                    position = duration;
                }
                self.player.seek(ClockTime::from_seconds(position));
            }
        }
    }

    pub fn set_volume(&mut self, v: f64) {
        // Set the volume of the player
        if v >= 0.0 {
            let mut md = self.metadata.lock().unwrap();
            md.volume = v;
            self.player.set_volume(v);
            std::mem::drop(md);
            self.update();
        }
    }

    pub fn toggle_mute(&mut self) {
        // Toggle the mute option
        let md = self.metadata.lock().unwrap();
        if self.player.volume() == 0.0 {
            self.player.set_volume(md.volume);
        } else {
            self.player.set_volume(0.0);
        }
        std::mem::drop(md);
        self.update();
    }

    pub fn set_position(&mut self, p: i64) {
        // Set the position of the player
        if let Some((_, duration, _)) = self.get_position() {
            let p = p.try_into().unwrap();
            if p > duration {
                return;
            }
            self.player.seek(ClockTime::from_seconds(p));
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn get_position(&self) -> Option<(u64, u64, f64)> {
        // Work out the current position of the player
        let time_pos = ClockTime::seconds(self.player.position()?);
        // Work out the duration of the current track
        let duration = ClockTime::seconds(self.player.duration()?);
        // Tupleize above values, and calculate the percentage way through
        let data = (time_pos, duration, time_pos as f64 / (duration as f64));
        // Update the position for mpris to read
        self.metadata.lock().unwrap().position = data;
        // Return formed values
        Some(data)
    }

    pub fn list_library(&self) -> String {
        // List all the tracks in the library
        let mut keys: Vec<usize> = self.database.tracks.keys().copied().collect();
        keys.sort_unstable();
        let mut result = String::new();
        for id in keys {
            result.push_str(&format!("{}: {}\n", id, self.database.tracks[&id].format()));
        }
        result
    }

    pub fn add_library(&mut self, track: Track) -> usize {
        // Add a track to the library
        let mut keys: Vec<usize> = self.database.tracks.keys().copied().collect();
        keys.sort_unstable();
        let mut i = 0;
        let mut result = None;
        for k in &keys {
            if i != *k {
                result = Some(i);
                break;
            }
            i += 1;
        }
        let result = result.unwrap_or(i);
        self.database.tracks.insert(result, track);
        self.database.display.simple.push(result);
        result
    }

    pub fn remove_library(&mut self, id: usize) {
        // Remove a track from the library
        self.database.tracks.remove(&id);
        // Remove from display
        let display_idx = self
            .database
            .display
            .simple
            .iter()
            .position(|x| *x == id)
            .unwrap();
        self.database.display.simple.remove(display_idx);
        // Remove from playlists
        for values in self.database.playlists.values_mut() {
            if let Some(idx) = values.iter().position(|x| *x == id) {
                values.remove(idx);
            }
        }
    }

    pub fn set_title(&mut self, id: usize, new: &str) {
        // Set the title of a track
        if let Some(track) = self.database.tracks.get_mut(&id) {
            track.set_title(new);
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn set_album(&mut self, id: usize, new: &str) {
        // Set the album of a track
        if let Some(track) = self.database.tracks.get_mut(&id) {
            track.set_album(new);
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn set_artist(&mut self, id: usize, new: &str) {
        // Set the artist of a track
        if let Some(track) = self.database.tracks.get_mut(&id) {
            track.set_artist(new);
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn set_year(&mut self, id: usize, new: &str) {
        // Set the year of a track
        if let Some(track) = self.database.tracks.get_mut(&id) {
            track.set_year(new);
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn update_tag(&mut self, id: usize) {
        // Reread the tags of a track
        if let Some(track) = self.database.tracks.get_mut(&id) {
            track.update();
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn view_track(&mut self, id: usize) {
        // View track metadata
        if let Some(track) = self.database.tracks.get_mut(&id) {
            println!("{}", track.format());
        } else {
            println!("ERROR: Track ID out of range: {}", id);
        }
    }

    pub fn update(&mut self) {
        // Send the update signal for mpris to update it's values
        self.updated = true;
        self.update_transmit.send(()).unwrap();
    }
}

// config.rs - manage config file and databases
use crate::track::Track;
use crate::util::{attempt_open, expand_path};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Default configuration and database formats
const DEFAULT_CONFIG: &str = include_str!("../synchron.ron");
const DEFAULT_DATABASE: &str = include_str!("../database.ron");
// Thread pulse time for rendering, and dbus actions.
// Lower = Quicker reaction times, worse performance
// Higher = Slower reaction times, better performance
pub const PULSE: u64 = 200;
pub const DBUS_PULSE: u64 = 500;

#[derive(Debug, Deserialize, Serialize)]
pub enum Pane {
    SimpleLibrary,
    SortedLibrary,
    Playlists,
    Files,
    Empty,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub prompt: String,
    pub panes: HashMap<u8, Pane>,
    pub open_on_pane: u8,
    pub indicators: HashMap<String, String>,
    pub show_hidden_files: bool,
}

impl Config {
    pub fn open() -> Self {
        if let Some(config) = attempt_open("~/.config/synchron.ron") {
            // Attempt opening config file in ~/.config directory
            ron::from_str(&config).expect("Invalid config file format!")
        } else if let Some(config) = attempt_open("./synchron.ron") {
            // Attempt opening config file from current working directory
            ron::from_str(&config).expect("Invalid config file format!")
        } else {
            // Use embedded config
            println!("Note: using default config");
            ron::from_str(DEFAULT_CONFIG).expect("Invalid config file format!")
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Display {
    pub simple: Vec<usize>,
    pub playlists: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Database {
    pub tracks: HashMap<usize, Track>,
    pub playlists: HashMap<String, Vec<usize>>,
    pub display: Display,
}

impl Database {
    pub fn open() -> Self {
        // Attempt to open the database
        let path_base =
            expand_path("~/.local/share").unwrap_or_else(|| "~/.local/share".to_string());
        std::fs::create_dir_all(format!("{}/synchron/", path_base)).ok();
        let path_full = format!("{}/synchron/database.ron", path_base);
        if std::path::Path::new(&path_full).exists() {
            // File exists
            if let Some(database) = attempt_open("~/.local/share/synchron/database.ron") {
                // Database read sucessfully
                ron::from_str(&database).expect("Database is corrupted")
            } else {
                // Failed to read database, use empty one
                println!("Note: failed to open database, using empty database");
                ron::from_str(DEFAULT_DATABASE).expect("Database is corrupted")
            }
        } else {
            // File doesn't exist, attempt to write an empty one
            println!("Note: Database not detected, creating empty database");
            if std::fs::write(&path_full, DEFAULT_DATABASE).is_err() {
                // Failed to create database, display error
                println!("ERROR: Failed to create database, using empty database");
            }
            // Read in an empty database
            ron::from_str(DEFAULT_DATABASE).expect("Database is corrupted")
        }
    }

    pub fn write(&self) {
        let path_base =
            expand_path("~/.local/share").unwrap_or_else(|| "~/.local/share".to_string());
        std::fs::create_dir_all(format!("{}/synchron/", path_base)).ok();
        let path_full = format!("{}/synchron/database.ron", path_base);
        if !std::path::Path::new(&path_full).exists() {
            println!("Warning: Database not found, these changes will not be saved");
            return;
        }
        if let Ok(write) = ron::ser::to_string(self) {
            if std::fs::write(path_full, write).is_err() {
                println!("ERROR: Failed to write to disk");
            }
        }
    }
}

/*
    Synchron - A terminal music player
    - Allows control through dbus, integrating into your bar and playerctl
    - Reads ID3 tags from music
    - Can be controlled through prompt
    - Can play most mainstream formats
*/

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_sign_loss)]
#![feature(hash_drain_filter)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_use]
mod util;
mod audio;
mod config;
mod mpris;
mod playlist;
mod track;
mod ui;

use audio::{LoopStatus, Manager, PlaybackStatus};
use config::PULSE;
use jargon_args::Jargon;
use mpris::Event;
use scanln::scanln;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use track::Track;
use ui::Ui;
use util::HELP;

fn main() {
    // Parse command line arguments
    let mut args = Jargon::from_env();
    // Handle help and version message
    if args.contains(["-h", "--help"]) {
        println!("{}", HELP);
        std::process::exit(0);
    } else if args.contains(["-V", "--version"]) {
        println!("v{}", VERSION);
        std::process::exit(0);
    }
    // Start into the correct mode
    if args.contains(["-c", "--cli"]) {
        start_cli();
    } else {
        start_tui();
    }
}

fn start_tui() {
    // Handle any panics that may occur
    std::panic::set_hook(Box::new(|e| {
        crossterm::terminal::disable_raw_mode().unwrap();
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )
        .ok();
        eprintln!("{}", e);
    }));
    // Build and initialise a manager
    let mut m = Manager::new();
    m.init();
    // Allow for it to be accessed from threads
    let m = Arc::new(Mutex::new(m));
    // Start mpris event loop
    spawn_mpris(&m);
    // Initiate a text user interface
    if let Ok(mut ui) = Ui::new(m) {
        // Initiate UI lifecycle
        ui.init().ok();
        ui.run().ok();
        ui.clean().ok();
    }
}

fn start_cli() {
    // Build and initialise a manager
    let mut m = Manager::new();
    m.init();
    // Allow for it to be accessed from threads
    let m = Arc::new(Mutex::new(m));
    // Start mpris event loop
    spawn_mpris(&m);
    // Initiate a control prompt for the player
    loop {
        let cmd = scanln!("{}", m.lock().unwrap().config.prompt);
        let mut m = m.lock().unwrap();
        match cmd.as_str().split(' ').collect::<Vec<&str>>().as_slice() {
            // Opening media
            ["open", "playlist", p] => m.load_playlist(p),
            ["open", t] => m.load(t.parse().unwrap_or(0)),
            // File tagging
            ["tag", "title", i, t @ ..] => m.set_title(i.parse().unwrap_or(0), &t.join(" ")),
            ["tag", "album", i, a @ ..] => m.set_album(i.parse().unwrap_or(0), &a.join(" ")),
            ["tag", "artist", i, a @ ..] => m.set_artist(i.parse().unwrap_or(0), &a.join(" ")),
            ["tag", "year", i, y] => m.set_year(i.parse().unwrap_or(0), y),
            ["tag", "update", i] => m.update_tag(i.parse().unwrap_or(0)),
            ["tag", i] => m.view_track(i.parse().unwrap_or(0)),
            // Library commands
            ["library"] => println!("{}", m.list_library()),
            ["library", "add", o @ ..] => {
                let _ = m.add_library(Track::load(&o.join(" ")));
            }
            ["library", "remove", i] => m.remove_library(i.parse().unwrap_or(0)),
            // Queue and playlist handling
            ["playlist", "add", p, i] => m.add_to_playlist(p, i.parse().unwrap_or(0)),
            ["playlist", "remove", p, i] => m.remove_from_playlist(p, i.parse().unwrap_or(0)),
            ["playlist", "new", p] => m.new_playlist(p),
            ["playlist"] => println!("{}", m.list_playlists()),
            ["playlist", p] => println!("{}", m.list_playlist(p)),
            ["playlist", "delete", p] => m.delete_playlist(p),
            ["playlist", "rename", o, n] => m.rename_playlist(o, n),
            ["queue", t] => m.queue(t.parse().unwrap_or(0)),
            ["clear"] => m.clear_queue(),
            ["next"] => m.next().unwrap_or(()),
            ["prev"] => m.previous().unwrap_or(()),
            // Metadata
            ["status"] => {
                let (p, d, pr) = m.get_position().unwrap_or((0, 0, 0.0));
                println!("{}s / {}s ({:.2}%)\n", p, d, pr * 100.);
                print!("{}", m.playlist.view());
            }
            // Playing and pausing commands
            ["toggle"] => m.play_pause(),
            ["play"] => m.play(),
            ["pause"] => m.pause(),
            ["stop"] => m.stop(),
            // Loop controls
            ["loop", "off"] => m.set_loop(LoopStatus::None),
            ["loop", "track"] => m.set_loop(LoopStatus::Track),
            ["loop", "playlist"] => m.set_loop(LoopStatus::Playlist),
            ["loop", "get"] => println!("{:?}", m.metadata.lock().unwrap().loop_status),
            // Shuffle controls
            ["shuffle", "on"] => m.set_shuffle(true),
            ["shuffle", "off"] => m.set_shuffle(false),
            ["shuffle", "get"] => println!(
                "{}",
                if m.metadata.lock().unwrap().shuffle_status {
                    "On"
                } else {
                    "Off"
                }
            ),
            // Volume controls
            ["volume", "up"] => {
                let volume = m.metadata.lock().unwrap().volume;
                m.set_volume(volume + 0.3);
            }
            ["volume", "down"] => {
                let volume = m.metadata.lock().unwrap().volume;
                m.set_volume(volume - 0.3);
            }
            ["volume", "set", v] => m.set_volume(v.parse().unwrap_or(1.0)),
            ["volume", "get"] => println!("{}", m.metadata.lock().unwrap().volume),
            ["volume", "reset"] => m.set_volume(1.0),
            // Position controls
            ["position", "set", p] => m.set_position(p.parse().unwrap_or(-1)),
            ["position", "get"] => {
                let (p, d, pr) = m.get_position().unwrap_or((0, 0, 0.0));
                println!("{}s / {}s ({:.2}%)", p, d, pr * 100.);
            }
            ["seek", "backward"] => m.seek(false, Duration::from_secs(5)),
            ["seek", "forward"] => m.seek(true, Duration::from_secs(5)),
            // Exit player
            ["exit"] => {
                m.database.write();
                std::process::exit(0)
            }
            // Unknown command
            _ => println!("Unknown command: '{}'", cmd),
        }
        std::mem::drop(m);
    }
}

fn spawn_mpris(m: &Arc<Mutex<Manager>>) {
    // Spawn a manager event loop, which handles mpris requests
    std::thread::spawn({
        let m = m.clone();
        move || {
            // Handle events
            loop {
                // Handle mpris event
                let mut m = m.lock().unwrap();
                if let Ok(e) = m.mpris.try_recv() {
                    match e {
                        Event::OpenUri(uri) => m.open(Track::load(&uri)),
                        Event::Pause => m.pause(),
                        Event::Play => m.play(),
                        Event::PlayPause => m.play_pause(),
                        Event::SetVolume(v) => m.set_volume(v),
                        Event::SetLoopStatus(s) => m.set_loop(s),
                        Event::SetShuffleStatus(s) => m.set_shuffle(s),
                        Event::SetPosition(p) => m.set_position(p),
                        Event::Seek(f, s) => m.seek(f, s),
                        Event::Stop => m.stop(),
                        Event::Next => m.next().unwrap_or(()),
                        Event::Previous => m.previous().unwrap_or(()),
                        Event::Raise | Event::Quit => (),
                    }
                }

                // Stop status after track has finished
                let status = m.metadata.lock().unwrap().playback_status;
                #[allow(clippy::float_cmp)]
                if m.get_position().unwrap_or((0, 0, 0.0)).2 == 1.
                    && status != PlaybackStatus::Stopped
                {
                    m.metadata.lock().unwrap().playback_status = PlaybackStatus::Stopped;
                    m.next();
                    m.update();
                }
                std::mem::drop(m);
                // Wait before next loop
                std::thread::sleep(Duration::from_millis(PULSE));
            }
        }
    });
}

// mpris.rs - handling mpris interactions
use crate::audio::{LoopStatus, Metadata};
use crate::config::DBUS_PULSE;
use crate::track::Tag;
use dbus::arg::{RefArg, Variant};
use dbus::blocking::Connection;
use dbus::channel::MatchingReceiver;
use dbus::ffidisp::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged as Ppc;
use dbus::message::SignalArgs;
use dbus::strings::Path as DbusPath;
use dbus::MethodErr;
use dbus_crossroads::{Crossroads, IfaceBuilder};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

// Types
type EventHandler = Arc<Mutex<dyn Fn(Event) + Send + 'static>>;

// Representation of control events
#[derive(Clone, Debug)]
pub enum Event {
    OpenUri(String),
    SetLoopStatus(LoopStatus),
    SetShuffleStatus(bool),
    SetPosition(i64),
    SetVolume(f64),
    Seek(bool, Duration),
    Play,
    Pause,
    PlayPause,
    Next,
    Previous,
    Stop,
    Raise,
    Quit,
}

#[allow(clippy::too_many_lines)]
pub fn connect(ev: EventHandler, md: &Arc<Mutex<Metadata>>, update: &mpsc::Receiver<()>) {
    // Names of the player
    let name = "synchron".to_string();
    let name2 = "org.mpris.MediaPlayer2.synchron";
    // Establish connection to dbus
    let c = Connection::new_session().unwrap();
    c.request_name(name2, false, true, false).unwrap();
    let mut cr = Crossroads::new();
    // Register MediaPlayer2
    let mp2 = cr.register("org.mpris.MediaPlayer2", {
        let ev = ev.clone();
        move |b| {
            register(b, &ev, "Raise", Event::Raise);
            register(b, &ev, "Quit", Event::Quit);
            b.property("Identity").get(move |_, _| Ok(name.clone()));
            b.property("CanQuit").get(move |_, _| Ok(true));
            b.property("CanRaise").get(move |_, _| Ok(true));
            b.property("HasTrackList").get(move |_, _| Ok(false));
            b.property("SupportedUriSchemes")
                .get(move |_, _| Ok(&[] as &[String]));
            b.property("SupportedMimeTypes")
                .get(move |_, _| Ok(&[] as &[String]));
        }
    });
    // Register Player
    let player_md = md.clone();
    let mp2p = cr.register("org.mpris.MediaPlayer2.Player", move |b| {
        // Register play, pause, next, preivous and stop events
        register(b, &ev, "Play", Event::Play);
        register(b, &ev, "Pause", Event::Pause);
        register(b, &ev, "PlayPause", Event::PlayPause);
        register(b, &ev, "Next", Event::Next);
        register(b, &ev, "Previous", Event::Previous);
        register(b, &ev, "Stop", Event::Stop);
        // Necessary for mpris
        b.property("CanControl").get(|_, _| Ok(true));
        b.property("CanPlay").get(|_, _| Ok(true));
        b.property("CanPause").get(|_, _| Ok(true));
        b.property("CanGoNext").get(|_, _| Ok(true));
        b.property("CanGoPrevious").get(|_, _| Ok(true));
        b.property("CanSeek").get(|_, _| Ok(true));
        // Get the playback status from the metadata
        b.property("PlaybackStatus").get({
            let md = player_md.clone();
            move |_, _| Ok(format!("{:?}", md.lock().unwrap().playback_status))
        });
        // Get and set the loop status from the metadata
        b.property("LoopStatus")
            .get({
                let md = player_md.clone();
                move |_, _| Ok(format!("{:?}", md.lock().unwrap().loop_status))
            })
            .set({
                let ev = ev.clone();
                move |_, _, status| {
                    // Trigger loop set event
                    (ev.lock().unwrap())(Event::SetLoopStatus(match status.as_str() {
                        "Track" => LoopStatus::Track,
                        "Playlist" => LoopStatus::Playlist,
                        _ => LoopStatus::None,
                    }));
                    Ok(None)
                }
            });
        // Get and set the shuffle status from the metadata
        b.property("Shuffle")
            .get({
                let md = player_md.clone();
                move |_, _| Ok(md.lock().unwrap().shuffle_status)
            })
            .set({
                let ev = ev.clone();
                move |_, _, status| {
                    // Trigger shuffle set event
                    (ev.lock().unwrap())(Event::SetShuffleStatus(status));
                    Ok(None)
                }
            });
        // Get the position status from the metadata
        b.property("Position").get({
            let md = player_md.clone();
            move |_, _| -> Result<i64, MethodErr> {
                Ok(md.lock().unwrap().position.0.try_into().unwrap())
            }
        });
        b.property("Volume")
            .get({
                let md = player_md.clone();
                move |_, _| -> Result<f64, MethodErr> { Ok(md.lock().unwrap().volume) }
            })
            .set({
                let ev = ev.clone();
                move |_, _, volume| {
                    // Trigger volume set event
                    (ev.lock().unwrap())(Event::SetVolume(volume));
                    Ok(None)
                }
            });
        // Get and format the track information from the metadata
        b.property("Metadata").get({
            let md = player_md.clone();
            move |_, _| {
                let mut export = mpris_metadata(&md.lock().unwrap().tag);
                export.insert(
                    "mpris:trackid".to_string(),
                    Variant(Box::new(DbusPath::new("/").unwrap())),
                );
                Ok(export)
            }
        });
        // Method to set the position as requested through dbus
        b.method("SetPosition", ("TrackID", "Position"), (), {
            let ev = ev.clone();
            move |_, _, (_, position): (DbusPath, i64)| {
                // Send to event handler, in the correct format (seconds)
                (ev.lock().unwrap())(Event::SetPosition(
                    Duration::from_micros(position.try_into().unwrap_or(0))
                        .as_secs()
                        .try_into()
                        .unwrap_or(0),
                ));
                Ok(())
            }
        });
        // Method for seeking
        b.method("Seek", ("Offset",), (), {
            let ev = ev.clone();
            move |_, _, (offset,): (i64,)| {
                // Work out direction and magnitude of seek
                let magnitude = offset.abs() as u64;
                let forwards = offset > 0;
                // Send to event handler
                (ev.lock().unwrap())(Event::Seek(forwards, Duration::from_micros(magnitude)));
                Ok(())
            }
        });
        // Method to open a new media file
        b.method("OpenUri", ("Uri",), (), {
            move |_, _, (uri,): (String,)| {
                // Send to event handler
                (ev.lock().unwrap())(Event::OpenUri(uri));
                Ok(())
            }
        });
    });
    // Insert into mpris
    cr.insert("/org/mpris/MediaPlayer2", &[mp2, mp2p], ());
    // Start recieving events
    c.start_receive(
        dbus::message::MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cr.handle_message(msg, conn).unwrap();
            true
        }),
    );
    // Start server loop
    loop {
        if update.try_recv().is_ok() {
            // When an update event is received, update information in the player
            let m = md.lock().unwrap();
            let mut changed = Ppc {
                interface_name: "org.mpris.MediaPlayer2.Player".to_string(),
                ..Ppc::default()
            };
            // Attach information
            add_prop!(
                changed.changed_properties,
                "PlaybackStatus",
                format!("{:?}", m.playback_status)
            );
            add_prop!(
                changed.changed_properties,
                "LoopStatus",
                format!("{:?}", m.loop_status)
            );
            add_prop!(changed.changed_properties, "Shuffle", m.shuffle_status);
            add_prop!(changed.changed_properties, "Volume", m.volume);
            add_prop!(
                changed.changed_properties,
                "Metadata",
                mpris_metadata(&m.tag)
            );
            // Send the message
            c.channel()
                .send(changed.to_emit_message(
                    &DbusPath::new("/org/mpris/MediaPlayer2".to_string()).unwrap(),
                ))
                .unwrap();
        }
        // Wait before checking again
        c.process(std::time::Duration::from_millis(DBUS_PULSE))
            .unwrap();
    }
}

pub fn register(b: &mut IfaceBuilder<()>, ev: &EventHandler, name: &'static str, event: Event) {
    // Register a new event for an event handler
    let ev = ev.clone();
    b.method(name, (), (), move |_, _, _: ()| {
        (ev.lock().unwrap())(event.clone());
        Ok(())
    });
}

fn mpris_metadata(tag: &Tag) -> HashMap<String, Variant<Box<dyn RefArg>>> {
    // Create a hashmap of id3 tags for mpris
    let mut md: HashMap<String, Variant<Box<dyn RefArg>>> = HashMap::new();
    add_prop!(md, "xesam:title", tag.title.clone());
    add_prop!(md, "xesam:album", tag.album.clone());
    add_prop!(md, "xesam:artist", tag.artist.clone());
    add_prop!(md, "xesam:year", tag.year.clone());
    md
}

// playlist.rs - tools for mananging playlists and queuing for the next and previous operations
use crate::Track;

#[derive(Default)]
pub struct PlayList {
    tracks: Vec<Track>,
    pub ids: Vec<usize>,
    pub ptr: Option<usize>,
    pub name: Option<String>,
}

impl PlayList {
    pub fn queue(&mut self, track: Track, id: usize) {
        // Add song onto the end of the playlist
        self.tracks.push(track);
        self.ids.push(id);
    }

    pub fn queue_next(&mut self, track: Track, id: usize) {
        // Add song to play immediately after the current one
        self.tracks.insert(self.get_ptr() + 1, track);
        self.ids.insert(self.get_ptr() + 1, id);
    }

    pub fn play(&mut self, track: Track, id: usize) -> Option<Track> {
        // Immediately add song and start playing it
        if !self.is_ready() {
            self.ptr = Some(0);
        }
        if self.tracks.is_empty() {
            self.queue(track, id);
            self.current()
        } else {
            self.queue_next(track, id);
            self.next()
        }
    }

    pub fn set(&mut self, ptr: usize, tracks: Vec<Track>, ids: Vec<usize>) {
        // Insert a custom playlist to use, as well as an index to start from
        self.ptr = Some(ptr);
        self.tracks = tracks;
        self.ids = ids;
    }

    pub fn clear(&mut self) {
        // Clear the playlist
        self.tracks.clear();
        self.ids.clear();
        self.ptr = Some(0);
    }

    pub fn next(&mut self) -> Option<Track> {
        // Switch to the next track in the queue
        if self.ptr? + 1 >= self.tracks.len() {
            None
        } else {
            self.ptr = Some(self.ptr? + 1);
            self.current()
        }
    }

    pub fn previous(&mut self) -> Option<Track> {
        // Switch to the previously played track
        if self.ptr? > 0 {
            self.ptr = Some(self.ptr? - 1);
            self.current()
        } else {
            None
        }
    }

    pub fn current_id(&self) -> Option<usize> {
        // Get the currently playing track ID
        Some(*self.ids.get(self.ptr?)?)
    }

    pub fn current(&self) -> Option<Track> {
        // Get the currently playing track
        if !self.is_ready() {
            return None;
        }
        Some(self.tracks.get(self.ptr?)?.clone())
    }

    pub fn is_ready(&self) -> bool {
        self.ptr.is_some()
    }

    pub fn get_ptr(&self) -> usize {
        self.ptr.unwrap()
    }

    pub fn move_down(&mut self, ptr: usize) {
        // Move a particular track downwards
        self.tracks.swap(ptr, ptr + 1);
        self.ids.swap(ptr, ptr + 1);
        if ptr == self.get_ptr() {
            self.ptr = Some(self.get_ptr() + 1);
        }
    }

    pub fn move_up(&mut self, ptr: usize) {
        // Move a particular track upwards
        self.tracks.swap(ptr, ptr.saturating_sub(1));
        self.ids.swap(ptr, ptr.saturating_sub(1));
        if ptr == self.get_ptr() {
            self.ptr = Some(self.get_ptr().saturating_sub(1));
        }
    }

    pub fn move_next(&mut self, ptr: usize) {
        // Move a particular song in this queue to play next
        let track = self.tracks.remove(ptr);
        let id = self.ids.remove(ptr);
        self.queue_next(track, id);
    }

    pub fn view(&mut self) -> String {
        let mut result = String::new();
        for (c, track) in self.tracks.iter().enumerate() {
            result.push_str(&format!(
                "{}{}\n",
                if c == self.get_ptr() { "-> " } else { "   " },
                track.format()
            ));
        }
        result
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

// track.rs - for managing track related activities
use crate::util::expand_path;
use id3::Version;
use serde::{Deserialize, Serialize};

// For holding tag information
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Tag {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub year: String,
}

impl Tag {
    pub fn from_id3(tag: &id3::Tag) -> Self {
        // Load from id3 tag
        Self {
            title: tag.title().unwrap_or("[unknown]").to_string(),
            album: tag.album().unwrap_or("[unknown]").to_string(),
            artist: tag.artist().unwrap_or("[unknown]").to_string(),
            year: tag.year().unwrap_or(0).to_string(),
        }
    }
}

impl Default for Tag {
    fn default() -> Self {
        // Default value for a tag
        Self {
            title: "[unknown]".to_string(),
            album: "[unknown]".to_string(),
            artist: "[unknown]".to_string(),
            year: "0".to_string(),
        }
    }
}

// Track struct to handle file reading, and tag extraction
#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
pub struct Track {
    pub path: String,
    pub tag: Tag,
}

impl Track {
    pub fn load(path: &str) -> Self {
        // Expand provided path, read the tags and create new instance
        let path = Track::format_path(path);
        let path = expand_path(&path).expect("File not found");
        let tag = id3::Tag::read_from_path(&path).unwrap_or_else(|_| id3::Tag::new());
        let path = format!("file://{}", path);
        Self {
            path,
            tag: Tag::from_id3(&tag),
        }
    }

    pub fn set_title(&mut self, title: &str) {
        // Set the title of this track
        let path = Track::format_path(&self.path);
        if let Ok(mut tag) = id3::Tag::read_from_path(&path) {
            tag.set_title(title);
            self.tag.title = title.to_string();
            tag.write_to_path(path, Version::Id3v24).ok();
        }
    }

    pub fn set_album(&mut self, album: &str) {
        // Set the title of this track
        let path = Track::format_path(&self.path);
        if let Ok(mut tag) = id3::Tag::read_from_path(&path) {
            tag.set_album(album);
            self.tag.album = album.to_string();
            tag.write_to_path(path, Version::Id3v24).ok();
        }
    }

    pub fn set_artist(&mut self, artist: &str) {
        // Set the title of this track
        let path = Track::format_path(&self.path);
        if let Ok(mut tag) = id3::Tag::read_from_path(&path) {
            tag.set_artist(artist);
            self.tag.artist = artist.to_string();
            tag.write_to_path(path, Version::Id3v24).ok();
        }
    }

    pub fn set_year(&mut self, year: &str) {
        // Set the title of this track
        let path = Track::format_path(&self.path);
        if let Ok(mut tag) = id3::Tag::read_from_path(&path) {
            tag.set_year(year.parse().unwrap_or(0));
            self.tag.year = year.to_string();
            tag.write_to_path(path, Version::Id3v24).ok();
        }
    }

    pub fn update(&mut self) {
        let path = Track::format_path(&self.path);
        if let Ok(tag) = id3::Tag::read_from_path(&path) {
            self.tag = Tag::from_id3(&tag);
        }
    }

    pub fn format_path(path: &str) -> String {
        // Unify the path format
        path.trim_start_matches("file://").to_string()
    }

    pub fn format_elements(&self) -> (String, &String, &String, &String, &String) {
        let tag = &self.tag;
        (
            Track::format_path(&self.path),
            &tag.title,
            &tag.album,
            &tag.artist,
            &tag.year,
        )
    }

    pub fn format(&self) -> String {
        let (path, title, album, artist, year) = self.format_elements();
        format!("{} | {} | {} | {} | {}", path, title, album, artist, year)
    }
}

// ui.rs - controls and renders the TUI
use crate::audio::{LoopStatus, Manager, PlaybackStatus};
use crate::config::{Pane, PULSE};
use crate::track::Track;
use crate::util::{
    align_sides, artist_tracks, expand_path, form_library_tree, format_artist_track,
    format_playlist, format_table, is_file, list_dir, pad_table, timefmt,
};
pub use crossterm::{
    cursor,
    event::{self, Event, KeyCode as KCode, KeyEvent, KeyModifiers as KMod},
    execute, queue,
    style::{self, Color, Print, SetBackgroundColor as SetBg, SetForegroundColor as SetFg},
    terminal::{self, ClearType},
    Command, Result,
};
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type OptionList = Option<Vec<String>>;
type TrackList = (Option<Vec<usize>>, OptionList);
type SortedList = Option<Vec<String>>;
type FileList = OptionList;

pub struct Size {
    width: u16,
    height: u16,
}

impl Size {
    pub fn screen() -> Result<Self> {
        // Form a Size struct from the screen size
        let (width, height) = terminal::size()?;
        Ok(Self { width, height })
    }
}

#[derive(PartialEq, Debug)]
pub enum State {
    Library {
        selection: usize,
        offset: usize,
    },
    Files {
        selection: usize,
        dir: String,
        list: Vec<String>,
    },
    SortedLibrary {
        depth: u8,
        artist: String,
        track: HashMap<String, usize>,
    },
    Playlists {
        depth: u8,
        playlist: String,
        track: HashMap<String, usize>,
    },
    Empty,
}

impl State {
    pub fn is_library(&self) -> bool {
        matches!(self, Self::Library { .. })
    }

    pub fn is_files(&self) -> bool {
        matches!(self, Self::Files { .. })
    }

    pub fn is_sorted_library(&self) -> bool {
        matches!(self, Self::SortedLibrary { .. })
    }

    pub fn is_playlists(&self) -> bool {
        matches!(self, Self::Playlists { .. })
    }

    pub fn get_selection(&self) -> usize {
        match self {
            Self::Library { selection, .. } => *selection,
            Self::Files { selection, .. } => *selection,
            _ => unreachable!(),
        }
    }
}

pub struct Ui {
    stdout: std::io::Stdout,
    mgmt: Arc<Mutex<Manager>>,
    states: HashMap<u8, State>,
    ptr: u8,
    play_ptr: u8,
    size: Size,
    active: bool,
    library_updated: bool,
}

impl Ui {
    pub fn new(m: Arc<Mutex<Manager>>) -> Result<Self> {
        // Create new UI
        let mgmt = m.lock().unwrap();
        // Create track pointers for artists
        let mut track = HashMap::new();
        let first_artist = mgmt
            .library_tree
            .keys()
            .next()
            .and_then(|x| Some(x.to_string()))
            .unwrap_or_else(|| "".to_string());
        for artist in mgmt.library_tree.keys() {
            track.insert(artist.clone(), 0);
        }
        // Create initial playlist data
        let mut playlist_ptrs = HashMap::new();
        for playlist in mgmt.database.playlists.keys() {
            playlist_ptrs.insert(playlist.to_string(), 0);
        }
        let playlist = mgmt
            .database
            .display
            .playlists
            .get(0)
            .and_then(|n| Some(n.to_string()))
            .unwrap_or_else(|| "".to_string());
        // Set up states
        let mut states = HashMap::default();
        for (key, pane) in &mgmt.config.panes {
            states.insert(
                *key,
                match pane {
                    Pane::SimpleLibrary => State::Library { selection: 0, offset: 0, },
                    Pane::SortedLibrary => State::SortedLibrary {
                        depth: 0,
                        track: track.clone(),
                        artist: first_artist.clone(),
                    },
                    Pane::Files => {
                        let dir = expand_path("~/").unwrap_or_else(|| ".".to_string());
                        State::Files {
                            selection: 0,
                            list: list_dir(&dir, !mgmt.config.show_hidden_files),
                            dir,
                        }
                    }
                    Pane::Playlists => State::Playlists {
                        depth: 0,
                        track: playlist_ptrs.clone(),
                        playlist: playlist.to_string(),
                    },
                    Pane::Empty => State::Empty,
                },
            );
        }
        let ptr = mgmt.config.open_on_pane;
        std::mem::drop(mgmt);
        // Form struct
        Ok(Self {
            stdout: std::io::stdout(),
            mgmt: m,
            states,
            ptr,
            play_ptr: ptr,
            size: Size::screen()?,
            active: true,
            library_updated: false,
        })
    }

    pub fn init(&mut self) -> Result<()> {
        // Initiate the UI
        execute!(self.stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        terminal::enable_raw_mode()?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        // Run the UI
        self.render()?;
        while self.active {
            let status = get_md!(self.mgmt).playback_status;
            if event::poll(std::time::Duration::from_millis(PULSE))? {
                match event::read()? {
                    Event::Key(k) => self.on_key(k),
                    Event::Resize(width, height) => {
                        self.size = Size { width, height };
                        self.fix_offset();
                        self.render()?;
                    }
                    Event::Mouse(..) => (),
                }
                self.render()?;
            } else if self.mgmt.lock().unwrap().updated {
                self.mgmt.lock().unwrap().updated = false;
                self.render()?;
            } else if status == PlaybackStatus::Playing {
                // Rerender the status line if playing, to keep up with the position of the song
                let status_idx = self.size.height.saturating_sub(1);
                queue!(
                    self.stdout,
                    cursor::MoveTo(0, status_idx),
                    terminal::Clear(ClearType::CurrentLine)
                )?;
                self.rerender_status()?;
                self.stdout.flush()?;
            }
        }
        Ok(())
    }

    pub fn on_key(&mut self, e: KeyEvent) {
        // Handle key event
        match (e.modifiers, e.code) {
            // Mode switching
            (KMod::NONE, KCode::Char('0')) => self.switch_mode(0),
            (KMod::NONE, KCode::Char('1')) => self.switch_mode(1),
            (KMod::NONE, KCode::Char('2')) => self.switch_mode(2),
            (KMod::NONE, KCode::Char('3')) => self.switch_mode(3),
            (KMod::NONE, KCode::Char('4')) => self.switch_mode(4),
            (KMod::NONE, KCode::Char('5')) => self.switch_mode(5),
            (KMod::NONE, KCode::Char('6')) => self.switch_mode(6),
            (KMod::NONE, KCode::Char('7')) => self.switch_mode(7),
            (KMod::NONE, KCode::Char('8')) => self.switch_mode(8),
            (KMod::NONE, KCode::Char('9')) => self.switch_mode(9),
            // [q] : Quit
            (KMod::NONE, KCode::Char('q')) => self.active = false,
            // [t] : Toggle playback
            (KMod::NONE, KCode::Char('t')) => self.mgmt.lock().unwrap().play_pause(),
            // [x] : Stop playback
            (KMod::NONE, KCode::Char('x')) => self.mgmt.lock().unwrap().stop(),
            // [c] : Play playback
            (KMod::NONE, KCode::Char('c')) => self.mgmt.lock().unwrap().play(),
            // [v] : Pause playback
            (KMod::NONE, KCode::Char('v')) => self.mgmt.lock().unwrap().pause(),
            // [d] : Delete from library / Delete playlist
            (KMod::NONE, KCode::Char('d')) => {
                if self.state().is_playlists() {
                    self.delete_playlist();
                } else {
                    self.remove();
                }
            }
            // [e] : Edit tag of selected song
            (KMod::NONE, KCode::Char('e')) => self.tag_edit().unwrap_or(()),
            // [Enter] : Play selection / Add track to library
            (KMod::NONE, KCode::Enter) => self.select(),
            // [/\] : Move up selection in library
            (KMod::NONE, KCode::Up) => self.selection_up(),
            // [\/] : Move down selection in library
            (KMod::NONE, KCode::Down) => self.selection_down(),
            // [Ctrl] + [\/] : Move selection to top of library
            (KMod::CONTROL, KCode::Up) => self.selection_top(),
            // [Ctrl] + [/\] : Move selection to bottom of library
            (KMod::CONTROL, KCode::Down) => self.selection_bottom(),
            // [Alt] + [\/] : Move track downwards
            (KMod::ALT, KCode::Up) => self.track_up(),
            // [Alt] + [/\] : Move track upwards
            (KMod::ALT, KCode::Down) => self.track_down(),
            // [<] : Seek backward 5 seconds
            (KMod::NONE, KCode::Left) => self
                .mgmt
                .lock()
                .unwrap()
                .seek(false, Duration::from_secs(5)),
            // [>] : Seek forward 5 seconds
            (KMod::NONE, KCode::Right) => {
                self.mgmt.lock().unwrap().seek(true, Duration::from_secs(5));
            }
            // [Ctrl] + [<] : Previous track
            (KMod::CONTROL, KCode::Left) => self.mgmt.lock().unwrap().previous().unwrap_or(()),
            // [Ctrl] + [>] : Next track
            (KMod::CONTROL, KCode::Right) => self.mgmt.lock().unwrap().next().unwrap_or(()),
            // [l] : Toggle loop status
            (KMod::NONE, KCode::Char('l')) => self.mgmt.lock().unwrap().cycle_loop(),
            // [h] : Toggle shuffle status
            (KMod::NONE, KCode::Char('h')) => self.mgmt.lock().unwrap().cycle_shuffle(),
            // [m] : Toggle mute
            (KMod::NONE, KCode::Char('m')) => self.mgmt.lock().unwrap().toggle_mute(),
            // [Shift] + [/\] : Volume up
            (KMod::SHIFT, KCode::Up) => {
                let v = get_md!(self.mgmt).volume;
                self.mgmt.lock().unwrap().set_volume(v + 0.1);
            }
            // [Shift] + [\/] : Volume down
            (KMod::SHIFT, KCode::Down) => {
                let v = get_md!(self.mgmt).volume;
                self.mgmt.lock().unwrap().set_volume(v - 0.1);
            }
            // [Tab] : Recurse deeper into sorted library
            (KMod::NONE, KCode::Tab) => self.deepen(),
            // [a] : Add to playlist
            (KMod::NONE, KCode::Char('a')) => self.add_to_playlist(),
            // [r] : Remove from playlist
            (KMod::NONE, KCode::Char('r')) => self.remove_from_playlist(),
            // [n] : New playlist
            (KMod::NONE, KCode::Char('n')) => self.create_playlist(),
            // [k] : Rename playlist
            (KMod::NONE, KCode::Char('k')) => self.rename_playlist(),
            // [;] or [:] : Command mode
            (KMod::NONE, KCode::Char(':' | ';')) => (),
            // Spam
            (KMod::NONE, KCode::Char('p')) => panic!("{:?}", self.states[&2]),
            // [???] : Do nothing
            _ => (),
        }
    }

    fn fix_offset(&mut self) {
        // Check if selection is off screen
        let height = self.size.height.saturating_sub(2).into();
        match self.state_mut() {
            State::Library { selection, .. } => {
                if *selection > height {
                    // Selection is off the screen
                    *selection = height;
                }
            }
            _ => (),
        }
    }

    fn create_playlist(&mut self) {
        // Create new playlist
        if let Ok(Some(name)) = self.get_input("Playlist name: ") {
            if name.is_empty() {
                return;
            }
            self.states.iter_mut().for_each(|(_, s)| {
                if let State::Playlists {
                    track, playlist: p, ..
                } = s
                {
                    track.insert(name.clone(), 0);
                    if p.is_empty() {
                        // Fix empty track pointer
                        *p = name.clone();
                    }
                }
            });
            self.mgmt.lock().unwrap().new_playlist(&name);
        }
    }

    fn delete_playlist(&mut self) {
        // Delete playlist
        if let State::Playlists {
            depth: 0, playlist, ..
        } = self.state()
        {
            if playlist.is_empty() {
                return;
            }
            // Confirm user choice
            let playlist = playlist.clone();
            let warning = format!(
                "WARNING: Are you sure you want '{}' to be deleted? (y/n): ",
                playlist
            );
            if let Ok(Some(confirm)) = self.get_input(&warning) {
                if confirm == "y" {
                    // Move selection up
                    self.selection_up();
                    // Delete track pointers from playlist states
                    self.states.iter_mut().for_each(|(_, s)| {
                        if let State::Playlists {
                            track, playlist: p, ..
                        } = s
                        {
                            // Do removal
                            track.remove(&playlist);
                            if self.mgmt.lock().unwrap().database.display.playlists.get(0)
                                == Some(&playlist)
                            {
                                // Pointer needs fixing
                                *p = self
                                    .mgmt
                                    .lock()
                                    .unwrap()
                                    .database
                                    .display
                                    .playlists
                                    .get(1)
                                    .and_then(|x| Some(x.to_string()))
                                    .unwrap_or_else(|| "".to_string());
                            }
                        }
                    });
                    // Do deletion
                    self.mgmt.lock().unwrap().delete_playlist(&playlist);
                }
            }
        }
    }

    fn rename_playlist(&mut self) {
        // Rename playlist
        if let State::Playlists {
            depth: 0, playlist, ..
        } = self.state()
        {
            if playlist.is_empty() {
                return;
            }
            // Get new playlist name
            let playlist = playlist.clone();
            let msg = format!("Rename '{}' to: ", playlist);
            if let Ok(Some(new)) = self.get_input(&msg) {
                if new.is_empty() {
                    return;
                }
                // Rename track pointers
                self.states.iter_mut().for_each(|(_, s)| {
                    if let State::Playlists {
                        track, playlist: p, ..
                    } = s
                    {
                        // Update playlist pointer if necessary
                        if *p == playlist {
                            *p = new.to_string();
                        }
                        let old: usize = track.remove(&playlist).unwrap();
                        track.insert(new.to_string(), old);
                    }
                });
                // Do renaming
                self.mgmt.lock().unwrap().rename_playlist(&playlist, &new);
            }
        }
    }

    fn add_to_playlist(&mut self) {
        // Add song to playlist from simple library pane
        if let Some(id) = self.get_selected_id() {
            // Get the desired playlist that the user wants to add to
            if let Ok(Some(playlist)) = self.get_input("Playlist name: ") {
                // Check the playlist exists
                if self
                    .mgmt
                    .lock()
                    .unwrap()
                    .database
                    .playlists
                    .contains_key(&playlist)
                {
                    self.mgmt.lock().unwrap().add_to_playlist(&playlist, id);
                }
            }
        }
    }

    fn remove_from_playlist(&mut self) {
        let mut fix_selection = false;
        if let State::Playlists {
            playlist,
            track,
            depth,
        } = self.state()
        {
            if playlist.is_empty() {
                return;
            }
            let length = self.mgmt.lock().unwrap().database.playlists[playlist].len();
            if length == 0 {
                return;
            }
            // Differentiate between deleting playlists and deleting tracks from playlists
            if depth == &1 {
                self.mgmt
                    .lock()
                    .unwrap()
                    .remove_from_playlist(playlist, track[playlist]);
            }
            // Determine if selection needs fixing (out of bounds)
            if track[playlist] > length.saturating_sub(2) {
                fix_selection = true;
            }
        }
        if fix_selection {
            self.selection_up();
        }
    }

    fn get_selected_id(&self) -> Option<usize> {
        // Get the track id that is selected (state independent)
        Some(match self.state() {
            State::Library { selection, .. } => {
                self.mgmt.lock().unwrap().database.display.simple[*selection]
            }
            State::SortedLibrary {
                artist,
                track,
                depth: 1,
                ..
            } => artist_tracks(&self.mgmt.lock().unwrap().library_tree, artist)[track[artist]],
            _ => return None,
        })
    }

    fn deepen(&mut self) {
        // Switch focus in the sorted library view
        match self.state_mut() {
            State::SortedLibrary { depth, .. } => {
                if depth == &1 {
                    *depth = 0;
                } else {
                    *depth += 1;
                }
            }
            State::Playlists {
                depth, playlist, ..
            } => {
                if playlist.is_empty() {
                    return;
                }
                if depth == &1 {
                    *depth = 0;
                } else {
                    *depth += 1;
                }
            }
            _ => (),
        }
    }

    fn tag_edit(&mut self) -> Result<()> {
        // Ensure there are available tracks
        if self.mgmt.lock().unwrap().database.tracks.is_empty() {
            return Ok(());
        }
        // If there is enough room...
        if self.size.height > 3 {
            // Get selected track
            if let Some(id) = self.get_selected_id() {
                // Establish tag type to edit
                let mut kind = String::new();
                while !["title", "album", "artist", "year"].contains(&kind.as_str()) {
                    kind = self
                        .get_input("title/album/artist/year: ")?
                        .unwrap_or_else(|| "".to_string());
                    if kind == "" {
                        return Ok(());
                    }
                }
                // Establish new tag value
                if let Some(value) = self.get_input("new value: ")? {
                    // Write tag value
                    match kind.as_str() {
                        "title" => self.mgmt.lock().unwrap().set_title(id, &value),
                        "album" => self.mgmt.lock().unwrap().set_album(id, &value),
                        "artist" => self.mgmt.lock().unwrap().set_artist(id, &value),
                        "year" => self.mgmt.lock().unwrap().set_year(id, &value),
                        _ => unreachable!(),
                    }
                }
            }
        }
        Ok(())
    }

    fn get_input(&mut self, prompt: &str) -> Result<Option<String>> {
        // If too few rows, don't bother doing prompt
        if self.size.height < 3 {
            return Ok(None);
        }
        // Establish empty row at the bottom
        let input_row = self.size.height;
        self.size.height -= 1;
        self.render()?;
        // Get user input
        let mut out = String::new();
        let mut entering = true;
        while entering {
            execute!(
                self.stdout,
                cursor::MoveTo(0, input_row),
                terminal::Clear(ClearType::CurrentLine),
                Print(prompt),
                Print(&out)
            )?;
            // Handle prompt input
            let status = get_md!(self.mgmt).playback_status;
            if event::poll(std::time::Duration::from_millis(PULSE))? {
                match event::read()? {
                    Event::Key(k) => match (k.modifiers, k.code) {
                        (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => out.push(c),
                        (KMod::NONE, KCode::Backspace) => {
                            let _ = out.pop();
                        }
                        (KMod::NONE, KCode::Enter) => {
                            entering = false;
                        }
                        (KMod::NONE, KCode::Esc) => {
                            self.size = Size::screen()?;
                            return Ok(None);
                        }
                        _ => (),
                    },
                    Event::Resize(width, height) => {
                        self.size = Size {
                            width,
                            height: height - 1,
                        };
                        self.render()?;
                    }
                    Event::Mouse(..) => (),
                }
                self.render()?;
            } else if self.mgmt.lock().unwrap().updated {
                self.mgmt.lock().unwrap().updated = false;
                self.render()?;
            } else if status == PlaybackStatus::Playing {
                // Rerender the status line if playing, to keep up with the position of the song
                let status_idx = self.size.height.saturating_sub(1);
                queue!(
                    self.stdout,
                    cursor::MoveTo(0, status_idx),
                    terminal::Clear(ClearType::CurrentLine)
                )?;
                self.rerender_status()?;
                self.stdout.flush()?;
            }
            self.render()?;
        }
        // Reset shifted row
        self.size = Size::screen()?;
        Ok(Some(out))
    }

    fn switch_mode(&mut self, mode: u8) {
        // Switch modes
        if self.states.contains_key(&mode) {
            self.ptr = mode;
        }
    }

    fn state(&self) -> &State {
        // Get the current state
        self.states.get(&self.ptr).unwrap()
    }

    fn state_mut(&mut self) -> &mut State {
        // Get the current state as a mutable reference
        self.states.get_mut(&self.ptr).unwrap()
    }

    fn remove(&mut self) {
        // Ensure there are available tracks
        if self.mgmt.lock().unwrap().database.tracks.is_empty() {
            return;
        }
        // Remove from library
        let mut selection_off = false;
        match self.state() {
            State::Library { selection, .. } => {
                // Get track ID
                if let Some(id) = self.get_selected_id() {
                    let mut mgmt = self.mgmt.lock().unwrap();
                    mgmt.remove_library(id);
                    // Check for selection issues
                    if selection > &mgmt.database.display.simple.len().saturating_sub(2) {
                        selection_off = true;
                    }
                    // Trigger library tree rerender
                    self.library_updated = true;
                }
            }
            State::SortedLibrary {
                depth,
                artist,
                track,
                ..
            } => {
                if *depth == 1 {
                    let mut mgmt = self.mgmt.lock().unwrap();
                    let tracks = artist_tracks(&mgmt.library_tree, artist);
                    // Get track ID
                    let id = tracks[track[artist]];
                    mgmt.remove_library(id);
                    // Check for selection issues
                    if track[artist] > tracks.len().saturating_sub(3) {
                        selection_off = true;
                    }
                    // Trigger library tree rerender
                    self.library_updated = true;
                }
            }
            _ => (),
        }
        // Correct selection issues
        if selection_off {
            self.selection_up();
        }
    }

    fn select(&mut self) {
        // Play the selected track
        match self.state() {
            State::Library { selection, .. } => {
                let mut mgmt = self.mgmt.lock().unwrap();
                mgmt.playlist.name = None;
                // Ensure there are available tracks
                if mgmt.database.tracks.is_empty() {
                    return;
                }
                let lookup = mgmt.database.display.simple.clone();
                let tracks = lookup
                    .iter()
                    .map(|x| mgmt.database.tracks[x].clone())
                    .collect();
                let id = lookup[*selection];
                mgmt.load(id);
                mgmt.playlist.set(*selection, tracks, lookup);
                self.play_ptr = self.ptr;
                mgmt.play();
            }
            State::SortedLibrary { artist, track, .. } => {
                let mut mgmt = self.mgmt.lock().unwrap();
                mgmt.playlist.name = None;
                let lookup = artist_tracks(&mgmt.library_tree, artist);
                let tracks = lookup
                    .iter()
                    .map(|x| mgmt.database.tracks[x].clone())
                    .collect();
                let id = lookup[track[artist]];
                mgmt.load(id);
                mgmt.playlist.set(track[artist], tracks, lookup);
                self.play_ptr = self.ptr;
                mgmt.play();
            }
            State::Files {
                selection,
                list,
                dir,
            } => {
                let mut mgmt = self.mgmt.lock().unwrap();
                let selection = *selection;
                let file = &list[selection];
                let dir = dir.to_owned() + "/" + file;
                if is_file(&dir) {
                    mgmt.add_library(Track::load(&dir));
                    // Trigger library tree rerender
                    self.library_updated = true;
                } else {
                    let list = list_dir(&dir, !mgmt.config.show_hidden_files);
                    *self.states.get_mut(&self.ptr).unwrap() = State::Files {
                        selection: 0,
                        list,
                        dir,
                    };
                }
            }
            State::Playlists {
                playlist, track, ..
            } => {
                let mut mgmt = self.mgmt.lock().unwrap();
                mgmt.playlist.name = Some(playlist.to_string());
                if playlist.is_empty() {
                    return;
                }
                let display = mgmt.database.playlists[playlist].clone();
                if !display.is_empty() {
                    let tracks = display
                        .iter()
                        .map(|x| mgmt.database.tracks[x].clone())
                        .collect();
                    let id = display[track[playlist]];
                    mgmt.load(id);
                    mgmt.playlist.set(track[playlist], tracks, display);
                    self.play_ptr = self.ptr;
                    mgmt.play();
                }
            }
            _ => (),
        }
    }

    fn track_up(&mut self) {
        // Move track upwards
        let mut mgmt = self.mgmt.lock().unwrap();
        // Ensure there are available tracks
        if mgmt.database.tracks.is_empty() {
            return;
        }
        match self.state() {
            State::Library { selection, offset, .. } => {
                let sel = *selection + *offset;
                if sel != 0 {
                    // Update database
                    mgmt.database
                        .display
                        .simple
                        .swap(sel, sel.saturating_sub(1));
                    std::mem::drop(mgmt);
                    self.selection_up();
                }
            }
            State::Playlists {
                depth,
                playlist,
                track,
                ..
            } => {
                if playlist.is_empty() {
                    return;
                }
                if *depth == 1 {
                    // Moving track display order around
                    let selection = track[playlist];
                    if selection != 0 {
                        mgmt.database
                            .playlists
                            .get_mut(playlist)
                            .unwrap()
                            .swap(selection, selection.saturating_sub(1));
                        std::mem::drop(mgmt);
                        self.selection_up();
                    }
                } else if *depth == 0 {
                    // Moving playlist display order around
                    let idx = mgmt
                        .database
                        .display
                        .playlists
                        .iter()
                        .position(|x| x == playlist);
                    if let Some(idx) = idx {
                        mgmt.database
                            .display
                            .playlists
                            .swap(idx, idx.saturating_sub(1));
                    }
                }
            }
            _ => (),
        }
    }

    fn track_down(&mut self) {
        // Move track downwards
        let mut mgmt = self.mgmt.lock().unwrap();
        // Ensure there are available tracks
        if mgmt.database.tracks.is_empty() {
            return;
        }
        match self.state() {
            State::Library { selection, offset, .. } => {
                let sel = *selection + *offset;
                if sel < mgmt.database.tracks.len().saturating_sub(1) {
                    // Update database
                    mgmt.database
                        .display
                        .simple
                        .swap(sel, sel + 1);
                }
                std::mem::drop(mgmt);
                self.selection_down();
            }
            State::Playlists {
                depth,
                playlist,
                track,
                ..
            } => {
                if playlist.is_empty() {
                    return;
                }
                if *depth == 1 {
                    // Move track display order around
                    let selection = track[playlist];
                    if selection < mgmt.database.playlists[playlist].len().saturating_sub(1) {
                        mgmt.database
                            .playlists
                            .get_mut(playlist)
                            .unwrap()
                            .swap(selection, selection + 1);
                        std::mem::drop(mgmt);
                        self.selection_down();
                    }
                } else if *depth == 0 {
                    // Moving playlist display order around
                    let idx = mgmt
                        .database
                        .display
                        .playlists
                        .iter()
                        .position(|x| x == playlist);
                    if let Some(idx) = idx {
                        if idx < mgmt.database.display.playlists.len().saturating_sub(1) {
                            mgmt.database.display.playlists.swap(idx, idx + 1);
                        }
                    }
                }
            }
            _ => (),
        }
    }

    fn selection_up(&mut self) {
        // Move the current selection down
        let artist_list = if self.state().is_sorted_library() {
            let mgmt = self.mgmt.lock().unwrap();
            let artists: Vec<String> = mgmt.library_tree.keys().map(|x| x.to_string()).collect();
            Some(artists)
        } else {
            None
        };
        let playlist_display = if self.state().is_playlists() {
            let mgmt = self.mgmt.lock().unwrap();
            Some(mgmt.database.display.playlists.clone())
        } else {
            None
        };
        match self.state_mut() {
            State::Library { selection, offset, .. } => {
                if *selection == 0 && *offset != 0 {
                    *offset -= 1;
                } else if *selection > 0 {
                    *selection -= 1
                }
            }
            State::Files { selection, .. } => {
                if *selection > 0 {
                    *selection -= 1
                }
            }
            State::SortedLibrary {
                artist,
                track,
                depth,
                ..
            } => {
                let artists_idx = artist_list
                    .as_ref()
                    .unwrap()
                    .iter()
                    .position(|x| x == artist)
                    .unwrap_or(0);
                if *depth == 0 && artists_idx > 0 {
                    *artist = artist_list.unwrap()[artists_idx - 1].to_string();
                } else if *depth == 1 && track[artist] > 0 {
                    *track.get_mut(artist).unwrap() -= 1;
                }
            }
            State::Playlists {
                playlist,
                track,
                depth,
                ..
            } => {
                if playlist.is_empty() {
                    return;
                }
                if *depth == 0 {
                    let playlist_display = playlist_display.unwrap();
                    let idx = playlist_display
                        .iter()
                        .position(|x| x == playlist)
                        .unwrap_or(0);
                    *playlist = playlist_display[idx.saturating_sub(1)].to_string();
                } else if *depth == 1 {
                    *track.get_mut(playlist).unwrap() = track[playlist].saturating_sub(1);
                }
            }
            _ => (),
        }
    }

    fn selection_down(&mut self) {
        // Move the current selection down
        let tracks_len = self.mgmt.lock().unwrap().database.tracks.len();
        let artists_len = self.mgmt.lock().unwrap().library_tree.len();
        // If in sorted library, get list of tracks and artists
        let (track_list, artist_list) = if let State::SortedLibrary { artist, .. } = self.state() {
            let mgmt = self.mgmt.lock().unwrap();
            let artists: Vec<String> = mgmt.library_tree.keys().map(|x| x.to_string()).collect();
            (
                Some(artist_tracks(&mgmt.library_tree, artist)),
                Some(artists),
            )
        } else {
            (None, None)
        };
        // If in playlists, get playlist display
        let playlist_data = if let State::Playlists { playlist, .. } = self.state() {
            if playlist.is_empty() {
                return;
            }
            let mgmt = self.mgmt.lock().unwrap();
            Some((
                mgmt.database.display.playlists.clone(),
                mgmt.database.playlists[playlist].len(),
            ))
        } else {
            None
        };
        // Perform selection move
        let available = self.size.height.saturating_sub(1) as usize;
        match self.state_mut() {
            State::Library { selection, offset, .. } => {
                if *selection + *offset + 1 < tracks_len {
                    if *selection == available.saturating_sub(1) {
                        *offset += 1;
                    } else {
                        *selection += 1;
                    }
                }
            }
            State::Files {
                selection, list, ..
            } => {
                if *selection + 1 < list.len() {
                    *selection += 1
                }
            }
            State::SortedLibrary {
                artist,
                track,
                depth,
                ..
            } => {
                let artists_idx = artist_list
                    .as_ref()
                    .unwrap()
                    .iter()
                    .position(|x| x == artist)
                    .unwrap_or(0);
                if *depth == 0 && artists_idx + 1 < artists_len {
                    *artist = artist_list.unwrap()[artists_idx + 1].to_string();
                } else if *depth == 1 && track[artist] + 1 < track_list.unwrap().len() {
                    *track.get_mut(artist).unwrap() += 1;
                }
            }
            State::Playlists {
                playlist,
                track,
                depth,
                ..
            } => {
                let (playlist_display, tracks) = playlist_data.unwrap();
                if *depth == 0 {
                    let idx = playlist_display
                        .iter()
                        .position(|x| x == playlist)
                        .unwrap_or_else(|| playlist_display.len().saturating_sub(1));
                    if let Some(next) = playlist_display.get(idx + 1) {
                        *playlist = next.to_string();
                    }
                } else if *depth == 1 && track[playlist] + 1 < tracks {
                    *track.get_mut(playlist).unwrap() = track[playlist] + 1;
                }
            }
            _ => (),
        }
    }

    fn selection_top(&mut self) {
        // Move the selection to the top of the library
        let first_artist: Option<String> = if self.state().is_sorted_library() {
            let mgmt = self.mgmt.lock().unwrap();
            Some(
                mgmt.library_tree
                    .keys()
                    .nth(0)
                    .and_then(|x| Some(x.to_string()))
                    .unwrap_or_else(|| "".to_string()),
            )
        } else {
            None
        };
        // If in playlists, get playlist display
        let playlist_display = if self.state().is_playlists() {
            let mgmt = self.mgmt.lock().unwrap();
            Some(mgmt.database.display.playlists.clone())
        } else {
            None
        };
        match self.state_mut() {
            State::Library { selection, offset } => {
                *selection = 0;
                *offset = 0;
            }
            State::Files { selection, .. } => {
                *selection = 0;
            }
            State::SortedLibrary {
                depth,
                artist,
                track,
                ..
            } => {
                if *depth == 0 {
                    *artist = first_artist.unwrap();
                } else {
                    *track.get_mut(artist).unwrap() = 0;
                }
            }
            State::Playlists {
                depth,
                playlist,
                track,
                ..
            } => {
                if playlist.is_empty() {
                    return;
                }
                let playlist_display = playlist_display.unwrap();
                if *depth == 0 {
                    *playlist = playlist_display
                        .get(0)
                        .and_then(|x| Some(x.to_string()))
                        .unwrap_or_else(|| "".to_string());
                } else {
                    *track.get_mut(playlist).unwrap() = 0;
                }
            }
            _ => (),
        }
    }

    fn selection_bottom(&mut self) {
        // Move the selection to the top of the library
        let tracks_len = self.mgmt.lock().unwrap().database.tracks.len();
        // If in sorted library, get list of tracks in artist
        let (track_list, artist_list) = if let State::SortedLibrary { artist, .. } = self.state() {
            let mgmt = self.mgmt.lock().unwrap();
            let artists: Vec<String> = mgmt.library_tree.keys().map(|x| x.to_string()).collect();
            (
                Some(artist_tracks(&mgmt.library_tree, artist)),
                Some(artists),
            )
        } else {
            (None, None)
        };
        // If in playlists, get playlist display
        let playlist_data = if let State::Playlists { playlist, .. } = self.state() {
            if playlist.is_empty() {
                return;
            }
            let mgmt = self.mgmt.lock().unwrap();
            Some((
                mgmt.database.display.playlists.clone(),
                mgmt.database.playlists[playlist].len(),
            ))
        } else {
            None
        };
        let available = self.size.height.saturating_sub(1) as usize;
        match self.state_mut() {
            State::Library { selection, offset } => {
                if tracks_len < available {
                    *selection = tracks_len.saturating_sub(1);
                    *offset = 0;
                } else {
                    *selection = available.saturating_sub(1);
                    *offset = tracks_len - available;
                }
            }
            State::Files {
                selection, list, ..
            } => {
                *selection = list.len().saturating_sub(1);
            }
            State::SortedLibrary {
                depth,
                artist,
                track,
                ..
            } => {
                if *depth == 0 {
                    let artists_len = artist_list.as_ref().unwrap().len();
                    *artist =
                        artist_list.as_ref().unwrap()[artists_len.saturating_sub(1)].to_string();
                } else {
                    *track.get_mut(artist).unwrap() = track_list.unwrap().len().saturating_sub(1);
                }
            }
            State::Playlists {
                depth,
                playlist,
                track,
            } => {
                let (playlist_display, tracks) = playlist_data.unwrap();
                if *depth == 0 {
                    *playlist = playlist_display
                        .iter()
                        .last()
                        .and_then(|x| Some(x.to_string()))
                        .unwrap_or_else(|| "".to_string());
                } else {
                    *track.get_mut(playlist).unwrap() = tracks.saturating_sub(1);
                }
            }
            _ => (),
        }
    }

    pub fn update_library(&mut self) {
        // Prevent rendering with outdated library tree
        if self.library_updated && self.state().is_sorted_library() {
            let mut mgmt = self.mgmt.lock().unwrap();
            let tracks = &mgmt.database.tracks;
            mgmt.library_tree = form_library_tree(tracks);
            let artists: Vec<String> = mgmt.library_tree.keys().map(|x| x.to_string()).collect();
            std::mem::drop(mgmt);
            if let State::SortedLibrary {
                track,
                artist: artist_ptr,
                ..
            } = self.state_mut()
            {
                for artist in &artists {
                    if !track.contains_key(artist) {
                        track.insert(artist.to_string(), 0);
                    }
                }
                track.drain_filter(|t, _| !artists.contains(t));
                if !artists.contains(&artist_ptr) {
                    *artist_ptr = artists
                        .get(0)
                        .and_then(|x| Some(x.to_string()))
                        .unwrap_or_else(|| "".to_string());
                }
            }
            self.library_updated = false;
        }
    }

    pub fn render(&mut self) -> Result<()> {
        self.update_library();
        // Acquire manager
        let mgmt = self.mgmt.lock().unwrap();
        // Update library tree if need be
        // Obtain render data for the current state
        let ((keys, tracks), paths, artist_track, playlists): (
            TrackList,
            FileList,
            SortedList,
            OptionList,
        ) = match self.state() {
            State::Library { offset, .. } => {
                // Obtain list of tracks
                let keys = mgmt.database.display.simple.clone();
                let tracks: Vec<&Track> = keys.iter().map(|x| &mgmt.database.tracks[x]).collect();
                let table = pad_table(format_table(&tracks, *offset), self.size.width as usize);
                ((Some(keys), Some(table)), None, None, None)
            }
            State::SortedLibrary {
                artist,
                track,
                depth,
                ..
            } => {
                let id_playing = if mgmt.playlist.is_ready() {
                    mgmt.playlist.current_id()
                } else {
                    None
                };
                let table = format_artist_track(
                    &mgmt.library_tree,
                    (artist.to_string(), track),
                    *depth,
                    &mgmt.database.tracks,
                    id_playing,
                    self.ptr == self.play_ptr,
                );
                ((None, None), None, Some(table), None)
            }
            State::Files { dir, .. } => {
                // Obtain list of files
                let files = list_dir(dir, !mgmt.config.show_hidden_files);
                ((None, None), Some(files), None, None)
            }
            State::Playlists {
                playlist,
                track,
                depth,
                ..
            } => {
                let playlists = format_playlist(
                    &mgmt.database.playlists,
                    &mgmt.database.display.playlists,
                    *depth,
                    &mgmt.database.tracks,
                    (&playlist, track),
                    mgmt.playlist.ptr,
                    &mgmt.playlist.name,
                    self.size.width,
                    &mgmt.config.indicators["playlist_icon"],
                );
                ((None, None), None, None, Some(playlists))
            }
            State::Empty => ((None, None), None, None, None),
        };
        std::mem::drop(mgmt);
        // Do render
        for line in 0..self.size.height {
            // Go to line and clear it
            queue!(
                self.stdout,
                cursor::MoveTo(0, line),
                terminal::Clear(ClearType::CurrentLine)
            )?;
            // Do maths
            let status_idx = self.size.height.saturating_sub(1);
            // Determine what to render on this line
            if line != status_idx && self.state().is_library() {
                queue!(self.stdout, terminal::Clear(ClearType::CurrentLine))?;
                // Acquire manager
                let mgmt = self.mgmt.lock().unwrap();
                // Render library view
                let selection = self.state().get_selection();
                if let Some(row) = tracks.as_ref().unwrap().get(line as usize) {
                    let is_selected = selection == line.into();
                    let this_id = keys
                        .as_ref()
                        .unwrap()
                        .get(line as usize)
                        .and_then(|i| Some(*i));
                    let is_playing = mgmt.playlist.is_ready()
                        && self.ptr == self.play_ptr
                        && mgmt.playlist.current_id() == this_id;
                    // Set up formatting for list
                    if is_selected {
                        queue!(self.stdout, SetBg(Color::DarkGrey))?;
                    }
                    if is_playing {
                        queue!(self.stdout, SetFg(Color::Green))?;
                    }
                    // Print row content
                    queue!(self.stdout, Print(row))?;
                    // Reset formatting for next row
                    queue!(self.stdout, SetBg(Color::Reset), SetFg(Color::Reset))?;
                } else if line == 0 {
                    // Print out placeholder
                    queue!(self.stdout, Print("[empty library]"))?;
                }
            } else if line != status_idx && self.state().is_files() {
                let selection = self.state().get_selection();
                if let Some(row) = paths.as_ref().unwrap().get(line as usize) {
                    // Add padding
                    let row = format!("{:<pad$}", row, pad = self.size.width as usize);
                    // Set up formatting for list
                    if selection == line.into() {
                        queue!(self.stdout, SetBg(Color::DarkGrey))?;
                    }
                    queue!(self.stdout, Print(row))?;
                    // Reset formatting for next row
                    queue!(self.stdout, SetBg(Color::Reset))?;
                }
            } else if line != status_idx && self.state().is_sorted_library() {
                if let Some(row) = artist_track.as_ref().unwrap().get(line as usize) {
                    // Add padding
                    let row = format!("{:<pad$}", row, pad = self.size.width as usize);
                    queue!(self.stdout, Print(row))?;
                    queue!(self.stdout, SetBg(Color::Reset), SetFg(Color::Reset))?;
                }
            } else if line != status_idx && self.state().is_playlists() {
                if let Some(row) = playlists.as_ref().unwrap().get(line as usize) {
                    queue!(self.stdout, Print(row))?;
                }
            } else if line == status_idx {
                // Render status line
                self.rerender_status()?;
            }
        }
        self.stdout.flush()
    }

    fn rerender_status(&mut self) -> Result<()> {
        // Render status line
        let mgmt = self.mgmt.lock().unwrap();
        // Form left hand side
        let lhs = if let Some(current) = mgmt.playlist.current() {
            let pb = mgmt.metadata.lock().unwrap().playback_status;
            let icon = match pb {
                PlaybackStatus::Playing => &mgmt.config.indicators["playing"],
                PlaybackStatus::Paused => &mgmt.config.indicators["paused"],
                PlaybackStatus::Stopped => &mgmt.config.indicators["stopped"],
            };
            format!("{}{} - {}", icon, current.tag.title, current.tag.artist)
        } else {
            "No track loaded".to_string()
        };
        // Obtain correct icons for current player state
        let md = mgmt.metadata.lock().unwrap();
        let loop_icon = match md.loop_status {
            LoopStatus::None => &mgmt.config.indicators["loop_none"],
            LoopStatus::Track => &mgmt.config.indicators["loop_track"],
            LoopStatus::Playlist => &mgmt.config.indicators["loop_playlist"],
        };
        let shuffle_icon = &mgmt.config.indicators[if md.shuffle_status {
            "shuffle_on"
        } else {
            "shuffle_off"
        }];
        #[allow(clippy::cast_possible_truncation)]
        let volume_icon = match (mgmt.player.volume() * 100.0) as u8 {
            // 0%: Mute icon
            0 => &mgmt.config.indicators["volume_mute"],
            // < 30%: Low speaker icon
            1..=30 => &mgmt.config.indicators["volume_low"],
            // < 60%: Medium speaker icon
            31..=60 => &mgmt.config.indicators["volume_medium"],
            // < 100%: Full speaker icon
            _ => &mgmt.config.indicators["volume_high"],
        };
        // Form right hand side
        #[allow(clippy::cast_possible_truncation)]
        let volume = (md.volume * 100.0) as usize;
        std::mem::drop(md);
        let (position, duration, percent) = if let Some(data) = mgmt.get_position() {
            data
        } else {
            mgmt.metadata.lock().unwrap().position
        };
        let rhs = format!(
            "{}/{} {}% {} {} {}",
            timefmt(position),
            timefmt(duration),
            volume,
            volume_icon,
            loop_icon,
            shuffle_icon
        );
        // Do alignment
        let space = align_sides(&lhs, &rhs, self.size.width as usize, 4).saturating_sub(4);
        if space > 3 {
            // Form progress bar
            #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
            let hl = ((space as f64 * percent) as usize).saturating_sub(1);
            let nohl = space - hl;
            let progress = format!(
                "|{}{}|",
                &mgmt.config.indicators["progress_bar_full"].repeat(hl),
                &mgmt.config.indicators["progress_bar_empty"].repeat(nohl)
            );
            // Put it all together and print it
            let status = format!("{} {} {}", lhs, progress, rhs);
            queue!(
                self.stdout,
                SetFg(Color::DarkBlue),
                Print(status),
                SetFg(Color::Reset)
            )?;
        }
        Ok(())
    }

    pub fn clean(&mut self) -> Result<()> {
        // Clean up before leaving
        self.mgmt.lock().unwrap().database.write();
        execute!(self.stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }
}

// util.rs - common utilities for helping out around the project
use crate::track::Track;
use crate::ui::{Color, SetBg, SetFg};
use std::collections::{BTreeMap, HashMap};
use unicode_width::UnicodeWidthStr;

// Help text
pub const HELP: &str = "Synchron:
    About:
        Synchron is a music player that can be run as a TUI or as a CLI. 
        It provides a way to organise, download, play and tag your
        music, podcasts, audiobooks and other forms of media.
        Please refer to the guide at https://github.com/curlpipe/synchron
        to get started.
    Options:
        -h, --help    : Prints this help message.
        -V, --version : Prints the version installed.
        -c, --cli     : Enters into CLI mode which displays a prompt that waits
                        for commands to be entered.
    Examples:
        synchron -h   : Show help message and exit.
        synchron -V   : Show version and exit.
        synchron      : Opens in the default TUI mode.
        synchron -c   : Opens in CLI mode and awaits for your instructions.";

// Utility macro for easy dbus property addition
#[macro_export]
macro_rules! add_prop {
    ($props:expr, $prop:expr, $value:expr) => {
        $props.insert($prop.to_string(), Variant(Box::new($value)));
    };
}

// Utility macro for getting metadata from manager
#[macro_export]
macro_rules! get_md {
    ($mgmt:expr) => {
        $mgmt.lock().unwrap().metadata.lock().unwrap()
    };
}

pub fn expand_path(path: &str) -> Option<String> {
    // Utility function for expanding paths
    let with_user = expanduser::expanduser(path).ok()?;
    let full_path = std::fs::canonicalize(with_user).ok()?;
    full_path.into_os_string().into_string().ok()
}

pub fn attempt_open(path: &str) -> Option<String> {
    // Attempt to open a file from an unstandardised path
    let path = expand_path(path)?;
    std::fs::read_to_string(path).ok()
}

pub fn width(s: &str, tab: usize) -> usize {
    // Find width of a string
    let s = s.replace('\t', &" ".repeat(tab));
    s.width()
}

pub fn pad_table(table: Vec<Vec<String>>, limit: usize) -> Vec<String> {
    // Check table isn't empty
    if table.is_empty() {
        return vec![];
    }
    // Apply padding to table and form into strings
    let mut result = vec![];
    // Calculate the lengths needed
    let length: usize = table[0].iter().map(|x| x.width()).sum();
    let inner = table[0].len().saturating_sub(1);
    // Determine if columns will be able to fit
    if length + inner < limit {
        // Columns will fit, distribute spacing between them
        let total = limit - length;
        let gaps = if inner == 0 {
            [0, 0, 0]
        } else {
            let gap = total / inner;
            let mut left_over = total % inner;
            let mut gaps = [gap, gap, gap];
            for i in gaps.iter_mut().take(2) {
                if left_over != 0 {
                    *i += 1;
                    left_over -= 1;
                }
            }
            gaps
        };
        // Format columns into strings
        for record in table {
            let mut row = String::new();
            for i in 0..4 {
                if record.len() > i {
                    row.push_str(&record[i]);
                    if record.len() > i + 1 {
                        row.push_str(&" ".repeat(gaps[i]));
                    }
                }
            }
            if record.len() > 4 {
                row.push_str(&record[4]);
            }
            result.push(row);
        }
    } else {
        // Recalculate padding with new column amount (rely on recursion)
        result = match table[0].len() {
            4 | 2 => pad_table(remove_column(table, 1), limit),
            3 => pad_table(remove_column(table, 2), limit),
            1 => (0..table.len()).map(|_| "...".to_string()).collect(),
            _ => vec![],
        }
    }
    result
}

pub fn remove_column(mut table: Vec<Vec<String>>, column: usize) -> Vec<Vec<String>> {
    // Remove a column from a table
    for i in &mut table {
        i.remove(column);
    }
    table
}

pub fn format_table(tracks: &[&Track], offset: usize) -> Vec<Vec<String>> {
    // Format a list of tracks into a table
    let mut result = vec![];
    let tracks: Vec<(String, &String, &String, &String, &String)> =
        tracks.iter().map(|x| x.format_elements()).collect();
    // Sort into columns
    let columns: Vec<Vec<&String>> = vec![
        tracks.iter().map(|x| x.1).collect(),
        tracks.iter().map(|x| x.2).collect(),
        tracks.iter().map(|x| x.3).collect(),
        tracks.iter().map(|x| x.4).collect(),
    ];
    // Find the longest item in each column
    let mut limits = vec![];
    for column in &columns {
        limits.push(find_longest(column));
    }
    // Reform back into rows, taking into account the maximum column size
    for i in offset..tracks.len() {
        let mut row = vec![];
        row.push(align_left(columns[0][i], limits[0]));
        row.push(align_left(columns[1][i], limits[1]));
        row.push(align_left(columns[2][i], limits[2]));
        row.push(align_left(columns[3][i], limits[3]));
        result.push(row);
    }
    result
}

pub fn find_longest(target: &[&String]) -> usize {
    // Find the longest string in a vector
    let mut longest = 0;
    for i in target {
        if i.width() > longest {
            longest = i.width();
        }
    }
    longest
}

pub fn find_longest_no_ref(target: &[String]) -> usize {
    // Find the longest string in a vector
    let mut longest = 0;
    for i in target {
        if i.width() > longest {
            longest = i.width();
        }
    }
    longest
}

pub fn align_left(target: &str, space: usize) -> String {
    let pad = " ".repeat(space.saturating_sub(target.width()));
    format!("{}{}", target, pad)
}

pub fn align_sides(lhs: &str, rhs: &str, space: usize, tab_width: usize) -> usize {
    // Align left and right hand side
    let total = width(lhs, tab_width) + width(rhs, tab_width);
    if total > space {
        0
    } else {
        space.saturating_sub(total)
    }
}

pub fn timefmt(duration: u64) -> String {
    let minutes: u64 = duration / 60;
    let seconds: u64 = duration % 60;
    format!("{}:{:02}", minutes, seconds)
}

pub fn is_file(path: &str) -> bool {
    std::path::Path::new(path).is_file()
}

pub fn list_dir(path: &str, no_hidden: bool) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(path)
        .unwrap()
        .map(|d| d.unwrap().file_name().into_string().unwrap())
        .filter(|d| if no_hidden { !d.starts_with(".") } else { true })
        .collect();
    files.push("..".to_string());
    files.sort();
    files
}

pub fn form_library_tree(
    tracks: &HashMap<usize, Track>,
) -> BTreeMap<String, BTreeMap<String, Vec<usize>>> {
    // Create a library tree from a list of tracks
    let mut result: BTreeMap<String, BTreeMap<String, Vec<usize>>> = BTreeMap::new();
    for (id, track) in tracks {
        if let Some(albums) = result.get_mut(&track.tag.artist) {
            if let Some(tracks) = albums.get_mut(&track.tag.album) {
                // Add it to existing entry if known
                tracks.push(*id);
            } else {
                // Create new key value pair
                albums.insert(track.tag.album.clone(), vec![*id]);
            }
        } else {
            // Create new key value pair
            result.insert(track.tag.artist.clone(), BTreeMap::new());
            result
                .get_mut(&track.tag.artist)
                .unwrap()
                .insert(track.tag.album.clone(), vec![*id]);
        }
    }
    result
}

pub fn format_artist_track(
    listing: &BTreeMap<String, BTreeMap<String, Vec<usize>>>,
    selection: (String, &HashMap<String, usize>),
    focus: u8,
    lookup: &HashMap<usize, Track>,
    playing: Option<usize>,
    playing_here: bool,
) -> Vec<String> {
    let mut result = vec![];
    let (artist_ptr, track_ptr) = selection;
    // Gather list of artists
    let mut artists: Vec<&String> = listing.keys().collect();
    // Gather list of selected artist's albums
    let albums: Vec<&String> = listing[&artist_ptr].keys().collect();
    // Gather years for albums
    let mut years = vec![];
    for album in &albums {
        let artist = &listing[&artist_ptr];
        let album = &artist[*album];
        let track_id = album[0];
        years.push(lookup[&track_id].tag.year.to_string());
    }
    // Gather list of all tracks from this artist
    let mut tracks: Vec<usize> = vec![];
    for album in &albums {
        let this = &listing[&artist_ptr][*album];
        for track in this {
            tracks.push(*track);
        }
    }
    // Format rhs of table
    let curve_bar = format!("{}╭{}", SetFg(Color::DarkBlue), SetFg(Color::Reset));
    let vertical_bar = format!("{}│{}", SetFg(Color::DarkBlue), SetFg(Color::Reset));
    for (album, year) in albums.iter().zip(years) {
        result.push(format!(
            "{} {}{} - {}{}",
            curve_bar,
            SetFg(Color::DarkBlue),
            album,
            year,
            SetFg(Color::Reset)
        ));
        let this = &listing[&artist_ptr][*album];
        for track in this {
            let track_title = if Some(*track) == playing && playing_here {
                format!(
                    "{}{}{}",
                    SetFg(Color::Green),
                    lookup[track].tag.title,
                    SetFg(Color::Reset)
                )
            } else {
                format!("{}", lookup[track].tag.title)
            };
            if *track == tracks[track_ptr[&artist_ptr]] {
                if focus == 0 {
                    result.push(format!("{} {}", vertical_bar, track_title,));
                } else {
                    result.push(format!(
                        "{} {}{}{}",
                        vertical_bar,
                        SetBg(Color::DarkGrey),
                        track_title,
                        SetBg(Color::Reset)
                    ));
                }
            } else {
                result.push(format!("{} {}", vertical_bar, track_title));
            }
        }
    }
    // Fill spaces
    if artists.len() > result.len() {
        let left = artists.len() - result.len();
        for _ in 0..left {
            result.push("".to_string());
        }
    }
    let empty = "".to_string();
    if result.len() > artists.len() {
        let left = result.len() - artists.len();
        for _ in 0..left {
            artists.push(&empty);
        }
    }
    // Splice lhs of table
    let pad = find_longest(&artists);
    for (row, artist) in result.iter_mut().zip(&artists) {
        if **artist == artist_ptr {
            if focus == 0 {
                *row = format!(
                    "{}{}{} {}",
                    SetBg(Color::DarkGrey),
                    align_left(artist, pad),
                    SetBg(Color::Reset),
                    row
                );
            } else {
                *row = format!(
                    "{}{}{} {}",
                    SetFg(Color::DarkBlue),
                    align_left(artist, pad),
                    SetFg(Color::Reset),
                    row
                );
            }
        } else {
            *row = format!("{} {}", align_left(artist, pad), row);
        }
    }
    result
}

pub fn artist_tracks(
    listing: &BTreeMap<String, BTreeMap<String, Vec<usize>>>,
    artist: &String,
) -> Vec<usize> {
    let albums: Vec<&String> = listing[artist].keys().collect();
    let mut result = vec![];
    for album in albums {
        for track in &listing[artist][album] {
            result.push(*track);
        }
    }
    result
}

pub fn format_playlist(
    playlist: &HashMap<String, Vec<usize>>,
    display: &Vec<String>,
    focus: u8,
    lookup: &HashMap<usize, Track>,
    selection: (&String, &HashMap<String, usize>),
    up_ptr: Option<usize>,
    playing_playlist: &Option<String>,
    width: u16,
    icon: &str,
) -> Vec<String> {
    let (selection, track_ptr) = selection;
    let mut result = vec![];
    let longest = find_longest_no_ref(display);
    if playlist.is_empty() {
        return vec![
            "No playlists have been created yet".to_string(),
            "Press `n` to create one!".to_string(),
        ];
    }
    let this = &playlist[selection];
    // Format lhs
    for name in display.iter() {
        if name == selection && focus == 0 {
            result.push(format!(
                "{}{} {}{} {} {} {}│{}",
                SetBg(Color::DarkGrey),
                SetFg(Color::DarkBlue),
                icon,
                SetFg(Color::Reset),
                align_left(name, longest),
                SetBg(Color::Reset),
                SetFg(Color::DarkBlue),
                SetFg(Color::Reset)
            ));
        } else if name == selection && focus == 1 {
            result.push(format!(
                " {}{} {}  │{}",
                SetFg(Color::DarkBlue),
                icon,
                align_left(name, longest),
                SetFg(Color::Reset)
            ));
        } else {
            result.push(format!(
                " {} {}  {}│{}",
                icon,
                align_left(name, longest),
                SetFg(Color::DarkBlue),
                SetFg(Color::Reset)
            ));
        }
    }
    // Fill spaces
    if this.len() > result.len() {
        let left = this.len() - result.len();
        for _ in 0..left {
            result.push(format!(
                "{} {}│{}",
                " ".repeat(longest + icon.width() + 3),
                SetFg(Color::DarkBlue),
                SetFg(Color::Reset)
            ));
        }
    }
    // Generate rhs table
    let tracks: Vec<&Track> = this.iter().map(|x| &lookup[x]).collect();
    let table = pad_table(
        format_table(&tracks, 0), // NOTE: CHANGE OFFSET HERE WHEN IMPLEMENTING IN FUTURE
        (width as usize).saturating_sub(longest + icon.width() + 6),
    );
    // Format rhs
    for c in 0..std::cmp::max(result.len(), this.len()) {
        let line = result.get_mut(c).unwrap();
        let track = table.get(c);
        let this_row = up_ptr == Some(c);
        let empty = "".to_string();
        let text = if let Some(line) = track { line } else { &empty };
        let title = if this_row && &Some(selection.to_string()) == playing_playlist {
            format!("{}{}{}", SetFg(Color::Green), text, SetFg(Color::Reset))
        } else {
            text.to_string()
        };
        if track_ptr[selection] == c && focus == 1 {
            *line += &format!(
                " {}{}{}",
                SetBg(Color::DarkGrey),
                title,
                SetBg(Color::Reset)
            );
        } else {
            *line += &format!(" {}", title);
        }
    }
    result
}
