/// document.rs - has Document, for opening, editing and saving documents
use crate::event::{Error, Event, Result, Status, EventMgmt};
use crate::map::{CharMap, form_map};
use crate::searching::{Searcher, Match};
use crate::utils::{Loc, Size, get_range, trim, width, tab_boundaries_backward, tab_boundaries_forward};
use ropey::Rope;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::ops::RangeBounds;

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
    /// Contains the number of lines buffered into the document
    pub loaded_to: usize,
    /// Cache of all the loaded lines in this document
    pub lines: Vec<String>,
    /// Stores the locations of double width characters
    pub dbl_map: CharMap,
    /// Stores the locations of tab characters
    pub tab_map: CharMap,
    /// Contains the size of this document for purposes of offset
    pub size: Size,
    /// Contains where the cursor is within the terminal
    pub cursor: Loc,
    /// Contains the offset (scrolling for longer documents)
    pub offset: Loc,
    /// Keeps track of where the character pointer is
    pub char_ptr: usize,
    /// Manages events, for the purpose of undo and redo
    pub event_mgmt: EventMgmt,
    /// true if the file has been modified since saving, false otherwise
    pub modified: bool,
    /// The number of spaces a tab should be rendered as
    pub tab_width: usize,
    /// Whether or not the document can be edited
    pub read_only: bool,
    /// Storage of the old cursor x position (to snap back to)
    pub old_cursor: usize,
    /// Flag for if the editor is currently in a redo action
    pub in_redo: bool,
}

impl Document {
    /// Creates a new, empty document with no file name.
    #[cfg(not(tarpaulin_include))]
    pub fn new(size: Size) -> Self {
        Self {
            file: Rope::from_str("\n"),
            lines: vec!["".to_string()],
            dbl_map: CharMap::default(),
            tab_map: CharMap::default(),
            loaded_to: 1,
            file_name: None,
            cursor: Loc::default(),
            offset: Loc::default(),
            size,
            char_ptr: 0,
            event_mgmt: EventMgmt::default(),
            modified: false,
            tab_width: 4,
            read_only: false,
            old_cursor: 0,
            in_redo: false,
        }
    }

    /// Open a document from a file name.
    /// # Errors
    /// Returns an error when file doesn't exist, or has incorrect permissions.
    /// Also returns an error if the rope fails to initialise due to character set issues or
    /// disk errors.
    #[cfg(not(tarpaulin_include))]
    pub fn open<S: Into<String>>(size: Size, file_name: S) -> Result<Self> {
        let file_name = file_name.into();
        Ok(Self {
            file: Rope::from_reader(BufReader::new(File::open(&file_name)?))?,
            lines: vec![],
            dbl_map: CharMap::default(),
            tab_map: CharMap::default(),
            loaded_to: 0,
            file_name: Some(file_name),
            cursor: Loc::default(),
            offset: Loc::default(),
            size,
            char_ptr: 0,
            event_mgmt: EventMgmt::default(),
            modified: false,
            tab_width: 4,
            read_only: false,
            old_cursor: 0,
            in_redo: false,
        })
    }

    /// Sets the tab display width measured in spaces, default being 4
    pub fn set_tab_width(&mut self, tab_width: usize) {
        self.tab_width = tab_width;
    }

    /// Save back to the file the document was opened from.
    /// # Errors
    /// Returns an error if the file fails to write, due to permissions
    /// or character set issues.
    pub fn save(&mut self) -> Result<()> {
        if !self.read_only {
            self.modified = false;
            if let Some(file_name) = &self.file_name {
                self.file.write_to(BufWriter::new(File::create(file_name)?))?;
                Ok(())
            } else {
                Err(Error::NoFileName)
            }
        } else {
            Err(Error::ReadOnlyFile)
        }
    }

    /// Save to a specified file.
    /// # Errors
    /// Returns an error if the file fails to write, due to permissions
    /// or character set issues.
    pub fn save_as(&self, file_name: &str) -> Result<()> {
        if !self.read_only {
            self.file.write_to(BufWriter::new(File::create(file_name)?))?;
            Ok(())
        } else {
            Err(Error::ReadOnlyFile)
        }
    }

    /// Execute an event, registering it in the undo / redo.
    /// You should always edit a document through this method to ensure undo and redo work.
    /// # Errors
    /// Will return an error if the event was unable to be completed.
    pub fn exe(&mut self, ev: Event) -> Result<()> {
        if !self.read_only {
            self.event_mgmt.register(ev.clone());
            self.forth(ev)?;
        }
        Ok(())
    }

    /// Undo the last patch in the document.
    /// # Errors
    /// Will return an error if any of the events failed to be reversed.
    pub fn undo(&mut self) -> Result<()> {
        for ev in self.event_mgmt.undo().unwrap_or_default() {
            self.forth(ev.reverse())?;
        }
        self.modified = !self.event_mgmt.is_undo_empty();
        Ok(())
    }

    /// Redo the last patch in the document.
    /// # Errors
    /// Will return an error if any of the events failed to be re-executed.
    pub fn redo(&mut self) -> Result<()> {
        self.in_redo = true;
        for ev in self.event_mgmt.redo().unwrap_or_default() {
            self.forth(ev)?;
        }
        self.modified = true;
        self.in_redo = false;
        Ok(())
    }

    /// Handle an editing event, use the method `exe` for executing events.
    /// # Errors
    /// Returns an error if there is a problem with the specified operation.
    pub fn forth(&mut self, ev: Event) -> Result<()> {
        match ev {
            Event::Insert(loc, ch) => self.insert(&loc, &ch),
            Event::Delete(loc, st) => self.delete_with_tab(&loc, &st),
            Event::InsertLine(loc, st) => self.insert_line(loc, st),
            Event::DeleteLine(loc, _) => self.delete_line(loc),
            Event::SplitDown(loc) => self.split_down(&loc),
            Event::SpliceUp(loc) => self.splice_up(loc.y),
        }
    }

    /// Inserts a string into this document.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn insert(&mut self, loc: &Loc, st: &str) -> Result<()> {
        self.out_of_range(loc.x, loc.y)?;
        self.modified = true;
        // Move cursor to location
        self.goto(loc);
        // Update rope
        let idx = self.file.line_to_char(loc.y) + loc.x;
        self.file.insert(idx, st);
        // Update cache
        let line: String = self.file.line(loc.y).chars().collect();
        self.lines[loc.y] = line.trim_end_matches(&['\n', '\r']).to_string();
        // Update unicode map
        let dbl_start = self.dbl_map.shift_insertion(loc, st, self.tab_width);
        let tab_start = self.tab_map.shift_insertion(loc, st, self.tab_width);
        // Register new double widths and tabs
        let (mut dbls, mut tabs) = form_map(st, self.tab_width);
        // Shift up to match insertion position in the document
        let tab_shift = self.tab_width.saturating_sub(1) * tab_start;
        for e in &mut dbls {
            *e = (e.0 + loc.x + dbl_start + tab_shift, e.1 + loc.x);
        }
        for e in &mut tabs {
            *e = (e.0 + loc.x + tab_shift + dbl_start, e.1 + loc.x);
        }
        self.dbl_map.splice(loc, dbl_start, dbls);
        self.tab_map.splice(loc, tab_start, tabs);
        // Go to end x position
        self.goto_x(loc.x + st.chars().count());
        self.old_cursor = self.char_ptr;
        Ok(())
    }

    /// Deletes a character at a location whilst checking for tab spaces
    pub fn delete_with_tab(&mut self, loc: &Loc, st: &str) -> Result<()> {
        // Check for tab spaces
        let boundaries = tab_boundaries_backward(
            &self.line(loc.y).unwrap_or_else(|| "".to_string()), 
            self.tab_width
        );
        if boundaries.contains(&loc.x.saturating_add(1)) && !self.in_redo {
            // Register other delete actions to delete the whole tab
            let mut loc_copy = loc.clone();
            self.delete(loc.x..=loc.x + st.chars().count(), loc.y)?;
            for _ in 1..self.tab_width {
                loc_copy.x -= 1;
                self.exe(Event::Delete(loc_copy, " ".to_string()))?;
            }
            Ok(())
        } else {
            // Normal character delete
            self.delete(loc.x..=loc.x + st.chars().count(), loc.y)
        }
    }

    /// Deletes a range from this document.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn delete<R>(&mut self, x: R, y: usize) -> Result<()>
    where
        R: RangeBounds<usize>,
    {
        let line_start = self.file.try_line_to_char(y)?;
        let line_end = line_start + self.line(y).ok_or(Error::OutOfRange)?.chars().count();
        // Extract range information
        let (mut start, mut end) = get_range(&x, line_start, line_end);
        self.valid_range(start, end, y)?;
        self.modified = true;
        self.goto(&Loc::at(start, y));
        start += line_start;
        end += line_start;
        let removed = self.file.slice(start..end).to_string();
        // Update unicode and tab map
        self.dbl_map.shift_deletion(&Loc::at(line_start, y), (start, end), &removed, self.tab_width);
        self.tab_map.shift_deletion(&Loc::at(line_start, y), (start, end), &removed, self.tab_width);
        // Update rope
        self.file.remove(start..end);
        // Update cache
        let line: String = self.file.line(y).chars().collect();
        self.lines[y] = line.trim_end_matches(&['\n', '\r']).to_string();
        self.old_cursor = self.char_ptr;
        Ok(())
    }

    /// Inserts a line into the document.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn insert_line(&mut self, loc: usize, contents: String) -> Result<()> {
        if !self.lines.is_empty() {
            if !(self.len_lines() == 0 && loc == 0) {
                self.out_of_range(0, loc.saturating_sub(1))?;
            }
        }
        self.modified = true;
        // Update unicode and tab map
        self.dbl_map.shift_down(loc);
        self.tab_map.shift_down(loc);
        // Calculate the unicode map and tab map of this line
        let (dbl_map, tab_map) = form_map(&contents, self.tab_width);
        self.dbl_map.insert(loc, dbl_map);
        self.tab_map.insert(loc, tab_map);
        // Update cache
        self.lines.insert(loc, contents.to_string());
        // Update rope
        let char_idx = self.file.line_to_char(loc);
        self.file.insert(char_idx, &(contents + "\n"));
        self.loaded_to += 1;
        // Goto line
        self.goto_y(loc);
        self.old_cursor = self.char_ptr;
        Ok(())
    }

    /// Deletes a line from the document.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn delete_line(&mut self, loc: usize) -> Result<()> {
        self.out_of_range(0, loc)?;
        // Update tab & unicode map
        self.dbl_map.delete(loc);
        self.tab_map.delete(loc);
        self.modified = true;
        // Shift down other line numbers in the hashmap
        self.dbl_map.shift_up(loc);
        self.tab_map.shift_up(loc);
        // Update cache
        self.lines.remove(loc);
        // Update rope
        let idx_start = self.file.line_to_char(loc);
        let idx_end = self.file.line_to_char(loc + 1);
        self.file.remove(idx_start..idx_end);
        self.loaded_to = self.loaded_to.saturating_sub(1);
        // Goto line
        self.goto_y(loc);
        self.old_cursor = self.char_ptr;
        Ok(())
    }

    /// Split a line in half, putting the right hand side below on a new line.
    /// For when the return key is pressed.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn split_down(&mut self, loc: &Loc) -> Result<()> {
        self.out_of_range(loc.x, loc.y)?;
        self.modified = true;
        // Gather context
        let line = self.line(loc.y).ok_or(Error::OutOfRange)?;
        let rhs: String = line.chars().skip(loc.x).collect();
        self.delete(loc.x.., loc.y)?;
        self.insert_line(loc.y + 1, rhs)?;
        self.goto(&Loc::at(0, loc.y + 1));
        self.old_cursor = self.char_ptr;
        Ok(())
    }

    /// Remove the line below the specified location and append that to it.
    /// For when backspace is pressed on the start of a line.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn splice_up(&mut self, y: usize) -> Result<()> {
        self.out_of_range(0, y + 1)?;
        self.modified = true;
        // Gather context
        let length = self.line(y).ok_or(Error::OutOfRange)?.chars().count();
        let below = self.line(y + 1).ok_or(Error::OutOfRange)?;
        self.delete_line(y + 1)?;
        self.insert(&Loc::at(length, y), &below)?;
        self.goto(&Loc::at(length, y));
        self.old_cursor = self.char_ptr;
        Ok(())
    }

    /// Move the cursor up
    pub fn move_up(&mut self) -> Status {
        // Return if already at start of document
        if self.loc().y == 0 {
            return Status::StartOfFile;
        }
        // Move up one line
        if self.cursor.y == 0 {
            self.offset.y -= 1;
        } else {
            self.cursor.y -= 1;
        }
        // Snap to end of line
        self.fix_dangling_cursor();
        // Move back if in the middle of a longer character
        self.fix_split();
        // Update the character pointer
        self.update_char_ptr();
        self.goto_x(self.old_cursor);
        Status::None
    }

    /// Move the cursor down
    pub fn move_down(&mut self) -> Status {
        // Return if already on end of document
        if self.len_lines() < self.loc().y + 1 {
            return Status::EndOfFile;
        }
        // Ensure that line is loaded from buffer
        self.load_to(self.loc().y + 2);
        // Move down one line
        if self.cursor.y == self.size.h.saturating_sub(1) {
            self.offset.y += 1;
        } else {
            self.cursor.y += 1;
        }
        // Snap to end of line
        self.fix_dangling_cursor();
        // Move back if in the middle of a longer character
        self.fix_split();
        // Update the character pointer
        self.update_char_ptr();
        self.goto_x(self.old_cursor);
        //panic!("{}", self.old_cursor);
        Status::None
    }

    /// Move the cursor left
    pub fn move_left(&mut self) -> Status {
        // Return if already at start of line
        if self.loc().x == 0 {
            return Status::StartOfLine;
        }
        // Determine the width of the character to traverse
        let line = self.line(self.loc().y).unwrap_or_else(|| "".to_string());
        let boundaries = tab_boundaries_backward(&line, self.tab_width);
        let width = if boundaries.contains(&self.char_ptr) {
            // Push the character pointer up
            self.char_ptr -= self.tab_width.saturating_sub(1);
            // There are spaces that should be treated as tabs (so should traverse the tab width)
            self.tab_width
        } else {
            // There are no spaces that should be treated as tabs
            self.width_of(self.loc().y, self.char_ptr.saturating_sub(1))
        };
        // Move back the correct amount
        for _ in 0..width {
            if self.cursor.x == 0 {
                self.offset.x -= 1;
            } else {
                self.cursor.x -= 1;
            }
        }
        // Update the character pointer
        self.char_ptr -= 1;
        self.old_cursor = self.char_ptr;
        Status::None
    }

    /// Move the cursor right
    pub fn move_right(&mut self) -> Status {
        // Return if already on end of line
        let line = self.line(self.loc().y).unwrap_or_else(|| "".to_string());
        let width = width(&line, self.tab_width);
        if width == self.loc().x {
            return Status::EndOfLine;
        }
        // Determine the width of the character to traverse
        let boundaries = tab_boundaries_forward(&line, self.tab_width);
        let width = if boundaries.contains(&self.char_ptr) {
            // Push the character pointer up
            self.char_ptr += self.tab_width.saturating_sub(1);
            // There are spaces that should be treated as tabs (so should traverse the tab width)
            self.tab_width
        } else {
            // There are no spaces that should be treated as tabs
            self.width_of(self.loc().y, self.char_ptr)
        };
        // Move forward the correct amount
        for _ in 0..width {
            if self.cursor.x == self.size.w.saturating_sub(1) {
                self.offset.x += 1;
            } else {
                self.cursor.x += 1;
            }
        }
        // Update the character pointer
        self.char_ptr += 1;
        self.old_cursor = self.char_ptr;
        Status::None
    }

    /// Move to the start of the line
    pub fn move_home(&mut self) {
        self.cursor.x = 0;
        self.offset.x = 0;
        self.char_ptr = 0;
        self.old_cursor = 0;
    }

    /// Move to the end of the line
    pub fn move_end(&mut self) {
        let line = self.line(self.loc().y).unwrap_or_else(|| "".to_string());
        let length = line.chars().count();
        self.goto_x(length);
        self.old_cursor = self.char_ptr;
    }

    /// Move to the top of the document
    pub fn move_top(&mut self) {
        self.goto(&Loc::at(0, 0));
        self.old_cursor = self.char_ptr;
    }

    /// Move to the bottom of the document
    pub fn move_bottom(&mut self) {
        let last = self.len_lines();
        self.goto(&Loc::at(0, last));
        self.old_cursor = self.char_ptr;
    }

    /// Move up by 1 page
    pub fn move_page_up(&mut self) {
        // Shift viewport to have current line at top of the document
        self.offset.y += self.cursor.y;
        let y = self.cursor.y;
        self.cursor.y = 0;
        self.char_ptr = 0;
        self.cursor.x = 0;
        self.offset.x = 0;
        self.old_cursor = 0;
        // Shift the offset up by 1 page
        self.offset.y = self.offset.y.saturating_sub(self.size.h + y);
    }

    /// Move down by 1 page
    pub fn move_page_down(&mut self) {
        // Shift viewport to have current line at top of document
        self.offset.y += self.cursor.y;
        let y = self.cursor.y;
        self.cursor.y = 0;
        self.char_ptr = 0;
        self.cursor.x = 0;
        self.offset.x = 0;
        self.old_cursor = 0;
        // Shift the offset down by 1 page
        let by = self.size.h.saturating_sub(y);
        let len = self.len_lines();
        if self.offset.y + by > len {
            self.offset.y = len;
        } else {
            self.offset.y += by;
            // Buffer new lines in viewport
            self.load_to(self.offset.y + self.size.h);
        }
    }

    /// Moves to the previous word in the document
    pub fn move_prev_word(&mut self) -> Status {
        let Loc { x, y } = self.char_loc();
        if x == 0 && y != 0 {
            return Status::StartOfLine;
        }
        let re = format!("(\t| {{{}}}|^| )", self.tab_width);
        if let Some(mut mtch) = self.prev_match(&re) {
            let len = mtch.text.chars().count();
            let same = mtch.loc.x + len == x;
            if !same {
                mtch.loc.x += len;
            }
            self.goto(&mtch.loc);
            if same && self.loc().x != 0 {
                return self.move_prev_word();
            }
        }
        self.old_cursor = self.char_ptr;
        Status::None
    }

    /// Moves to the next word in the document
    pub fn move_next_word(&mut self) -> Status {
        let Loc { x, y } = self.char_loc();
        let line = self.line(y).unwrap_or_else(|| "".to_string());
        if x == line.chars().count() && y != self.len_lines() {
            return Status::EndOfLine;
        }
        let re = format!("(\t| {{{}}}|$|^ +| )", self.tab_width);
        if let Some(mut mtch) = self.next_match(&re, 0) {
            mtch.loc.x += mtch.text.chars().count();
            self.goto(&mtch.loc);
        }
        self.old_cursor = self.char_ptr;
        Status::None
    }

    /// Function to search the document to find the next occurance of a regex
    pub fn next_match(&mut self, regex: &str, inc: usize) -> Option<Match> {
        // Prepare
        let mut srch = Searcher::new(regex);
        // Check current line for matches
        let current: String = self.line(self.loc().y)?
            .chars()
            .skip(self.char_ptr + inc)
            .collect();
        if let Some(mut mtch) = srch.lfind(&current) {
            mtch.loc.y = self.loc().y;
            mtch.loc.x += self.char_ptr + inc;
            return Some(mtch)
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
        let current: String = self.line(self.loc().y)?
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
            if line_no == 0 { break; }
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
        self.goto(&Loc::at(0, 0));
        while let Some(mtch) = self.next_match(target, 1) {
            drop(self.replace(mtch.loc, &mtch.text, into));
        }
    }

    /// Function to go to a specific position
    pub fn goto(&mut self, loc: &Loc) {
        self.goto_y(loc.y);
        self.goto_x(loc.x);
    }

    /// Function to go to a specific x position
    pub fn goto_x(&mut self, x: usize) {
        let line = self.line(self.loc().y).unwrap_or_else(|| "".to_string());
        // If we're already at this x coordinate, just exit
        if self.char_ptr == x {
            return;
        }
        // If the move position is out of bounds, move to the end of the line
        if line.chars().count() < x {
            let line = self.line(self.loc().y).unwrap_or_else(|| "".to_string());
            let length = line.chars().count();
            self.goto_x(length);
            return;
        }
        // Update char position
        self.char_ptr = x;
        // Calculate display index
        let x = self.display_idx(&Loc::at(x, self.loc().y));
        let viewport = self.offset.x..self.offset.x + self.size.w;
        // Move cursor
        if x < self.size.w {
            // Cursor will be in the viewport if the offset is 0
            self.offset.x = 0;
            self.cursor.x = x;
        } else if viewport.contains(&x) {
            // If the idx is already in viewport, don't adjust offset
            self.cursor.x = x - self.offset.x;
        } else {
            // Index is outside of viewport
            self.cursor.x = 0;
            self.offset.x = x;
        }
    }

    /// Function to go to a specific y position
    pub fn goto_y(&mut self, y: usize) {
        // Bounds checking
        if self.loc().y != y && y <= self.len_lines() {
            // Move cursor
            let viewport = self.offset.y..self.offset.y + self.size.h;
            if y < self.size.h {
                // Cursor will be in viewport if the offset is 0
                self.offset.y = 0;
                self.cursor.y = y;
            } else if viewport.contains(&y) {
                // If the line is in viewport already, only move the cursor
                self.cursor.y = y - self.offset.y;
            } else {
                // Index is outside of viewport
                self.cursor.y = self.size.h.saturating_sub(1);
                self.offset.y = y - (self.size.h.saturating_sub(1));
            }
        }
        // Snap to end of line
        self.fix_dangling_cursor();
        // Ensure cursor isn't in the middle of a longer character
        self.fix_split();
        // Correct the character pointer
        self.update_char_ptr();
        // Load any lines necessary
        self.load_to(self.offset.y + self.size.h);
    }

    /// Determines if specified coordinates are out of range of the document.
    /// # Errors
    /// Returns an error when the given coordinates are out of range.
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
        idx -= dbl_count;
        let tab_count = self.tab_map.count(&self.loc(), true).unwrap_or(0);
        idx -= tab_count * self.tab_width.saturating_sub(1);
        self.char_ptr = idx;
    }

    /// A utility function to make sure the cursor doesn't go out of range when moving
    fn fix_dangling_cursor(&mut self) {
        if let Some(line) = self.line(self.loc().y) {
            if self.loc().x > width(&line, self.tab_width) {
                self.goto_x(line.chars().count());
            }
        } else {
            self.move_home();
        }
    }

    /// Fixes double width and tab boundary issues
    fn fix_split(&mut self) {
        let mut magnitude = 0;
        let Loc { x, y } = self.loc();
        if let Some(map) = self.dbl_map.get(y) {
            let last_dbl = self.dbl_map.count(&self.loc(), true).unwrap().saturating_sub(1);
            let start = map[last_dbl].0;
            if x == start + 1 {
                magnitude += 1;
            }
        }
        if let Some(map) = self.tab_map.get(y) {
            let last_tab = self.tab_map.count(&self.loc(), true).unwrap().saturating_sub(1);
            let start = map[last_tab].0;
            let range = start..start + self.tab_width;
            if range.contains(&x) {
                magnitude += x - start;
            }
        }
        for _ in 0..magnitude {
            if self.cursor.x == 0 {
                self.offset.x -= 1;
            } else {
                self.cursor.x -= 1;
            }
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
        if to > self.loaded_to {
            // For each line, run through each character and make note of any double width characters
            for i in self.loaded_to..to {
                let line: String = self.file.line(i).chars().collect();
                // Add to char maps
                let (dbl_map, tab_map) = form_map(&line, self.tab_width);
                self.dbl_map.insert(i, dbl_map);
                self.tab_map.insert(i, tab_map);
                // Cache this line
                self.lines.push(line.trim_end_matches(&['\n', '\r']).to_string());
            }
            // Store new loaded point
            self.loaded_to = to;
        }
    }

    /// Get the line at a specified index
    #[must_use]
    pub fn line(&self, line: usize) -> Option<String> {
        Some(self.lines.get(line)?.to_string())
    }

    /// Get the line at a specified index and trim it
    #[must_use]
    pub fn line_trim(&self, line: usize, start: usize, length: usize) -> Option<String> {
        let line = self.line(line);
        Some(trim(&line?, start, length, self.tab_width))
    }

    /// Returns the number of lines in the document
    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.file.len_lines().saturating_sub(1)
    }

    /// Evaluate the line number text for a specific line
    #[must_use]
    pub fn line_number(&self, request: usize) -> String {
        let total = self.len_lines().to_string().len();
        let num = if request + 1 > self.len_lines() {
            "~".to_string()
        } else {
            (request + 1).to_string()
        };
        format!("{}{}", " ".repeat(total - num.len()), num)
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

    /// Get the current position within the document, including offset
    #[must_use]
    pub const fn loc(&self) -> Loc {
        Loc {
            x: self.cursor.x + self.offset.x,
            y: self.cursor.y + self.offset.y,
        }
    }

    /// Get the current position within the document, with x being the character index
    #[must_use]
    pub const fn char_loc(&self) -> Loc {
        Loc {
            x: self.char_ptr,
            y: self.cursor.y + self.offset.y,
        }
    }
}
