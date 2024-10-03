use crate::error::Result;
use kaolinite::event::Event;
use kaolinite::utils::Loc;

use super::Editor;

impl Editor {
    /// Execute an edit event
    pub fn exe(&mut self, ev: Event) -> Result<()> {
        if !self.doc().undo_mgmt.last_event.same_type(&ev) && !self.plugin_active {
            self.doc_mut().commit();
        }
        self.doc_mut().exe(ev)?;
        Ok(())
    }

    /// Insert a character into the document, creating a new row if editing
    /// on the last line of the document
    pub fn character(&mut self, ch: char) -> Result<()> {
        if !self.doc().is_selection_empty() {
            self.doc_mut().remove_selection();
            self.reload_highlight();
        }
        self.new_row()?;
        // Handle the character insertion
        if ch == '\n' {
            self.enter()?;
        } else {
            let loc = self.doc().char_loc();
            self.exe(Event::Insert(loc, ch.to_string()))?;
            self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
        }
        Ok(())
    }

    /// Handle the return key
    pub fn enter(&mut self) -> Result<()> {
        // Perform the changes
        if self.doc().loc().y == self.doc().len_lines() {
            // Enter pressed on the empty line at the bottom of the document
            self.new_row()?;
        } else {
            // Enter pressed in the start, middle or end of the line
            let loc = self.doc().char_loc();
            self.exe(Event::SplitDown(loc))?;
            let line = &self.doc[self.ptr].lines[loc.y + 1];
            self.highlighter[self.ptr].insert_line(loc.y + 1, line);
            let line = &self.doc[self.ptr].lines[loc.y];
            self.highlighter[self.ptr].edit(loc.y, line);
        }
        Ok(())
    }

    /// Handle the backspace key
    pub fn backspace(&mut self) -> Result<()> {
        if !self.doc().is_selection_empty() {
            // Removing a selection is significant and worth an undo commit
            self.doc_mut().commit();
            self.doc_mut().undo_mgmt.set_dirty();
            self.doc_mut().remove_selection();
            self.reload_highlight();
            return Ok(());
        }
        let mut c = self.doc().char_ptr;
        let on_first_line = self.doc().loc().y == 0;
        let out_of_range = self.doc().out_of_range(0, self.doc().loc().y).is_err();
        if c == 0 && !on_first_line && !out_of_range {
            // Backspace was pressed on the start of the line, move line to the top
            self.new_row()?;
            let mut loc = self.doc().char_loc();
            self.highlighter().remove_line(loc.y);
            loc.y = loc.y.saturating_sub(1);
            loc.x = self.doc().line(loc.y).unwrap().chars().count();
            self.exe(Event::SpliceUp(loc))?;
            let line = &self.doc[self.ptr].lines[loc.y];
            self.highlighter[self.ptr].edit(loc.y, line);
        } else if !(c == 0 && on_first_line) {
            // Backspace was pressed in the middle of the line, delete the character
            c = c.saturating_sub(1);
            if let Some(line) = self.doc().line(self.doc().loc().y) {
                if let Some(ch) = line.chars().nth(c) {
                    let loc = Loc {
                        x: c,
                        y: self.doc().loc().y,
                    };
                    self.exe(Event::Delete(loc, ch.to_string()))?;
                    self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
                }
            }
        }
        Ok(())
    }

    /// Delete the character in place
    pub fn delete(&mut self) -> Result<()> {
        let c = self.doc().char_ptr;
        if let Some(line) = self.doc().line(self.doc().loc().y) {
            if let Some(ch) = line.chars().nth(c) {
                let loc = Loc {
                    x: c,
                    y: self.doc().loc().y,
                };
                self.exe(Event::Delete(loc, ch.to_string()))?;
                self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
            }
        }
        Ok(())
    }

    /// Insert a new row at the end of the document if the cursor is on it
    fn new_row(&mut self) -> Result<()> {
        if self.doc().loc().y == self.doc().len_lines() {
            self.exe(Event::InsertLine(self.doc().loc().y, String::new()))?;
            self.highlighter().append(&String::new());
        }
        Ok(())
    }

    /// Delete the current line
    pub fn delete_line(&mut self) -> Result<()> {
        // Delete the line
        if self.doc().loc().y < self.doc().len_lines() {
            let y = self.doc().loc().y;
            let line = self.doc().line(y).unwrap();
            self.exe(Event::DeleteLine(y, line))?;
            self.highlighter().remove_line(y);
        }
        Ok(())
    }

    /// Perform redo action
    pub fn redo(&mut self) -> Result<()> {
        let result = Ok(self.doc_mut().redo()?);
        self.reload_highlight();
        result
    }

    /// Perform undo action
    pub fn undo(&mut self) -> Result<()> {
        let result = Ok(self.doc_mut().undo()?);
        self.reload_highlight();
        result
    }

    /// Copy the selected text
    pub fn copy(&mut self) -> Result<()> {
        let selected_text = self.doc().selection_text();
        self.terminal.copy(&selected_text)
    }

    /// Cut the selected text
    pub fn cut(&mut self) -> Result<()> {
        self.copy()?;
        self.doc_mut().remove_selection();
        self.reload_highlight();
        Ok(())
    }
}
