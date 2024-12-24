/// General functions for editing a document
use crate::error::Result;
use kaolinite::event::Event;
use kaolinite::utils::Loc;

use super::Editor;

impl Editor {
    /// Execute an edit event
    pub fn exe(&mut self, ev: Event) -> Result<()> {
        if self.try_doc().is_some() {
            let multi_cursors = !self.try_doc().unwrap().secondary_cursors.is_empty();
            if !(self.plugin_active || self.pasting || self.macro_man.playing || multi_cursors) {
                let last_ev = self.try_doc().unwrap().event_mgmt.last_event.as_ref();
                // If last event is present and the same as this one, commit
                let event_type_differs = last_ev.map(|e1| e1.same_type(&ev)) != Some(true);
                // If last event is present and on a different line from the previous, commit
                let event_on_different_line =
                    last_ev.map(|e| e.loc().y == ev.loc().y) != Some(true);
                // Commit if necessary
                if event_type_differs || event_on_different_line {
                    self.try_doc_mut().unwrap().commit();
                }
            } else if self.try_doc().unwrap().event_mgmt.history.is_empty() {
                // If there is no initial commit and a plug-in changes things without commiting
                // It can cause the initial state of the document to be lost
                // This condition makes sure there is a copy to go back to if this is the case
                self.try_doc_mut().unwrap().commit();
            }
            self.try_doc_mut().unwrap().exe(ev)?;
        }
        Ok(())
    }

    /// Insert a character into the document, creating a new row if editing
    /// on the last line of the document
    pub fn character(&mut self, ch: char) -> Result<()> {
        if self.try_doc().is_some() {
            let doc = self.try_doc().unwrap();
            let selection_overwrite = !doc.is_selection_empty() && !doc.info.read_only;
            if selection_overwrite {
                self.try_doc_mut().unwrap().commit();
                self.try_doc_mut().unwrap().remove_selection();
            }
            self.new_row()?;
            // Handle the character insertion
            if ch == '\n' {
                self.enter()?;
            } else {
                let doc = self.try_doc().unwrap();
                let loc = doc.char_loc();
                self.exe(Event::Insert(loc, ch.to_string()))?;
                if let Some(file) = self.files.get_mut(self.ptr.clone()) {
                    if !file.doc.info.read_only {
                        file.highlighter.edit(loc.y, &file.doc.lines[loc.y]);
                    }
                }
            }
            if selection_overwrite {
                self.reload_highlight();
            }
        }
        Ok(())
    }

    /// Handle the return key
    pub fn enter(&mut self) -> Result<()> {
        if let Some(doc) = self.try_doc_mut() {
            // Perform the changes
            if doc.loc().y == doc.len_lines() {
                // Enter pressed on the empty line at the bottom of the document
                self.new_row()?;
            } else {
                // Enter pressed in the start, middle or end of the line
                let loc = doc.char_loc();
                self.exe(Event::SplitDown(loc))?;
                if let Some(file) = self.files.get_mut(self.ptr.clone()) {
                    if !file.doc.info.read_only {
                        let line = &file.doc.lines[loc.y + 1];
                        file.highlighter.insert_line(loc.y + 1, line);
                        let line = &file.doc.lines[loc.y];
                        file.highlighter.edit(loc.y, line);
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle the backspace key
    pub fn backspace(&mut self) -> Result<()> {
        if self.try_doc().is_some() {
            let doc = self.try_doc().unwrap();
            if !doc.is_selection_empty() && !doc.info.read_only {
                // Removing a selection is significant and worth an undo commit
                let doc = self.try_doc_mut().unwrap();
                doc.commit();
                doc.remove_selection();
                self.reload_highlight();
                return Ok(());
            }
            let doc = self.try_doc().unwrap();
            let mut c = doc.char_ptr;
            let on_first_line = doc.loc().y == 0;
            let out_of_range = doc.out_of_range(0, doc.loc().y).is_err();
            if c == 0 && !on_first_line && !out_of_range {
                // Backspace was pressed on the start of the line, move line to the top
                self.new_row()?;
                let mut loc = self.try_doc().unwrap().char_loc();
                let file = self.files.get_mut(self.ptr.clone()).unwrap();
                if !file.doc.info.read_only {
                    self.highlighter().remove_line(loc.y);
                }
                loc.y = loc.y.saturating_sub(1);
                let file = self.files.get_mut(self.ptr.clone()).unwrap();
                loc.x = file.doc.line(loc.y).unwrap().chars().count();
                self.exe(Event::SpliceUp(loc))?;
                let file = self.files.get_mut(self.ptr.clone()).unwrap();
                let line = &file.doc.lines[loc.y];
                if !file.doc.info.read_only {
                    file.highlighter.edit(loc.y, line);
                }
            } else if !(c == 0 && on_first_line) {
                // Backspace was pressed in the middle of the line, delete the character
                c = c.saturating_sub(1);
                if let Some(line) = doc.line(doc.loc().y) {
                    if let Some(ch) = line.chars().nth(c) {
                        let loc = Loc {
                            x: c,
                            y: doc.loc().y,
                        };
                        self.exe(Event::Delete(loc, ch.to_string()))?;
                        let file = self.files.get_mut(self.ptr.clone()).unwrap();
                        if !file.doc.info.read_only {
                            file.highlighter.edit(loc.y, &file.doc.lines[loc.y]);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Delete the character in place
    pub fn delete(&mut self) -> Result<()> {
        if let Some(doc) = self.try_doc() {
            let c = doc.char_ptr;
            if let Some(line) = doc.line(doc.loc().y) {
                if let Some(ch) = line.chars().nth(c) {
                    let loc = Loc {
                        x: c,
                        y: doc.loc().y,
                    };
                    self.exe(Event::Delete(loc, ch.to_string()))?;
                    if let Some(file) = self.files.get_mut(self.ptr.clone()) {
                        if !file.doc.info.read_only {
                            file.highlighter.edit(loc.y, &file.doc.lines[loc.y]);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Insert a new row at the end of the document if the cursor is on it
    fn new_row(&mut self) -> Result<()> {
        if self.try_doc().is_some() {
            let doc = self.try_doc().unwrap();
            if doc.loc().y == doc.len_lines() {
                self.exe(Event::InsertLine(doc.loc().y, String::new()))?;
                let doc = self.try_doc().unwrap();
                if !doc.info.read_only {
                    self.highlighter().append("");
                }
            }
        }
        Ok(())
    }

    /// Delete the current line
    pub fn delete_line(&mut self) -> Result<()> {
        if self.try_doc().is_some() {
            // Delete the line
            let doc = self.try_doc().unwrap();
            if doc.loc().y < doc.len_lines() {
                let y = doc.loc().y;
                let line = doc.line(y).unwrap();
                self.exe(Event::DeleteLine(y, line))?;
                let doc = self.try_doc().unwrap();
                if !doc.info.read_only {
                    self.highlighter().remove_line(y);
                }
            }
        }
        Ok(())
    }

    /// Perform redo action
    pub fn redo(&mut self) -> Result<()> {
        if let Some(doc) = self.try_doc_mut() {
            doc.redo()?;
            self.reload_highlight();
        }
        Ok(())
    }

    /// Perform undo action
    pub fn undo(&mut self) -> Result<()> {
        if let Some(doc) = self.try_doc_mut() {
            doc.undo()?;
            self.reload_highlight();
        }
        Ok(())
    }

    /// Copy the selected text
    pub fn copy(&mut self) -> Result<()> {
        if let Some(doc) = self.try_doc() {
            let selected_text = doc.selection_text();
            self.terminal.copy(&selected_text)
        } else {
            Ok(())
        }
    }

    /// Cut the selected text
    pub fn cut(&mut self) -> Result<()> {
        if self.try_doc().is_some() {
            self.copy()?;
            self.try_doc_mut().unwrap().remove_selection();
            self.reload_highlight();
        }
        Ok(())
    }

    /// Shortcut to help rehighlight a line
    pub fn hl_edit(&mut self, y: usize) {
        if let Some(doc) = self.try_doc() {
            let line = doc.line(y).unwrap_or_default();
            self.highlighter().edit(y, &line);
        }
    }
}
