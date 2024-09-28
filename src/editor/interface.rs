use crate::error::Result;
use crate::ui::{size, Feedback};
use crossterm::{
    event::{read, Event as CEvent, KeyCode as KCode, KeyModifiers as KMod},
    style::{Attribute, SetAttribute, SetBackgroundColor as Bg, SetForegroundColor as Fg},
    terminal::{Clear, ClearType as ClType},
};
use kaolinite::utils::{Loc, Size};
use mlua::Lua;
use std::io::Write;
use synoptic::{trim, Highlighter, TokOpt};

use super::Editor;

impl Editor {
    /// Render a single frame of the editor in it's current state
    pub fn render(&mut self, lua: &Lua) -> Result<()> {
        if !self.needs_rerender {
            return Ok(());
        }
        self.needs_rerender = false;
        self.terminal.hide_cursor()?;
        let Size { w, mut h } = size()?;
        h = h.saturating_sub(1 + self.push_down);
        // Update the width of the document in case of update
        let max = self.dent();
        self.doc_mut().size.w = w.saturating_sub(max) as usize;
        // Render the tab line
        let tab_enabled = self.config.tab_line.borrow().enabled;
        if tab_enabled {
            self.render_tab_line(w)?;
        }
        // Run through each line of the terminal, rendering the correct line
        self.render_document(w, h)?;
        // Leave last line for status line
        self.render_status_line(&lua, w, h)?;
        // Render greeting or help message if applicable
        if self.greet {
            self.render_greeting(lua, w, h)?;
        } else if self.config.help_message.borrow().enabled {
            self.render_help_message(lua, w, h)?;
        }
        // Render feedback line
        self.render_feedback_line(w, h)?;
        // Move cursor to the correct location and perform render
        if let Some(Loc { x, y }) = self.doc().cursor_loc_in_screen() {
            self.terminal.show_cursor()?;
            self.terminal.goto(x + max, y + self.push_down)?;
        }
        self.terminal.flush()?;
        Ok(())
    }

    /// Render the lines of the document
    pub fn render_document(&mut self, _w: usize, h: usize) -> Result<()> {
        for y in 0..(h as u16) {
            self.terminal
                .goto(0, y as usize + self.push_down as usize)?;
            // Start background colour
            write!(
                self.terminal.stdout,
                "{}",
                Bg(self.config.colors.borrow().editor_bg.to_color()?)
            )?;
            write!(
                self.terminal.stdout,
                "{}",
                Fg(self.config.colors.borrow().editor_fg.to_color()?)
            )?;
            // Write line number of document
            if self.config.line_numbers.borrow().enabled {
                let num = self.doc().line_number(y as usize + self.doc().offset.y);
                let padding_left = " ".repeat(self.config.line_numbers.borrow().padding_left);
                let padding_right = " ".repeat(self.config.line_numbers.borrow().padding_right);
                write!(
                    self.terminal.stdout,
                    "{}{}{}{}{}│{}{}",
                    Bg(self.config.colors.borrow().line_number_bg.to_color()?),
                    Fg(self.config.colors.borrow().line_number_fg.to_color()?),
                    padding_left,
                    num,
                    padding_right,
                    Fg(self.config.colors.borrow().editor_fg.to_color()?),
                    Bg(self.config.colors.borrow().editor_bg.to_color()?),
                )?;
            }
            write!(self.terminal.stdout, "{}", Clear(ClType::UntilNewLine))?;
            // Render line if it exists
            let idx = y as usize + self.doc().offset.y;
            if let Some(line) = self.doc().line(idx) {
                let tokens = self.highlighter().line(idx, &line);
                let tokens = trim(&tokens, self.doc().offset.x);
                let mut x_pos = self.doc().offset.x;
                for token in tokens {
                    let text = match token {
                        TokOpt::Some(text, kind) => {
                            // Try to get the corresponding colour for this token
                            let colour = self.config.syntax_highlighting.borrow().get_theme(&kind);
                            match colour {
                                // Success, write token
                                Ok(col) => {
                                    write!(self.terminal.stdout, "{}", Fg(col),)?;
                                }
                                // Failure, show error message and don't highlight this token
                                Err(err) => {
                                    self.feedback = Feedback::Error(err.to_string());
                                }
                            }
                            text
                        }
                        TokOpt::None(text) => text,
                    };
                    for c in text.chars() {
                        let at_x = self.doc().character_idx(&Loc { y: idx, x: x_pos });
                        let is_selected = self.doc().is_loc_selected(Loc { y: idx, x: at_x });
                        if is_selected {
                            write!(
                                self.terminal.stdout,
                                "{}{}",
                                Bg(self.config.colors.borrow().selection_bg.to_color()?),
                                Fg(self.config.colors.borrow().selection_fg.to_color()?),
                            )?;
                        } else {
                            write!(
                                self.terminal.stdout,
                                "{}",
                                Bg(self.config.colors.borrow().editor_bg.to_color()?)
                            )?;
                        }
                        write!(self.terminal.stdout, "{c}")?;
                        x_pos += 1;
                    }
                    write!(
                        self.terminal.stdout,
                        "{}",
                        Fg(self.config.colors.borrow().editor_fg.to_color()?)
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Render the tab line at the top of the document
    pub fn render_tab_line(&mut self, w: usize) -> Result<()> {
        self.terminal.goto(0 as usize, 0 as usize)?;
        write!(
            self.terminal.stdout,
            "{}{}",
            Fg(self.config.colors.borrow().tab_inactive_fg.to_color()?),
            Bg(self.config.colors.borrow().tab_inactive_bg.to_color()?)
        )?;
        for (c, document) in self.doc.iter().enumerate() {
            let document_header = self.config.tab_line.borrow().render(document);
            if c == self.ptr {
                // Representing the document we're currently looking at
                write!(
                    self.terminal.stdout,
                    "{}{}{}{document_header}{}{}{}│",
                    Bg(self.config.colors.borrow().tab_active_bg.to_color()?),
                    Fg(self.config.colors.borrow().tab_active_fg.to_color()?),
                    SetAttribute(Attribute::Bold),
                    SetAttribute(Attribute::Reset),
                    Fg(self.config.colors.borrow().tab_inactive_fg.to_color()?),
                    Bg(self.config.colors.borrow().tab_inactive_bg.to_color()?),
                )?;
            } else {
                // Other document that is currently open
                write!(self.terminal.stdout, "{document_header}│")?;
            }
        }
        write!(self.terminal.stdout, "{}", " ".to_string().repeat(w))?;
        Ok(())
    }

    /// Render the status line at the bottom of the document
    pub fn render_status_line(&mut self, lua: &Lua, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + self.push_down)?;
        write!(
            self.terminal.stdout,
            "{}{}{}{}{}{}{}",
            Bg(self.config.colors.borrow().status_bg.to_color()?),
            Fg(self.config.colors.borrow().status_fg.to_color()?),
            SetAttribute(Attribute::Bold),
            self.config.status_line.borrow().render(&self, &lua, w),
            SetAttribute(Attribute::Reset),
            Fg(self.config.colors.borrow().editor_fg.to_color()?),
            Bg(self.config.colors.borrow().editor_bg.to_color()?),
        )?;
        Ok(())
    }

    /// Render the feedback line
    pub fn render_feedback_line(&mut self, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + 2)?;
        write!(
            self.terminal.stdout,
            "{}",
            self.feedback.render(&self.config.colors.borrow(), w)?,
        )?;
        Ok(())
    }

    /// Render the greeting message
    fn render_help_message(&mut self, lua: &Lua, w: usize, h: usize) -> Result<()> {
        let colors = self.config.colors.borrow();
        let message = self.config.help_message.borrow().render(lua, &colors)?;
        for (c, line) in message.iter().enumerate().take(h.saturating_sub(h / 4)) {
            self.terminal.goto(w.saturating_sub(30), h / 4 + c + 1)?;
            write!(self.terminal.stdout, "{line}")?;
        }
        Ok(())
    }

    /// Render the help message
    fn render_greeting(&mut self, lua: &Lua, w: usize, h: usize) -> Result<()> {
        let colors = self.config.colors.borrow();
        let greeting = self.config.greeting_message.borrow().render(lua, &colors)?;
        let message: Vec<&str> = greeting.split('\n').collect();
        for (c, line) in message.iter().enumerate().take(h.saturating_sub(h / 4)) {
            self.terminal.goto(4, h / 4 + c + 1)?;
            write!(
                self.terminal.stdout,
                "{}",
                alinio::align::center(&line, w.saturating_sub(4)).unwrap_or_else(|| "".to_string()),
            )?;
        }
        Ok(())
    }

    /// Display a prompt in the document
    pub fn prompt<S: Into<String>>(&mut self, prompt: S) -> Result<String> {
        let prompt = prompt.into();
        let mut input = String::new();
        let mut done = false;
        // Enter into a menu that asks for a prompt
        while !done {
            let h = size()?.h;
            let w = size()?.w;
            // Render prompt message
            self.terminal.prepare_line(h)?;
            write!(
                self.terminal.stdout,
                "{}",
                Bg(self.config.colors.borrow().editor_bg.to_color()?)
            )?;
            write!(
                self.terminal.stdout,
                "{}: {}{}",
                prompt,
                input,
                " ".to_string().repeat(w)
            )?;
            self.terminal.goto(prompt.len() + input.len() + 2, h)?;
            self.terminal.flush()?;
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Remove from the input string if the user presses backspace
                    (KMod::NONE, KCode::Backspace) => {
                        input.pop();
                    }
                    // Add to the input string if the user presses a character
                    (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => input.push(c),
                    _ => (),
                }
            }
        }
        // Return input string result
        Ok(input)
    }

    /// Confirmation dialog
    pub fn confirm(&mut self, msg: &str) -> Result<bool> {
        let mut done = false;
        let mut result = false;
        // Enter into the confirmation menu
        self.terminal.hide_cursor()?;
        while !done {
            let h = size()?.h;
            let w = size()?.w;
            // Render message
            self.feedback = Feedback::Warning(msg.to_string());
            self.render_feedback_line(w, h)?;
            self.terminal.flush()?;
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Esc) => {
                        done = true;
                        self.feedback = Feedback::None;
                    }
                    // Add to the input string if the user presses a character
                    (KMod::CONTROL, KCode::Char('q')) => {
                        done = true;
                        result = true;
                        self.feedback = Feedback::None;
                    }
                    _ => (),
                }
            }
        }
        self.terminal.show_cursor()?;
        Ok(result)
    }

    /// Append any missed lines to the syntax highlighter
    pub fn update_highlighter(&mut self) -> Result<()> {
        if self.active {
            let actual = self
                .doc
                .get(self.ptr)
                .and_then(|d| Some(d.loaded_to))
                .unwrap_or(0);
            let percieved = self.highlighter().line_ref.len();
            if percieved < actual {
                let diff = actual.saturating_sub(percieved);
                for i in 0..diff {
                    let line = &self.doc[self.ptr].lines[percieved + i];
                    self.highlighter[self.ptr].append(line);
                }
            }
        }
        Ok(())
    }

    /// Returns a highlighter at a certain index
    pub fn get_highlighter(&mut self, idx: usize) -> &mut Highlighter {
        self.highlighter.get_mut(idx).unwrap()
    }

    /// Gets a mutable reference to the current document
    pub fn highlighter(&mut self) -> &mut Highlighter {
        self.highlighter.get_mut(self.ptr).unwrap()
    }

    /// Reload the whole document in the highlighter
    pub fn reload_highlight(&mut self) {
        self.highlighter[self.ptr].run(&self.doc[self.ptr].lines);
    }

    /// Work out how much to push the document to the right (to make way for line numbers)
    pub fn dent(&self) -> usize {
        if self.config.line_numbers.borrow().enabled {
            let padding_left = self.config.line_numbers.borrow().padding_left;
            let padding_right = self.config.line_numbers.borrow().padding_right;
            self.doc().len_lines().to_string().len() + 1 + padding_left + padding_right
        } else {
            0
        }
    }
}
