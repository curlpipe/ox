use crate::config::Config;
use crate::error::{OxError, Result};
use crate::ui::{size, Feedback, Terminal};
use crossterm::event::{Event as CEvent, KeyCode as KCode, KeyModifiers as KMod, MouseEventKind};
use kaolinite::event::Error as KError;
use kaolinite::Document;
use mlua::{Error as LuaError, Lua};
use std::io::ErrorKind;
use std::time::Instant;
use synoptic::Highlighter;

mod cursor;
mod editing;
mod interface;
mod mouse;
mod scanning;

/// For managing all editing and rendering of cactus
pub struct Editor {
    /// Interface for writing to the terminal
    pub terminal: Terminal,
    /// Whether to rerender the editor on the next cycle
    pub needs_rerender: bool,
    /// Configuration information for the editor
    pub config: Config,
    /// Storage of all the documents opened in the editor
    pub doc: Vec<Document>,
    /// Syntax highlighting integration
    pub highlighter: Vec<Highlighter>,
    /// Pointer to the document that is currently being edited
    pub ptr: usize,
    /// true if the editor is still running, false otherwise
    pub active: bool,
    /// true if the editor should show a greeting message on next render
    pub greet: bool,
    /// The feedback message to display below the status line
    pub feedback: Feedback,
    /// Will be some if there is an outstanding command to be run
    pub command: Option<String>,
    /// Will store the last time the editor was interacted with (to track inactivity)
    pub last_active: Instant,
    /// Used for storing amount to push document down
    push_down: usize,
    /// Used to cache the location of the configuration file
    pub config_path: String,
}

impl Editor {
    /// Create a new instance of the editor
    pub fn new(lua: &Lua) -> Result<Self> {
        let config = Config::new(lua)?;
        Ok(Self {
            doc: vec![],
            ptr: 0,
            terminal: Terminal::new(config.terminal.clone()),
            config,
            active: true,
            greet: false,
            needs_rerender: true,
            highlighter: vec![],
            feedback: Feedback::None,
            command: None,
            last_active: Instant::now(),
            push_down: 1,
            config_path: "~/.oxrc".to_string(),
        })
    }

    /// Initialise the editor
    pub fn init(&mut self) -> Result<()> {
        self.terminal.start()?;
        Ok(())
    }

    /// Function to create a new document (without moving to it)
    pub fn blank(&mut self) -> Result<()> {
        let mut size = size()?;
        size.h = size.h.saturating_sub(1 + self.push_down);
        let mut doc = Document::new(size);
        doc.set_tab_width(self.config.document.borrow().tab_width);
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

    /// Create a blank document if none are already opened
    pub fn new_if_empty(&mut self) -> Result<()> {
        // If no documents were provided, create a new empty document
        if self.doc.is_empty() {
            self.blank()?;
            self.greet = self.config.greeting_message.borrow().enabled;
        }
        Ok(())
    }

    /// Function to open a document into the editor
    pub fn open(&mut self, file_name: &str) -> Result<()> {
        let mut size = size()?;
        size.h = size.h.saturating_sub(1 + self.push_down);
        let mut doc = Document::open(size, file_name)?;
        doc.set_tab_width(self.config.document.borrow().tab_width);
        // Load all the lines within viewport into the document
        doc.load_to(size.h);
        // Update in the syntax highlighter
        let mut ext = file_name.split('.').last().unwrap_or("");
        if ext == "oxrc" {
            ext = "lua";
        }
        let mut highlighter = self
            .config
            .syntax_highlighting
            .borrow()
            .get_highlighter(ext);
        highlighter.run(&doc.lines);
        self.highlighter.push(highlighter);
        doc.undo_mgmt.saved();
        // Add document to documents
        self.doc.push(doc);
        Ok(())
    }

    /// Function to ask the user for a file to open
    pub fn open_document(&mut self) -> Result<()> {
        let path = self.prompt("File to open")?;
        self.open(&path)?;
        self.ptr = self.doc.len().saturating_sub(1);
        Ok(())
    }

    /// Function to try opening a document, and if it doesn't exist, create it
    pub fn open_or_new(&mut self, file_name: String) -> Result<()> {
        let file = self.open(&file_name);
        if let Err(OxError::Kaolinite(KError::Io(ref os))) = file {
            if os.kind() == ErrorKind::NotFound {
                self.blank()?;
                let binding = file_name.clone();
                let ext = binding.split('.').last().unwrap_or("");
                self.doc.last_mut().unwrap().file_name = Some(file_name);
                self.doc.last_mut().unwrap().info.modified = true;
                let highlighter = self
                    .config
                    .syntax_highlighting
                    .borrow()
                    .get_highlighter(ext);
                *self.highlighter.last_mut().unwrap() = highlighter;
                self.highlighter
                    .last_mut()
                    .unwrap()
                    .run(&self.doc.last_mut().unwrap().lines);
                Ok(())
            } else {
                file
            }
        } else {
            file
        }
    }

    /// save the document to the disk
    pub fn save(&mut self) -> Result<()> {
        // Commit events to event manager (for undo / redo)
        self.doc_mut().commit();
        // Perform the save
        self.doc_mut().save()?;
        // All done
        self.feedback = Feedback::Info("Document saved successfully".to_string());
        Ok(())
    }

    /// save the document to the disk at a specified path
    pub fn save_as(&mut self) -> Result<()> {
        let file_name = self.prompt("Save as")?;
        self.doc_mut().save_as(&file_name)?;
        if self.doc().file_name.is_none() {
            let ext = file_name.split('.').last().unwrap_or("");
            self.highlighter[self.ptr] = self
                .config
                .syntax_highlighting
                .borrow()
                .get_highlighter(ext);
            self.doc_mut().file_name = Some(file_name.clone());
            self.doc_mut().info.modified = false;
        }
        // Commit events to event manager (for undo / redo)
        self.doc_mut().commit();
        // All done
        self.feedback = Feedback::Info(format!("Document saved as {file_name} successfully"));
        Ok(())
    }

    /// Save all the open documents to the disk
    pub fn save_all(&mut self) -> Result<()> {
        for doc in &mut self.doc {
            doc.save()?;
            // Commit events to event manager (for undo / redo)
            doc.commit();
        }
        self.feedback = Feedback::Info("Saved all documents".to_string());
        Ok(())
    }

    /// Quit the editor
    pub fn quit(&mut self) -> Result<()> {
        self.active = !self.doc.is_empty();
        // If there are still documents open, only close the requested document
        if self.active {
            let msg = "This document isn't saved, press Ctrl + Q to force quit or Esc to cancel";
            if !self.doc().info.modified || self.confirm(msg)? {
                self.doc.remove(self.ptr);
                self.highlighter.remove(self.ptr);
                self.prev();
            }
        }
        self.active = !self.doc.is_empty();
        Ok(())
    }

    /// Move to the next document opened in the editor
    pub fn next(&mut self) {
        if self.ptr + 1 < self.doc.len() {
            self.ptr += 1;
        }
    }

    /// Move to the previous document opened in the editor
    pub fn prev(&mut self) {
        if self.ptr != 0 {
            self.ptr = self.ptr.saturating_sub(1);
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

    /// Load the configuration values
    pub fn load_config(&mut self, path: &str, lua: &Lua) -> Option<LuaError> {
        self.config_path = path.to_string();
        let result = self.config.read(path, lua);
        // Display any warnings if the user configuration couldn't be found
        match result {
            Ok(()) => (),
            Err(OxError::Config(msg)) => {
                if msg == "Not Found" {
                    let warn =
                        "No configuration file found, using default configuration".to_string();
                    self.feedback = Feedback::Warning(warn);
                }
            }
            Err(OxError::Lua(err)) => return Some(err),
            _ => unreachable!(),
        }
        // Calculate the correct push down based on config
        self.push_down = usize::from(self.config.tab_line.borrow().enabled);
        None
    }

    /// Handle event
    pub fn handle_event(&mut self, event: CEvent) -> Result<()> {
        self.needs_rerender = match event {
            CEvent::Mouse(event) => event.kind != MouseEventKind::Moved,
            _ => true,
        };
        match event {
            CEvent::Key(key) => self.handle_key_event(key.modifiers, key.code)?,
            CEvent::Resize(w, h) => self.handle_resize(w, h),
            CEvent::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
            CEvent::Paste(text) => self.handle_paste(&text)?,
            _ => (),
        }
        Ok(())
    }

    /// Handle key event
    pub fn handle_key_event(&mut self, modifiers: KMod, code: KCode) -> Result<()> {
        // Check period of inactivity
        let end = Instant::now();
        let inactivity = end.duration_since(self.last_active).as_millis() as usize;
        // Commit if over user-defined period of inactivity
        if inactivity > self.config.document.borrow().undo_period * 1000 {
            self.doc_mut().commit();
        }
        // Register this activity
        self.last_active = Instant::now();
        // Editing - these key bindings can't be modified (only added to)!
        match (modifiers, code) {
            // Core key bindings (non-configurable behaviour)
            (KMod::SHIFT | KMod::NONE, KCode::Char(ch)) => self.character(ch)?,
            (KMod::NONE, KCode::Tab) => self.character('\t')?,
            (KMod::NONE, KCode::Backspace) => self.backspace()?,
            (KMod::NONE, KCode::Delete) => self.delete()?,
            (KMod::NONE, KCode::Enter) => self.enter()?,
            _ => (),
        }
        Ok(())
    }

    /// Handle resize
    pub fn handle_resize(&mut self, w: u16, h: u16) {
        // Ensure all lines in viewport are loaded
        let max = self.dent();
        self.doc_mut().size.w = w.saturating_sub(u16::try_from(max).unwrap_or(u16::MAX)) as usize;
        self.doc_mut().size.h = h.saturating_sub(3) as usize;
        let max = self.doc().offset.x + self.doc().size.h;
        self.doc_mut().load_to(max + 1);
    }

    /// Handle paste
    pub fn handle_paste(&mut self, text: &str) -> Result<()> {
        for ch in text.chars() {
            self.character(ch)?;
        }
        Ok(())
    }
}
