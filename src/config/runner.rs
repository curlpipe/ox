//! Configuration for defining how programs should be compiled and run

use mlua::prelude::*;

/// Main struct to determine how a language should be compiled / run
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct RunCommand {
    pub compile: Option<String>,
    pub run: Option<String>,
}

impl FromLua for RunCommand {
    fn from_lua(val: LuaValue, _: &Lua) -> LuaResult<Self> {
        if let LuaValue::Table(table) = val {
            Ok(Self {
                compile: table.get("compile")?,
                run: table.get("run")?,
            })
        } else {
            Ok(Self::default())
        }
    }
}
