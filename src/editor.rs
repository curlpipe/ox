use crate::ui::{size, Terminal, Feedback, HELP_TEXT};
use crate::config::Config;
use crate::error::{OxError, Result};
use crossterm::{
    event::{read, Event as CEvent, KeyCode as KCode, KeyModifiers as KMod},
    style::{SetBackgroundColor as Bg, SetForegroundColor as Fg, SetAttribute, Attribute},
    terminal::{Clear, ClearType as ClType},
};
use kaolinite::event::{Event, Status, Error as KError};
use kaolinite::utils::{Loc, Size};
use kaolinite::Document;
use synoptic::{Highlighter, TokOpt, trim, from_extension};
use std::io::{Write, ErrorKind};

/// For managing all editing and rendering of cactus
pub struct Editor {
    /// Interface for writing to the terminal
    pub terminal: Terminal,
    /// Configuration information for the editor
    config: Config,
    /// Storage of all the documents opened in the editor
    doc: Vec<Document>,
    /// Syntax highlighting integration
    highlighter: Vec<Highlighter>,
    /// Pointer to the document that is currently being edited
    ptr: usize,
    /// true if the editor is still running, false otherwise
    active: bool,
    /// true if the editor should show a greeting message on next render
    greet: bool,
    /// Whether or not to show the help message
    help: bool,
    /// The feedback message to display below the status line
    pub feedback: Feedback,
}

impl Editor {
    /// Create a new instance of the editor
    pub fn new(config: String) -> Result<Self> {
        Ok(Self {
            doc: vec![],
            ptr: 0,
            terminal: Terminal::new(),
            config: Config::read(config)?,
            active: true,
            greet: false,
            help: false,
            highlighter: vec![],
            feedback: Feedback::None,
        })
    }

    /// Function to create a new document
    pub fn blank(&mut self) -> Result<()> {
        let mut size = size()?;
        size.h -= 2;
        let mut doc = Document::new(size);
        // Load all the lines within viewport into the document
        doc.load_to(size.h);
        // Update in the syntax highlighter
        let mut highlighter = Highlighter::new(4);
        highlighter.run(&doc.lines);
        self.highlighter.push(highlighter);
        // Add document to documents
        self.doc.push(doc);
        Ok(())
    }

    /// Create a new document and move to it
    pub fn new_document(&mut self) -> Result<()> {
        self.blank()?;
        self.ptr = self.doc.len().saturating_sub(1);
        Ok(())
    }

    /// Function to open a document into the editor
    pub fn open(&mut self, file_name: String) -> Result<()> {
        let mut size = size()?;
        size.h -= 2;
        let mut doc = Document::open(size, file_name.clone())?;
        // Load all the lines within viewport into the document
        doc.load_to(size.h);
        // Update in the syntax highlighter
        let ext = file_name.split('.').last().unwrap();
        let mut highlighter = from_extension(ext, 4).unwrap_or(Highlighter::new(4));
        highlighter.run(&doc.lines);
        self.highlighter.push(highlighter);
        // Add document to documents
        self.doc.push(doc);
        Ok(())
    }

    /// Function to ask the user for a file to open
    pub fn open_document(&mut self) -> Result<()> {
        let path = self.prompt("File to open")?;
        self.open(path)?;
        self.ptr = self.doc.len().saturating_sub(1);
        Ok(())
    }

    /// Function to try opening a document, and if it doesn't exist, create it
    pub fn open_or_new(&mut self, file_name: String) -> Result<()> {
        let file = self.open(file_name.clone());
        if let Err(OxError::Kaolinite(KError::Io(ref os))) = file {
            if os.kind() == ErrorKind::NotFound {
                self.blank()?;
                self.doc.last_mut().unwrap().file_name = Some(file_name);
                self.doc.last_mut().unwrap().modified = true;
                Ok(())
            } else {
                file
            }
        } else {
            file
        }
    }

    /// Returns a document at a certain index
    pub fn get_doc(&mut self, idx: usize) -> &mut Document {
        self.doc.get_mut(idx).unwrap()
    }

    /// Gets a reference to the current document
    pub fn doc(&self) -> &Document {
        self.doc.get(self.ptr).unwrap()
    }

    /// Gets a mutable reference to the current document
    pub fn doc_mut(&mut self) -> &mut Document {
        self.doc.get_mut(self.ptr).unwrap()
    }

    /// Gets the number of documents currently open
    pub fn doc_len(&mut self) -> usize {
        self.doc.len()
    }

    /// Returns a highlighter at a certain index
    pub fn get_highlighter(&mut self, idx: usize) -> &mut Highlighter {
        self.highlighter.get_mut(idx).unwrap()
    }

    /// Gets a mutable reference to the current document
    pub fn highlighter(&mut self) -> &mut Highlighter {
        self.highlighter.get_mut(self.ptr).unwrap()
    }

    /// Execute an edit event
    pub fn exe(&mut self, ev: Event) -> Result<()> {
        self.doc_mut().exe(ev)?;
        Ok(())
    }

    /// Initialise, render and handle events as they come in
    pub fn run(&mut self) -> Result<()> {
        self.terminal.start()?;
        // If no documents were provided, create a new empty document
        if self.doc.is_empty() {
            self.blank()?;
            self.greet = true && self.config.greeting_message.borrow().enabled;
        }
        // Run the editor
        while self.active {
            self.render()?;
            self.feedback = Feedback::None;
            // Wait for an event
            match read()? {
                CEvent::Key(key) => match (key.modifiers, key.code) {
                    // Movement
                    (KMod::NONE, KCode::Up) => self.up(),
                    (KMod::NONE, KCode::Down) => self.down(),
                    (KMod::NONE, KCode::Left) => self.left(),
                    (KMod::NONE, KCode::Right) => self.right(),
                    (KMod::CONTROL, KCode::Up) => self.doc_mut().move_top(),
                    (KMod::CONTROL, KCode::Down) => self.doc_mut().move_bottom(),
                    (KMod::CONTROL, KCode::Left) => self.prev_word(),
                    (KMod::CONTROL, KCode::Right) => self.next_word(),
                    (KMod::NONE, KCode::Home) => self.doc_mut().move_home(),
                    (KMod::NONE, KCode::End) => self.doc_mut().move_end(),
                    (KMod::NONE, KCode::PageUp) => self.doc_mut().move_page_up(),
                    (KMod::NONE, KCode::PageDown) => self.doc_mut().move_page_down(),
                    // Searching & Replacing
                    (KMod::CONTROL, KCode::Char('f')) => self.search()?,
                    (KMod::CONTROL, KCode::Char('r')) => self.replace()?,
                    // Document management
                    (KMod::CONTROL, KCode::Char('n')) => self.new_document()?,
                    (KMod::CONTROL, KCode::Char('o')) => self.open_document()?,
                    (KMod::CONTROL, KCode::Char('s')) => self.save()?,
                    (KMod::ALT, KCode::Char('s')) => self.save_as()?,
                    (KMod::CONTROL, KCode::Char('a')) => self.save_all()?,
                    (KMod::CONTROL, KCode::Char('q')) => self.quit()?,
                    (KMod::SHIFT, KCode::Left) => self.prev(),
                    (KMod::SHIFT, KCode::Right) => self.next(),
                    // Undo & Redo
                    (KMod::CONTROL, KCode::Char('z')) => self.doc_mut().undo()?,
                    (KMod::CONTROL, KCode::Char('y')) => self.doc_mut().redo()?,
                    // Editing
                    (KMod::SHIFT | KMod::NONE, KCode::Char(ch)) => self.character(ch)?,
                    (KMod::NONE, KCode::Tab) => self.character('\t')?,
                    (KMod::NONE, KCode::Backspace) => self.backspace()?,
                    (KMod::NONE, KCode::Delete) => self.delete()?,
                    (KMod::NONE, KCode::Enter) => self.enter()?,
                    (KMod::CONTROL, KCode::Char('d')) => self.delete_line()?,
                    // Command
                    (KMod::CONTROL, KCode::Char('k')) => self.command()?,
                    _ => (),
                },
                CEvent::Resize(w, h) => {
                    // Ensure all lines in viewport are loaded
                    let max = self.dent();
                    self.doc_mut().size.w = w.saturating_sub(max as u16) as usize;
                    self.doc_mut().size.h = h.saturating_sub(3) as usize;
                    let max = self.doc().offset.x + self.doc().size.h;
                    self.doc_mut().load_to(max);
                    // Make sure cursor hasn't broken out of bounds
                    if self.doc().cursor.y >= self.doc().size.h - 1 {
                        let y = self.doc().size.h.saturating_sub(1);
                        self.doc_mut().cursor.y = y;
                        self.doc_mut().move_home();
                    }
                }
                _ => (),
            }
            self.greet = false;
            // Append any missed lines to the syntax highlighter
            if self.active {
                let actual = self.doc.get(self.ptr).and_then(|d| Some(d.loaded_to)).unwrap_or(0);
                let percieved = self.highlighter().line_ref.len();
                if percieved < actual {
                    let diff = actual - percieved;
                    for i in 0..diff {
                        let line = &self.doc[self.ptr].lines[percieved + i];
                        self.highlighter[self.ptr].append(line);
                    }
                }
            }
        }
        self.terminal.end()?;
        Ok(())
    }

    /// Render a single frame of the editor in it's current state
    pub fn render(&mut self) -> Result<()> {
        self.terminal.hide_cursor()?;
        let Size { w, mut h } = size()?;
        h -= 2;
        // Update the width of the document in case of update
        let max = self.dent();
        self.doc_mut().size.w = w.saturating_sub(max) as usize;
        // Render the tab line
        self.render_tab_line(w)?;
        // Run through each line of the terminal, rendering the correct line
        self.render_document(w, h)?;
        // Leave last line for status line
        self.render_status_line(w, h)?;
        // Render greeting or help message if applicable
        if self.greet {
            self.render_greeting(w, h)?;
        } else if self.help {
            self.render_help_message(w, h)?;
        }
        // Render feedback line
        self.render_feedback_line(w, h)?;
        // Move cursor to the correct location and perform render
        let Loc { x, y } = self.doc().cursor;
        self.terminal.show_cursor()?;
        self.terminal.goto(x + max, y + 1)?;
        self.terminal.flush()?;
        Ok(())
    }

    /// Render the lines of the document
    fn render_document(&mut self, _w: usize, h: usize) -> Result<()> {
        for y in 0..(h as u16) {
            self.terminal.goto(0, y + 1)?;
            // Start background colour
            write!(self.terminal.stdout, "{}", Bg(self.config.colors.borrow().editor_bg.to_color()?))?;
            write!(self.terminal.stdout, "{}", Fg(self.config.colors.borrow().editor_fg.to_color()?))?;
            // Write line number of document
            if self.config.line_numbers.borrow().enabled {
                let num = self.doc().line_number(y as usize + self.doc().offset.y);
                write!(
                    self.terminal.stdout,
                    "{}{} {} │{}{}",
                    Bg(self.config.colors.borrow().line_number_bg.to_color()?),
                    Fg(self.config.colors.borrow().line_number_fg.to_color()?),
                    num,
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
                for token in tokens {
                    match token {
                        TokOpt::Some(text, kind) => {
                            // Try to get the corresponding colour for this token
                            let colour = self.config.syntax_highlighting.borrow().get_theme(&kind);
                            match colour {
                                // Success, write token
                                Ok(col) => write!(
                                    self.terminal.stdout,
                                    "{}{text}{}",
                                    Fg(col),
                                    Fg(self.config.colors.borrow().editor_fg.to_color()?),
                                ),
                                // Failure, show error message and don't highlight this token
                                Err(err) => {
                                    self.feedback = Feedback::Error(err.to_string());
                                    write!(self.terminal.stdout, "{text}")
                                }
                            }
                        }
                        TokOpt::None(text) => write!(self.terminal.stdout, "{text}"),
                    }?
                }
            }
        }
        Ok(())
    }

    /// Render the tab line at the top of the document
    fn render_tab_line(&mut self, w: usize) -> Result<()> {
        self.terminal.prepare_line(0)?;
        write!(
            self.terminal.stdout, 
            "{}{}", 
            Fg(self.config.colors.borrow().tab_inactive_fg.to_color()?),
            Bg(self.config.colors.borrow().tab_inactive_bg.to_color()?)
        )?;
        for (c, document) in self.doc.iter().enumerate() {
            let file_name = document.file_name.clone().unwrap_or_else(|| "[No Name]".to_string());
            let modified = if document.modified { "[+]" } else { "" };
            if c == self.ptr {
                // Representing the document we're currently looking at
                write!(
                    self.terminal.stdout, 
                    "{}{}{}  {file_name}{modified}  {}{}{}│",
                    Bg(self.config.colors.borrow().tab_active_bg.to_color()?),
                    Fg(self.config.colors.borrow().tab_active_fg.to_color()?),
                    SetAttribute(Attribute::Bold),
                    SetAttribute(Attribute::Reset),
                    Fg(self.config.colors.borrow().tab_inactive_fg.to_color()?),
                    Bg(self.config.colors.borrow().tab_inactive_bg.to_color()?),
                )?;
            } else {
                // Other document that is currently open
                write!(self.terminal.stdout, "  {file_name}{modified}  │")?;
            }
        }
        write!(self.terminal.stdout, "{}", " ".to_string().repeat(w))?;
        Ok(())
    }

    /// Render the status line at the bottom of the document
    fn render_status_line(&mut self, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + 1)?;
        write!(
            self.terminal.stdout, 
            "{}{}{}{}{}{}{}", 
            Bg(self.config.colors.borrow().status_bg.to_color()?),
            Fg(self.config.colors.borrow().status_fg.to_color()?),
            SetAttribute(Attribute::Bold),
            self.config.status_line.borrow().render(&self, w),
            SetAttribute(Attribute::Reset),
            Fg(self.config.colors.borrow().editor_fg.to_color()?),
            Bg(self.config.colors.borrow().editor_bg.to_color()?),
        )?;
        Ok(())
    }

    /// Render the feedback line
    fn render_feedback_line(&mut self, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + 2)?;
        write!(
            self.terminal.stdout, 
            "{}",
            self.feedback.render(&self.config.colors.borrow(), w)?,
        )?;
        Ok(())
    }

    /// Render the greeting message
    fn render_help_message(&mut self, w: usize, h: usize) -> Result<()> {
        let color = self.config.colors.borrow().highlight.to_color()?;
        let editor_fg = self.config.colors.borrow().editor_fg.to_color()?;
        let message: Vec<&str> = HELP_TEXT.split('\n').collect();
        for (c, line) in message.iter().enumerate().take(h - h / 4) {
            self.terminal.goto(w - 30, h / 4 + c + 1)?;
            write!(self.terminal.stdout, "{}{line}{}", Fg(color), Fg(editor_fg))?;
        }
        Ok(())
    }

    /// Render the help message
    fn render_greeting(&mut self, w: usize, h: usize) -> Result<()> {
        let colors = self.config.colors.borrow();
        let greeting = self.config.greeting_message.borrow().render(&colors)?;
        let message: Vec<&str> = greeting.split('\n').collect();
        for (c, line) in message.iter().enumerate().take(h - h / 4) {
            self.terminal.goto(4, h / 4 + c + 1)?;
            write!(
                self.terminal.stdout, 
                "{}",
                alinio::align::center(&line, w - 4)
                    .unwrap_or_else(|| "".to_string()),
            )?;
        }
        Ok(())
    }

    /// Display a prompt in the document
    fn prompt<S: Into<String>>(&mut self, prompt: S) -> Result<String> {
        let prompt = prompt.into();
        let mut input = String::new();
        let mut done = false;
        // Enter into a menu that asks for a prompt
        while !done {
            let h = size()?.h;
            let w = size()?.w;
            // Render prompt message
            self.terminal.prepare_line(h)?;
            write!(self.terminal.stdout, "{}", Bg(self.config.colors.borrow().editor_bg.to_color()?))?;
            write!(self.terminal.stdout, "{}: {}{}", prompt, input, " ".to_string().repeat(w))?;
            self.terminal.goto(prompt.len() + input.len() + 2, h)?;
            self.terminal.flush()?;
            // Handle events
            if let CEvent::Key(key) = read()? {
                match (key.modifiers, key.code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Remove from the input string if the user presses backspace
                    (KMod::NONE, KCode::Backspace) => { input.pop(); },
                    // Add to the input string if the user presses a character
                    (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => input.push(c),
                    _ => (),
                }
            }
        }
        // Return input string result
        Ok(input)
    }

    /// Work out how much to push the document to the right (to make way for line numbers)
    fn dent(&self) -> usize {
        if self.config.line_numbers.borrow().enabled {
            self.doc().len_lines().to_string().len() + 3
        } else {
            0
        }
    }

    /// Move to the next document opened in the editor
    fn next(&mut self) {
        if self.ptr + 1 < self.doc.len() {
            self.ptr += 1;
        }
    }

    /// Move to the previous document opened in the editor
    fn prev(&mut self) {
        if self.ptr != 0 {
            self.ptr -= 1;
        }
    }

    /// Move the cursor up
    fn up(&mut self) {
        self.doc_mut().move_up();
    }

    /// Move the cursor down
    fn down(&mut self) {
        self.doc_mut().move_down();
    }

    /// Move the cursor left
    fn left(&mut self) {
        let status = self.doc_mut().move_left();
        // Cursor wrapping if cursor hits the start of the line
        if status == Status::StartOfLine && self.doc().loc().y != 0 {
            self.doc_mut().move_up();
            self.doc_mut().move_end();
        }
    }

    /// Move the cursor right
    fn right(&mut self) {
        let status = self.doc_mut().move_right();
        // Cursor wrapping if cursor hits the end of a line
        if status == Status::EndOfLine {
            self.doc_mut().move_down();
            self.doc_mut().move_home();
        }
    }

    /// Move the cursor to the previous word in the line
    fn prev_word(&mut self) {
        let status = self.doc_mut().move_prev_word();
        if status == Status::StartOfLine {
            self.doc_mut().move_up();
            self.doc_mut().move_end();
        }
    }

    /// Move the cursor to the next word in the line
    fn next_word(&mut self) {
        let status = self.doc_mut().move_next_word();
        if status == Status::EndOfLine {
            self.doc_mut().move_down();
            self.doc_mut().move_home();
        }
    }

    /// Insert a character into the document, creating a new row if editing
    /// on the last line of the document
    fn character(&mut self, ch: char) -> Result<()> {
        self.new_row()?;
        let loc = self.doc().char_loc();
        self.exe(Event::Insert(loc, ch.to_string()))?;
        self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
        Ok(())
    }

    /// Handle the return key
    fn enter(&mut self) -> Result<()> {
        if self.doc().loc().y != self.doc().len_lines() {
            // Enter pressed in the start, middle or end of the line
            let loc = self.doc().char_loc();
            self.exe(Event::SplitDown(loc))?;
            let line = &self.doc[self.ptr].lines[loc.y + 1];
            self.highlighter[self.ptr].insert_line(loc.y + 1, line);
            let line = &self.doc[self.ptr].lines[loc.y];
            self.highlighter[self.ptr].edit(loc.y, line);
        } else {
            // Enter pressed on the empty line at the bottom of the document
            self.new_row()?;
        }
        Ok(())
    }

    /// Handle the backspace key
    fn backspace(&mut self) -> Result<()> {
        let mut c = self.doc().char_ptr;
        let on_first_line = self.doc().loc().y == 0;
        let out_of_range = self.doc().out_of_range(0, self.doc().loc().y).is_err();
        if c == 0 && !on_first_line && !out_of_range {
            // Backspace was pressed on the start of the line, move line to the top
            self.new_row()?;
            let mut loc = self.doc().char_loc();
            self.highlighter().remove_line(loc.y);
            loc.y -= 1;
            loc.x = self.doc().line(loc.y).unwrap().chars().count();
            self.exe(Event::SpliceUp(loc))?;
            let line = &self.doc[self.ptr].lines[loc.y];
            self.highlighter[self.ptr].edit(loc.y, line);
        } else {
            // Backspace was pressed in the middle of the line, delete the character
            c -= 1;
            if let Some(line) = self.doc().line(self.doc().loc().y) {
                if let Some(ch) = line.chars().nth(c) {
                    let loc = Loc { x: c, y: self.doc().loc().y };
                    self.exe(Event::Delete(loc, ch.to_string()))?;
                    self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
                }
            }
        }
        Ok(())
    }

    /// Delete the character in place
    fn delete(&mut self) -> Result<()> {
        let c = self.doc().char_ptr;
        if let Some(line) = self.doc().line(self.doc().loc().y) {
            if let Some(ch) = line.chars().nth(c) {
                let loc = Loc { x: c, y: self.doc().loc().y };
                self.exe(Event::Delete(loc, ch.to_string()))?;
                self.highlighter[self.ptr].edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
            }
        }
        Ok(())
    }

    /// Insert a new row at the end of the document if the cursor is on it
    fn new_row(&mut self) -> Result<()> {
        if self.doc().loc().y == self.doc().len_lines() {
            self.exe(Event::InsertLine(self.doc().loc().y, "".to_string()))?;
            self.highlighter().append(&"".to_string());
        }
        Ok(())
    }

    /// Delete the current line
    fn delete_line(&mut self) -> Result<()> {
        if self.doc().loc().y < self.doc().len_lines() {
            let y = self.doc().loc().y;
            let line = self.doc().line(y).unwrap();
            self.exe(Event::DeleteLine(y, line))?;
            self.highlighter().remove_line(y);
        }
        Ok(())
    }

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
            self.render_document(w, h - 2)?;
            // Render custom status line with mode information
            self.terminal.prepare_line(h)?;
            write!(self.terminal.stdout, "[<-]: Search previous | [->]: Search next")?;
            self.terminal.flush()?;
            // Move back to correct cursor position
            let Loc { x, y } = self.doc().cursor;
            let max = self.dent();
            self.terminal.goto(x + max, y + 1)?;
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
        }
        Ok(())
    }

    /// Move to the next match
    fn next_match(&mut self, target: &str) -> Option<String> {
        let mtch = self.doc_mut().next_match(target, 1)?;
        self.doc_mut().goto(&mtch.loc);
        Some(mtch.text)
    }

    /// Move to the previous match
    fn prev_match(&mut self, target: &str) -> Option<String> {
        let mtch = self.doc_mut().prev_match(target)?;
        self.doc_mut().goto(&mtch.loc);
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
        // Enter into the replace menu
        while !done {
            // Render just the document part
            self.render_document(w, h - 2)?;
            // Write custom status line for the replace mode
            self.terminal.prepare_line(h)?;
            write!(self.terminal.stdout, "[<-] Previous | [->] Next | [Enter] Replace | [Tab] Replace All")?;
            self.terminal.flush()?;
            // Move back to correct cursor location
            let Loc { x, y } = self.doc().cursor;
            let max = self.dent();
            self.terminal.goto(x + max, y + 1)?;
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
        }
        Ok(())
    }

    /// Replace an instance in a document
    fn do_replace(&mut self, into: &str, text: &str) -> Result<()> {
        let loc = self.doc().char_loc();
        self.doc_mut().replace(loc, text, into)?;
        self.doc_mut().goto(&loc);
        Ok(())
    }

    /// Replace all instances in a document
    fn do_replace_all(&mut self, target: &str, into: &str) {
        self.doc_mut().replace_all(target, into);
    }

    /// save the document to the disk
    pub fn save(&mut self) -> Result<()> {
        self.doc_mut().save()?;
        self.feedback = Feedback::Info("Document saved successfully".to_string());
        Ok(())
    }

    /// save the document to the disk at a specified path
    pub fn save_as(&mut self) -> Result<()> {
        let file_name = self.prompt("Save as")?;
        self.doc_mut().save_as(&file_name)?;
        self.feedback = Feedback::Info(format!("Document saved as {file_name} successfully"));
        if self.doc().file_name.is_none() {
            let ext = file_name.split('.').last().unwrap();
            self.highlighter[self.ptr] = from_extension(ext, 4).unwrap_or(Highlighter::new(4));
            self.doc_mut().file_name = Some(file_name);
            self.doc_mut().modified = false;
        }
        Ok(())
    }

    /// Save all the open documents to the disk
    pub fn save_all(&mut self) -> Result<()> {
        for doc in self.doc.iter_mut() {
            doc.save()?;
        }
        self.feedback = Feedback::Info(format!("Saved all documents"));
        Ok(())
    }

    /// Quit the editor
    pub fn quit(&mut self) -> Result<()> {
        self.active = !self.doc.is_empty();
        // If there are still documents open, only close the requested document
        if self.active {
            let msg = "This document isn't saved, press Ctrl + Q to force quit or Esc to cancel";
            if !self.doc().modified || self.confirm(msg)? {
                self.doc.remove(self.ptr);
                self.highlighter.remove(self.ptr);
                self.prev();
            }
        }
        self.active = !self.doc.is_empty();
        Ok(())
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

    /// Open command line
    pub fn command(&mut self) -> Result<()> {
        let cmd = self.prompt("Command")?;
        self.run_command(cmd)?;
        Ok(())
    }

    /// Run a command
    pub fn run_command(&mut self, cmd: String) -> Result<()> {
        match cmd.split(' ').collect::<Vec<&str>>().as_slice() {
            ["filetype", ext] => {
                // Change the highlighter of the current file
                self.highlighter[self.ptr] = from_extension(ext, 4)
                    .unwrap_or_else(|| Highlighter::new(4));
            }
            ["readonly", "true"] => self.doc_mut().read_only = true,
            ["readonly", "false"] => self.doc_mut().read_only = false,
            ["help"] => self.help = !self.help,
            _ => {
                self.feedback = Feedback::Error(format!("Command '{}' not found", cmd));
            }
        }
        Ok(())
    }
}
