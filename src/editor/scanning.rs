/// Functions for searching and replacing
use crate::error::{OxError, Result};
use crate::events::wait_for_event_hog;
use crate::ui::{key_event, size};
use crate::{config, display};
use crossterm::{
    event::{KeyCode as KCode, KeyModifiers as KMod},
    style::{Attribute, Print, SetAttribute, SetBackgroundColor as Bg},
};
use kaolinite::utils::{Loc, Size};
use mlua::Lua;

use super::Editor;

impl Editor {
    /// Use search feature
    pub fn search(&mut self, lua: &Lua) -> Result<()> {
        // Block any non-documents from activating search
        if self.try_doc().is_none() {
            return Ok(());
        }
        // Gather data
        let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
        let cache = self.try_doc().unwrap().char_loc();
        // Prompt for a search term
        let mut target = String::new();
        let mut done = false;
        while !done {
            let Size { w, h } = size()?;
            // Rerender the editor
            self.needs_rerender = true;
            self.render(lua)?;
            // Render prompt message
            self.terminal.prepare_line(h);
            display!(
                self,
                editor_bg,
                "Search: ",
                target.clone(),
                "│",
                " ".to_string().repeat(w)
            );
            // Move back to correct cursor position
            if let Some(Loc { x, y }) = self.cursor_position() {
                self.terminal.goto(x, y);
                self.terminal.show_cursor();
            } else {
                self.terminal.hide_cursor();
            }
            self.terminal.flush()?;
            if let Some((modifiers, code)) =
                key_event(&wait_for_event_hog(self), &mut self.macro_man)
            {
                match (modifiers, code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Cancel operation
                    (KMod::NONE, KCode::Esc) => {
                        self.try_doc_mut().unwrap().move_to(&cache);
                        self.try_doc_mut().unwrap().cancel_selection();
                        return Err(OxError::Cancelled);
                    }
                    // Remove from the input string if the user presses backspace
                    (KMod::NONE, KCode::Backspace) => {
                        target.pop();
                        self.try_doc_mut().unwrap().move_to(&cache);
                        self.next_match(&target);
                    }
                    // Add to the input string if the user presses a character
                    (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => {
                        target.push(c);
                        self.try_doc_mut().unwrap().move_to(&cache);
                        self.next_match(&target);
                    }
                    _ => (),
                }
            }
        }

        // If no target is given, do nothing
        if target.is_empty() {
            return Ok(());
        }

        // Main body of the search feature
        let mut done = false;
        let Size { w, h } = size()?;
        // Enter into search menu
        while !done {
            // Rerender the editor
            self.needs_rerender = true;
            self.render(lua)?;
            // Render custom status line with mode information
            self.terminal.prepare_line(h);
            display!(
                self,
                editor_bg,
                Print("[<-]: Search previous | [->]: Search next | [Enter] Finish | [Esc] Cancel"),
                Print(" ".repeat(w.saturating_sub(73)))
            );
            // Move back to correct cursor position
            if let Some(Loc { x, y }) = self.cursor_position() {
                self.terminal.goto(x, y);
                self.terminal.show_cursor();
            } else {
                self.terminal.hide_cursor();
            }
            self.terminal.flush()?;
            // Handle events
            if let Some((modifiers, code)) =
                key_event(&wait_for_event_hog(self), &mut self.macro_man)
            {
                match (modifiers, code) {
                    // On return or escape key, exit menu
                    (KMod::NONE, KCode::Enter) => done = true,
                    (KMod::NONE, KCode::Esc) => {
                        self.try_doc_mut().unwrap().move_to(&cache);
                        done = true;
                    }
                    // On left key, move to the previous match in the document
                    (KMod::NONE, KCode::Left) => std::mem::drop(self.prev_match(&target)),
                    // On right key, move to the next match in the document
                    (KMod::NONE, KCode::Right) => std::mem::drop(self.next_match(&target)),
                    _ => (),
                }
            }
            self.update_highlighter();
        }
        self.try_doc_mut().unwrap().cancel_selection();
        Ok(())
    }

    /// Move to the next match
    pub fn next_match(&mut self, target: &str) -> Option<String> {
        if target.is_empty() {
            return None;
        }

        if let Some(doc) = self.try_doc_mut() {
            let mtch = doc.next_match(target, 1)?;
            // Select match
            doc.cancel_selection();
            let mut move_to = mtch.loc;
            move_to.x += mtch.text.chars().count();
            doc.move_to(&move_to);
            doc.select_to(&mtch.loc);
            // Update highlighting
            self.update_highlighter();
            Some(mtch.text)
        } else {
            None
        }
    }

    /// Move to the previous match
    pub fn prev_match(&mut self, target: &str) -> Option<String> {
        if target.is_empty() {
            return None;
        }

        if let Some(doc) = self.try_doc_mut() {
            let mtch = doc.prev_match(target)?;
            doc.move_to(&mtch.loc);
            // Select match
            doc.cancel_selection();
            let mut move_to = mtch.loc;
            move_to.x += mtch.text.chars().count();
            doc.move_to(&move_to);
            doc.select_to(&mtch.loc);
            // Update highlighting
            self.update_highlighter();
            Some(mtch.text)
        } else {
            None
        }
    }

    /// Use replace feature
    pub fn replace(&mut self, lua: &Lua) -> Result<()> {
        // Block any non-documents from activating replace
        if self.try_doc().is_none() {
            return Ok(());
        }
        // Gather data
        let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
        // Request replace information
        let target = self.prompt("Replace")?;
        // If no target is given, do nothing
        if target.is_empty() {
            return Ok(());
        }
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
        self.update_highlighter();
        // Enter into the replace menu
        while !done {
            // Rerender
            self.needs_rerender = true;
            self.render(lua)?;
            // Write custom status line for the replace mode
            self.terminal.prepare_line(h);
            display!(
                self,
                editor_bg,
                Print(
                    "[<-] Previous | [->] Next | [Enter] Replace | [Tab] Replace All | [Esc] Exit"
                ),
                Print(" ".repeat(w.saturating_sub(76)))
            );
            // Move back to correct cursor location
            if let Some(Loc { x, y }) = self.cursor_position() {
                self.terminal.goto(x, y);
                self.terminal.show_cursor();
            } else {
                self.terminal.hide_cursor();
            }
            self.terminal.flush()?;
            // Handle events
            if let Some((modifiers, code)) =
                key_event(&wait_for_event_hog(self), &mut self.macro_man)
            {
                match (modifiers, code) {
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
            self.update_highlighter();
        }
        self.try_doc_mut().unwrap().cancel_selection();
        Ok(())
    }

    /// Replace an instance in a document
    fn do_replace(&mut self, into: &str, text: &str) -> Result<()> {
        if let Some(doc) = self.try_doc_mut() {
            // Commit events to event manager (for undo / redo)
            doc.commit();
            // Do the replacement
            let loc = doc.char_loc();
            doc.replace(loc, text, into)?;
            doc.move_to(&loc);
            // Update syntax highlighter
            self.update_highlighter();
            if let Some(file) = self.files.get_mut(self.ptr.clone()) {
                file.highlighter.edit(loc.y, &file.doc.lines[loc.y]);
            }
        }
        Ok(())
    }

    /// Replace all instances in a document
    fn do_replace_all(&mut self, target: &str, into: &str) {
        if self.try_doc().is_some() {
            // Commit events to event manager (for undo / redo)
            self.try_doc_mut().unwrap().commit();
            // Replace everything top to bottom
            self.try_doc_mut().unwrap().move_to(&Loc::at(0, 0));
            while let Some(mtch) = self.try_doc_mut().unwrap().next_match(target, 1) {
                drop(
                    self.try_doc_mut()
                        .unwrap()
                        .replace(mtch.loc, &mtch.text, into),
                );
                self.update_highlighter();
                if let Some(file) = self.files.get_mut(self.ptr.clone()) {
                    file.highlighter
                        .edit(mtch.loc.y, &file.doc.lines[mtch.loc.y]);
                }
            }
        }
    }
}
