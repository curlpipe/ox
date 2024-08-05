use crate::config::Colors;
use crate::error::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{SetBackgroundColor as Bg, SetForegroundColor as Fg, SetAttribute, Attribute},
    terminal::{self, Clear, ClearType as ClType, EnterAlternateScreen, LeaveAlternateScreen, EnableLineWrap, DisableLineWrap},
    event::{PushKeyboardEnhancementFlags, KeyboardEnhancementFlags},
};
use kaolinite::utils::{Size};
use std::io::{stdout, Stdout, Write};

/// Constant that shows the help message
pub const HELP_TEXT: &str = "
   Default Key Bindings:       
   Ctrl + N:   New             
   Ctrl + O:   Open            
   Ctrl + Q:   Quit            
   Ctrl + S:   Save            
   Ctrl + W:   Save as         
   Ctrl + A:   Save all        
   Ctrl + Z:   Undo            
   Ctrl + Y:   Redo            
   Ctrl + F:   Find            
   Ctrl + R:   Replace         
   Ctrl + W:   Delete Word     
   Ctrl + D:   Delete Line     
   Ctrl + K:   Command Line    
   Alt + Up:   Move line up    
   Alt + Down: Move line down  
   Shift + ->: Next Tab        
   Shift + <-: Previous Tab    
";

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
            Self::Info(_) => 
                format!("{}{}", Fg(colors.info_fg.to_color()?), Bg(colors.info_bg.to_color()?)),
            Self::Warning(_) => 
                format!("{}{}", Fg(colors.warning_fg.to_color()?), Bg(colors.warning_bg.to_color()?)),
            Self::Error(_) => 
                format!("{}{}", Fg(colors.error_fg.to_color()?), Bg(colors.error_bg.to_color()?)),
            Self::None => "".to_string(),
        };
        let empty = "".to_string();
        let msg = match self {
            Self::Info(msg) => msg,
            Self::Warning(msg) => msg,
            Self::Error(msg) => msg,
            Self::None => &empty,
        };
        let end_fg = Fg(colors.editor_fg.to_color()?).to_string();
        let end_bg = Bg(colors.editor_bg.to_color()?).to_string();
        Ok(format!(
            "{}{}{}{}{}{}", 
            SetAttribute(Attribute::Bold), 
            start, 
            alinio::align::center(&msg, w)
                .unwrap_or_else(|| "".to_string()),
            end_bg, end_fg, 
            SetAttribute(Attribute::Reset)
        ))
    }
}

pub struct Terminal {
    pub stdout: Stdout,
}

impl Terminal {
    pub fn new() -> Self {
        Terminal {
            stdout: stdout(),
        }
    }

    /// Set up the terminal so that it is clean and doesn't affect existing terminal text
    pub fn start(&mut self) -> Result<()> {
        std::panic::set_hook(Box::new(|e| {
            terminal::disable_raw_mode().unwrap();
            execute!(stdout(), LeaveAlternateScreen, Show).unwrap();
            eprintln!("{}", e);
        }));
        execute!(self.stdout, EnterAlternateScreen, Clear(ClType::All), DisableLineWrap)?;
        terminal::enable_raw_mode()?;
        execute!(
            self.stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        )?;
        Ok(())
    }

    /// Restore terminal back to state before the editor was started
    pub fn end(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(self.stdout, LeaveAlternateScreen, EnableLineWrap)?;
        Ok(())
    }

    pub fn show_cursor(&mut self) -> Result<()> {
        execute!(self.stdout, Show)?;
        Ok(())
    }

    pub fn hide_cursor(&mut self) -> Result<()> {
        execute!(self.stdout, Hide)?;
        Ok(())
    }

    pub fn goto<Num: Into<usize>>(&mut self, x: Num, y: Num) -> Result<()> {
        let x: usize = x.into();
        let y: usize = y.into();
        execute!(self.stdout, MoveTo(x as u16, y as u16))?;
        Ok(())
    }

    pub fn clear_current_line(&mut self) -> Result<()> {
        execute!(self.stdout, Clear(ClType::CurrentLine))?;
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
}
