/// Tools for recording and playing back macros for bulk editing
use crossterm::event::{Event as CEvent, KeyCode, KeyEvent, KeyModifiers};

/// Macro manager struct
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MacroMan {
    pub sequence: Vec<CEvent>,
    pub recording: bool,
    pub playing: bool,
    pub ptr: usize,
    pub just_completed: bool,
    pub reps: usize,
}

impl MacroMan {
    /// Register an event
    pub fn register(&mut self, ev: CEvent) {
        self.just_completed = false;
        let valid_event = matches!(ev, CEvent::Key(_) | CEvent::Mouse(_) | CEvent::Paste(_));
        if self.recording && valid_event {
            self.sequence.push(ev);
        }
    }

    /// Activate recording
    pub fn record(&mut self) {
        self.just_completed = false;
        self.sequence.clear();
        self.recording = true;
    }

    /// Stop recording
    pub fn finish(&mut self) {
        self.just_completed = false;
        self.recording = false;
        self.remove_macro_calls();
    }

    /// Activate macro
    pub fn play(&mut self, reps: usize) {
        self.reps = reps;
        self.just_completed = false;
        self.playing = true;
        self.ptr = 0;
    }

    /// Get next event from macro man
    pub fn next(&mut self) -> Option<CEvent> {
        if self.playing {
            let result = self.sequence.get(self.ptr).cloned();
            self.ptr += 1;
            if self.ptr >= self.sequence.len() {
                self.reps = self.reps.saturating_sub(1);
                self.playing = self.reps != 0;
                self.ptr = 0;
                self.just_completed = true;
            }
            result
        } else {
            self.just_completed = false;
            None
        }
    }

    /// Remove the stop key binding from being included
    pub fn remove_macro_calls(&mut self) {
        if let Some(CEvent::Key(KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            code: KeyCode::Esc,
            ..
        })) = self.sequence.last()
        {
            self.sequence.pop();
        }
    }
}
