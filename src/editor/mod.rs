use crate::config::{Config, Indentation};
use crate::error::{OxError, Result};
use crate::ui::{size, Feedback, Terminal};
use crossterm::event::{
    Event as CEvent, KeyCode as KCode, KeyModifiers as KMod, MouseEvent, MouseEventKind,
};
use kaolinite::event::Error as KError;
use kaolinite::Document;
use kaolinite::utils::get_absolute_path;
use mlua::{Error as LuaError, Lua};
use std::io::ErrorKind;
use std::time::Instant;
use std::env;
use std::path::Path;
use synoptic::Highlighter;

mod cursor;
mod documents;
mod editing;
mod filetypes;
mod interface;
mod mouse;
mod scanning;

pub use documents::FileContainer;
pub use filetypes::{FileType, FileTypes};

/// For managing all editing and rendering of cactus
#[allow(clippy::struct_excessive_bools)]
pub struct Editor {
    /// Interface for writing to the terminal
    pub terminal: Terminal,
    /// Whether to rerender the editor on the next cycle
    pub needs_rerender: bool,
    /// Configuration information for the editor
    pub config: Config,
    /// Storage of all the documents opened in the editor
    pub files: Vec<FileContainer>,
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
    pub push_down: usize,
    /// Used to cache the location of the configuration file
    pub config_path: String,
    /// Flag to determine whether or not the editor is under control by a plug-in
    pub plugin_active: bool,
    /// Stores the last click the user made (in order to detect double-click)
    pub last_click: Option<(Instant, MouseEvent)>,
}

impl Editor {
    /// Create a new instance of the editor
    pub fn new(lua: &Lua) -> Result<Self> {
        let config = Config::new(lua)?;
        Ok(Self {
            files: vec![],
            ptr: 0,
            terminal: Terminal::new(config.terminal.clone()),
            config,
            active: true,
            greet: false,
            needs_rerender: true,
            feedback: Feedback::None,
            command: None,
            last_active: Instant::now(),
            push_down: 1,
            config_path: "~/.oxrc".to_string(),
            plugin_active: false,
            last_click: None,
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
        // Mark as not saved on disk
        doc.info.modified = true;
        // Add document to documents
        self.files.push(FileContainer {
            highlighter,
            file_type: Some(FileType::default()),
            doc,
        });
        Ok(())
    }

    /// Create a new document and move to it
    pub fn new_document(&mut self) -> Result<()> {
        self.blank()?;
        self.ptr = self.files.len().saturating_sub(1);
        Ok(())
    }

    /// Create a blank document if none are already opened
    pub fn new_if_empty(&mut self) -> Result<()> {
        // If no documents were provided, create a new empty document
        if self.files.is_empty() {
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
        // Collect various data from the document
        let tab_width = self.config.document.borrow().tab_width;
        let file_type = self.config.document.borrow().file_types.identify(&mut doc);
        // Set up the document
        doc.set_tab_width(tab_width);
        doc.load_to(size.h);
        doc.undo_mgmt.saved();
        // Update in the syntax highlighter
        let mut highlighter = file_type.as_ref().map_or(Highlighter::new(tab_width), |t| {
            t.get_highlighter(&self.config, tab_width)
        });
        highlighter.run(&doc.lines);
        // Add in the file
        self.files.push(FileContainer {
            doc,
            highlighter,
            file_type,
        });
        Ok(())
    }

    /// Function to ask the user for a file to open
    pub fn open_document(&mut self) -> Result<()> {
        let path = self.path_prompt()?;
        self.open(&path)?;
        self.ptr = self.files.len().saturating_sub(1);
        self.update_cwd();
        Ok(())
    }

    /// Function to try opening a document, and if it doesn't exist, create it
    pub fn open_or_new(&mut self, file_name: String) -> Result<()> {
        let file = self.open(&file_name);
        if let Err(OxError::Kaolinite(KError::Io(ref os))) = file {
            if os.kind() == ErrorKind::NotFound {
                // Create a new document if not found
                self.blank()?;
                let file = self.files.last_mut().unwrap();
                file.doc.file_name = Some(file_name);
                // Work out information for the document
                let tab_width = self.config.document.borrow().tab_width;
                let file_type = self
                    .config
                    .document
                    .borrow()
                    .file_types
                    .identify(&mut file.doc);
                // Set up the document
                file.doc.info.modified = true;
                file.doc.set_tab_width(tab_width);
                // Attach the correct highlighter
                let highlighter = file_type.clone().map_or(Highlighter::new(tab_width), |t| {
                    t.get_highlighter(&self.config, tab_width)
                });
                file.highlighter = highlighter;
                file.highlighter.run(&file.doc.lines);
                file.file_type = file_type;
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
            // Get information about the document
            let file = self.files.last_mut().unwrap();
            let tab_width = self.config.document.borrow().tab_width;
            let file_type = self
                .config
                .document
                .borrow()
                .file_types
                .identify(&mut file.doc);
            // Reattach an appropriate highlighter
            let highlighter = file_type.map_or(Highlighter::new(tab_width), |t| {
                t.get_highlighter(&self.config, tab_width)
            });
            file.highlighter = highlighter;
            file.highlighter.run(&file.doc.lines);
            file.doc.file_name = Some(file_name.clone());
            file.doc.info.modified = false;
        }
        // Commit events to event manager (for undo / redo)
        self.doc_mut().commit();
        // All done
        self.feedback = Feedback::Info(format!("Document saved as {file_name} successfully"));
        Ok(())
    }

    /// Save all the open documents to the disk
    pub fn save_all(&mut self) -> Result<()> {
        for file in &mut self.files {
            file.doc.save()?;
            // Commit events to event manager (for undo / redo)
            file.doc.commit();
        }
        self.feedback = Feedback::Info("Saved all documents".to_string());
        Ok(())
    }

    /// Quit the editor
    pub fn quit(&mut self) -> Result<()> {
        self.active = !self.files.is_empty();
        // If there are still documents open, only close the requested document
        if self.active {
            let msg = "This document isn't saved, press Ctrl + Q to force quit or Esc to cancel";
            if !self.doc().info.modified || self.confirm(msg)? {
                self.files.remove(self.ptr);
                self.prev();
            }
        }
        self.active = !self.files.is_empty();
        Ok(())
    }

    /// Move to the next document opened in the editor
    pub fn next(&mut self) {
        if self.ptr + 1 < self.files.len() {
            self.ptr += 1;
            self.update_cwd();
        }
    }

    /// Move to the previous document opened in the editor
    pub fn prev(&mut self) {
        if self.ptr != 0 {
            self.ptr = self.ptr.saturating_sub(1);
            self.update_cwd();
        }
    }

    /// Updates the current working directory of the editor
    pub fn update_cwd(&self) {
        if let Some(name) = get_absolute_path(&self.doc().file_name.clone().unwrap_or_default()) {
            let file = Path::new(&name);
            if let Some(cwd) = file.parent() {
                let _ = env::set_current_dir(cwd);
            }
        }
    }

    /// Returns a document at a certain index
    pub fn get_doc(&mut self, idx: usize) -> &mut Document {
        &mut self.files.get_mut(idx).unwrap().doc
    }

    /// Gets a reference to the current document
    pub fn doc(&self) -> &Document {
        &self.files.get(self.ptr).unwrap().doc
    }

    /// Gets a mutable reference to the current document
    pub fn doc_mut(&mut self) -> &mut Document {
        &mut self.files.get_mut(self.ptr).unwrap().doc
    }

    /// Gets the number of documents currently open
    pub fn doc_len(&mut self) -> usize {
        self.files.len()
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
    pub fn handle_event(&mut self, lua: &Lua, event: CEvent) -> Result<()> {
        self.needs_rerender = match event {
            CEvent::Mouse(event) => event.kind != MouseEventKind::Moved,
            _ => true,
        };
        match event {
            CEvent::Key(key) => self.handle_key_event(key.modifiers, key.code)?,
            CEvent::Resize(w, h) => self.handle_resize(w, h),
            CEvent::Mouse(mouse_event) => self.handle_mouse_event(lua, mouse_event),
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
            (KMod::NONE, KCode::Tab) => self.handle_tab()?,
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

    /// Handle tab character being inserted
    pub fn handle_tab(&mut self) -> Result<()> {
        if self.config.document.borrow().indentation == Indentation::Tabs {
            self.character('\t')?;
        } else {
            let tab_width = self.config.document.borrow().tab_width;
            for _ in 0..tab_width {
                self.character(' ')?;
            }
        }
        Ok(())
    }
}
