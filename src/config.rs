// Config.rs - In charge of storing configuration information
use std::fs;
use termion::color;
use toml::Value;

// Struct for storing and managing configuration
pub struct Reader {
    pub file: String,
    pub window_bg: color::Bg<color::Rgb>,
    pub status_bg: color::Bg<color::Rgb>,
    pub status_fg: color::Fg<color::Rgb>,
    pub window_fg: color::Fg<color::Rgb>,
    pub line_number_fg: color::Fg<color::Rgb>,
    pub line_number_padding: usize,
    pub tab_width: usize,
    pub undo_period: u64,
}

#[allow(clippy::cast_sign_loss)]
impl Reader {
    pub fn new(config: String) -> Self {
        // Initialise a config reader with default values
        Self {
            file: config,
            window_bg: color::Bg(color::Rgb(41, 41, 61)),
            window_fg: color::Fg(color::Rgb(255, 255, 255)),
            status_bg: color::Bg(color::Rgb(51, 51, 72)),
            status_fg: color::Fg(color::Rgb(35, 240, 144)),
            line_number_fg: color::Fg(color::Rgb(51, 51, 72)),
            line_number_padding: 1,
            tab_width: 4,
            undo_period: 5,
        }
    }
    pub fn read_config(&mut self) {
        // Populate this config reader with values from the config file
        if let Ok(raw) = fs::read_to_string(&self.file) {
            let data = raw.parse::<Value>().unwrap();
            // Ensure the theme section is intact
            if let Some(theme) = data.get("theme") {
                // Collect theme values
                if let Some(rgb) = Reader::get_rgb(&theme, "bg") {
                    self.window_bg = color::Bg(color::Rgb(rgb.0, rgb.1, rgb.2));
                }
                if let Some(rgb) = Reader::get_rgb(&theme, "fg") {
                    self.window_fg = color::Fg(color::Rgb(rgb.0, rgb.1, rgb.2));
                }
                if let Some(rgb) = Reader::get_rgb(&theme, "status_bg") {
                    self.status_bg = color::Bg(color::Rgb(rgb.0, rgb.1, rgb.2));
                }
                if let Some(rgb) = Reader::get_rgb(&theme, "status_fg") {
                    self.status_fg = color::Fg(color::Rgb(rgb.0, rgb.1, rgb.2));
                }
                if let Some(rgb) = Reader::get_rgb(&theme, "line_number_fg") {
                    self.line_number_fg = color::Fg(color::Rgb(rgb.0, rgb.1, rgb.2));
                }
            }
            // Ensure the general section is intact
            if let Some(general) = data.get("general") {
                // Collect the general values
                if let Some(raw) = general.get("line_number_padding_right") {
                    if let Some(num) = raw.as_integer() {
                        self.line_number_padding = num as usize;
                    }
                }
                if let Some(raw) = general.get("tab_width") {
                    if let Some(num) = raw.as_integer() {
                        self.tab_width = num as usize;
                    }
                }
                if let Some(raw) = general.get("undo_period") {
                    if let Some(num) = raw.as_integer() {
                        self.undo_period = num as u64;
                    }
                }
            }
        }
    }
    fn get_rgb(data: &Value, item: &str) -> Option<(u8, u8, u8)> {
        data.get(item)?.as_array().and_then(|value| {
            let r = value[0].as_integer().map(|r| r as u8)?;
            let g = value[1].as_integer().map(|r| r as u8)?;
            let b = value[2].as_integer().map(|r| r as u8)?;
            Some((r, g, b))
        })
    }
}
