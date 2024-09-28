use crate::error::Result;
use crate::ui::size;
use crossterm::event::{read, Event as CEvent, KeyCode as KCode, KeyModifiers as KMod};
use kaolinite::utils::{Loc, Size};
use std::io::Write;

use super::Editor;

impl Editor {
    /// Use search feature
    pub fn search(&mut self) -> Result<()> {
        // Prompt for a search term
        let target = self.prompt("Search")?;
        let mut done = false;
        let Size { w, h } = size()?;
        // Jump to the next match after search term is provided
        self.next_match(&target);
        // Enter into search menu
        while !done {
            // Render just the document part
            self.terminal.hide_cursor()?;
            self.render_document(w, h.saturating_sub(2))?;
            // Render custom status line with mode information
            self.terminal.goto(0, h)?;
            write!(
                self.terminal.stdout,
                "[<-]: Search previous | [->]: Search next"
            )?;
            self.terminal.flush()?;
            // Move back to correct cursor position
            if let Some(Loc { x, y }) = self.doc().cursor_loc_in_screen() {
                let max = self.dent();
                self.terminal.goto(x + max, y + 1)?;
                self.terminal.show_cursor()?;
            } else {
                self.terminal.hide_cursor()?;
            }
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // On return or escape key, exit menu
                    (KMod::NONE, KCode::Enter | KCode::Esc) => done = true,
                    // On left key, move to the previous match in the document
                    (KMod::NONE, KCode::Left) => std::mem::drop(self.prev_match(&target)),
                    // On right key, move to the next match in the document
                    (KMod::NONE, KCode::Right) => std::mem::drop(self.next_match(&target)),
                    _ => (),
                }
            }
            self.update_highlighter()?;
        }
        Ok(())
    }

    /// Move to the next match
    pub fn next_match(&mut self, target: &str) -> Option<String> {
        let mtch = self.doc_mut().next_match(target, 1)?;
        self.doc_mut().move_to(&mtch.loc);
        // Update highlighting
        self.update_highlighter().ok()?;
        Some(mtch.text)
    }

    /// Move to the previous match
    pub fn prev_match(&mut self, target: &str) -> Option<String> {
        let mtch = self.doc_mut().prev_match(target)?;
        self.doc_mut().move_to(&mtch.loc);
        // Update highlighting
        self.update_highlighter().ok()?;
        Some(mtch.text)
    }

    /// Use replace feature
    pub fn replace(&mut self) -> Result<()> {
        // Request replace information
        let target = self.prompt("Replace")?;
        let into = self.prompt("With")?;
        let mut done = false;
        let Size { w, h } = size()?;
        // Jump to match
        let mut mtch;
        if let Some(m) = self.next_match(&target) {
            // Automatically move to next match, keeping note of what that match is
            mtch = m;
        } else if let Some(m) = self.prev_match(&target) {
            // Automatically move to previous match, keeping not of what that match is
            // This happens if there are no matches further down the document, only above
            mtch = m;
        } else {
            // Exit if there are no matches in the document
            return Ok(());
        }
        self.update_highlighter()?;
        // Enter into the replace menu
        while !done {
            // Render just the document part
            self.terminal.hide_cursor()?;
            self.render_document(w, h.saturating_sub(2))?;
            // Write custom status line for the replace mode
            self.terminal.goto(0, h)?;
            write!(
                self.terminal.stdout,
                "[<-] Previous | [->] Next | [Enter] Replace | [Tab] Replace All"
            )?;
            self.terminal.flush()?;
            // Move back to correct cursor location
            if let Some(Loc { x, y }) = self.doc().cursor_loc_in_screen() {
                let max = self.dent();
                self.terminal.goto(x + max, y + 1)?;
                self.terminal.show_cursor()?;
            } else {
                self.terminal.hide_cursor()?;
            }
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // On escape key, exit
                    (KMod::NONE, KCode::Esc) => done = true,
                    // On right key, move to the previous match, keeping note of what that match is
                    (KMod::NONE, KCode::Left) => mtch = self.prev_match(&target).unwrap_or(mtch),
                    // On left key, move to the next match, keeping note of what that match is
                    (KMod::NONE, KCode::Right) => mtch = self.next_match(&target).unwrap_or(mtch),
                    // On return key, perform replacement
                    (KMod::NONE, KCode::Enter) => self.do_replace(&into, &mtch)?,
                    // On tab key, replace all instances within the document
                    (KMod::NONE, KCode::Tab) => self.do_replace_all(&target, &into),
                    _ => (),
                }
            }
            // Update syntax highlighter if necessary
            self.update_highlighter()?;
        }
        Ok(())
    }

    /// Replace an instance in a document
    fn do_replace(&mut self, into: &str, text: &str) -> Result<()> {
        // Commit events to event manager (for undo / redo)
        self.doc_mut().commit();
        // Do the replacement
        let loc = self.doc().char_loc();
        self.doc_mut().replace(loc, text, into)?;
        self.doc_mut().move_to(&loc);
        // Update syntax highlighter
        self.update_highlighter()?;
        self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
        Ok(())
    }

    /// Replace all instances in a document
    fn do_replace_all(&mut self, target: &str, into: &str) {
        // Commit events to event manager (for undo / redo)
        self.doc_mut().commit();
        // Replace everything top to bottom
        self.doc_mut().move_to(&Loc::at(0, 0));
        while let Some(mtch) = self.doc_mut().next_match(target, 1) {
            drop(self.doc_mut().replace(mtch.loc, &mtch.text, into));
            drop(self.update_highlighter());
            self.highlighter[self.ptr].edit(mtch.loc.y, &self.doc[self.ptr].lines[mtch.loc.y]);
        }
    }
}
