<!-- PROJECT LOGO -->
<br />
<p align="center">
  <a href="https://github.com/curlpipe/ox/">
    <img src="assets/logo.png" alt="Logo" width="200" height="200">
  </a>

  <h1 align="center" style="font-size: 50px;">Ox editor</h1>

  <p align="center" style="font-size: 20px;">
    Ox is a fast text editor that runs in your terminal.
    <br><br>
    <img src="https://i.postimg.cc/hGRgs97Z/image.png">
    <br>
</p>

<!-- TABLE OF CONTENTS -->
## Table of Contents

* [About the Project](#about-the-project)
    * [Built With](#built-with)
* [Getting Started](#getting-started)
    * [Prerequisites](#prerequisites)
    * [Installation](#installation)
* [Usage](#usage)
* [Roadmap](#roadmap)
* [License](#license)
* [Contact](#contact)
* [Acknowledgements](#acknowledgements)

<!-- ABOUT THE PROJECT -->
## About The Project

Ox is a text editor that differs to many text editors that are already out there. It's is built in the Rust language to ensure that it almost never crashes, leaks or fails. It is different from Vim due to its non-modal approach and uses keybindings just like nano does but has a lot more features including a secret command mode similar to Vim. Ox doesn't have a plugin system nor a special language like vimscript because that gives the potential for bugs errors and unintended events, instead of having plugins, one can either edit the source code directly and build using the simple instructions followed by submitting a pull request into Ox or uploading your own distrobution to a git repository! This ensures that the plugins and modifications are optimised and fast and don't require learning a new editor-specific language.

### Built With

Ox is super minimal and aims to use as few dependencies as possible, allowing for rapid compile time and low risk of breakage.

* [Rust language](https://rust-lang.org)
* [Termion](https://gitlab.redox-os.org/redox-os/termion)

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

#### Opening Ox
At the moment, you can open ox by using the command
```
ox
```

This will open up an empty buffer.

If you wish to open a file, you can run
```
ox /path/to/file
```
To open and edit a file.

#### Moving the cursor around
You can use the arrow keys to move the cursor around

You can also use:
 - `PageUp` - Go to the top of the window
 - `PageDown` - Go to the bottom of the window
 - `Home` - Go to the start of the current line
 - `End` - Go to the end of the current line

#### Editing the file
You can use the keys `Backspace` and `Return` / `Enter` as well as all the characters on your keyboard to edit the opened file!

#### Saving the file
The simple keyboard shortcut of `Ctrl + S` can be used to save the current file.

#### Closing Ox
You can use the keybinding `Ctrl + Q` to exit Ox.

<!-- ROADMAP -->
## Roadmap

You can see the `roadmap.todo.md` file to see my plans for the future of the editor!

<!-- LICENSE -->
## License

Distributed under the GNU GPLv2 License. See `LICENSE` for more information.

<!-- CONTACT -->
## Contact
You can contact me on Discord at `curlpipe#1496`. I'll be happy to answer any questions you may have!

<!-- ACKNOWLEDGEMENTS -->
## Acknowledgements

* [Curlpipe](https://github.com/curlpipe)
