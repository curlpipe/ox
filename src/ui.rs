use crate::config::{Colors, Terminal as TerminalConfig};
use crate::error::Result;
use base64::prelude::*;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        KeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, queue,
    style::{Attribute, SetAttribute, SetBackgroundColor as Bg, SetForegroundColor as Fg},
    terminal::{
        self, Clear, ClearType as ClType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use kaolinite::utils::Size;
use std::cell::RefCell;
use std::io::{stdout, Stdout, Write};
use std::rc::Rc;

/// Gets the size of the terminal
pub fn size() -> Result<Size> {
    let (w, h) = terminal::size()?;
    Ok(Size {
        w: w as usize,
        h: (h as usize).saturating_sub(1),
    })
}

/// Represents different status messages
pub enum Feedback {
    Info(String),
    Warning(String),
    Error(String),
    None,
}

impl Feedback {
    /// Actually render the status message
    pub fn render(&self, colors: &Colors, w: usize) -> Result<String> {
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
        let empty = String::new();
        let msg = match self {
            Self::Info(msg) | Self::Warning(msg) | Self::Error(msg) => msg,
            Self::None => &empty,
        };
        let end = format!(
            "{}{}",
            Bg(colors.editor_bg.to_color()?),
            Fg(colors.editor_fg.to_color()?),
        );
        Ok(format!(
            "{}{}{}{}{}",
            SetAttribute(Attribute::Bold),
            start,
            alinio::align::center(msg, w).unwrap_or_default(),
            end,
            SetAttribute(Attribute::Reset)
        ))
    }
}

pub struct Terminal {
    pub stdout: Stdout,
    pub config: Rc<RefCell<TerminalConfig>>,
}

impl Terminal {
    pub fn new(config: Rc<RefCell<TerminalConfig>>) -> Self {
        Terminal {
            stdout: stdout(),
            config,
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
                EnableBracketedPaste
            )
            .unwrap();
            eprintln!("{e}");
        }));
        execute!(
            self.stdout,
            EnterAlternateScreen,
            Clear(ClType::All),
            DisableLineWrap,
            EnableBracketedPaste,
        )?;
        if self.config.borrow().mouse_enabled {
            execute!(self.stdout, EnableMouseCapture)?;
        }
        terminal::enable_raw_mode()?;
        execute!(
            self.stdout,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
        Ok(())
    }

    /// Restore terminal back to state before the editor was started
    pub fn end(&mut self) -> Result<()> {
        self.show_cursor()?;
        terminal::disable_raw_mode()?;
        execute!(
            self.stdout,
            LeaveAlternateScreen,
            EnableLineWrap,
            DisableBracketedPaste
        )?;
        if self.config.borrow().mouse_enabled {
            execute!(self.stdout, DisableMouseCapture)?;
        }
        Ok(())
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        queue!(self.stdout, Show)?;
        Ok(())
    }

    pub fn hide_cursor(&mut self) -> Result<()> {
        queue!(self.stdout, Hide)?;
        Ok(())
    }

    pub fn goto<Num: Into<usize>>(&mut self, x: Num, y: Num) -> Result<()> {
        let x: usize = x.into();
        let y: usize = y.into();
        queue!(
            self.stdout,
            MoveTo(
                u16::try_from(x).unwrap_or(u16::MAX),
                u16::try_from(y).unwrap_or(u16::MAX)
            )
        )?;
        Ok(())
    }

    pub fn clear_current_line(&mut self) -> Result<()> {
        queue!(self.stdout, Clear(ClType::CurrentLine))?;
        Ok(())
    }

    pub fn prepare_line(&mut self, y: usize) -> Result<()> {
        self.goto(0, y)?;
        self.clear_current_line()
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdout.flush()?;
        Ok(())
    }

    /// Put text into the clipboard
    pub fn copy(&mut self, text: &str) -> Result<()> {
        write!(
            self.stdout,
            "\x1b]52;c;{}\x1b\\",
            BASE64_STANDARD.encode(text)
        )?;
        Ok(())
    }
}
