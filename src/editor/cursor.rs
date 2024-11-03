/// Functions for moving the cursor around
use crate::{config, ged, handle_event, CEvent, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kaolinite::event::Status;
use mlua::{AnyUserData, Lua};

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

/// Handle multiple cursors (replay a key event for each of them)
pub fn handle_multiple_cursors(editor: &AnyUserData, event: &CEvent, lua: &Lua) -> Result<()> {
    // Cache the state of the document
    let cursor = ged!(&editor).doc().cursor;
    let char_ptr = ged!(&editor).doc().char_ptr;
    let old_cursor = ged!(&editor).doc().old_cursor;
    // For each secondary cursor, replay the key event
    ged!(mut &editor).macro_man.playing = true;
    let secondary_cursors = ged!(&editor).doc().secondary_cursors.clone();
    for (id, cursor) in secondary_cursors.iter().enumerate() {
        ged!(mut &editor).doc_mut().move_to(cursor);
        handle_event(editor, event, lua)?;
        let char_loc = ged!(&editor).doc().char_loc();
        *ged!(mut &editor)
            .doc_mut()
            .secondary_cursors
            .get_mut(id)
            .unwrap() = char_loc;
    }
    ged!(mut &editor).macro_man.playing = false;
    // Restore back to the state of the document beforehand
    ged!(mut &editor).doc_mut().cursor = cursor;
    ged!(mut &editor).doc_mut().char_ptr = char_ptr;
    ged!(mut &editor).doc_mut().old_cursor = old_cursor;
    Ok(())
}

// Determine whether an event should be acted on by the multi cursor
#[allow(clippy::module_name_repetitions)]
pub fn allowed_by_multi_cursor(event: &CEvent) -> bool {
    matches!(
        event,
        CEvent::Key(KeyEvent {
            code: KeyCode::Char(_) | KeyCode::Tab | KeyCode::Backspace | KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            ..
        })
    )
}
