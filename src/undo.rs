// Undo.rs - Utilities for undoing, redoing and storing events
use crate::{Position, Row};

// Event enum to store the types of events that occur
#[derive(Debug, Clone)]
pub enum Event {
    InsertTab(Position),                   // Insert Tab
    InsertMid(Position, char),             // Insert character
    BackspaceStart(Position),              // Delete from start
    BackspaceMid(Position, char),          // Delete from middle
    ReturnStart(Position),                 // Return key in the middle of line
    ReturnMid(Position, usize),            // Return from middle of the line
    ReturnEnd(Position),                   // Return on the end of line
    UpdateLine(usize, Box<Row>, Box<Row>), // For holding entire line updates
    MoveCursor(i128, i128),                // For moving the cursor
    GotoCursor(Position),                  // For setting the cursor position
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
    pub fn append(&mut self, patch: Vec<Event>) {
        self.history.push(patch);
    }
    pub fn pop(&mut self) -> Option<Vec<Event>> {
        // Take a patch off the event stack
        self.history.pop()
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
}
