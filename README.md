<!-- Heading -->
<br />
<p align="center">
  <a href="https://github.com/curlpipe/ox/">
    <img src="assets/logo.png" alt="Logo" width="150" height="150">
  </a>

  <h1 align="center" style="font-size: 50px;">Ox editor</h1>

  <p align="center" style="font-size: 20px;">
    The simple but flexible text editor
    <br><br>
    <div align="center" style="display:inline;">
      <img src="/assets/showcase.gif?raw=true" width="100%">
    </div>
    <br>
</p>


![Build Status](https://img.shields.io/github/forks/curlpipe/ox.svg?style=for-the-badge)
![Build Status](https://img.shields.io/github/stars/curlpipe/ox.svg?style=for-the-badge)
![License](https://img.shields.io/github/license/curlpipe/ox.svg?style=for-the-badge)

[About](#about)    -    [Installation](#installation)    -    [Quick Start Guide](#quick-start-guide)

## About

Ox is a text editor that can be used to write everything from text to code.

If you're looking for a text editor that...
1. :feather: Is lightweight and efficient
2. :wrench: Can be configured to your heart's content
3. :package: Has useful features out of the box and a library of plug-ins for everything else

...then Ox is right up your street

It runs in your terminal as a text-user-interface, just like vim, nano and micro, however, it is not based on any existing editors and has been built from the ground up.

It works best on linux, but macOS and Windows are also supported.

## Selling Points

### Strong configurability

- :electric_plug: Plug-In system where you can write your own plug-ins or choose from pre-existing ones
    - 💬 Discord RPC
    - 📗 Git integration
    - 🕸️ Emmet and HTML viewer
    - ⏲️ Pomodoro timer and todo list tracker
    - 🤖 AI code & advice
- :wrench: Configure everything including colours, key bindings and behaviours
- :moon: Write Lua code for configuration
- :handshake: A set-up wizard to make Ox yours from the start

### Out of the box features

- :paintbrush: Syntax highlighting
- :arrow_right_hook: Undo and redo
- :mag: Search and replace text
- :file_folder: Opening multiple files at once
- :eye: UI that shows you the state of the editor and file
- :computer_mouse: You can move the cursor and select text with your mouse
- :writing_hand: Convenient shortcuts when writing code
- :crossed_swords: Multi-editing features such as multiple cursors and recordable macros
- :window: Splits to view multiple documents on the same screen at the same time
- :file_cabinet: File tree to view, open, create, delete, copy and move files
- :keyboard: Access to terminals within the editor

### Detailed Documentation

Become a power user and take advantage of everything on offer.

Found on the [wiki page](https://github.com/curlpipe/ox/wiki/)

This will take you step-by-step in great detail through 6 different stages:

1. **Installation** - advice and how-tos on installation
2. **Configuring** - changing the layout, adding to and changing the syntax highlighting
3. **General Editing** - editing a document and controlling the editor
4. **Command Line** - using the command line interface
5. **Plugins** - installing or uninstalling community plug-ins and writing or distributing your own plug-ins
6. **Roadmap** - planned features

## Installation

To get started, please click on your operating system

- :penguin: [Linux](#linux)
- :window: [Windows](#windows)
- :apple: [MacOS](#macos)

### Linux

Here are the list of available methods for installing on Linux:
- [Manually](#manual)
- [Binary](#binaries)
- [Arch Linux](#arch-linux)
- [Fedora](#fedora)
- [Debian / Ubuntu](#debian)

#### Arch Linux

Install one of the following from the AUR:
- `ox-bin` - install the pre-compiled binary (fastest)
- `ox-git` - compile from source (best)

#### Fedora

You can find an RPM in the [releases page](https://github.com/curlpipe/ox/releases)

Install using the following command:

```sh
sudo dnf install /path/to/rpm/file
```

#### Debian

You can find a deb file in the [releases page](https://github.com/curlpipe/ox/releases)

Install using the following command:

```sh
sudo dpkg -i /path/to/deb/file
```

### Windows

Here are the list of available methods for installing on Windows:
- [Manually (best)](#manual)
- [Binary](#binaries)



### MacOS

Here are the list of available methods for installing on macOS:
- [Manually](#manual)
- [Binary](#binaries)
- [Homebrew](#homebrew)
- [MacPorts](#macports)

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

### Binaries

There are precompiled binaries available for all platforms in the [releases page](https://github.com/curlpipe/ox/releases).

- For Linux: download the `ox` executable and copy it to `/usr/bin/ox`, then run `sudo chmod +x /usr/bin/ox`
- For MacOS: download the `ox-macos` executable and copy it to `/usr/local/bin/ox`, then run `sudo chmod +x /usr/local/bin/ox`
- For Windows: download the `ox.exe` executable and copy it into a location in `PATH` see [this guide](https://zwbetz.com/how-to-add-a-binary-to-your-path-on-macos-linux-windows/#windows-cli) for how to do it

### Manual

This is the absolute best way to install Ox, it will ensure you always have the latest version and everything works for your system.

You must have a working installation of the Rust compiler to use this method. Visit the website for [rustup](https://rustup.rs/) and follow the instructions there for your operating system.

Now with a working version of rust, you can run the command:

```sh
cargo install --git https://github.com/curlpipe/ox
```

This will take at worst around 2 minutes. On some more modern systems, it will take around 30 seconds.

Please note that you should add `.cargo/bin` to your path, which is where the `ox` executable will live, although `rustup` will likely do that for you, so no need to worry too much.

## Quick Start Guide

Once you have installed Ox, it's time to get started.

### Set-Up

You can open Ox using the command

```sh
ox
```

At first, if you don't have a configuration file in place, Ox will walk you through a set-up wizard.

When you've completed it, you should be greeted by ox itself, with an empty, unnamed document.

At the top is your tab line, this shows you files that are open.

At the bottom is your status line, this shows you the state of the editor.

At the far bottom is your feedback line, you'll see information, warnings and errors appear there.

### Editing

Toggle the built-in help message using <kbd>Ctrl</kbd> + <kbd>H</kbd>. You can press <kbd>Ctrl</kbd> + <kbd>H</kbd> again to hide this message if it gets in the way. This should introduce you to most of the key bindings on offer.

Ox isn't a modal text editor, so you can begin typing straight away. Give it a go! Type in letters and numbers, delete with backspace, indent with tab, break up lines with the enter key.

Move your cursor by clicking, or using the arrow keys. You can also click and drag to select text.

If you modify a file, you may notice a `[+]` symbol, this means the file open in the editor differs from it's state on the disk. Save the file to update it on the disk and this indicator will disappear.

Because the file we're editing is new and doesn't have a name, you'll need to save as using <kbd>Alt</kbd> + <kbd>S</kbd> and give it a name.

Now, if you were to edit it again, because it is on the disk and has a name, you can use the standard <kbd>Ctrl</kbd> + <kbd>S</kbd> to save it.

You can open files through <kbd>Ctrl</kbd> + <kbd>O</kbd> - try opening a file!

If you modify it you can then use the standard <kbd>Ctrl</kbd> + <kbd>S</kbd> to update it on the disk, as this file already exists.

When mutltiple files are open, you can navigate back and forth using <kbd>Alt</kbd> + <kbd>Left</kbd> and <kbd>Alt</kbd> + <kbd>Right</kbd>

Once you're done with a file, you can use <kbd>Ctrl</kbd> + <kbd>Q</kbd> to quit out of it.

If all files are closed, Ox will exit.

If you're interested in finding out all the key bindings on offer, click [here](https://github.com/curlpipe/ox/wiki/General-editing#quick-reference)

Now you've exited Ox, let's check out some command line options.

### CLI

You can open files straight from the command line like this:

```sh
ox /path/to/file1 /path/to/file2
```

If you try to open a file that doesn't actually exist, Ox will open it in memory, and as soon as you save, it will save it will create it for you.

See more information regarding command line options using the command.

```sh
ox --help
```

This provides everything you need to know to do some basic editing, but there is so much more you can take advantage of, from plug-ins to opening multiple files on the same screen, to using the built-in terminal and using the file tree to manage your project.

If you are curious in learning more, click [here](https://github.com/curlpipe/ox/wiki) to access the wiki where you will be introduced to all the wide range of features and really make your experience smooth like butter 🧈.

## License

Distributed under the GNU GPLv2 License. See `LICENSE` for more information.

## Contact

You can contact me on Discord at my handle `curlpipe`. I'll be happy to answer any questions you may have!

## Acknowledgements

- [Luke (curlpipe)](https://github.com/curlpipe), principal developer
- [HKalbasi](https://github.com/HKalbasi), key contributor
- [Spike (spikecodes)](https://github.com/spikecodes), for the logo
- The community, for the stars, ideas, suggestions and bug reports

The creators of the following technologies:

* [Rust language](https://rust-lang.org)
* [Kaolinite](https://github.com/curlpipe/kaolinite)
* [Synoptic](https://github.com/curlpipe/synoptic)
* [Crossterm](https://github.com/crossterm-rs/crossterm)
* [Mlua](https://github.com/mlua-rs/mlua)
* [Jargon-args](https://crates.io/crates/jargon-args)
* [Regex](https://docs.rs/regex/1.3.9/regex/)
* [Unicode-rs](https://unicode-rs.github.io/)
* [Quick-error](https://github.com/tailhook/quick-error)
* [Shellexpand](https://github.com/netvl/shellexpand)

