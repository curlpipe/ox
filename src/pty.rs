/// User friendly interface for dealing with pseudo terminals
use ptyprocess::PtyProcess;
use std::io::{BufReader, Read, Result, Write};
use std::process::Command;

#[derive(Debug)]
pub struct Pty {
    pub process: PtyProcess,
    pub output: String,
    pub input: String,
    pub shell: Shell,
}

#[derive(Debug)]
pub enum Shell {
    Bash,
    Dash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn manual_input_echo(&self) -> bool {
        matches!(self, Self::Bash | Self::Dash)
    }

    pub fn inserts_extra_newline(&self) -> bool {
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

impl Pty {
    pub fn new(shell: Shell) -> Result<Self> {
        let mut pty = Self {
            process: PtyProcess::spawn(Command::new(shell.command()))?,
            output: String::new(),
            input: String::new(),
            shell,
        };
        pty.process.set_echo(false, None)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        pty.run_command("")?;
        Ok(pty)
    }

    pub fn run_command(&mut self, cmd: &str) -> Result<()> {
        let mut stream = self.process.get_raw_handle()?;
        // Write the command
        write!(stream, "{cmd}")?;
        std::thread::sleep(std::time::Duration::from_millis(100));
        if self.shell.manual_input_echo() {
            // println!("Adding (pre-cmd) {:?}", cmd);
            self.output += &cmd;
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

    pub fn char_input(&mut self, c: char) -> Result<()> {
        self.input.push(c);
        if c == '\n' {
            // Return key pressed, send the input
            self.run_command(&self.input.to_string())?;
            self.input.clear();
        }
        Ok(())
    }
}
