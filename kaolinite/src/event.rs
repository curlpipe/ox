/// event.rs - manages editing events and provides tools for error handling
use crate::{document::Cursor, utils::Loc, Document};
use error_set::error_set;
use ropey::Rope;

/// A snapshot stores the state of a document at a certain time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub content: Rope,
    pub cursor: Cursor,
}

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
    pub fn loc(&self) -> Loc {
        match self {
            Event::Insert(loc, _)
            | Event::Delete(loc, _)
            | Event::SplitDown(loc)
            | Event::SpliceUp(loc) => *loc,
            Event::InsertLine(loc, _) | Event::DeleteLine(loc, _) => Loc { x: 0, y: *loc },
        }
    }

    /// Work out if the event is of the same type
    #[must_use]
    pub fn same_type(&self, ev: &Self) -> bool {
        matches!(
            (self, ev),
            (&Event::Insert(_, _), &Event::Insert(_, _))
                | (&Event::Delete(_, _), &Event::Delete(_, _))
                | (&Event::InsertLine(_, _), &Event::InsertLine(_, _))
                | (&Event::DeleteLine(_, _), &Event::DeleteLine(_, _))
                | (&Event::SplitDown(_), &Event::SplitDown(_))
                | (&Event::SpliceUp(_), &Event::SpliceUp(_))
        )
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

error_set! {
    /// Error enum for handling all possible errors
    Error = {
        #[display("I/O error: {0}")]
        Io(std::io::Error),
        #[display("Rope error: {0}")]
        Rope(ropey::Error),
        NoFileName,
        OutOfRange,
        ReadOnlyFile
    };
}

/// For managing events for purposes of undo and redo
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct EventMgmt {
    /// Contains all the snapshots in the current timeline
    pub history: Vec<Snapshot>,
    /// Stores where the document currently is
    pub ptr: Option<usize>,
    /// Store where the file on the disk is currently at
    pub on_disk: Option<usize>,
    /// Store the last event to occur (so that we can see if there is a change)
    pub last_event: Option<Event>,
    /// Flag to force the file not to be with disk (i.e. file only exists in memory)
    pub force_not_with_disk: bool,
}

impl Document {
    #[must_use]
    pub fn take_snapshot(&self) -> Snapshot {
        Snapshot {
            content: self.file.clone(),
            cursor: self.cursor,
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: Snapshot) {
        self.file = snapshot.content;
        self.cursor = snapshot.cursor;
        self.char_ptr = self.character_idx(&snapshot.cursor.loc);
        self.reload_lines();
        self.bring_cursor_in_viewport();
    }
}

impl EventMgmt {
    /// In the event of some changes, redo should be cleared
    pub fn clear_redo(&mut self) {
        if let Some(ptr) = self.ptr {
            self.history.drain(ptr + 1..);
        }
    }

    /// To be called when a snapshot needs to be registered
    pub fn commit(&mut self, snapshot: Snapshot) {
        // Only commit when previous snapshot differs
        let ptr = self.ptr.unwrap_or(0);
        if self.history.get(ptr).map(|s| &s.content) != Some(&snapshot.content) {
            self.clear_redo();
            self.history.push(snapshot);
            self.ptr = Some(self.history.len().saturating_sub(1));
        }
    }

    /// To be called when writing to disk
    pub fn disk_write(&mut self, snapshot: &Snapshot) {
        self.force_not_with_disk = false;
        self.commit(snapshot.clone());
        self.on_disk = self.ptr;
    }

    /// A way to query whether we're currently up to date with the disk
    #[must_use]
    pub fn with_disk(&self, snapshot: &Snapshot) -> bool {
        if self.force_not_with_disk {
            false
        } else if let Some(disk) = self.on_disk {
            self.history.get(disk).map(|s| &s.content) == Some(&snapshot.content)
        } else if self.history.is_empty() {
            true
        } else {
            self.history.first().map(|s| &s.content) == Some(&snapshot.content)
        }
    }

    /// Get previous snapshot to restore to
    pub fn undo(&mut self, snapshot: Snapshot) -> Option<Snapshot> {
        // Push cursor back by 1
        self.commit(snapshot);
        if let Some(ptr) = self.ptr {
            if ptr != 0 {
                let new_ptr = ptr.saturating_sub(1);
                self.ptr = Some(new_ptr);
                self.history.get(new_ptr).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get snapshot that used to be in place
    pub fn redo(&mut self, snapshot: &Snapshot) -> Option<Snapshot> {
        if let Some(ptr) = self.ptr {
            // If the user has edited since the undo, wipe the redo stack
            if self.history.get(ptr).map(|s| &s.content) != Some(&snapshot.content) {
                self.clear_redo();
            }
            // Perform the redo
            let new_ptr = if ptr + 1 < self.history.len() {
                ptr + 1
            } else {
                return None;
            };
            self.ptr = Some(new_ptr);
            self.history.get(new_ptr).cloned()
        } else {
            None
        }
    }
}
