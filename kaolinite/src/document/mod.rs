/// document.rs - has Document, for opening, editing and saving documents
use crate::event::{Error, Event, EventMgmt, Result};
use crate::map::CharMap;
use crate::searching::{Match, Searcher};
use crate::utils::{modeline, width, Loc, Size};
use ropey::Rope;
use std::path::Path;

pub mod cursor;
pub mod disk;
pub mod editing;
pub mod lines;
pub mod words;

pub use cursor::Cursor;
pub use disk::DocumentInfo;

/// A document struct manages a file.
/// It has tools to read, write and traverse a document.
/// By default, it uses file buffering so it can open almost immediately.
/// To start executing events, remember to use the `Document::exe` function and check out
/// the documentation for `Event` to learn how to form editing events.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Document {
    /// The file name of the document opened
    pub file_name: Option<String>,
    /// The rope of the document to facilitate reading and writing to disk
    pub file: Rope,
    /// Cache of all the loaded lines in this document
    pub lines: Vec<String>,
    /// Stores information about the underlying file
    pub info: DocumentInfo,
    /// Stores the locations of double width characters
    pub dbl_map: CharMap,
    /// Stores the locations of tab characters
    pub tab_map: CharMap,
    /// Contains the size of this document for purposes of offset
    pub size: Size,
    /// Contains the cursor data structure
    pub cursor: Cursor,
    /// Contains the offset (scrolling for longer documents)
    pub offset: Loc,
    /// Keeps track of where the character pointer is
    pub char_ptr: usize,
    /// Manages events, for the purpose of undo and redo
    pub event_mgmt: EventMgmt,
    /// Storage of the old cursor x position (to snap back to)
    pub old_cursor: usize,
    /// Flag for if the editor is currently in a redo action
    pub in_redo: bool,
    /// The number of spaces a tab should be rendered as
    pub tab_width: usize,
    /// Secondary cursor (for multi-cursors)
    pub secondary_cursors: Vec<Loc>,
}

impl Document {
    /// Determine the file type of this file (represented by an extension)
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn get_file_type(&self) -> Option<&str> {
        let mut result = None;
        // Try to use modeline first off
        if let Some(first_line) = self.lines.first() {
            result = modeline(first_line);
        }
        // If an extension is available, use that instead
        if let Some(file_name) = &self.file_name {
            if let Some(extension) = Path::new(file_name).extension() {
                result = extension.to_str();
            }
        }
        result
    }

    /// Sets the tab display width measured in spaces, default being 4
    pub fn set_tab_width(&mut self, tab_width: usize) {
        self.tab_width = tab_width;
    }

    /// Execute an event, registering it in the undo / redo.
    /// You should always edit a document through this method to ensure undo and redo work.
    /// # Errors
    /// Will return an error if the event was unable to be completed.
    pub fn exe(&mut self, ev: Event) -> Result<()> {
        if !self.info.read_only {
            self.event_mgmt.last_event = Some(ev.clone());
            self.forth(ev)?;
        }
        self.cancel_selection();
        Ok(())
    }

    /// Undo the last patch in the document.
    /// # Errors
    /// Will return an error if any of the events failed to be reversed.
    pub fn undo(&mut self) -> Result<()> {
        if let Some(s) = self.event_mgmt.undo(self.take_snapshot()) {
            self.apply_snapshot(s);
        }
        Ok(())
    }

    /// Redo the last patch in the document.
    /// # Errors
    /// Will return an error if any of the events failed to be re-executed.
    pub fn redo(&mut self) -> Result<()> {
        if let Some(s) = self.event_mgmt.redo(&self.take_snapshot()) {
            self.apply_snapshot(s);
        }
        Ok(())
    }

    /// Handle an editing event, use the method `exe` for executing events.
    /// # Errors
    /// Returns an error if there is a problem with the specified operation.
    pub fn forth(&mut self, ev: Event) -> Result<()> {
        // Perform the event
        match ev {
            Event::Insert(loc, ch) => self.insert(&loc, &ch),
            Event::Delete(loc, st) => self.delete_with_tab(&loc, &st),
            Event::InsertLine(loc, st) => self.insert_line(loc, st),
            Event::DeleteLine(loc, _) => self.delete_line(loc),
            Event::SplitDown(loc) => self.split_down(&loc),
            Event::SpliceUp(loc) => self.splice_up(loc.y),
        }
    }

    /// Takes a loc and converts it into a char index for ropey
    #[must_use]
    pub fn loc_to_file_pos(&self, loc: &Loc) -> usize {
        self.file.line_to_char(loc.y) + loc.x
    }

    /// Function to search the document to find the next occurance of a regex
    pub fn next_match(&mut self, regex: &str, inc: usize) -> Option<Match> {
        // Prepare
        let mut srch = Searcher::new(regex);
        // Check current line for matches
        let current: String = self
            .line(self.loc().y)?
            .chars()
            .skip(self.char_ptr + inc)
            .collect();
        if let Some(mut mtch) = srch.lfind(&current) {
            mtch.loc.y = self.loc().y;
            mtch.loc.x += self.char_ptr + inc;
            return Some(mtch);
        }
        // Check subsequent lines for matches
        let mut line_no = self.loc().y + 1;
        self.load_to(line_no + 1);
        while let Some(line) = self.line(line_no) {
            if let Some(mut mtch) = srch.lfind(&line) {
                mtch.loc.y = line_no;
                return Some(mtch);
            }
            line_no += 1;
            self.load_to(line_no + 1);
        }
        None
    }

    /// Function to search the document to find the previous occurance of a regex
    pub fn prev_match(&mut self, regex: &str) -> Option<Match> {
        // Prepare
        let mut srch = Searcher::new(regex);
        // Check current line for matches
        let current: String = self
            .line(self.loc().y)?
            .chars()
            .take(self.char_ptr)
            .collect();
        if let Some(mut mtch) = srch.rfind(&current) {
            mtch.loc.y = self.loc().y;
            return Some(mtch);
        }
        // Check antecedent lines for matches
        self.load_to(self.loc().y + 1);
        let mut line_no = self.loc().y.saturating_sub(1);
        while let Some(line) = self.line(line_no) {
            if let Some(mut mtch) = srch.rfind(&line) {
                mtch.loc.y = line_no;
                return Some(mtch);
            }
            if line_no == 0 {
                break;
            }
            line_no = line_no.saturating_sub(1);
        }
        None
    }

    /// Replace a specific part of the document with another string.
    /// # Errors
    /// Will error if the replacement failed to be executed.
    pub fn replace(&mut self, loc: Loc, target: &str, into: &str) -> Result<()> {
        self.exe(Event::Delete(loc, target.to_string()))?;
        self.exe(Event::Insert(loc, into.to_string()))?;
        Ok(())
    }

    /// Replace all instances of a regex with another string
    pub fn replace_all(&mut self, target: &str, into: &str) {
        self.move_to(&Loc::at(0, 0));
        while let Some(mtch) = self.next_match(target, 1) {
            drop(self.replace(mtch.loc, &mtch.text, into));
        }
    }

    /// Brings the cursor into the viewport so it can be seen
    pub fn bring_cursor_in_viewport(&mut self) {
        if self.offset.y > self.cursor.loc.y {
            self.offset.y = self.cursor.loc.y;
        }
        if self.offset.y + self.size.h <= self.cursor.loc.y {
            self.offset.y = self.cursor.loc.y.saturating_sub(self.size.h) + 1;
        }
        if self.offset.x > self.cursor.loc.x {
            self.offset.x = self.cursor.loc.x;
        }
        if self.offset.x + self.size.w <= self.cursor.loc.x {
            self.offset.x = self.cursor.loc.x.saturating_sub(self.size.w) + 1;
        }
        self.load_to(self.offset.y + self.size.h);
    }

    /// Determines if specified coordinates are out of range of the document.
    /// # Errors
    /// Returns an error when the given coordinates are out of range.
    /// # Panics
    /// When you try using this function on a location that has not yet been loaded into buffer
    /// If you see this error, you should double check that you have used `Document::load_to`
    /// enough
    pub fn out_of_range(&self, x: usize, y: usize) -> Result<()> {
        let msg = "Did you forget to use load_to?";
        if y >= self.len_lines() || x > self.line(y).expect(msg).chars().count() {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Determines if a range is in range of the document.
    /// # Errors
    /// Returns an error when the given range is out of range.
    pub fn valid_range(&self, start: usize, end: usize, y: usize) -> Result<()> {
        self.out_of_range(start, y)?;
        self.out_of_range(end, y)?;
        if start > end {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Calculate the character index from the display index on a certain line
    #[must_use]
    pub fn character_idx(&self, loc: &Loc) -> usize {
        let mut idx = loc.x;
        // Account for double width characters
        idx = idx.saturating_sub(self.dbl_map.count(loc, true).unwrap_or(0));
        // Account for tab characters
        let tabs_behind = self.tab_map.count(loc, true).unwrap_or(0);
        idx = if let Some(inner_idx) = self.tab_map.inside(self.tab_width, loc.x, loc.y) {
            // Display index is within a tab, account for it properly
            let existing_tabs = tabs_behind.saturating_sub(1) * self.tab_width.saturating_sub(1);
            idx.saturating_sub(existing_tabs + inner_idx)
        } else {
            // Display index isn't in a tab
            idx.saturating_sub(tabs_behind * self.tab_width.saturating_sub(1))
        };
        idx
    }

    /// Calculate the display index from the character index on a certain line
    fn display_idx(&self, loc: &Loc) -> usize {
        let mut idx = loc.x;
        // Account for double width characters
        idx += self.dbl_map.count(loc, false).unwrap_or(0);
        // Account for tab characters
        idx += self.tab_map.count(loc, false).unwrap_or(0) * self.tab_width.saturating_sub(1);
        idx
    }

    /// A utility function to update the character pointer when moving up or down
    fn update_char_ptr(&mut self) {
        let mut idx = self.loc().x;
        let dbl_count = self.dbl_map.count(&self.loc(), true).unwrap_or(0);
        idx = idx.saturating_sub(dbl_count);
        let tab_count = self.tab_map.count(&self.loc(), true).unwrap_or(0);
        idx = idx.saturating_sub(tab_count * self.tab_width.saturating_sub(1));
        self.char_ptr = idx;
    }

    /// A utility function to make sure the cursor doesn't go out of range when moving
    fn fix_dangling_cursor(&mut self) {
        if let Some(line) = self.line(self.loc().y) {
            if self.loc().x > width(&line, self.tab_width) {
                self.select_to_x(line.chars().count());
            }
        } else {
            self.select_home();
        }
    }

    /// Fixes double width and tab boundary issues
    fn fix_split(&mut self) {
        let mut magnitude = 0;
        let Loc { x, y } = self.loc();
        if let Some(map) = self.dbl_map.get(y) {
            let last_dbl = self
                .dbl_map
                .count(&self.loc(), true)
                .unwrap()
                .saturating_sub(1);
            let start = map[last_dbl].0;
            if x == start + 1 {
                magnitude += 1;
            }
        }
        if let Some(map) = self.tab_map.get(y) {
            let last_tab = self
                .tab_map
                .count(&self.loc(), true)
                .unwrap()
                .saturating_sub(1);
            let start = map[last_tab].0;
            let range = start..start + self.tab_width;
            if range.contains(&x) {
                magnitude += x.saturating_sub(start);
            }
        }
        self.cursor.loc.x = self.cursor.loc.x.saturating_sub(magnitude);
    }

    /// Determine if a character at a certain location is a double width character.
    /// x is the display index.
    #[must_use]
    pub fn is_dbl_width(&self, y: usize, x: usize) -> bool {
        if let Some(line) = self.dbl_map.get(y) {
            line.iter().any(|i| x == i.1)
        } else {
            false
        }
    }

    /// Determine if a character at a certain location is a tab character.
    /// x is the display index.
    #[must_use]
    pub fn is_tab(&self, y: usize, x: usize) -> bool {
        if let Some(line) = self.tab_map.get(y) {
            line.iter().any(|i| x == i.1)
        } else {
            false
        }
    }

    /// Determine the width of a character at a certain location
    #[must_use]
    pub fn width_of(&self, y: usize, x: usize) -> usize {
        if self.is_dbl_width(y, x) {
            2
        } else if self.is_tab(y, x) {
            self.tab_width
        } else {
            1
        }
    }

    /// Commit a change to the undo management system
    pub fn commit(&mut self) {
        let s = self.take_snapshot();
        self.event_mgmt.commit(s);
    }

    /// Completely reload the file
    pub fn reload_lines(&mut self) {
        let to = std::mem::take(&mut self.info.loaded_to);
        self.lines.clear();
        self.load_to(to);
    }
}
