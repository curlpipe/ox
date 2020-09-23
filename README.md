<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/curlpipe/ox/">
    <img src="assets/logo.png" alt="Logo" width="150" height="150">
  </a>

  <h1 align="center" style="font-size: 50px;">Ox editor</h1>

  <p align="center" style="font-size: 20px;">
    Ox is a fast text editor that runs in your terminal.
    <br><br>
    <div align="center" style="display:inline;">
      <img src="https://i.postimg.cc/N0tqYD40/image.png" width="49%">
      <img src="https://i.postimg.cc/xC02G5JZ/image.png" width="49%">
    </div>
    <br>
</p>

[![Build Status](https://img.shields.io/github/forks/curlpipe/ox.svg?style=for-the-badge)](https://github.com/curlpipe/ox)
[![Build Status](https://img.shields.io/github/stars/curlpipe/ox.svg?style=for-the-badge)](https://github.com/curlpipe/ox)
[![License](https://img.shields.io/github/license/curlpipe/ox.svg?style=for-the-badge)](https://github.com/curlpipe/ox)

## About The Project

Ox is a text editor with IDE-like features. It was written in Rust using ANSI escape sequences. It assists developers with programming by providing several tools to speed up and make programming easier. It is a refreshing alternative to heavily bloated and resource hungry editors such as VSCode and JetBrains. It is so lightweight that it can be used on even the older computers. 

It runs in the terminal and runs on platforms like Linux and MacOS but doesn't work on Windows directly (it works if you use WSL) due to a lack of a good command line. There are many text editors out there and each one of them has their flaws and I hope to have a text editor that overcomes many of the burdens and issues.

Ox is not based off any other editor and has been built from the ground up without any base at all.

## What features does Ox have and why should I use it?

Ox aims to be an editor that takes features from some of the most popular editors out there, gaining the best of all worlds.

**Vim** http://vim.org: Vim provides a plugin system for adding features to it as it is very minimal and only provides basic text editing functionality by default. It is quite extensive and has its own programming language for configuring and writing plugins for it. It has a steep learning curve due to being a “modal” text editor, having special modes for editing text. Ox is easier to use then Vim because it doesn’t have modes where the keyboard is repurposed, however it takes the idea of being a keyboard-only editor and being able to act just like an IDE after some configuration.

**Nano** https://www.nano-editor.org/: Nano is an editor that is very simple to grasp due to its intuitive key bindings such as “Ctrl+S” to save and “Ctrl+?” for the help menu etc. Ox took the idea for the keybindings from this editor, they are simple to remember, “Ctrl+F” for “Find”, “Ctrl+Q” for “Quit”, meaning that Ox doesn’t have as steep a learning curve. 

**Micro** https://micro-editor.github.io/: Micro has a plugin system that is programmed with a language called Lua however I can’t seem to find any up to date plugins for it and it lacks features such as a file tree. It is micro that inspired me to add mouse functionality and other features.

**Emacs** https://www.gnu.org/software/emacs/: Emacs is still actively used today due to its freedom to modify and change the source code. Ox took the idea for the customization and extensibility of Emacs and made a configuration system where you can change the colours and appearance of the editor.

**Xi** https://xi-editor.io/: Xi is also written in Rust but is purely a backend at the moment, I decided to make Ox both a frontend and a backend because Xi has many frontends, but most of them are broken and it lacks a lot of features.

**Kiro** https://github.com/rhysd/kiro-editor: Kiro is written in Rust and adds features such as unicode support, a nicer colour scheme and small things like resizing. Ox took the ideas for the improvements from Kiro, however implemented them differently. Kiro’s source code is also very difficult to understand at points, so I decided to keep Ox’s syntax simple and use a minimal set of the vast features that Rust has to offer.

### Built With

Ox is super minimal and aims to use as few dependencies as possible, allowing for rapid compile time and low risk of breakage.

* [Rust language](https://rust-lang.org)
* [Termion](https://github.com/redox-os/termion)

<!-- GETTING STARTED -->
## Getting Started

You can currently only build Ox from source.
While this may sound daunting to many people, it really isn't that hard and takes 1 minute worst case scenario!

### Prerequisites

Because Ox is written in Rust, you must have a modern and working version of `rustc` and `cargo`.
On Arch Linux, you can run this command:
```sh
sudo pacman -S rustup
rustup toolchain install stable
```

If you are not using Arch, you can easily set it up on other distros by running the distro-neutral command:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
/usr/bin/rustup toolchain install stable
```
You must have `curl` installed in order to run this command.

### Installation
 
1. Clone the repo and change into it
```sh
git clone https://github.com/curlpipe/ox
cd ox
```
2. Build Ox
```sh
cargo build --release
```
3. Copy the binary into your `/usr/bin` directory
```sh
sudo cp target/release/ox /usr/bin/ox
```

That's all there is to it!

<!-- USAGE EXAMPLES -->
## Usage

#### Opening files in Ox

At the moment, you can open ox by using the command
```
ox
```

This will open up an empty document.

If you wish to open a file straight from the command line, you can run
```
ox /path/to/file
```
To open and edit a file.

You can also open a file from within Ox by using the <kbd>Ctrl + O</kbd> Key binding

If at any time, you wish to create a new file, you can use <kbd>Ctrl + N</kbd> to do so.

#### Moving the cursor around

You can use the arrow keys to move the cursor around

You can also use:
 - <kbd>PageUp</kbd> - Go to the top of the document
 - <kbd>PageDown</kbd> - Go to the bottom of the document
 - <kbd>Home</kbd> - Go to the start of the current line
 - <kbd>End</kbd> - Go to the end of the current line

#### Editing the file

You can use the keys <kbd>Backspace</kbd> and <kbd>Return</kbd> / <kbd>Enter</kbd> as well as all the characters on your keyboard to edit files!

#### Saving the file

The simple keyboard shortcut of <kbd>Ctrl + S</kbd> can be used to save the current file and <kbd>Ctrl + W</kbd> can be used to "save as" the current file to a specific path.

#### Closing Ox

You can use the keybinding <kbd>Ctrl + Q</kbd> to exit Ox.

#### Searching a file in Ox

You can search both back and forth by activating the search feature through <kbd>Ctrl + F</kbd>, typing out what you wish to search and then using <kbd>→</kbd> or <kbd>↓</kbd> To search forward and <kbd>←</kbd> or <kbd>↑</kbd> to search backwards. 

If at any time you wish to exit the search feature and return to the location in the document that you were in before activating the search feature, you can press <kbd>esc</kbd> on your keyboard, otherwise you can press any other key to exit the search feature and start editing your document at the new location.

#### Undoing / Redoing

Undoing and Redoing in Ox is as simple as <kbd>Ctrl + U</kbd> to undo and <kbd>Ctrl + Y</kbd> to redo. The changes are commited to the undo stack every time you press the space bar, create / destroy a new line and when there is inactivity longer than a specific period of time. (e.g. Ox will commit to the undo stack after 10 seconds of inactivity, possibly while you pause for thought or a break)

## Roadmap

You can see the `tasks.todo.md` file to see my full plans for the future of the editor!

Here is the current summary
 - [X] Initial Research (0.1.0, 0.1.1) [62 commits]
 - [X] Basic editing functions (0.2.0)
 - [X] Line numbers (0.2.0)
 - [X] Searching (0.2.0) [33 commits]
 - [X] Undo and Redo (0.2.1) [28 commits]
 - [ ] Clipboard support (0.2.2)
 - [ ] Good command line interface (0.2.3)
 - [ ] Config files (0.2.3)
 - [ ] Replacing text (0.2.3)
 - [ ] Syntax highlighting (0.2.4)
 - [ ] Tabs for multitasking (0.2.5)
 - [ ] Macros (0.2.6)
 - [ ] Mouse support (0.2.7)
 - [ ] Auto indentation (0.3.0)
 - [ ] Prettifier / Automatic code formatter (0.3.0)
 - [ ] Built In linter (0.3.0)
 - [ ] Auto brackets (0.3.1)
 - [ ] Auto complete (0.3.2)
 - [ ] File tree (0.3.4)
 - [ ] Start page (0.3.5)

## License

Distributed under the GNU GPLv2 License. See `LICENSE` for more information.

## Contact

You can contact me on Discord at `curlpipe#1496`. I'll be happy to answer any questions you may have!

## Acknowledgements

* [Curlpipe](https://github.com/curlpipe)
