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

## About The Project

Ox is a text editor that differs to many text editors that are already out there. 
It is programmed in Rust to ensure that it is memory and thread safe as well as light and quick.

Ox is not based off any other editor and has been built from the ground up without any base at all.

## What features does Ox have and why should I use it?

Ox aims to be an editor that takes features from some of the most popular editors out there, gaining the best of all worlds.

#### Ox vs Vim

Vim is a text editor that came about in 1991 and derived from Vi.

 - Vim has its own scripting language
    - Ox doesn't have it's own scripting language in order to stay light and remain fast.
    - Instead, modifications can be applied through editing the Rust code directly to take advantage of the optimisation. 
 - Vim has a plugin system
    - Vim's plugin system is great but there is a major flaw with it because many plugins are poor quality and conflict with each other.
    - Ox implements the majority of plugins that you'd need directly into the editor, ensuring that they work well and are effcient.
 - Vim is a modal editor
    - Vim is modal text editor meaning that they have modes that repurpose your keyboard depending on what mode you are in.
    - Ox isn't modal in the way Vim is because pressing <kbd>esc</kbd> over and over again can become incredibly labour intensive and doesn't flow very well.

#### Ox vs Nano

Nano is an editor from around 1999 and has the advantage of being very easy to use.

 - Nano is easy to use and intuitive
    - Nano uses keybindings on Ctrl to manage the editor.
    - Ox uses Ctrl keybindings that the majority of GUI text editors use, just like nano. This makes it easy to use.
 - Nano is simple
    - Nano can be used to edit text and that's about it, it does one thing well.
    - Ox is more modern than Nano because it implements many features that Nano is unable to get, making it a great and easy replacement for nano.

### Built With

Ox is super minimal and aims to use as few dependencies as possible, allowing for rapid compile time and low risk of breakage.

* [Rust language](https://rust-lang.org)
* [Termion](https://github.com/redox-os/termion)

<!-- GETTING STARTED -->
## Getting Started

You can currently only build Ox from source.
While this may sound daunting to many people, it really isn't that hard!

### Prerequisites

Because Ox is written in Rust, you must have a modern and working version of `rustc` and `cargo`.
On Arch Linux, you can run this command:
```sh
sudo pacman -S rust
```

If you are not using Arch, you can easily set it up on other distros by running the distro-neutral command:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
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

This will open up an empty buffer.

If you wish to open a file straight from the command line, you can run
```
ox /path/to/file
```
To open and edit a file.

<!-- 
You can also open a file from within Ox by using the <kbd>Ctrl + O</kbd> Key binding
If at any time, you wish to create a new file, you can use <kbd>Ctrl + N</kbd> to do so.
-->

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
The simple keyboard shortcut of <kbd>Ctrl + S</kbd> can be used to save the current file.

The simple keyboard shortcut of <kbd>Ctrl + W</kbd> can be used to "save as" the current file to a specific path.

#### Closing Ox
You can use the keybinding <kbd>Ctrl + Q</kbd> to exit Ox.

## Roadmap

You can see the `tasks.todo.md` file to see my plans for the future of the editor!

## License

Distributed under the GNU GPLv2 License. See `LICENSE` for more information.

## Contact
You can contact me on Discord at `curlpipe#1496`. I'll be happy to answer any questions you may have!

## Acknowledgements

* [Curlpipe](https://github.com/curlpipe)
