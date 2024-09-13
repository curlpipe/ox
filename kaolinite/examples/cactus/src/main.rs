/*
  Cactus - A complete text editor in under 500 source lines of code
  - File buffering for efficient file reading and writing
  - Full double width character support
  - Openining multiple files
  - Undo and redo capability
  - File type detection
  - Line numbers and status bar
  - Quickly move by pages, words, and get to the top or bottom of a document instantly
  - Search forward and backward in a document
  - Replace text in a document
  - Efficient syntax highlighting
  - Compiles in under half a minute on most modern computers
*/

#![allow(unused_must_use)]

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, Event as CEvent, KeyCode as KCode, KeyModifiers as KMod},
    execute,
    style::{Color, SetBackgroundColor as Bg, SetForegroundColor as Fg},
    terminal::{self, Clear, ClearType as ClType, EnterAlternateScreen, LeaveAlternateScreen, EnableLineWrap, DisableLineWrap},
};
use jargon_args::Jargon;
use kaolinite::event::{Event, Result, Status};
use kaolinite::utils::{filetype, Loc, Size};
use kaolinite::Document;
use synoptic::{Highlighter, TokOpt, trim, from_extension};
use std::io::{stdout, Stdout, Write};

/// Store the version number at compile time
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Help text for the command line interface
const HELP: &str = "\
Cactus: A compact and complete kaolinite implementation

USAGE: cactus [options] [files]

OPTIONS:
    --help, -h    : Show this help message
    --version, -v : Show the version number 

EXAMPLES:
    cactus test.txt
    cactus test.txt test2.txt
    cactus /home/user/docs/test.txt
";

fn main() {
    // Reset teriminal in the event of a crash
    std::panic::set_hook(Box::new(|e| {
        terminal::disable_raw_mode().unwrap();
        execute!(stdout(), LeaveAlternateScreen, Show).unwrap();
        eprintln!("{}", e);
    }));
    // Run cactus
    if let Err(e) = run() {
        terminal::disable_raw_mode().unwrap();
        execute!(stdout(), LeaveAlternateScreen, Show).unwrap();
        eprintln!("{}", e);
    }
}

/// This will parse arguments, and run cactus, handling any errors that occur
fn run() -> Result<()> {
    let mut args = Jargon::from_env();
    if args.contains(["-h", "--help"]) {
        print!("{}", HELP);
    } else if args.contains(["-v", "--version"]) {
        println!("{}", VERSION);
    } else {
        let mut e = Editor::new()?;
        let mut error = false;
        // Try to open the requested files
        for file in args.finish() {
            if let Err(err) = e.open(file.clone()) {
                // If the file failed to open, make a note of it and display it
                println!("Couldn't open file \"{}\": {}", file, err);
                error = true;
            }
        }
        // If all files opened without error, run cactus
        if !error {
            e.run()?;
        }
    }
    Ok(())
}

/// Gets the size of the terminal
fn size() -> Result<Size> {
    let (w, h) = terminal::size()?;
    Ok(Size {
        w: w as usize,
        h: (h as usize).saturating_sub(1),
    })
}

/// For managing all editing and rendering of cactus
pub struct Editor {
    /// Interface for writing to the terminal
    stdout: Stdout,
    /// Storage of all the documents opened in the editor
    doc: Vec<Document>,
    /// Syntax highlighting integration
    highlighter: Highlighter,
    /// Pointer to the document that is currently being edited
    ptr: usize,
    /// true if the editor is still running, false otherwise
    active: bool,
}

impl Editor {
    /// Create a new instance of the editor
    pub fn new() -> Result<Self> {
        Ok(Self {
            doc: vec![],
            ptr: 0,
            stdout: stdout(),
            active: true,
            highlighter: Highlighter::new(4),
        })
    }

    /// Function to open a document into the editor
    pub fn open(&mut self, file_name: String) -> Result<()> {
        let size = size()?;
        let mut doc = Document::open(size, file_name.clone())?;
        // Load all the lines within viewport into the document
        doc.load_to(size.h);
        // Update in the syntax highlighter
        let ext = file_name.split('.').last().unwrap();
        self.highlighter = from_extension(ext, 4).unwrap_or(Highlighter::new(4));
        self.highlighter.run(&doc.lines);
        // Add document to documents
        self.doc.push(doc);
        Ok(())
    }

    /// Gets a reference to the current document
    pub fn doc(&self) -> &Document {
        self.doc.get(self.ptr).unwrap()
    }

    /// Gets a mutable reference to the current document
    pub fn doc_mut(&mut self) -> &mut Document {
        self.doc.get_mut(self.ptr).unwrap()
    }

    /// Set up the terminal so that it is clean and doesn't effect existing terminal text
    pub fn start(&mut self) -> Result<()> {
        execute!(self.stdout, EnterAlternateScreen, Clear(ClType::All), DisableLineWrap)?;
        terminal::enable_raw_mode()?;
        Ok(())
    }

    /// Restore terminal back to state before the editor was started
    pub fn end(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(self.stdout, LeaveAlternateScreen, EnableLineWrap)?;
        Ok(())
    }

    /// Execute an edit event
    pub fn exe(&mut self, ev: Event) -> Result<()> {
        self.doc_mut().exe(ev)
    }

    /// Initialise, render and handle events as they come in
    pub fn run(&mut self) -> Result<()> {
        // If no documents were provided, just exit
        self.active = !self.doc.is_empty();
        self.start()?;
        while self.active {
            self.render()?;
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
                    (KMod::CONTROL, KCode::Char('s')) => self.save(),
                    (KMod::ALT, KCode::Char('s')) => self.save_as()?,
                    (KMod::CONTROL, KCode::Char('a')) => self.save_all(),
                    (KMod::CONTROL, KCode::Char('q')) => self.quit(),
                    (KMod::SHIFT, KCode::Left) => self.prev(),
                    (KMod::SHIFT, KCode::Right) => self.next(),
                    // Undo & Redo
                    (KMod::CONTROL, KCode::Char('z')) => self.doc_mut().undo()?,
                    (KMod::CONTROL, KCode::Char('y')) => self.doc_mut().redo()?,
                    // Editing
                    (KMod::SHIFT | KMod::NONE, KCode::Char(ch)) => self.character(ch),
                    (KMod::NONE, KCode::Tab) => self.character('\t'),
                    (KMod::NONE, KCode::Backspace) => self.backspace(),
                    (KMod::NONE, KCode::Enter) => self.enter(),
                    (KMod::CONTROL, KCode::Char('d')) => self.delete_line(),
                    _ => (),
                },
                CEvent::Resize(w, h) => {
                    // Ensure all lines in viewport are loaded
                    let max = self.doc().len_lines().to_string().len() + 2;
                    self.doc_mut().size.w = w.saturating_sub(max as u16) as usize;
                    self.doc_mut().size.h = h.saturating_sub(1) as usize;
                    let max = self.doc().offset.x + self.doc().size.h;
                    self.doc_mut().load_to(max);
                }
                _ => (),
            }
            // Append any missed lines to the syntax highlighter
            let actual = self.doc.get(self.ptr).and_then(|d| Some(d.loaded_to)).unwrap_or(0);
            let percieved = self.highlighter.line_ref.len();
            if percieved < actual {
                let diff = actual - percieved;
                for i in 0..diff {
                    let line = &self.doc[self.ptr].lines[percieved + i];
                    self.highlighter.append(line);
                }
            }
        }
        self.end()?;
        Ok(())
    }

    /// Render a single frame of the editor in it's current state
    pub fn render(&mut self) -> Result<()> {
        execute!(self.stdout, Hide)?;
        let Size { w, h } = size()?;
        // Update the width of the document in case of update
        let max = self.doc().len_lines().to_string().len() + 2;
        self.doc_mut().size.w = w.saturating_sub(max) as usize;
        // Run through each line of the terminal, rendering the correct line
        self.render_document(w, h)?;
        // Leave last line for status line
        self.render_status_line(w, h)?;
        // Move cursor to the correct location and perform render
        let Loc { x, y } = self.doc().cursor;
        execute!(self.stdout, Show, MoveTo((x + max) as u16, y as u16))?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Render the lines of the document
    fn render_document(&mut self, w: usize, h: usize) -> Result<()> {
        for y in 0..(h as u16) {
            execute!(self.stdout, MoveTo(0, y))?;
            // Write line number of document
            let num = self.doc().line_number(y as usize + self.doc().offset.y);
            write!(
                self.stdout,
                "{}{} │{}{}",
                Fg(Color::Rgb { r: 150, g: 150, b: 150 }),
                num,
                Fg(Color::Reset),
                Clear(ClType::UntilNewLine),
            )?;
            // Render line if it exists
            let idx = y as usize + self.doc().offset.y;
            if let Some(line) = self.doc().line(idx) {
                let tokens = self.highlighter.line(idx, &line);
                let tokens = trim(&tokens, self.doc().offset.x);
                for token in tokens {
                    match token {
                        TokOpt::Some(text, kind) => write!(
                            self.stdout, 
                            "{}{text}{}", 
                            self.highlight_colour(&kind), 
                            Fg(Color::Reset)
                        ),
                        TokOpt::None(text) => write!(self.stdout, "{text}"),
                    }?
                }
            }
        }
        Ok(())
    }

    /// Render the status line at the bottom of the document
    fn render_status_line(&mut self, w: usize, h: usize) -> Result<()> {
        execute!(self.stdout, MoveTo(0, h as u16))?;
        let ext = self.doc().file_name.as_ref().unwrap().split('.').last().unwrap().to_string();
        // Form left hand side of status bar
        let lhs = format!(
            "{}{} │ {} │",
            self.doc().file_name.as_ref().unwrap().split('/').last().unwrap(),
            if self.doc().modified { "[+]" } else { "" },
            filetype(&ext).unwrap_or(ext)
        );
        // Form right hand side of status bar
        let rhs = format!(
            "│ {}/{} {} {}",
            self.doc().loc().y + 1,
            self.doc().len_lines(),
            self.doc().char_ptr,
            self.doc().loc().x,
        );
        // Use alinio to align left and right with padding between
        let status_line = alinio::align::between(&[&lhs, &rhs], w.saturating_sub(2))
            .unwrap_or_else(|| "".to_string());
        // Write the status bar
        write!(
            self.stdout,
            "{}{} {} {}{}",
            Fg(Color::Black),
            //Bg(Color::Rgb { r: 54, g: 161, b: 102 }),
            Bg(Color::Rgb { r: 91, g: 157, b: 72 }),
            status_line,
            Bg(Color::Reset),
            Fg(Color::Reset),
        )?;
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
            // Render prompt message
            execute!(self.stdout, MoveTo(0, h as u16), Clear(ClType::CurrentLine))?;
            write!(self.stdout, "{}: {}", prompt, input);
            self.stdout.flush()?;
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

    /// Find the appropriate syntax highlighting colour
    fn highlight_colour(&self, name: &str) -> String {
        match name {
            "string" => Fg(Color::Rgb { r: 54, g: 161, b: 102 }),
            "comment" => Fg(Color::Rgb { r: 108, g: 107, b: 90 }),
            "digit" => Fg(Color::Rgb { r: 157, g: 108, b: 124 }),
            "keyword" => Fg(Color::Rgb { r: 91, g: 157, b: 72 }),
            "attribute" => Fg(Color::Rgb { r: 95, g: 145, b: 130 }),
            "character" => Fg(Color::Rgb { r: 125, g: 151, b: 38 }),
            "type" => Fg(Color::Rgb { r: 165, g: 152, b: 13 }),
            "function" => Fg(Color::Rgb { r: 174, g: 115, b: 19 }),
            "header" => Fg(Color::Rgb { r: 174, g: 115, b: 19 }),
            "macro" => Fg(Color::Rgb { r: 157, g: 108, b: 124 }),
            "namespace" => Fg(Color::Rgb { r: 125, g: 151, b: 38 }),
            "struct" => Fg(Color::Rgb { r: 125, g: 151, b: 38 }),
            "operator" => Fg(Color::Rgb { r: 95, g: 145, b: 130 }),
            "boolean" => Fg(Color::Rgb { r: 54, g: 161, b: 102 }),
            "reference" => Fg(Color::Rgb { r: 91, g: 157, b: 72 }),
            "tag" => Fg(Color::Rgb { r: 95, g: 145, b: 130 }),
            "heading" => Fg(Color::Rgb { r: 174, g: 115, b: 19 }),
            "link" => Fg(Color::Rgb { r: 157, g: 108, b: 124 }),
            "key" => Fg(Color::Rgb { r: 157, g: 108, b: 124 }),
            _ => panic!("Invalid token name: {name}"),
        }.to_string()
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
    fn character(&mut self, ch: char) {
        self.new_row();
        let loc = self.doc().char_loc();
        self.exe(Event::Insert(loc, ch.to_string()));
        self.highlighter.edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
    }

    /// Handle the return key
    fn enter(&mut self) {
        if self.doc().loc().y != self.doc().len_lines() {
            // Enter pressed in the start, middle or end of the line
            let loc = self.doc().char_loc();
            self.exe(Event::SplitDown(loc));
            let line = &self.doc[self.ptr].lines[loc.y + 1];
            self.highlighter.insert_line(loc.y + 1, line);
            let line = &self.doc[self.ptr].lines[loc.y];
            self.highlighter.edit(loc.y, line);
        } else {
            // Enter pressed on the empty line at the bottom of the document
            self.new_row();
        }
    }

    /// Handle the backspace key
    fn backspace(&mut self) {
        let mut c = self.doc().char_ptr;
        let on_first_line = self.doc().loc().y == 0;
        let out_of_range = self.doc().out_of_range(0, self.doc().loc().y).is_err();
        if c == 0 && !on_first_line && !out_of_range {
            // Backspace was pressed on the start of the line, move line to the top
            self.new_row();
            let mut loc = self.doc().char_loc();
            self.highlighter.remove_line(loc.y);
            loc.y -= 1;
            loc.x = self.doc().line(loc.y).unwrap().chars().count();
            self.exe(Event::SpliceUp(loc));
            let line = &self.doc[self.ptr].lines[loc.y];
            self.highlighter.edit(loc.y, line);
        } else {
            // Backspace was pressed in the middle of the line, delete the character
            c -= 1;
            if let Some(line) = self.doc().line(self.doc().loc().y) {
                if let Some(ch) = line.chars().nth(c) {
                    let loc = Loc { x: c, y: self.doc().loc().y };
                    self.exe(Event::Delete(loc, ch.to_string()));
                    self.highlighter.edit(loc.y, &self.doc[self.ptr].lines[loc.y]);
                }
            }
        }
    }

    /// Insert a new row at the end of the document if the cursor is on it
    fn new_row(&mut self) {
        if self.doc().loc().y == self.doc().len_lines() {
            self.exe(Event::InsertLine(self.doc().loc().y, "".to_string()));
            self.highlighter.append(&"".to_string());
        }
    }

    /// Delete the current line
    fn delete_line(&mut self) {
        if self.doc().loc().y < self.doc().len_lines() {
            let y = self.doc().loc().y;
            let line = self.doc().line(y).unwrap();
            self.exe(Event::DeleteLine(y, line));
            self.highlighter.remove_line(y);
        }
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
            self.render_document(w, h)?;
            // Render custom status line with mode information
            execute!(self.stdout, MoveTo(0, h as u16), Clear(ClType::CurrentLine))?;
            write!(self.stdout, "[<-]: Search previous | [->]: Search next");
            self.stdout.flush()?;
            // Move back to correct cursor position
            let Loc { x, y } = self.doc().cursor;
            let max = self.doc().len_lines().to_string().len() + 2;
            execute!(self.stdout, MoveTo((x + max) as u16, y as u16))?;
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
            self.render_document(w, h)?;
            // Write custom status line for the replace mode
            execute!(self.stdout, MoveTo(0, h as u16), Clear(ClType::CurrentLine))?;
            write!(self.stdout, "[<-] Previous | [->] Next | [Enter] Replace | [Tab] Replace All");
            self.stdout.flush()?;
            // Move back to correct cursor location
            let Loc { x, y } = self.doc().cursor;
            let max = self.doc().len_lines().to_string().len() + 2;
            execute!(self.stdout, MoveTo((x + max) as u16, y as u16))?;
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
                    (KMod::NONE, KCode::Enter) => self.do_replace(&into, &mtch),
                    // On tab key, replace all instances within the document
                    (KMod::NONE, KCode::Tab) => self.do_replace_all(&target, &into),
                    _ => (),
                }
            }
        }
        Ok(())
    }

    /// Replace an instance in a document
    fn do_replace(&mut self, into: &str, text: &str) {
        let loc = self.doc().char_loc();
        self.doc_mut().replace(loc, text, into);
        self.doc_mut().goto(&loc);
    }

    /// Replace all instances in a document
    fn do_replace_all(&mut self, target: &str, into: &str) {
        self.doc_mut().replace_all(target, into);
    }

    /// save the document to the disk
    pub fn save(&mut self) {
        self.doc_mut().save();
    }

    /// save the document to the disk at a specified path
    pub fn save_as(&mut self) -> Result<()> {
        let file_name = self.prompt("Save as")?;
        self.doc_mut().save_as(&file_name)?;
        Ok(())
    }

    /// Save all the open documents to the disk
    pub fn save_all(&mut self) {
        for doc in self.doc.iter_mut() {
            doc.save();
        }
    }

    /// Quit the editor
    pub fn quit(&mut self) {
        self.active = !self.doc.is_empty();
        // If there are still documents open, only close the requested document
        if self.active {
            self.doc.remove(self.ptr);
            self.prev();
        }
        self.active = !self.doc.is_empty();
    }
}
