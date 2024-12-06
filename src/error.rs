/// Error handling utilities
use error_set::error_set;
use kaolinite::event::Error as KError;

error_set! {
    OxError = {
        #[display("Error in I/O: {0}")]
        Render(std::io::Error),
        #[display("{}",
            match source {
                KError::NoFileName => "This document has no file name, please use 'save as' instead".to_string(),
                KError::OutOfRange => "Requested operation is out of range".to_string(),
                KError::ReadOnlyFile => "This file is read only and can't be saved or edited".to_string(),
                KError::Rope(rerr) => format!("Backend had an issue processing text: {rerr}"),
                KError::Io(ioerr) => format!("I/O Error: {ioerr}"),
            }
        )]
        Kaolinite(KError),
        #[display("Error in config file: {}", msg)]
        Config {
            msg: String
        },
        #[display("Error in lua: {0}")]
        Lua(mlua::prelude::LuaError),
        #[display("Operation Cancelled")]
        Cancelled,
        #[display("File '{}' is already open", file)]
        AlreadyOpen {
            file: String,
        },
        InvalidPath,
        // None, <--- Needed???
    };
}

/// Easy syntax sugar to have functions return the custom error type
pub type Result<T> = std::result::Result<T, OxError>;
