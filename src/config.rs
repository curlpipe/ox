// Config.rs - In charge of storing configuration information
use termion::color;

// Set up background colours
pub const BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(40, 42, 54));
pub const STATUS_BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(68, 71, 90));
pub const RESET_BG: color::Bg<color::Reset> = color::Bg(color::Reset);

// Set up foreground colours
pub const FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 255, 255));
pub const STATUS_FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(80, 250, 123));
pub const RESET_FG: color::Fg<color::Reset> = color::Fg(color::Reset);

// For holding the tab width (how many spaces in a tab)
pub const TAB_WIDTH: usize = 4;

// Line numbers
pub const LINE_NUMBER_FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(68, 71, 90));
pub const LINE_NUMBER_PADDING: usize = 1;

// Undo features
pub const UNDO_INACTIVITY_PERIOD: u64 = 5;
