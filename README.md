<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/curlpipe/ox/">
    <img src="assets/logo.png" alt="Logo" width="150" height="150">
  </a>

  <h1 align="center" style="font-size: 50px;">Ox editor</h1>

  <p align="center" style="font-size: 20px;">
    Ox is a code editor that runs in your terminal.
    <br><br>
    <div align="center" style="display:inline;">
      <img src="https://i.postimg.cc/nrs9jksB/image.png" width="49%">
      <img src="https://i.postimg.cc/KcQ0nv1Y/image.png" width="49%">
    </div>
    <br>
</p>

[![Build Status](https://img.shields.io/github/forks/curlpipe/ox.svg?style=for-the-badge)](https://github.com/curlpipe/ox)
[![Build Status](https://img.shields.io/github/stars/curlpipe/ox.svg?style=for-the-badge)](https://github.com/curlpipe/ox)
[![License](https://img.shields.io/github/license/curlpipe/ox.svg?style=for-the-badge)](https://github.com/curlpipe/ox)

## About The Project

Ox is a code editor. It was written in Rust using ANSI escape sequences. It assists developers with programming by providing several tools to speed up and make programming easier and a refreshing alternative to heavily bloated and resource hungry editors such as VS Code and JetBrains. Ox is lightweight so it can be used on older computers.

Bear in mind, this is a personal project and is nowhere near ready to replace your existing tools just yet. 

It runs in the terminal and runs on platforms like Linux and macOS but doesn't work on Windows directly (it works if you use WSL) due to a lack of a good command line. There are many text editors out there and each one of them has their flaws and I hope to have a text editor that overcomes many of the burdens and issues.

Ox is not based on any other editor and has been built from the ground up without any base at all.

## What features does Ox have and why should I use it?
Ox aims to be an editor that takes features from some of the most popular editors out there, gaining the best of all worlds.

**Vim** http://vim.org: Vim provides a plugin system for adding features to it as it is very minimal and only provides basic text editing functionality by default. It is quite extensive and has its own programming language for configuring and writing plugins for it. It has a steep learning curve due to being a “modal” text editor, having special modes for editing text. Ox is easier to use than Vim because it doesn’t have modes where the keyboard is repurposed, however it takes the idea of being a keyboard-only editor and being able to act just like an IDE after some configuration.

**Nano** https://www.nano-editor.org/: Nano is an editor that is very simple to grasp due to its intuitive key bindings such as <kbd>Ctrl</kbd>+<kbd>S</kbd> to save and <kbd>Ctrl</kbd>+<kbd>?</kbd> for the help menu etc. Ox took the idea for the key bindings from this editor, they are simple to remember, <kbd>Ctrl</kbd>+<kbd>F</kbd> for “Find”, <kbd>Ctrl</kbd>+<kbd>Q</kbd> for “Quit”, meaning that Ox doesn’t have as steep a learning curve.

**Micro** https://micro-editor.github.io/: Micro has a plugin system that is programmed with a language called Lua however I can’t seem to find any up to date plugins for it and it lacks features such as a file tree. It is micro that inspired me to add mouse functionality and other features.

**Emacs** https://www.gnu.org/software/emacs/: Emacs is still actively used today due to its freedom to modify and change the source code. Ox took the idea for the customization and extensibility of Emacs and made a configuration system where you can change the colours and appearance of the editor.

**Xi** https://xi-editor.io/: Xi is also written in Rust but is purely a backend at the moment, I decided to make Ox both a frontend and a backend because Xi has many frontends, but most of them are broken and it lacks a lot of features.

**Kiro** https://github.com/rhysd/kiro-editor: Kiro is an amazing text editor written in Rust and adds features such as Unicode support, a nicer colour scheme and small things like resizing and it is a very inspiring editor. Ox took the ideas for the improvements from Kiro, however implemented them differently. Kiro’s source code also seems to be quite advanced in some areas, so I decided to keep Ox as simple as I could. 

### Built With

Ox is super minimal and aims to use as few dependencies as possible, allowing for rapid compile time and low risk of breakage.

* [Rust language](https://rust-lang.org)
* [Crossterm](https://github.com/crossterm-rs/crossterm)
* [Unicode-rs](https://unicode-rs.github.io/)
* [Clap](https://clap.rs/)
* [Regex](https://docs.rs/regex/1.3.9/regex/)
* [Ron](https://docs.rs/ron/0.6.2/ron/)
* [Serde](https://docs.rs/serde/1.0.116/serde/)
* [Shellexpand](https://github.com/netvl/shellexpand)

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

#### Icons
Ox uses NerdFonts to display icons. You can install nerdfonts from https://nerdfonts.com
If you use Arch Linux, you can install it by installing the package `ttf-nerd-fonts-symbols-mono`.
There is a potential that you will need to add it to your terminal emulator.

#### Manual

```sh
cargo install --git https://github.com/curlpipe/ox
```

#### Arch Linux

Install `ox-bin` or `ox-git` from the Arch User Repository.

That's all there is to it!

#### Fedora/CentOS

Install `ox` from the [COPR Repository](https://copr.fedorainfracloud.org/coprs/atim/ox/): 

```
sudo dnf copr enable atim/ox -y
sudo dnf install ox
```

#### Homebrew

Install `ox` from Homebrew core tap.

```sh
brew install ox
```

#### MacPorts

On macOS, you can install `ox` via [MacPorts](https://www.macports.org)

```sh
sudo port selfupdate
sudo port install ox
```

<!-- USAGE EXAMPLES -->
## Usage

#### Opening files in Ox

At the moment, you can open ox by using the command
```sh
ox
```

This will open up an empty document.

If you wish to open a file straight from the command line, you can run
```sh
ox /path/to/file
```
To open and edit a file.

You can also open a file from within Ox by using the <kbd>Ctrl + O</kbd> Key binding

If at any time, you wish to create a new file, you can use <kbd>Ctrl + N</kbd> to do so.

You can find more command line options for Ox by typing:
```sh
ox --help
```

#### Moving the cursor around

You can use the arrow keys to move the cursor around

You can also use:
 - <kbd>PageUp</kbd> - Go to the top of the document
 - <kbd>PageDown</kbd> - Go to the bottom of the document
 - <kbd>Home</kbd> - Go to the start of the current line
 - <kbd>End</kbd> - Go to the end of the current line

#### Editing the file

You can use the keys <kbd>Backspace</kbd> and <kbd>Return</kbd> / <kbd>Enter</kbd> as well as all the characters on your keyboard to edit files!


Ox is controlled via your keyboard shortcuts. Here are the default shortcuts that you can use:

| Keybinding  | What it does  |
| ------------ | ------------ |
| `Ctrl + Q`  | Exits the current tab or the editor if only one tab open.  | 
| `Ctrl + S`  | Saves the open file to the disk.  | 
| `Alt + S`   | Prompts you for a file name and saves it to disk as that file name.  | 
| `Ctrl + W`  | Saves all the currently open files to the disk. | 
| `Ctrl + N`  | Creates a new tab with a blank document.  | 
| `Ctrl + O`  | Prompts you for a file and opens that file in a new tab.  | 
| `Ctrl + F`  | Searches the document for a search query. Allows pressing of <kbd>↑</kbd> and <kbd>←</kbd> to move the cursor to the previous occurance fof the query and <kbd>↓</kbd> and <kbd>→</kbd> to move to the next occurance of the query. Press <kbd>Return</kbd> to cancel the search at the current cursor position or <kbd>Esc</kbd> to cancel the search and return to the initial location of the cursor. Note: this allows you to use regular expressions. | 
| `Ctrl + Z`  | Undoes your last action. The changes are committed to the undo stack every time you press the space bar, create / destroy a new line and when there is no activity after a certain period of time which can be used to capture points where you pause for thought or grab a coffee etc... | 
| `Ctrl + Y`  | Redoes your last action. The changes are committed to the undo stack every time you press the space bar, create / destroy a new line and when there is no activity after a certain period of time which can be used to capture points where you pause for thought or grab a coffee etc... | 
| `Ctrl + R`  | Allows replacing of occurances in the document. Uses the same keybindings as the search feature: <kbd>↑</kbd> and <kbd>←</kbd> to move the cursor to the previous occurance fof the query and <kbd>↓</kbd> and <kbd>→</kbd> to move to the next occurance of the query. You can also press <kbd>Return</kbd>, <kbd>y</kbd> or <kbd>Space</kbd> to carry out the replace action. To exit replace mode once you're finished, you can press <kbd>Esc</kbd> to cancel and return back to your initial cursor position. Note: this allows you to use regular expressions. | 
| `Ctrl + A`  | Carries out a batch replace option. It will prompt you for a target to replace and what you want to replace it with and will then replace every occurance in the document. Note: this allows you to use regular expressions. | 
| `Ctrl + Left`  | Navigates to the previous tab.  | 
| `Ctrl + Right`  | Navigates to the next tab.  | 
| `Alt + A`  | Focuses the command line. | 


#### Configuring Ox

Ox features a configuration system that allows modification and personalization of the editor.

By default, Ox will look for a file here: `$XDG_CONFIG_HOME/ox/ox.ron` or `~/.config/ox/ox.ron`.

Ox's configuration language is [RON](https://github.com/ron-rs/ron).

There is a default config in the 'config' folder. You will have to either download it and place it in the default config directory or create your own using the example ones as a reference.
If you don't have a config file, don't worry :), Ox will just ignore it if you don't have one.

If you wish to specify the configuration file path, you can do so using the '--config' option (or '-c' if you prefer). For Example:

```
ox --config /path/to/my_config.ron file_to_edit.txt
```

## Roadmap

You can see the `tasks.todo.md` file to see my full plans for the future of the editor!

Here is the current summary
 - [X] Initial Research (0.1.0, 0.1.1) [741 lines]
 - [X] Basic editing functions (0.2.0)
 - [X] Line numbers (0.2.0)
 - [X] Searching (0.2.0) [1040 lines]
 - [X] Undo and Redo (0.2.1) [1282 lines]
 - [X] Input bug (0.2.2) [1278 lines]
 - [X] Good command line interface (0.2.3)
 - [X] Config files (0.2.3)
 - [X] Replacing text (0.2.3) [1549 lines]
 - [X] Syntax highlighting (0.2.4) [1894 lines]
 - [X] Tabs for multitasking (0.2.5) [2050 lines]
 - [X] Macros (0.2.6) [3414 lines]
 - [X] Tweaks (0.2.7) [3241 lines]
 - [ ] Mouse support (0.2.8)
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

* [Curlpipe (Luke), for actually building Ox](https://github.com/curlpipe)
* [Spike, for the logo](https://github.com/spikecodes)
* The community, for the ideas and suggestions
