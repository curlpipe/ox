/// For dealing with keys in the configuration file
use crossterm::event::{KeyCode as KCode, KeyModifiers as KMod, MediaKeyCode, ModifierKeyCode};

/// This contains the code for running code after a key binding is pressed
pub fn run_key(key: &str) -> String {
    format!(
        "
        globalevent = (global_event_mapping[\"*\"] or {{}})
        for _, f in ipairs(globalevent) do
            f()
        end
        key = (global_event_mapping[\"{key}\"] or error(\"key not bound\"))
        for _, f in ipairs(key) do
            f()
        end
        "
    )
}

/// This contains the code for running code before a key binding is fully processed
pub fn run_key_before(key: &str) -> String {
    format!(
        "
        globalevent = (global_event_mapping[\"before:*\"] or {{}})
        for _, f in ipairs(globalevent) do
            f()
        end
        key = (global_event_mapping[\"before:{key}\"] or {{}})
        for _, f in ipairs(key) do
            f()
        end
        "
    )
}

/// Converts a key taken from a crossterm event into string format
pub fn key_to_string(modifiers: KMod, key: KCode) -> String {
    let mut result = String::new();
    // Deal with modifiers
    if modifiers.contains(KMod::CONTROL) {
        result += "ctrl_";
    }
    if modifiers.contains(KMod::ALT) {
        result += "alt_";
    }
    if modifiers.contains(KMod::SHIFT) {
        result += "shift_";
    }
    result += &match key {
        KCode::Char('\\') => "\\\\".to_string(),
        KCode::Char('"') => "\\\"".to_string(),
        KCode::Backspace => "backspace".to_string(),
        KCode::Enter => "enter".to_string(),
        KCode::Left => "left".to_string(),
        KCode::Right => "right".to_string(),
        KCode::Up => "up".to_string(),
        KCode::Down => "down".to_string(),
        KCode::Home => "home".to_string(),
        KCode::End => "end".to_string(),
        KCode::PageUp => "pageup".to_string(),
        KCode::PageDown => "pagedown".to_string(),
        KCode::Tab => "tab".to_string(),
        KCode::BackTab => "backtab".to_string(),
        KCode::Delete => "delete".to_string(),
        KCode::Insert => "insert".to_string(),
        KCode::F(num) => format!("f{num}"),
        KCode::Char(ch) => format!("{}", ch.to_lowercase()),
        KCode::Null => "null".to_string(),
        KCode::Esc => "esc".to_string(),
        KCode::CapsLock => "capslock".to_string(),
        KCode::ScrollLock => "scrolllock".to_string(),
        KCode::NumLock => "numlock".to_string(),
        KCode::PrintScreen => "printscreen".to_string(),
        KCode::Pause => "pause".to_string(),
        KCode::Menu => "menu".to_string(),
        KCode::KeypadBegin => "keypadbegin".to_string(),
        KCode::Media(key) => match key {
            MediaKeyCode::Play => "play",
            MediaKeyCode::Pause => "pause",
            MediaKeyCode::PlayPause => "playpause",
            MediaKeyCode::Reverse => "reverse",
            MediaKeyCode::Stop => "stop",
            MediaKeyCode::FastForward => "fastforward",
            MediaKeyCode::TrackNext => "next",
            MediaKeyCode::TrackPrevious => "previous",
            MediaKeyCode::Record => "record",
            MediaKeyCode::Rewind => "rewind",
            MediaKeyCode::LowerVolume => "lowervolume",
            MediaKeyCode::RaiseVolume => "raisevolume",
            MediaKeyCode::MuteVolume => "mutevolume",
        }
        .to_string(),
        KCode::Modifier(key) => match key {
            ModifierKeyCode::LeftShift => "lshift",
            ModifierKeyCode::LeftControl => "lctrl",
            ModifierKeyCode::LeftAlt => "lalt",
            ModifierKeyCode::LeftSuper => "lsuper",
            ModifierKeyCode::LeftHyper => "lhyper",
            ModifierKeyCode::LeftMeta => "lmeta",
            ModifierKeyCode::RightControl => "rctrl",
            ModifierKeyCode::RightAlt => "ralt",
            ModifierKeyCode::RightSuper => "rsuper",
            ModifierKeyCode::RightHyper => "rhyper",
            ModifierKeyCode::RightMeta => "rmeta",
            ModifierKeyCode::RightShift => "rshift",
            ModifierKeyCode::IsoLevel3Shift => "iso3shift",
            ModifierKeyCode::IsoLevel5Shift => "iso5shift",
        }
        .to_string(),
    };
    result
}
