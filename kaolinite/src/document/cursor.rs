use crate::event::Status;
use crate::utils::{tab_boundaries_backward, tab_boundaries_forward, width};
use crate::{Document, Loc};
use std::ops::Range;

/// Defines a cursor's position and any selection it may be covering
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Cursor {
    pub loc: Loc,
    pub selection_end: Loc,
}

impl Document {
    /// Move the cursor up
    pub fn move_up(&mut self) -> Status {
        let r = self.select_up();
        self.cancel_selection();
        r
    }

    /// Select with the cursor up
    pub fn select_up(&mut self) -> Status {
        // Return if already at start of document
        if self.loc().y == 0 {
            return Status::StartOfFile;
        }
        self.cursor.loc.y = self.cursor.loc.y.saturating_sub(1);
        self.cursor.loc.x = self.old_cursor;
        // Snap to end of line
        self.fix_dangling_cursor();
        // Move back if in the middle of a longer character
        self.fix_split();
        // Update the character pointer
        self.update_char_ptr();
        self.bring_cursor_in_viewport();
        Status::None
    }

    /// Move the cursor down
    pub fn move_down(&mut self) -> Status {
        let r = self.select_down();
        self.cancel_selection();
        r
    }

    /// Select with the cursor down
    pub fn select_down(&mut self) -> Status {
        // Return if already on end of document
        if self.len_lines() < self.loc().y + 1 {
            return Status::EndOfFile;
        }
        self.cursor.loc.y += 1;
        self.cursor.loc.x = self.old_cursor;
        // Snap to end of line
        self.fix_dangling_cursor();
        // Move back if in the middle of a longer character
        self.fix_split();
        // Update the character pointer
        self.update_char_ptr();
        self.bring_cursor_in_viewport();
        Status::None
    }

    /// Move the cursor left
    pub fn move_left(&mut self) -> Status {
        let r = self.select_left();
        self.cancel_selection();
        r
    }

    /// Select with the cursor left
    pub fn select_left(&mut self) -> Status {
        // Return if already at start of line
        if self.loc().x == 0 {
            return Status::StartOfLine;
        }
        // Determine the width of the character to traverse
        let line = self.line(self.loc().y).unwrap_or_default();
        let boundaries = tab_boundaries_backward(&line, self.tab_width);
        let width = if boundaries.contains(&self.char_ptr) {
            // Push the character pointer up
            self.char_ptr = self
                .char_ptr
                .saturating_sub(self.tab_width.saturating_sub(1));
            // There are spaces that should be treated as tabs (so should traverse the tab width)
            self.tab_width
        } else {
            // There are no spaces that should be treated as tabs
            self.width_of(self.loc().y, self.char_ptr.saturating_sub(1))
        };
        // Move back the correct amount
        self.cursor.loc.x = self.cursor.loc.x.saturating_sub(width);
        // Update the character pointer
        self.char_ptr = self.char_ptr.saturating_sub(1);
        self.bring_cursor_in_viewport();
        self.old_cursor = self.loc().x;
        Status::None
    }

    /// Move the cursor right
    pub fn move_right(&mut self) -> Status {
        let r = self.select_right();
        self.cancel_selection();
        r
    }

    /// Select with the cursor right
    pub fn select_right(&mut self) -> Status {
        // Return if already on end of line
        let line = self.line(self.loc().y).unwrap_or_default();
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
        self.cursor.loc.x += width;
        // Update the character pointer
        self.char_ptr += 1;
        self.bring_cursor_in_viewport();
        self.old_cursor = self.loc().x;
        Status::None
    }

    /// Move to the start of the line
    pub fn move_home(&mut self) {
        self.select_home();
        self.cancel_selection();
    }

    /// Select to the start of the line
    pub fn select_home(&mut self) {
        self.cursor.loc.x = 0;
        self.char_ptr = 0;
        self.old_cursor = 0;
        self.bring_cursor_in_viewport();
    }

    /// Move to the end of the line
    pub fn move_end(&mut self) {
        self.select_end();
        self.cancel_selection();
    }

    /// Select to the end of the line
    pub fn select_end(&mut self) {
        let line = self.line(self.loc().y).unwrap_or_default();
        let length = line.chars().count();
        self.select_to_x(length);
        self.old_cursor = self.loc().x;
    }

    /// Move to the top of the document
    pub fn move_top(&mut self) {
        self.move_to(&Loc::at(0, 0));
    }

    /// Move to the bottom of the document
    pub fn move_bottom(&mut self) {
        let last = self.len_lines();
        self.move_to(&Loc::at(0, last));
    }

    /// Select to the top of the document
    pub fn select_top(&mut self) {
        self.select_to(&Loc::at(0, 0));
        self.old_cursor = self.loc().x;
    }

    /// Select to the bottom of the document
    pub fn select_bottom(&mut self) {
        let last = self.len_lines();
        self.select_to(&Loc::at(0, last));
        self.old_cursor = self.loc().x;
    }

    /// Move up by 1 page
    pub fn move_page_up(&mut self) {
        self.clear_cursors();
        // Set x to 0
        self.cursor.loc.x = 0;
        self.char_ptr = 0;
        self.old_cursor = 0;
        // Calculate where to move the cursor
        let new_cursor_y = self.cursor.loc.y.saturating_sub(self.size.h);
        // Move to the new location and shift down offset proportionally
        self.cursor.loc.y = new_cursor_y;
        self.offset.y = self.offset.y.saturating_sub(self.size.h);
        // Clean up
        self.cancel_selection();
    }

    /// Move down by 1 page
    pub fn move_page_down(&mut self) {
        self.clear_cursors();
        // Set x to 0
        self.cursor.loc.x = 0;
        self.char_ptr = 0;
        self.old_cursor = 0;
        // Calculate where to move the cursor
        let new_cursor_y = self.cursor.loc.y + self.size.h;
        if new_cursor_y <= self.len_lines() {
            // Cursor is in range, move to the new location and shift down offset proportionally
            self.cursor.loc.y = new_cursor_y;
            self.offset.y += self.size.h;
        } else if self.len_lines() < self.offset.y + self.size.h {
            // End line is in view, no need to move offset
            self.cursor.loc.y = self.len_lines().saturating_sub(1);
        } else {
            // Cursor would be out of range (adjust to bottom of document)
            self.cursor.loc.y = self.len_lines().saturating_sub(1);
            self.offset.y = self.len_lines().saturating_sub(self.size.h);
        }
        // Clean up
        self.load_to(self.offset.y + self.size.h);
        self.cancel_selection();
    }

    /// Function to go to a specific position
    pub fn move_to(&mut self, loc: &Loc) {
        self.select_to(loc);
        self.cancel_selection();
    }

    /// Function to go to a specific position
    pub fn select_to(&mut self, loc: &Loc) {
        self.select_to_y(loc.y);
        self.select_to_x(loc.x);
    }

    /// Function to go to a specific x position
    pub fn move_to_x(&mut self, x: usize) {
        self.select_to_x(x);
        self.cancel_selection();
    }

    /// Function to select to a specific x position
    pub fn select_to_x(&mut self, x: usize) {
        let line = self.line(self.loc().y).unwrap_or_default();
        // If the move position is out of bounds, move to the end of the line
        if line.chars().count() < x {
            let line = self.line(self.loc().y).unwrap_or_default();
            let length = line.chars().count();
            self.select_to_x(length);
            return;
        }
        // Update char position
        self.char_ptr = x;
        // Calculate display index
        let x = self.display_idx(&Loc::at(x, self.loc().y));
        // Move cursor
        self.cursor.loc.x = x;
        self.bring_cursor_in_viewport();
    }

    /// Function to go to a specific y position
    pub fn move_to_y(&mut self, y: usize) {
        self.select_to_y(y);
        self.cancel_selection();
    }

    /// Function to select to a specific y position
    pub fn select_to_y(&mut self, y: usize) {
        // Bounds checking
        if self.loc().y != y && y <= self.len_lines() {
            self.cursor.loc.y = y;
        } else if y > self.len_lines() {
            self.cursor.loc.y = self.len_lines();
        }
        // Snap to end of line
        self.fix_dangling_cursor();
        // Ensure cursor isn't in the middle of a longer character
        self.fix_split();
        // Correct the character pointer
        self.update_char_ptr();
        self.bring_cursor_in_viewport();
        // Load any lines necessary
        self.load_to(self.offset.y + self.size.h);
    }

    /// Move the view down
    pub fn scroll_down(&mut self) {
        self.offset.y += 1;
        self.load_to(self.offset.y + self.size.h);
    }

    /// Move the view up
    pub fn scroll_up(&mut self) {
        self.offset.y = self.offset.y.saturating_sub(1);
        self.load_to(self.offset.y + self.size.h);
    }

    /// Get the current position within the document, including offset
    #[must_use]
    pub const fn loc(&self) -> Loc {
        Loc {
            x: self.cursor.loc.x,
            y: self.cursor.loc.y,
        }
    }

    /// Get the current position within the document, with x being the character index
    #[must_use]
    pub const fn char_loc(&self) -> Loc {
        Loc {
            x: self.char_ptr,
            y: self.cursor.loc.y,
        }
    }

    /// If the cursor is within the viewport, this will return where it is relatively
    #[must_use]
    pub fn cursor_loc_in_screen(&self) -> Option<Loc> {
        if self.cursor.loc.x < self.offset.x {
            return None;
        }
        if self.cursor.loc.y < self.offset.y {
            return None;
        }
        let result = Loc {
            x: self.cursor.loc.x.saturating_sub(self.offset.x),
            y: self.cursor.loc.y.saturating_sub(self.offset.y),
        };
        if result.x > self.size.w || result.y >= self.size.h {
            return None;
        }
        Some(result)
    }

    /// Returns true if there is no active selection and vice versa
    #[must_use]
    pub fn is_selection_empty(&self) -> bool {
        self.cursor.loc == self.cursor.selection_end
    }

    /// Will return the bounds of the current active selection
    #[must_use]
    pub fn selection_loc_bound_disp(&self) -> (Loc, Loc) {
        let mut left = self.cursor.loc;
        let mut right = self.cursor.selection_end;
        // Convert into character indices
        if left > right {
            std::mem::swap(&mut left, &mut right);
        }
        (left, right)
    }

    /// Will return the bounds of the current active selection
    #[must_use]
    pub fn selection_loc_bound(&self) -> (Loc, Loc) {
        let (mut left, mut right) = self.selection_loc_bound_disp();
        // Convert into character indices
        left.x = self.character_idx(&left);
        right.x = self.character_idx(&right);
        (left, right)
    }

    /// Returns true if the provided location is within the current active selection
    #[must_use]
    pub fn is_loc_selected(&self, loc: Loc) -> bool {
        self.is_this_loc_selected(loc, self.selection_loc_bound())
    }

    /// Returns true if the provided location is within the provided selection argument
    #[must_use]
    pub fn is_this_loc_selected(&self, loc: Loc, selection_bound: (Loc, Loc)) -> bool {
        let (left, right) = selection_bound;
        left <= loc && loc < right
    }

    /// Returns true if the provided location is within the provided selection argument
    #[must_use]
    pub fn is_this_loc_selected_disp(&self, loc: Loc, selection_bound: (Loc, Loc)) -> bool {
        let (left, right) = selection_bound;
        left <= loc && loc < right
    }

    /// Will return the current active selection as a range over file characters
    #[must_use]
    pub fn selection_range(&self) -> Range<usize> {
        let mut cursor = self.cursor.loc;
        let mut selection_end = self.cursor.selection_end;
        cursor.x = self.character_idx(&cursor);
        selection_end.x = self.character_idx(&selection_end);
        let mut left = self.loc_to_file_pos(&cursor);
        let mut right = self.loc_to_file_pos(&selection_end);
        if left > right {
            std::mem::swap(&mut left, &mut right);
        }
        left..right
    }

    /// Will return the text contained within the current selection
    #[must_use]
    pub fn selection_text(&self) -> String {
        self.file.slice(self.selection_range()).to_string()
    }

    /// Delete the currently selected text
    pub fn remove_selection(&mut self) {
        self.file.remove(self.selection_range());
        self.reload_lines();
        let mut goto = self.selection_loc_bound().0;
        goto.x = self.display_idx(&goto);
        self.cursor.loc = goto;
        self.char_ptr = self.character_idx(&self.cursor.loc);
        self.cancel_selection();
        self.bring_cursor_in_viewport();
    }

    /// Cancels the current selection
    pub fn cancel_selection(&mut self) {
        self.cursor.selection_end = self.cursor.loc;
    }

    /// Create a new alternative cursor
    pub fn new_cursor(&mut self, loc: Loc) {
        if let Some(idx) = self.has_cursor(loc) {
            self.secondary_cursors.remove(idx);
        } else if self.out_of_range(loc.x, loc.y).is_ok() {
            self.secondary_cursors.push(loc);
        }
    }

    /// Clear all secondary cursors
    pub fn clear_cursors(&mut self) {
        self.secondary_cursors.clear();
    }

    /// Determine if there is a secondary cursor at a certain position
    #[must_use]
    pub fn has_cursor(&self, loc: Loc) -> Option<usize> {
        self.secondary_cursors.iter().position(|c| *c == loc)
    }
}
