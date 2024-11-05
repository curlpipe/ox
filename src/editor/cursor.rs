/// Functions for moving the cursor around
use crate::{config, ged, handle_event, CEvent, Loc, Result};
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
pub fn handle_multiple_cursors(
    editor: &AnyUserData,
    event: &CEvent,
    lua: &Lua,
    original_loc: &Loc,
) -> Result<()> {
    let mut original_loc = *original_loc;
    // Cache the state of the document
    let mut cursor = ged!(&editor).doc().cursor;
    // For each secondary cursor, replay the key event
    ged!(mut &editor).macro_man.playing = true;
    let mut secondary_cursors = ged!(&editor).doc().secondary_cursors.clone();
    // Prevent interference
    adjust_other_cursors(
        &mut secondary_cursors,
        &original_loc.clone(),
        event,
        &mut original_loc,
    );
    // Update each secondary cursor
    let mut ptr = 0;
    while ptr < secondary_cursors.len() {
        // Move to the secondary cursor position
        let sec_cursor = secondary_cursors[ptr];
        ged!(mut &editor).doc_mut().move_to(&sec_cursor);
        // Replay the event
        let char_loc = ged!(&editor).doc().char_loc();
        handle_event(editor, event, lua)?;
        // Prevent any interference
        cursor.loc =
            adjust_other_cursors(&mut secondary_cursors, &char_loc, event, &mut cursor.loc);
        // Update the secondary cursor
        let char_loc = ged!(&editor).doc().char_loc();
        *secondary_cursors.get_mut(ptr).unwrap() = char_loc;
        // Move to the next secondary cursor
        ptr += 1;
    }
    ged!(mut &editor).doc_mut().secondary_cursors = secondary_cursors;
    ged!(mut &editor).macro_man.playing = false;
    // Restore back to the state of the document beforehand
    // TODO: calculate char_ptr and old_cursor too
    ged!(mut &editor).doc_mut().cursor = cursor;
    let char_ptr = ged!(&editor).doc().character_idx(&cursor.loc);
    ged!(mut &editor).doc_mut().char_ptr = char_ptr;
    ged!(mut &editor).doc_mut().old_cursor = cursor.loc.x;
    ged!(mut &editor).doc_mut().cancel_selection();
    Ok(())
}

/// Adjust other secondary cursors based of a change in one
fn adjust_other_cursors(
    cursors: &mut Vec<Loc>,
    moved: &Loc,
    event: &CEvent,
    primary: &mut Loc,
) -> Loc {
    cursors.push(*primary);
    match event {
        CEvent::Key(KeyEvent {
            code: KeyCode::Enter,
            ..
        }) => {
            // Enter key, push all cursors below this line downwards
            for c in cursors.iter_mut() {
                if c == moved {
                    continue;
                }
                let mut new_loc = *c;
                // Adjust x position
                if moved.y == c.y && moved.x < c.x {
                    new_loc.x -= moved.x;
                }
                // If this cursor is after the currently moved cursor, shift down
                if c.y > moved.y || (c.y == moved.y && c.x > moved.x) {
                    new_loc.y += 1;
                }
                // Update the secondary cursor
                *c = new_loc;
            }
        }
        // TODO: Handle backspace
        _ => (),
    }
    cursors.pop().unwrap()
}

// Determine whether an event should be acted on by the multi cursor
#[allow(clippy::module_name_repetitions)]
pub fn allowed_by_multi_cursor(event: &CEvent) -> bool {
    matches!(
        event,
        CEvent::Key(
            KeyEvent {
                code: KeyCode::Char(_)
                    | KeyCode::Tab
                    | KeyCode::Backspace
                    | KeyCode::Enter
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right,
                modifiers: KeyModifiers::CONTROL,
                ..
            }
        )
    )
}
