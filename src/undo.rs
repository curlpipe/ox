// Undo.rs - Utilities for undoing, redoing and storing events
use crate::util::line_offset;
use crate::{Direction, Position, Row, Variable};

// Enum for the the types of banks
#[derive(Debug, Clone)]
pub enum BankType {
    Line,   // For holding lines from the document
    Cursor, // For holding cursor positions
}

// Event enum to store the types of events that occur
#[derive(Debug, Clone)]
pub enum Event {
    Store(BankType, usize),                         // Store an item in a bank
    Load(BankType, usize),                          // Load an item from a bank
    SpliceUp(Position, Position),                   // Delete from start
    SplitDown(Position, Position),                  // Return from middle of the line
    InsertLineAbove(Position),                      // Return key in the middle of line
    InsertLineBelow(Position),                      // Return on the end of line
    Deletion(Position, char),                       // Delete from middle
    Insertion(Position, char),                      // Insert character
    InsertTab(Position),                            // Insert a tab character
    DeleteTab(Position),                            // Delete a tab character
    DeleteLine(Position, i128, Box<Row>),           // For deleting a line
    UpdateLine(Position, i128, Box<Row>, Box<Row>), // For holding entire line updates
    MoveCursor(i128, Direction),                    // For moving the cursor
    GotoCursor(Position),                           // For setting the cursor position
    MoveWord(Direction),                            // Move cursor through words
    DeleteWord(Position, String),                   // Delete word
    Theme(String),                                  // Theme change event
    Search,                                         // Search the document
    Replace,                                        // Replace certain occurances
    ReplaceAll,                                     // Replace everything
    Cmd,                                            // Trigger command mode
    Home,                                           // Moving cursor to the start of line
    End,                                            // Moving cursor to the end of line
    PageUp,                                         // Moving cursor one page up
    PageDown,                                       // Moving cursor one page down
    Overwrite(Vec<Row>, Vec<Row>),                  // Overwrite document
    New,                                            // New document
    Open(Option<String>),                           // Open document
    Save(Option<String>, bool),                     // Save document
    SaveAll,                                        // Save all documents
    Undo,                                           // Undo event
    Redo,                                           // Redo event
    Commit,                                         // Commit undo event
    Quit(bool),                                     // Quit document
    QuitAll(bool),                                  // Quit all
    NextTab,                                        // Next tab
    PrevTab,                                        // Previous tab
    ReloadConfig,                                   // Reload the configuration file
    Shell(String, bool, bool, bool),                // Running a shell command
    Set(Variable, bool),                            // For updating variables of the document
}

// A struct for holding all the events taken by the user
#[derive(Debug)]
pub struct EventStack {
    history: Vec<Vec<Event>>,  // For storing the history of events
    current_patch: Vec<Event>, // For storing the current group
}

// Methods for the EventStack
impl EventStack {
    pub fn new() -> Self {
        // Initialise an Event stack
        Self {
            history: vec![],
            current_patch: vec![],
        }
    }
    pub fn push(&mut self, event: Event) {
        // Add an event to the event stack
        self.current_patch.insert(0, event);
    }
    pub fn pop(&mut self) -> Option<Vec<Event>> {
        // Take a patch off the event stack
        self.history.pop()
    }
    pub fn append(&mut self, patch: Vec<Event>) {
        // Append a patch to the stack
        self.history.push(patch);
    }
    pub fn empty(&mut self) {
        // Empty the stack
        self.history.clear();
    }
    pub fn commit(&mut self) {
        // Commit patch to history
        if !self.current_patch.is_empty() {
            self.history.push(self.current_patch.clone());
            self.current_patch.clear();
        }
    }
    pub fn len(&self) -> usize {
        // Find the length of the undo stack
        self.history.len()
    }
}

pub fn reverse(before: Event, limit: usize) -> Option<Vec<Event>> {
    // Turn an event into the opposite of itself
    // Used for undo
    Some(match before {
        Event::SpliceUp(before, after) => vec![Event::SplitDown(after, before)],
        Event::SplitDown(before, after) => vec![Event::SpliceUp(after, before)],
        Event::InsertLineAbove(pos) => vec![Event::DeleteLine(pos, 0, Box::new(Row::from("")))],
        Event::InsertLineBelow(pos) => vec![Event::DeleteLine(pos, 1, Box::new(Row::from("")))],
        Event::Deletion(pos, ch) => vec![Event::Insertion(pos, ch)],
        Event::Insertion(pos, ch) => vec![Event::Deletion(
            Position {
                x: pos.x.saturating_add(1),
                y: pos.y,
            },
            ch,
        )],
        Event::DeleteLine(pos, offset, before) => vec![
            Event::InsertLineAbove(Position {
                x: pos.x,
                y: line_offset(pos.y, offset, limit),
            }),
            Event::UpdateLine(pos, offset, Box::new(Row::from("")), before),
        ],
        Event::UpdateLine(pos, offset, before, after) => {
            vec![Event::UpdateLine(pos, offset, after, before)]
        }
        Event::Overwrite(before, after) => vec![Event::Overwrite(after, before)],
        Event::InsertTab(pos) => vec![Event::DeleteTab(pos)],
        Event::DeleteTab(pos) => vec![Event::InsertTab(pos)],
        _ => return None,
    })
}
