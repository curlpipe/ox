//! User friendly interface for dealing with pseudo terminals

use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use mlua::prelude::*;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use ptyprocess::PtyProcess;
use std::io::{BufReader, Read, Result, Write};
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub struct Pty {
    pub process: PtyProcess,
    pub output: String,
    pub input: String,
    pub shell: Shell,
    pub force_rerender: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Shell {
    Bash,
    Dash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn manual_input_echo(self) -> bool {
        matches!(self, Self::Bash | Self::Dash)
    }

    pub fn inserts_extra_newline(self) -> bool {
        !matches!(self, Self::Zsh)
    }

    pub fn command(&self) -> &str {
        match self {
            Self::Bash => "bash",
            Self::Dash => "dash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }
}

impl IntoLua for Shell {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let string = lua.create_string(self.command())?;
        Ok(LuaValue::String(string))
    }
}

impl FromLua for Shell {
    fn from_lua(val: LuaValue, _: &Lua) -> LuaResult<Self> {
        Ok(if let LuaValue::String(inner) = val {
            if let Ok(s) = inner.to_str() {
                match s.to_owned().as_str() {
                    "dash" => Self::Dash,
                    "zsh" => Self::Zsh,
                    "fish" => Self::Fish,
                    _ => Self::Bash,
                }
            } else {
                Self::Bash
            }
        } else {
            Self::Bash
        })
    }
}

impl Pty {
    pub fn new(shell: Shell) -> Result<Arc<Mutex<Self>>> {
        let pty = Arc::new(Mutex::new(Self {
            process: PtyProcess::spawn(Command::new(shell.command()))?,
            output: String::new(),
            input: String::new(),
            shell,
            force_rerender: false,
        }));
        pty.lock().unwrap().process.set_echo(false, None)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        pty.lock().unwrap().run_command("")?;
        // Spawn thread to constantly read from the terminal
        let pty_clone = Arc::clone(&pty);
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let mut pty = pty_clone.lock().unwrap();
            pty.force_rerender = matches!(pty.catch_up(), Ok(true));
            std::mem::drop(pty);
        });
        // Return the pty
        Ok(pty)
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<()> {
        let mut stream = self.process.get_raw_handle()?;
        // Write the command
        write!(stream, "{cmd}")?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        if self.shell.manual_input_echo() {
            // println!("Adding (pre-cmd) {:?}", cmd);
            self.output += cmd;
        }
        // Read the output
        let mut reader = BufReader::new(stream);
        let mut buf = [0u8; 10240];
        let bytes_read = reader.read(&mut buf)?;
        let mut output = String::from_utf8_lossy(&buf[..bytes_read]).to_string();
        // Add on the output
        if self.shell.inserts_extra_newline() {
            output = output.replace("\u{1b}[?2004l\r\r\n", "");
        }
        // println!("Adding (aftercmd) \"{:?}\"", output);
        self.output += &output;
        Ok(())
    }

    pub fn silent_run_command(&mut self, cmd: &str) -> Result<()> {
        self.output.clear();
        self.run_command(cmd)?;
        if self.output.starts_with(cmd) {
            self.output = self.output.chars().skip(cmd.chars().count()).collect();
        }
        Ok(())
    }

    pub fn char_input(&mut self, c: char) -> Result<()> {
        self.input.push(c);
        if c == '\n' {
            // Return key pressed, send the input
            self.run_command(&self.input.to_string())?;
            self.input.clear();
        }
        Ok(())
    }

    pub fn char_pop(&mut self) {
        self.input.pop();
    }

    pub fn clear(&mut self) -> Result<()> {
        self.output.clear();
        self.run_command("\n")?;
        self.output = self.output.trim_start_matches('\n').to_string();
        Ok(())
    }

    pub fn catch_up(&mut self) -> Result<bool> {
        let stream = self.process.get_raw_handle()?;
        let raw_fd = stream.as_raw_fd();
        let flags = fcntl(raw_fd, FcntlArg::F_GETFL).unwrap();
        fcntl(
            raw_fd,
            FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK),
        )
        .unwrap();
        let mut source = SourceFd(&raw_fd);
        // Set up mio Poll and register the raw_fd
        let mut poll = Poll::new()?;
        let mut events = Events::with_capacity(128);
        poll.registry()
            .register(&mut source, Token(0), Interest::READABLE)?;
        match poll.poll(&mut events, Some(Duration::from_millis(100))) {
            Ok(()) => {
                // Data is available to read
                let mut reader = BufReader::new(stream);
                let mut buf = [0u8; 10240];
                let bytes_read = reader.read(&mut buf)?;

                // Process the read data
                let mut output = String::from_utf8_lossy(&buf[..bytes_read]).to_string();
                if self.shell.inserts_extra_newline() {
                    output = output.replace("\u{1b}[?2004l\r\r\n", "");
                }

                // Append the output to self.output
                // println!("Adding (aftercmd) \"{:?}\"", output);
                self.output += &output;
                Ok(!output.is_empty())
            }
            Err(e) => Err(e),
        }
    }
}
