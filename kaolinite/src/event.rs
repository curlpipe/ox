/// event.rs - manages editing events and provides tools for error handling
use crate::utils::Loc;
use quick_error::quick_error;

/// Represents an editing event.
/// All possible editing events can be made up of a combination these events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Insert(Loc, String),
    Delete(Loc, String),
    InsertLine(usize, String),
    DeleteLine(usize, String),
    SplitDown(Loc),
    SpliceUp(Loc),
}

impl Event {
    /// Given an event, provide the opposite of that event (for purposes of undoing)
    #[must_use]
    pub fn reverse(self) -> Event {
        match self {
            Event::Insert(loc, ch) => Event::Delete(loc, ch),
            Event::Delete(loc, ch) => Event::Insert(loc, ch),
            Event::InsertLine(loc, st) => Event::DeleteLine(loc, st),
            Event::DeleteLine(loc, st) => Event::InsertLine(loc, st),
            Event::SplitDown(loc) => Event::SpliceUp(loc),
            Event::SpliceUp(loc) => Event::SplitDown(loc),
        }
    }

    /// Get the location of an event
    #[must_use]
    pub fn loc(self) -> Loc {
        match self {
            Event::Insert(loc, _) => loc,
            Event::Delete(loc, _) => loc,
            Event::InsertLine(loc, _) => Loc { x: 0, y: loc },
            Event::DeleteLine(loc, _) => Loc { x: 0, y: loc },
            Event::SplitDown(loc) => loc,
            Event::SpliceUp(loc) => loc,
        }
    }
}

/// Represents various statuses of functions
#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    StartOfFile,
    EndOfFile,
    StartOfLine,
    EndOfLine,
    None,
}

/// Easy result type for unified error handling
pub type Result<T> = std::result::Result<T, Error>;

quick_error! {
    /// Error enum for handling all possible errors
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) {
            from()
            display("I/O error: {}", err)
            source(err)
        }
        Rope(err: ropey::Error) {
            from()
            display("Rope error: {}", err)
            source(err)
        }
        NoFileName
        OutOfRange
        ReadOnlyFile
    }
}

/// For managing events for purposes of undo and redo
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct EventMgmt {
    /// The patch is the current sequence of editing actions
    pub patch: Vec<Event>,
    /// Undo contains all the patches that have been applied
    pub undo: Vec<Vec<Event>>,
    /// Redo contains all the patches that have been undone
    pub redo: Vec<Vec<Event>>,
}

impl EventMgmt {
    /// Register that an event has occurred with the event manager
    pub fn register(&mut self, ev: Event) {
        self.redo.clear();
        self.patch.push(ev);
    }

    /// This will commit the current patch to the undo stack, ready to be undone.
    /// You can call this after every space character, for example, which would
    /// make it so that every undo action would remove the previous word the user typed.
    pub fn commit(&mut self) {
        if !self.patch.is_empty() {
            let mut patch = vec![];
            std::mem::swap(&mut self.patch, &mut patch);
            self.undo.push(patch);
        }
    }

    /// Provide a list of actions to perform in order of when they should be applied for purposes
    /// of undoing (you'll need to reverse the events themselves manually)
    pub fn undo(&mut self) -> Option<Vec<Event>> {
        self.commit();
        let mut ev = self.undo.pop()?;
        self.redo.push(ev.clone());
        ev.reverse();
        Some(ev)
    }

    /// Provide a list of events to execute in order of when they should be applied for purposes of
    /// redoing
    pub fn redo(&mut self) -> Option<Vec<Event>> {
        self.commit();
        let ev = self.redo.pop()?;
        self.undo.push(ev.clone());
        Some(ev)
    }

    /// Returns true if the undo stack is empty, meaning no patches have been applied
    #[must_use]
    pub fn is_undo_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// Returns true if the redo stack is empty, meaning no patches have been undone
    #[must_use]
    pub fn is_redo_empty(&self) -> bool {
        self.redo.is_empty()
    }

    /// Returns true if the current patch is empty, meaning no edits have been done since the last
    /// commit
    #[must_use]
    pub fn is_patch_empty(&self) -> bool {
        self.patch.is_empty()
    }

    /// Get the last event that was committed
    #[must_use]
    pub fn last(&self) -> Option<&Event> {
        if self.patch.is_empty() {
            self.undo.last().and_then(|u| u.last())
        } else {
            self.patch.last()
        }
    }
}
