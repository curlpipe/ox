use crate::event::{Error, Event, Result};
use crate::map::form_map;
use crate::utils::{get_range, tab_boundaries_backward};
use crate::{Document, Loc};
use std::ops::RangeBounds;

impl Document {
    /// Inserts a string into this document.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn insert(&mut self, loc: &Loc, st: &str) -> Result<()> {
        self.out_of_range(loc.x, loc.y)?;
        // Move cursor to location
        self.move_to(loc);
        // Update rope
        let idx = self.loc_to_file_pos(loc);
        self.file.insert(idx, st);
        // Update cache
        let line: String = self.file.line(loc.y).chars().collect();
        self.lines[loc.y] = line.trim_end_matches(['\n', '\r']).to_string();
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
        self.move_to_x(loc.x + st.chars().count());
        self.old_cursor = self.loc().x;
        Ok(())
    }

    /// Deletes a character at a location whilst checking for tab spaces
    ///
    /// # Errors
    /// This code will error if the location is invalid
    pub fn delete_with_tab(&mut self, loc: &Loc, st: &str) -> Result<()> {
        // Check for tab spaces
        let boundaries =
            tab_boundaries_backward(&self.line(loc.y).unwrap_or_default(), self.tab_width);
        if boundaries.contains(&loc.x.saturating_add(1)) && !self.in_redo {
            // Register other delete actions to delete the whole tab
            let mut loc_copy = *loc;
            self.delete(loc.x..=loc.x + st.chars().count(), loc.y)?;
            for _ in 1..self.tab_width {
                loc_copy.x = loc_copy.x.saturating_sub(1);
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
        self.move_to(&Loc::at(start, y));
        start += line_start;
        end += line_start;
        let removed = self.file.slice(start..end).to_string();
        // Update unicode and tab map
        self.dbl_map.shift_deletion(
            &Loc::at(line_start, y),
            (start, end),
            &removed,
            self.tab_width,
        );
        self.tab_map.shift_deletion(
            &Loc::at(line_start, y),
            (start, end),
            &removed,
            self.tab_width,
        );
        // Update rope
        self.file.remove(start..end);
        // Update cache
        let line: String = self.file.line(y).chars().collect();
        self.lines[y] = line.trim_end_matches(['\n', '\r']).to_string();
        self.old_cursor = self.loc().x;
        Ok(())
    }

    /// Inserts a line into the document.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn insert_line(&mut self, loc: usize, contents: String) -> Result<()> {
        if !(self.lines.is_empty() || self.len_lines() == 0 && loc == 0) {
            self.out_of_range(0, loc.saturating_sub(1))?;
        }
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
        self.info.loaded_to += 1;
        // Goto line
        self.move_to_y(loc);
        self.old_cursor = self.loc().x;
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
        // Shift down other line numbers in the hashmap
        self.dbl_map.shift_up(loc);
        self.tab_map.shift_up(loc);
        // Update cache
        self.lines.remove(loc);
        // Update rope
        let idx_start = self.file.line_to_char(loc);
        let idx_end = self.file.line_to_char(loc + 1);
        self.file.remove(idx_start..idx_end);
        self.info.loaded_to = self.info.loaded_to.saturating_sub(1);
        // Goto line
        self.move_to_y(loc);
        self.old_cursor = self.loc().x;
        Ok(())
    }

    /// Split a line in half, putting the right hand side below on a new line.
    /// For when the return key is pressed.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn split_down(&mut self, loc: &Loc) -> Result<()> {
        self.out_of_range(loc.x, loc.y)?;
        // Gather context
        let line = self.line(loc.y).ok_or(Error::OutOfRange)?;
        let rhs: String = line.chars().skip(loc.x).collect();
        self.delete(loc.x.., loc.y)?;
        self.insert_line(loc.y + 1, rhs)?;
        self.move_to(&Loc::at(0, loc.y + 1));
        self.old_cursor = self.loc().x;
        Ok(())
    }

    /// Remove the line below the specified location and append that to it.
    /// For when backspace is pressed on the start of a line.
    /// # Errors
    /// Returns an error if location is out of range.
    pub fn splice_up(&mut self, y: usize) -> Result<()> {
        self.out_of_range(0, y + 1)?;
        // Gather context
        let length = self.line(y).ok_or(Error::OutOfRange)?.chars().count();
        let below = self.line(y + 1).ok_or(Error::OutOfRange)?;
        self.delete_line(y + 1)?;
        self.insert(&Loc::at(length, y), &below)?;
        self.move_to(&Loc::at(length, y));
        self.old_cursor = self.loc().x;
        Ok(())
    }
}
