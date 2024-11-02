/// Functions for moving the cursor around
use crate::config;
use kaolinite::event::Status;

use super::Editor;

impl Editor {
    /// Move the cursor up
    pub fn select_up(&mut self) {
        self.doc_mut().select_up();
    }

    /// Move the cursor down
    pub fn select_down(&mut self) {
        self.doc_mut().select_down();
    }

    /// Move the cursor left
    pub fn select_left(&mut self) {
        let status = self.doc_mut().select_left();
        // Cursor wrapping if cursor hits the start of the line
        let wrapping = config!(self.config, document).wrap_cursor;
        if status == Status::StartOfLine && self.doc().loc().y != 0 && wrapping {
            self.doc_mut().select_up();
            self.doc_mut().select_end();
        }
    }

    /// Move the cursor right
    pub fn select_right(&mut self) {
        let status = self.doc_mut().select_right();
        // Cursor wrapping if cursor hits the end of a line
        let wrapping = config!(self.config, document).wrap_cursor;
        if status == Status::EndOfLine && wrapping {
            self.doc_mut().select_down();
            self.doc_mut().select_home();
        }
    }

    /// Select the whole document
    pub fn select_all(&mut self) {
        self.doc_mut().move_top();
        self.doc_mut().select_bottom();
    }

    /// Move the cursor up
    pub fn up(&mut self) {
        self.doc_mut().move_up();
    }

    /// Move the cursor down
    pub fn down(&mut self) {
        self.doc_mut().move_down();
    }

    /// Move the cursor left
    pub fn left(&mut self) {
        let status = self.doc_mut().move_left();
        // Cursor wrapping if cursor hits the start of the line
        let wrapping = config!(self.config, document).wrap_cursor;
        if status == Status::StartOfLine && self.doc().loc().y != 0 && wrapping {
            self.doc_mut().move_up();
            self.doc_mut().move_end();
        }
    }

    /// Move the cursor right
    pub fn right(&mut self) {
        let status = self.doc_mut().move_right();
        // Cursor wrapping if cursor hits the end of a line
        let wrapping = config!(self.config, document).wrap_cursor;
        if status == Status::EndOfLine && wrapping {
            self.doc_mut().move_down();
            self.doc_mut().move_home();
        }
    }

    /// Move the cursor to the previous word in the line
    pub fn prev_word(&mut self) {
        let status = self.doc_mut().move_prev_word();
        let wrapping = config!(self.config, document).wrap_cursor;
        if status == Status::StartOfLine && wrapping {
            self.doc_mut().move_up();
            self.doc_mut().move_end();
        }
    }

    /// Move the cursor to the next word in the line
    pub fn next_word(&mut self) {
        let status = self.doc_mut().move_next_word();
        let wrapping = config!(self.config, document).wrap_cursor;
        if status == Status::EndOfLine && wrapping {
            self.doc_mut().move_down();
            self.doc_mut().move_home();
        }
    }
}
