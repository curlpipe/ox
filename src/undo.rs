// Undo.rs - Utilities for undoing, redoing and storing events
use crate::Position;

// Event enum to store the types of events that occur
#[derive(Clone, Copy, Debug)]
pub enum Event {
    InsertTab(Position),          // Insert Tab
    InsertMid(Position, char),    // Insert character
    BackspaceStart(Position),     // Delete from start
    BackspaceMid(Position, char), // Delete from middle
    ReturnStart(Position),        // Return key in the middle of line
    ReturnMid(Position, usize),   // Return from middle of the line
    ReturnEnd(Position),          // Return on the end of line
}

// A struct for holding all the events taken by the user
#[derive(Debug)]
pub struct EventStack {
    history: Vec<Event>, // For storing the history of events
}

// Methods for the EventStack
impl EventStack {
    pub fn new() -> Self {
        // Initialise an Event stack
        Self { history: vec![] }
    }
    pub fn push(&mut self, event: Event) {
        // Add an event to the event stack
        self.history.push(event);
    }
    pub fn pop(&mut self) -> Option<Event> {
        // Take an event off the event stack
        self.history.pop()
    }
    pub fn empty(&mut self) {
        self.history.clear();
    }
}
