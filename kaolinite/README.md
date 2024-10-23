<h1 align="center">
  <br>
  <a href="https://github.com/curlpipe/kaolinite"><img src="https://i.postimg.cc/253c9YVX/image.png" alt="Markdownify" width="200"></a>
  <br>
  Kaolinite
  <br>
</h1>

<h4 align="center">A crate to assist in the creation of TUI text editors.</h4>

<p align="center">
  <a href="#key-features">Key Features</a> •
  <a href="#how-to-use">How To Use</a> •
  <a href="#credits">Credits</a> •
  <a href="#license">License</a>
</p>

## Key Features

- Buffers files to prevent hold ups when opening and saving your files
- Unicode safe - supports double width characters on the terminal
- Handles scrolling and cursor - No more janky cursor incrementing code
- Dynamically handles formatting of files - Determines style on read, keeps that style on write
	+ Unix and DOS line endings
	+ Tabs & Spaces
- Includes searching & replacing features
- Line number formatting utility
- File type recognition
- Advanced moving abilities (by page, words, characters)
- Includes undo & redo functionality
- Lightweight - very few dependencies for quick compilation of your editor
- Front-end agnostic - Feel free to use [Crossterm](https://github.com/crossterm-rs/crossterm) or [Termion](https://gitlab.redox-os.org/redox-os/termion) or anything else!

## How To Use

You'll need to have a modern Rust toolchain. Click [here](https://www.rust-lang.org/tools/install) if you need that.

```bash
# If you already have a project set up, ignore this step
$ cargo new [app name]
$ cd [app name]

# Simplest way to add to your project is using cargo-edit
# You can also manually add it into your Cargo.toml if you wish
$ cargo install cargo-edit
$ cargo add kaolinite

# You should be ready to use the crate now!
```

If you require documentation, please consult https://docs.rs/kaolinite. You'll find detailed API explainations and examples.

Don't hesitate to contact me (see bottom of readme) if you require assistance.


## Credits

This software uses the following open source crates:

- [unicode-width](https://github.com/unicode-rs/unicode-width)
- [regex](https://github.com/rust-lang/regex)
- [ropey](https://github.com/cessen/ropey)
- [error_set](https://github.com/mcmah309/error_set)

## License

MIT

---

> Github [@curlpipe](https://github.com/curlpipe) &nbsp;&middot;&nbsp;
> Discord [curlpipe#1496](https://discord.com) &nbsp;&middot;&nbsp;
> Crates.io [curlpipe](https://crates.io/users/curlpipe)
