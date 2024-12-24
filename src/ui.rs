/// Utilities for rendering the user interface
use crate::config::{Colors, Terminal as TerminalConfig};
use crate::editor::MacroMan;
use crate::error::Result;
use base64::prelude::*;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as CEvent, KeyCode as KCode, KeyEvent, KeyEventKind, KeyModifiers as KMod,
        KeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, queue,
    style::{Attribute, Color, SetAttribute, SetBackgroundColor as Bg, SetForegroundColor as Fg},
    terminal::{
        self, Clear, ClearType as ClType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use kaolinite::utils::{width, Size};
use mlua::AnyUserData;
use std::collections::HashMap;
use std::env;
use std::io::{stdout, Stdout, Write};
#[cfg(not(target_os = "windows"))]
use synoptic::Regex;

/// Printing macro
#[macro_export]
macro_rules! display {
    ( $self:expr, $( $x:expr ),* ) => {
        $self.terminal.cache += &SetAttribute(Attribute::NormalIntensity).to_string();
        $(
            $self.terminal.cache += &$x.to_string();
        )*
    };
}

/// Gets the size of the terminal
pub fn size() -> Result<Size> {
    let (w, h) = terminal::size()?;
    Ok(Size {
        w: w as usize,
        h: (h as usize).saturating_sub(1),
    })
}

/// Fatal Error
pub fn fatal_error(msg: &str) {
    // Prevent upset terminal state
    terminal::disable_raw_mode().unwrap();
    execute!(
        stdout(),
        LeaveAlternateScreen,
        Show,
        DisableMouseCapture,
        DisableBracketedPaste,
    )
    .unwrap();
    // Display the error information
    eprintln!(
        "{}{}[Error]{}{} {msg}",
        SetAttribute(Attribute::Bold),
        Fg(Color::Red),
        Fg(Color::Reset),
        SetAttribute(Attribute::Reset)
    );
    std::process::exit(1);
}

/// Shorthand to read key events
pub fn key_event(kev: &CEvent, mm: &mut MacroMan) -> Option<(KMod, KCode)> {
    mm.register(kev.clone());
    if let CEvent::Key(KeyEvent {
        modifiers,
        code,
        kind: KeyEventKind::Press,
        ..
    }) = kev
    {
        Some((*modifiers, *code))
    } else {
        None
    }
}

/// Represents different status messages
#[derive(Debug)]
pub enum Feedback {
    Info(String),
    Warning(String),
    Error(String),
    None,
}

impl Feedback {
    /// Actually render the status message
    pub fn render(&self, colors: &Colors, w: usize) -> Result<String> {
        // Calculate correct colours
        let start = match self {
            Self::Info(_) => format!(
                "{}{}",
                Fg(colors.info_fg.to_color()?),
                Bg(colors.info_bg.to_color()?)
            ),
            Self::Warning(_) => format!(
                "{}{}",
                Fg(colors.warning_fg.to_color()?),
                Bg(colors.warning_bg.to_color()?)
            ),
            Self::Error(_) => format!(
                "{}{}",
                Fg(colors.error_fg.to_color()?),
                Bg(colors.error_bg.to_color()?)
            ),
            Self::None => String::new(),
        };
        let end = format!(
            "{}{}",
            Bg(colors.editor_bg.to_color()?),
            Fg(colors.editor_fg.to_color()?),
        );
        // Calculate the contents of the feedback line
        let empty = String::new();
        let msg = match self {
            Self::Info(msg) | Self::Warning(msg) | Self::Error(msg) => msg,
            Self::None => &empty,
        };
        let text = if let Some(text) = alinio::align::center(msg, w) {
            text
        } else {
            // Could not fit, trim and add ellipsis
            let mut total_display = 0;
            let mut result = String::new();
            let safe_msg = msg.replace('\t', "    ");
            let mut chars = safe_msg.chars();
            while total_display < w.saturating_sub(3) {
                if let Some(chr) = chars.next() {
                    let chr = chr.to_string();
                    result += &chr;
                    total_display += width(&chr, 4);
                } else {
                    break;
                }
            }
            result += "...";
            result
        };
        // Put everything together and return
        Ok(format!(
            "{}{}{}{}{}",
            SetAttribute(Attribute::Bold),
            start,
            text,
            end,
            SetAttribute(Attribute::Reset)
        ))
    }
}

pub struct Terminal {
    pub stdout: Stdout,
    pub cache: String,
    pub config: AnyUserData,
    pub last_copy: String,
}

impl Terminal {
    pub fn new(config: AnyUserData) -> Self {
        Terminal {
            stdout: stdout(),
            cache: String::with_capacity(size().map(|s| s.w * s.h).unwrap_or(1000)),
            config,
            last_copy: String::new(),
        }
    }

    /// Set up the terminal so that it is clean and doesn't affect existing terminal text
    pub fn start(&mut self) -> Result<()> {
        std::panic::set_hook(Box::new(|e| {
            terminal::disable_raw_mode().unwrap();
            execute!(
                stdout(),
                LeaveAlternateScreen,
                Show,
                DisableMouseCapture,
                DisableBracketedPaste,
            )
            .unwrap();
            eprintln!("{e}");
        }));
        execute!(
            self.stdout,
            EnterAlternateScreen,
            Clear(ClType::All),
            DisableLineWrap,
        )?;
        let cfg = self.config.borrow::<TerminalConfig>().unwrap();
        if cfg.mouse_enabled {
            execute!(self.stdout, EnableMouseCapture)?;
        }
        terminal::enable_raw_mode()?;
        if cfg!(not(target_os = "windows")) {
            execute!(
                self.stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
                EnableBracketedPaste,
            )?;
        }
        self.flush()?;
        Ok(())
    }

    /// Restore terminal back to state before the editor was started
    pub fn end(&mut self) -> Result<()> {
        self.show_cursor();
        terminal::disable_raw_mode()?;
        execute!(self.stdout, LeaveAlternateScreen, EnableLineWrap,)?;
        if cfg!(not(target_os = "windows")) {
            execute!(self.stdout, DisableBracketedPaste,)?;
        }
        let cfg = self.config.borrow::<TerminalConfig>().unwrap();
        if cfg.mouse_enabled {
            execute!(self.stdout, DisableMouseCapture)?;
        }
        self.flush()?;
        Ok(())
    }

    /// Shows the cursor on the screen
    pub fn show_cursor(&mut self) {
        self.cache += &Show.to_string();
    }

    /// Hides the cursor on the screen
    pub fn hide_cursor(&mut self) {
        self.cache += &Hide.to_string();
    }

    /// Moves the cursor to a specific position on screen
    pub fn goto<Num: Into<usize>>(&mut self, x: Num, y: Num) {
        let x: usize = x.into();
        let y: usize = y.into();
        self.cache += &MoveTo(
            u16::try_from(x).unwrap_or(u16::MAX),
            u16::try_from(y).unwrap_or(u16::MAX),
        )
        .to_string();
    }

    /// Clears the current line
    pub fn clear_current_line(&mut self) {
        self.cache += &Clear(ClType::CurrentLine).to_string();
    }

    /// Moves to a line and makes sure it is cleared
    pub fn prepare_line(&mut self, y: usize) {
        self.goto(0, y);
        self.clear_current_line();
    }

    /// Flush the stdout (push the queued events to the screen)
    pub fn flush(&mut self) -> Result<()> {
        let mut queue = String::new();
        std::mem::swap(&mut queue, &mut self.cache);
        queue!(self.stdout, crossterm::style::Print(&queue))?;
        queue.clear();
        std::mem::swap(&mut queue, &mut self.cache);
        self.stdout.flush()?;
        Ok(())
    }

    /// Put text into the clipboard
    pub fn copy(&mut self, text: &str) -> Result<()> {
        self.last_copy = text.to_string();
        write!(
            self.stdout,
            "\x1b]52;c;{}\x1b\\",
            BASE64_STANDARD.encode(text)
        )?;
        Ok(())
    }
}

/// Determines if this terminal supports 256 bit colours
pub fn supports_true_color() -> bool {
    // Get the TERM and COLORTERM environment variables
    let term = env::var("TERM").unwrap_or_default();
    let colorterm = env::var("COLORTERM").unwrap_or_default();
    // Check for common true color indicators
    if term.contains("truecolor") || term.contains("screen") {
        return true;
    }
    // Some terminals use COLORTERM to indicate support for true color
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return true;
    }
    false
}

/// Converts rgb to the closest xterm equivalent
pub fn rgb_to_xterm256(r: u8, g: u8, b: u8) -> u8 {
    let lookup = get_xterm_lookup();
    let mut min_distance = f64::INFINITY;
    let mut closest_index = 0;
    for (index, &(xr, xg, xb)) in &lookup {
        // Calculate the Euclidean distance in RGB space
        let distance = ((f64::from(r) - f64::from(xr)).powi(2)
            + (f64::from(g) - f64::from(xg)).powi(2)
            + (f64::from(b) - f64::from(xb)).powi(2))
        .sqrt();
        if distance < min_distance {
            min_distance = distance;
            closest_index = *index;
        }
    }

    closest_index
}

/// Data representing xterm colours and their equivalent RGB values
pub const XTERMLOOKUP: &str = "0:0,0,0|1:128,0,0|2:0,128,0|3:128,128,0|4:0,0,128|5:128,0,128|6:0,128,128|7:192,192,192|8:128,128,128|9:255,0,0|10:0,255,0|11:255,255,0|12:0,0,255|13:255,0,255|14:0,255,255|15:255,255,255|16:0,0,0|17:0,0,95|18:0,0,135|19:0,0,175|20:0,0,215|21:0,0,255|22:0,95,0|23:0,95,95|24:0,95,135|25:0,95,175|26:0,95,215|27:0,95,255|28:0,135,0|29:0,135,95|30:0,135,135|31:0,135,175|32:0,135,215|33:0,135,255|34:0,175,0|35:0,175,95|36:0,175,135|37:0,175,175|38:0,175,215|39:0,175,255|40:0,215,0|41:0,215,95|42:0,215,135|43:0,215,175|44:0,215,215|45:0,215,255|46:0,255,0|47:0,255,95|48:0,255,135|49:0,255,175|50:0,255,215|51:0,255,255|52:95,0,0|53:95,0,95|54:95,0,135|55:95,0,175|56:95,0,215|57:95,0,255|58:95,95,0|59:95,95,95|60:95,95,135|61:95,95,175|62:95,95,215|63:95,95,255|64:95,135,0|65:95,135,95|66:95,135,135|67:95,135,175|68:95,135,215|69:95,135,255|70:95,175,0|71:95,175,95|72:95,175,135|73:95,175,175|74:95,175,215|75:95,175,255|76:95,215,0|77:95,215,95|78:95,215,135|79:95,215,175|80:95,215,215|81:95,215,255|82:95,255,0|83:95,255,95|84:95,255,135|85:95,255,175|86:95,255,215|87:95,255,255|88:135,0,0|89:135,0,95|90:135,0,135|91:135,0,175|92:135,0,215|93:135,0,255|94:135,95,0|95:135,95,95|96:135,95,135|97:135,95,175|98:135,95,215|99:135,95,255|100:135,135,0|101:135,135,95|102:135,135,135|103:135,135,175|104:135,135,215|105:135,135,255|106:135,175,0|107:135,175,95|108:135,175,135|109:135,175,175|110:135,175,215|111:135,175,255|112:135,215,0|113:135,215,95|114:135,215,135|115:135,215,175|116:135,215,215|117:135,215,255|118:135,255,0|119:135,255,95|120:135,255,135|121:135,255,175|122:135,255,215|123:135,255,255|124:175,0,0|125:175,0,95|126:175,0,135|127:175,0,175|128:175,0,215|129:175,0,255|130:175,95,0|131:175,95,95|132:175,95,135|133:175,95,175|134:175,95,215|135:175,95,255|136:175,135,0|137:175,135,95|138:175,135,135|139:175,135,175|140:175,135,215|141:175,135,255|142:175,175,0|143:175,175,95|144:175,175,135|145:175,175,175|146:175,175,215|147:175,175,255|148:175,215,0|149:175,215,95|150:175,215,135|151:175,215,175|152:175,215,215|153:175,215,255|154:175,255,0|155:175,255,95|156:175,255,135|157:175,255,175|158:175,255,215|159:175,255,255|160:215,0,0|161:215,0,95|162:215,0,135|163:215,0,175|164:215,0,215|165:215,0,255|166:215,95,0|167:215,95,95|168:215,95,135|169:215,95,175|170:215,95,215|171:215,95,255|172:215,135,0|173:215,135,95|174:215,135,135|175:215,135,175|176:215,135,215|177:215,135,255|178:215,175,0|179:215,175,95|180:215,175,135|181:215,175,175|182:215,175,215|183:215,175,255|184:215,215,0|185:215,215,95|186:215,215,135|187:215,215,175|188:215,215,215|189:215,215,255|190:215,255,0|191:215,255,95|192:215,255,135|193:215,255,175|194:215,255,215|195:215,255,255|196:255,0,0|197:255,0,95|198:255,0,135|199:255,0,175|200:255,0,215|201:255,0,255|202:255,95,0|203:255,95,95|204:255,95,135|205:255,95,175|206:255,95,215|207:255,95,255|208:255,135,0|209:255,135,95|210:255,135,135|211:255,135,175|212:255,135,215|213:255,135,255|214:255,175,0|215:255,175,95|216:255,175,135|217:255,175,175|218:255,175,215|219:255,175,255|220:255,215,0|221:255,215,95|222:255,215,135|223:255,215,175|224:255,215,215|225:255,215,255|226:255,255,0|227:255,255,95|228:255,255,135|229:255,255,175|230:255,255,215|231:255,255,255|232:8,8,8|233:18,18,18|234:28,28,28|235:38,38,38|236:48,48,48|237:58,58,58|238:68,68,68|239:78,78,78|240:88,88,88|241:98,98,98|242:108,108,108|243:118,118,118|244:128,128,128|245:138,138,138|246:148,148,148|247:158,158,158|248:168,168,168|249:178,178,178|250:188,188,188|251:198,198,198|252:208,208,208|253:218,218,218|254:228,228,228|255:238,238,238";

/// Based on the xterm lookup data, generate a rust hashmap to interpret
pub fn get_xterm_lookup() -> HashMap<u8, (u8, u8, u8)> {
    let mut result = HashMap::default();
    for line in XTERMLOOKUP.split('|') {
        let mut parts = line.split(':');
        let (id, mut rgb_str) = (
            parts.next().unwrap().parse::<u8>().unwrap(),
            parts.next().unwrap().split(','),
        );
        let (r, g, b) = (
            rgb_str.next().unwrap().parse::<u8>().unwrap(),
            rgb_str.next().unwrap().parse::<u8>().unwrap(),
            rgb_str.next().unwrap().parse::<u8>().unwrap(),
        );
        result.insert(id, (r, g, b));
    }
    result
}

/// Define various ANSI code regex patterns for matching
#[cfg(not(target_os = "windows"))]
pub const CURSORS: &str = r"\x1b(?:\[(?:\d+(?:;\d+)?(?:A|B|C|D|F|f|G|H)|(?:#[A-G]|6n|s|u|H)|[0-6] q|(?:[0-9]{1,3};)*[0-9]{1,3}t)|[7-8])";
#[cfg(not(target_os = "windows"))]
pub const ERASING: &str = r"\x1b\[[0-3]?(?:J|K)";
#[cfg(not(target_os = "windows"))]
pub const DISPLAY: &str =
    r"\x1b\[(?:(?:[0-9]|22|23|24|25|27|28|29)(?:t|m)|(?:[0-9]{1,3};)*[0-9]{1,3}m)";
#[cfg(not(target_os = "windows"))]
pub const SC_MODE: &str = r"\x1b\[(?:=|\?)[0-9]{1,4}(?:h|l)";
#[cfg(not(target_os = "windows"))]
pub const AC_JUNK: &str = r"(\a|\b|\n|\v|\f|\r)";

/// Global ANSI removal
#[cfg(not(target_os = "windows"))]
fn global() -> String {
    format!("({CURSORS}|{ERASING}|{DISPLAY}|{SC_MODE}|{AC_JUNK})")
}

#[cfg(not(target_os = "windows"))]
/// Remove ANSI codes from a string
pub fn remove_ansi_codes(input: &str) -> String {
    // Define a regular expression to match ANSI escape codes and other control sequences
    let re = Regex::new(&global()).unwrap();
    let weird_newline = Regex::new(r"⏎\s*⏎\s?").unwrap();
    let long_spaces = Regex::new(r"%(?:\x1b\[1m)?\s{5,}").unwrap();
    // Replace all matches with an empty string
    let result = re.replace_all(input, "").to_string();
    // Replace weird new line stuff
    let result = weird_newline.replace_all(&result, "").to_string();
    // Replace long spaces
    let result = long_spaces.replace_all(&result, "").to_string();
    // Return result
    result
}

#[cfg(not(target_os = "windows"))]
/// Remove all ANSI codes outside of color and attribute codes
pub fn strip_escape_codes(input: &str) -> String {
    let re = Regex::new(&global()).unwrap();
    let display = Regex::new(DISPLAY).unwrap();
    let weird_newline = Regex::new(r"⏎\s*⏎\s?").unwrap();
    let long_spaces = Regex::new(r"%(?:\x1b\[1m)?\s{5,}").unwrap();
    // Replace escape sequences, keeping those for attributes and colors
    let result = re
        .replace_all(input, |caps: &regex::Captures| {
            let code = caps.get(0).unwrap().as_str();
            if display.is_match(code) {
                // Return the escape code unchanged
                code.to_string()
            } else {
                // Return an empty string for non-allowed escape codes (including cursor styling)
                String::new()
            }
        })
        .to_string();
    // Replace weird new line stuff
    let result = weird_newline.replace_all(&result, "").to_string();
    // Replace long spaces
    let result = long_spaces.replace_all(&result, "").to_string();
    // Return result
    result
}

#[cfg(not(target_os = "windows"))]
#[allow(clippy::similar_names)]
/// Replace reset colour ANSI codes with a custom background color.
pub fn replace_reset(input: &str, custom_bg: &str, custom_fg: &str) -> String {
    // Replace total reset with appropriate codes
    let total_reset_regex = Regex::new(r"\x1b\[0m").unwrap();
    // Define the regex to match reset background ANSI codes
    let reset_bg_regex = Regex::new(r"\x1b\[49m").unwrap();
    // Define the regex to match reset foreground ANSI codes
    let reset_fg_regex = Regex::new(r"\x1b\[39m").unwrap();
    // Replace reset background with the custom background color
    let attr_full_reset =
        "\u{1b}[22m\u{1b}[23m\u{1b}[24m\u{1b}[25m\u{1b}[27m\u{1b}[28m\u{1b}[29m".to_string();
    let full_reset = format!("{custom_bg}{custom_fg}{attr_full_reset}");
    let input = total_reset_regex.replace_all(input, full_reset).to_string();
    let input = reset_bg_regex.replace_all(&input, custom_bg).to_string();
    let input = reset_fg_regex.replace_all(&input, custom_fg).to_string();
    input
}
