use crate::display;
use crate::error::{OxError, Result};
use crate::ui::{size, Feedback};
use crossterm::{
    event::{read, Event as CEvent, KeyCode as KCode, KeyModifiers as KMod},
    queue,
    style::{
        Attribute, Color, Print, SetAttribute, SetBackgroundColor as Bg, SetForegroundColor as Fg,
    },
};
use kaolinite::utils::{file_or_dir, get_cwd, get_parent, list_dir, width, Loc, Size};
use mlua::Lua;
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
        self.doc_mut().size.w = w.saturating_sub(max);
        // Render the tab line
        let tab_enabled = self.config.tab_line.borrow().enabled;
        if tab_enabled {
            self.render_tab_line(w)?;
        }
        // Run through each line of the terminal, rendering the correct line
        self.render_document(w, h)?;
        // Leave last line for status line
        self.render_status_line(lua, w, h)?;
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
    #[allow(clippy::similar_names)]
    pub fn render_document(&mut self, w: usize, h: usize) -> Result<()> {
        for y in 0..u16::try_from(h).unwrap_or(0) {
            self.terminal.goto(0, y as usize + self.push_down)?;
            // Start colours
            let editor_bg = Bg(self.config.colors.borrow().editor_bg.to_color()?);
            let editor_fg = Fg(self.config.colors.borrow().editor_fg.to_color()?);
            let line_number_bg = Bg(self.config.colors.borrow().line_number_bg.to_color()?);
            let line_number_fg = Fg(self.config.colors.borrow().line_number_fg.to_color()?);
            let selection_bg = Bg(self.config.colors.borrow().selection_bg.to_color()?);
            let selection_fg = Fg(self.config.colors.borrow().selection_fg.to_color()?);
            display!(self, editor_bg, editor_fg);
            // Write line number of document
            if self.config.line_numbers.borrow().enabled {
                let num = self.doc().line_number(y as usize + self.doc().offset.y);
                let padding_left = " ".repeat(self.config.line_numbers.borrow().padding_left);
                let padding_right = " ".repeat(self.config.line_numbers.borrow().padding_right);
                display!(
                    self,
                    line_number_bg,
                    line_number_fg,
                    padding_left,
                    num,
                    padding_right,
                    "│",
                    editor_fg,
                    editor_bg
                );
            }
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
                                    display!(self, Fg(col));
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
                            display!(self, selection_bg, selection_fg);
                        } else {
                            display!(self, editor_bg);
                        }
                        display!(self, c);
                        x_pos += 1;
                    }
                    display!(self, editor_fg, editor_bg);
                }
                // Pad out the line (to remove any junk left over from previous render)
                let tab_width = self.config.document.borrow().tab_width;
                let line_width = width(&line, tab_width);
                let pad_amount = w.saturating_sub(self.dent()).saturating_sub(line_width) + 1;
                display!(self, " ".repeat(pad_amount));
            } else {
                // Render empty line
                let pad_amount = w.saturating_sub(self.dent()) + 1;
                display!(self, " ".repeat(pad_amount));
            }
        }
        Ok(())
    }

    /// Render the tab line at the top of the document
    #[allow(clippy::similar_names)]
    pub fn render_tab_line(&mut self, w: usize) -> Result<()> {
        self.terminal.goto(0_usize, 0_usize)?;
        let tab_inactive_bg = Bg(self.config.colors.borrow().tab_inactive_bg.to_color()?);
        let tab_inactive_fg = Fg(self.config.colors.borrow().tab_inactive_fg.to_color()?);
        let tab_active_bg = Bg(self.config.colors.borrow().tab_active_bg.to_color()?);
        let tab_active_fg = Fg(self.config.colors.borrow().tab_active_fg.to_color()?);
        display!(self, tab_inactive_fg, tab_inactive_bg);
        for (c, document) in self.doc.iter().enumerate() {
            let document_header = self.config.tab_line.borrow().render(document);
            if c == self.ptr {
                // Representing the document we're currently looking at
                display!(
                    self,
                    tab_active_bg,
                    tab_active_fg,
                    SetAttribute(Attribute::Bold),
                    document_header,
                    SetAttribute(Attribute::Reset),
                    tab_inactive_fg,
                    tab_inactive_bg,
                    "│"
                );
            } else {
                // Other document that is currently open
                display!(self, document_header, "│");
            }
        }
        display!(self, " ".to_string().repeat(w));
        Ok(())
    }

    /// Render the status line at the bottom of the document
    #[allow(clippy::similar_names)]
    pub fn render_status_line(&mut self, lua: &Lua, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + self.push_down)?;
        let editor_bg = Bg(self.config.colors.borrow().editor_bg.to_color()?);
        let editor_fg = Fg(self.config.colors.borrow().editor_fg.to_color()?);
        let status_bg = Bg(self.config.colors.borrow().status_bg.to_color()?);
        let status_fg = Fg(self.config.colors.borrow().status_fg.to_color()?);
        let content = self.config.status_line.borrow().render(self, lua, w);
        display!(
            self,
            status_bg,
            status_fg,
            SetAttribute(Attribute::Bold),
            content,
            SetAttribute(Attribute::Reset),
            editor_fg,
            editor_bg
        );
        Ok(())
    }

    /// Render the feedback line
    pub fn render_feedback_line(&mut self, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + 2)?;
        let content = self.feedback.render(&self.config.colors.borrow(), w)?;
        display!(self, content);
        Ok(())
    }

    /// Render the greeting message
    fn render_help_message(&mut self, lua: &Lua, w: usize, h: usize) -> Result<()> {
        let colors = self.config.colors.borrow();
        let message = self.config.help_message.borrow().render(lua, &colors)?;
        for (c, line) in message.iter().enumerate().take(h.saturating_sub(h / 4)) {
            self.terminal.goto(w.saturating_sub(30), h / 4 + c + 1)?;
            display!(self, line);
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
            let content = alinio::align::center(line, w.saturating_sub(4)).unwrap_or_default();
            display!(self, content);
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
            self.terminal.show_cursor()?;
            let editor_bg = Bg(self.config.colors.borrow().editor_bg.to_color()?);
            display!(
                self,
                editor_bg,
                prompt.clone(),
                ": ",
                input.clone(),
                " ".to_string().repeat(w)
            );
            self.terminal.goto(prompt.len() + input.len() + 2, h)?;
            self.terminal.flush()?;
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Cancel operation
                    (KMod::NONE, KCode::Esc) => return Err(OxError::Cancelled),
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

    /// Prompt for selecting a file
    pub fn path_prompt(&mut self) -> Result<String> {
        let mut input = get_cwd().map(|s| s + "/").unwrap_or_default();
        let mut offset = 0;
        let mut done = false;
        let mut old_suggestions = vec![];
        // Enter into a menu that asks for a prompt
        while !done {
            // Find the suggested files and folders
            let parent = if input.ends_with('/') {
                input.to_string()
            } else {
                get_parent(&input).unwrap_or_default()
            };
            let suggestions = list_dir(&parent)
                .unwrap_or_default()
                .iter()
                .filter(|p| p.starts_with(&input))
                .cloned()
                .collect::<Vec<_>>();
            // Reset offset if we've changed suggestions / out of bounds
            if suggestions != old_suggestions || offset >= suggestions.len() {
                offset = 0;
            }
            old_suggestions.clone_from(&suggestions);
            // Select suggestion
            let mut suggestion = suggestions
                .get(offset)
                .map(std::string::ToString::to_string)
                .unwrap_or(input.clone());
            // Render prompt message
            let h = size()?.h;
            self.terminal.prepare_line(h)?;
            self.terminal.show_cursor()?;
            let suggestion_text = suggestion
                .chars()
                .skip(input.chars().count())
                .collect::<String>();
            let editor_fg = Fg(self.config.colors.borrow().editor_fg.to_color()?);
            display!(
                self,
                "Path: ",
                input.clone(),
                Fg(Color::DarkGrey),
                suggestion_text,
                editor_fg
            );
            let tab_width = self.config.document.borrow_mut().tab_width;
            self.terminal.goto(6 + width(&input, tab_width), h)?;
            self.terminal.flush()?;
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Cancel when escape key is pressed
                    (KMod::NONE, KCode::Esc) => return Err(OxError::Cancelled),
                    // Remove from the input string if the user presses backspace
                    (KMod::NONE, KCode::Backspace) => {
                        input.pop();
                    }
                    // Add to the input string if the user presses a character
                    (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => input.push(c),
                    // Autocomplete path
                    (KMod::NONE, KCode::Right) => {
                        if file_or_dir(&suggestion) == "directory" {
                            suggestion += "/";
                        }
                        input = suggestion;
                        offset = 0;
                    }
                    // Cycle through suggestions
                    (KMod::SHIFT, KCode::BackTab) => offset = offset.saturating_sub(1),
                    (KMod::NONE, KCode::Tab) => {
                        if offset + 1 < suggestions.len() {
                            offset += 1;
                        }
                    }
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
    pub fn update_highlighter(&mut self) {
        if self.active {
            let actual = self.doc.get(self.ptr).map_or(0, |d| d.info.loaded_to);
            let percieved = self.highlighter().line_ref.len();
            if percieved < actual {
                let diff = actual.saturating_sub(percieved);
                for i in 0..diff {
                    let line = &self.doc[self.ptr].lines[percieved + i];
                    self.highlighter[self.ptr].append(line);
                }
            }
        }
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
