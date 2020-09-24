// Config.rs - In charge of storing configuration information
use termion::color;

// Struct for storing and managing configuration
pub struct ConfigReader {
    pub config_fl: String,
    pub window_bg: color::Bg<color::Rgb>,
    pub status_bg: color::Bg<color::Rgb>,
    pub status_fg: color::Fg<color::Rgb>,
    pub window_fg: color::Fg<color::Rgb>,
    pub lineno_fg: color::Fg<color::Rgb>,
    pub lineno_pd: usize,
    pub tab_width: usize,
    pub undo_time: u64,
}

impl ConfigReader {
    pub fn new(config: String) -> Self{
        // Initialise a config reader with default values
        Self {
            config_fl: config,
            window_bg: color::Bg(color::Rgb(33, 33, 48)),
            status_bg: color::Bg(color::Rgb(51, 51, 72)),
            status_fg: color::Fg(color::Rgb(35, 240, 144)),
            window_fg: color::Fg(color::Rgb(255, 255, 255)),
            lineno_fg: color::Fg(color::Rgb(51, 51, 72)),
            lineno_pd: 1,
            tab_width: 4,
            undo_time: 5,
        }
    }
}

/*
// Set up background colours
pub const BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(33, 33, 48));
pub const STATUS_BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(51, 51, 72));

// Set up foreground colours
pub const FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 255, 255));
pub const STATUS_FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(35, 240, 144));

// For holding the tab width (how many spaces in a tab)
pub const TAB_WIDTH: usize = 4;

// Line numbers
pub const LINE_NUMBER_FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(51, 51, 72));
pub const LINE_NUMBER_PADDING: usize = 1;

// Undo features
pub const UNDO_INACTIVITY_PERIOD: u64 = 5;
*/
