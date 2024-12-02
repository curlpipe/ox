/// Functions for moving the cursor around
use crate::{config, ged, handle_event, CEvent, Loc, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kaolinite::event::Status;
use mlua::{AnyUserData, Lua};

use super::Editor;

impl Editor {
    /// Move the cursor up
    pub fn select_up(&mut self) {
        if let Some(doc) = self.try_doc_mut() {
            doc.select_up();
        }
    }

    /// Move the cursor down
    pub fn select_down(&mut self) {
        if let Some(doc) = self.try_doc_mut() {
            doc.select_down();
        }
    }

    /// Move the cursor left
    pub fn select_left(&mut self) {
        let wrapping = config!(self.config, document).wrap_cursor;
        if let Some(doc) = self.try_doc_mut() {
            let status = doc.select_left();
            // Cursor wrapping if cursor hits the start of the line
            if status == Status::StartOfLine && doc.loc().y != 0 && wrapping {
                doc.select_up();
                doc.select_end();
            }
        }
    }

    /// Move the cursor right
    pub fn select_right(&mut self) {
        let wrapping = config!(self.config, document).wrap_cursor;
        if let Some(doc) = self.try_doc_mut() {
            let status = doc.select_right();
            // Cursor wrapping if cursor hits the end of a line
            if status == Status::EndOfLine && wrapping {
                doc.select_down();
                doc.select_home();
            }
        }
    }

    /// Select the whole document
    pub fn select_all(&mut self) {
        if let Some(doc) = self.try_doc_mut() {
            doc.move_top();
            doc.select_bottom();
        }
    }

    /// Move the cursor up
    pub fn up(&mut self) {
        if let Some(doc) = self.try_doc_mut() {
            doc.move_up();
        }
    }

    /// Move the cursor down
    pub fn down(&mut self) {
        if let Some(doc) = self.try_doc_mut() {
            doc.move_down();
        }
    }

    /// Move the cursor left
    pub fn left(&mut self) {
        let wrapping = config!(self.config, document).wrap_cursor;
        if let Some(doc) = self.try_doc_mut() {
            let status = doc.move_left();
            // Cursor wrapping if cursor hits the start of the line
            if status == Status::StartOfLine && doc.loc().y != 0 && wrapping {
                doc.move_up();
                doc.move_end();
            }
        }
    }

    /// Move the cursor right
    pub fn right(&mut self) {
        let wrapping = config!(self.config, document).wrap_cursor;
        if let Some(doc) = self.try_doc_mut() {
            let status = doc.move_right();
            // Cursor wrapping if cursor hits the end of a line
            if status == Status::EndOfLine && wrapping {
                doc.move_down();
                doc.move_home();
            }
        }
    }

    /// Move the cursor to the previous word in the line
    pub fn prev_word(&mut self) {
        let wrapping = config!(self.config, document).wrap_cursor;
        if let Some(doc) = self.try_doc_mut() {
            let status = doc.move_prev_word();
            if status == Status::StartOfLine && wrapping {
                doc.move_up();
                doc.move_end();
            }
        }
    }

    /// Move the cursor to the next word in the line
    pub fn next_word(&mut self) {
        let wrapping = config!(self.config, document).wrap_cursor;
        if let Some(doc) = self.try_doc_mut() {
            let status = doc.move_next_word();
            if status == Status::EndOfLine && wrapping {
                doc.move_down();
                doc.move_home();
            }
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
    if ged!(&editor).try_doc().is_none() {
        return Ok(());
    }
    let mut original_loc = *original_loc;
    // Cache the state of the document
    let mut cursor = ged!(&editor).try_doc().unwrap().cursor;
    let mut secondary_cursors = ged!(&editor).try_doc().unwrap().secondary_cursors.clone();
    ged!(mut &editor).macro_man.playing = true;
    // Prevent interference
    adjust_other_cursors(
        &mut secondary_cursors,
        &original_loc.clone(),
        &cursor.loc,
        event,
        &mut original_loc,
    );
    // Update each secondary cursor
    let mut ptr = 0;
    while ptr < secondary_cursors.len() {
        // Move to the secondary cursor position
        let sec_cursor = secondary_cursors[ptr];
        ged!(mut &editor)
            .try_doc_mut()
            .unwrap()
            .move_to(&sec_cursor);
        // Replay the event
        let old_loc = ged!(&editor).try_doc().unwrap().char_loc();
        handle_event(editor, event, lua)?;
        // Prevent any interference
        let char_loc = ged!(&editor).try_doc().unwrap().char_loc();
        cursor.loc = adjust_other_cursors(
            &mut secondary_cursors,
            &old_loc,
            &char_loc,
            event,
            &mut cursor.loc,
        );
        // Update the secondary cursor
        *secondary_cursors.get_mut(ptr).unwrap() = char_loc;
        // Move to the next secondary cursor
        ptr += 1;
    }
    ged!(mut &editor).try_doc_mut().unwrap().secondary_cursors = secondary_cursors;
    ged!(mut &editor).macro_man.playing = false;
    // Restore back to the state of the document beforehand
    // TODO: calculate char_ptr and old_cursor too
    ged!(mut &editor).try_doc_mut().unwrap().cursor = cursor;
    let char_ptr = ged!(&editor).try_doc().unwrap().character_idx(&cursor.loc);
    ged!(mut &editor).try_doc_mut().unwrap().char_ptr = char_ptr;
    ged!(mut &editor).try_doc_mut().unwrap().old_cursor = cursor.loc.x;
    ged!(mut &editor).try_doc_mut().unwrap().cancel_selection();
    Ok(())
}

/// Adjust other secondary cursors based of a change in one
fn adjust_other_cursors(
    cursors: &mut Vec<Loc>,
    old_pos: &Loc,
    new_pos: &Loc,
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
                if c == old_pos {
                    continue;
                }
                let mut new_loc = *c;
                // Adjust x position
                if old_pos.y == c.y && old_pos.x < c.x {
                    new_loc.x -= old_pos.x;
                }
                // If this cursor is after the currently moved cursor, shift down
                if c.y > old_pos.y || (c.y == old_pos.y && c.x > old_pos.x) {
                    new_loc.y += 1;
                }
                // Update the secondary cursor
                *c = new_loc;
            }
        }
        CEvent::Key(KeyEvent {
            code: KeyCode::Backspace,
            ..
        }) => {
            // Backspace key, push all cursors below this line upwards
            for c in cursors.iter_mut() {
                if c == old_pos {
                    continue;
                }
                let mut new_loc = *c;
                let at_line_start = old_pos.x == 0 && old_pos.y != 0;
                // Adjust x position
                if old_pos.y == c.y && old_pos.x < c.x && at_line_start {
                    new_loc.x += new_pos.x;
                }
                // If this cursor is after the currently moved cursor, shift up
                if (c.y > old_pos.y || (c.y == old_pos.y && c.x > old_pos.x)) && at_line_start {
                    new_loc.y -= 1;
                }
                // Update the secondary cursor
                *c = new_loc;
            }
        }
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
                code: KeyCode::Tab
                    | KeyCode::Backspace
                    | KeyCode::Enter
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char(_),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } | KeyEvent {
                code: KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right,
                modifiers: KeyModifiers::CONTROL,
                ..
            }
        )
    )
}
