use kaolinite::event::Error as KError;
use quick_error::quick_error;

quick_error! {
    #[derive(Debug)]
    pub enum OxError {
        Render(err: std::io::Error) {
            from()
            display("Error in I/O: {}", err)
        }
        Kaolinite(err: KError) {
            from()
            display("{}", {
                match err {
                    KError::NoFileName => "This document has no file name, please use 'save as' instead".to_string(),
                    KError::OutOfRange => "Requested operation is out of range".to_string(),
                    KError::ReadOnlyFile => "This file is read only and can't be saved or edited".to_string(),
                    KError::Rope(rerr) => format!("Backend had an issue processing text: {rerr}"),
                    KError::Io(ioerr) => format!("I/O Error: {ioerr}"),
                }
            })
        }
        Config(msg: String) {
            display("Error in config file: {}", msg)
        }
        Lua(err: mlua::prelude::LuaError) {
            from()
            display("Error in lua: {}", err)
        }
        None
    }
}

pub type Result<T> = std::result::Result<T, OxError>;
