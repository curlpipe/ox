# Ox editor

## What is Ox?

Ox is an efficient, fast and safe text editor that runs in your command line. It is inspired by projects like `kilo`, `hecto`, `kiro`, `vim` and `emacs`. I want everything built in to Ox, e.g. a file tree, automatic formatting plugins and other features that developers can't live without. It caters to people who are used to nano as well as catering to people who are used to modal editors like vim. Ox is not a modal editor because I find switching modes annoying. Ox uses a very speedy and more ergonomic method to run custom macros and commands.

## Justify Ox's existence
Ox doesn't have a plugin system because plugin systems allow for code that can be broken, has the potential to be slow, may not have good documentation and just be super janky. With Ox, you can edit your config file and give it to anyone else without needing them to install some horrific plugin management system and then use that to install a boatload of plugins from git (super janky).

Here are the closest projects to what Ox achieves:

 - `Xi` - Xi is a new editor written in Rust and it is backed by Google. Xi sounds good but I don't feel like it can be used as a proper editor yet and has a million and one different concepts to get your head around. Also features a plugin system (bad idea).
 - `Vim / Neovim` - Vim is a very robust editor but the minimalism it has is its major downfall. It provides a language called `vimscript` which is not only slow but requires you to learn an entirely new language. It also opens up window for breakage and spamming of errors all over your buffer. Ox implements everything you'll ever need right into itself. It implements it in one effective and efficient way that integrates amazingly with the editor without having 100 different types of plugins and confusing users on which one to use.
 - `Emacs` - Emacs is an incredibly advanced editor that can pretty much do everything including playing tetris. While you would be able to do this by opening a terminal in Ox, Ox's main focus is to be a text editor and not an entire user interface. Also there are about 1 million different types of Emacs interfaces compared to just one Ox interface.
 - `Nano` - This is just too simple for me, however I love the simplicity and therefore Ox works very similar to this but it has a ton more plugins.
 - `Kiro` - This is a very good editor, it supports a lot of things including UTF-8 and in fact is an outstanding example of what `kilo` can become if you edit it a bit. However this doesn't have seamless mouse integration and lacks quite a few features in my opinion.
 - `Kilo / Hecto` - These are probably some of the most simple editors in existence. They are designed to teach people how to bulid an editor and I was incredibly inspired by them.
 
## How to use it?
You can customise Ox's key bindings in its config file, in your config file you'll be able to find these keybindings:

I've grouped commands to each modifier key:
 - `Ctrl` is the key for managing the editor (e.g. saving files, opening files, creating new buffers)
 - `Alt` is for custom commands and macros that you can add to your editor to make it super comfy

 - `Ctrl + q`: Quit the editor (spam it to force exit the editor)
 - `Ctrl + Shift + q`: Quit the current buffer

 - `Ctrl + n`: Create a new buffer in a new tab
 - `Ctrl + Shift + n`: Create a new buffer in the current tab

 - `Ctrl + w`: Write the current buffer
 - `Ctrl + Shift + w`: Write all of the open buffers

 - `Ctrl + c`: Copy text within the current buffer
 - `Ctrl + v`: Paste text within the current buffer
 - `Ctrl + x`: Cut text within the current buffer

 - `Ctrl + f`: Search for text in the buffer

 - `Ctrl + u`: Undo
 - `Ctrl + r`: Redo

 - `Ctrl + ->`: Move to the tab
 - `Ctrl + <-`: Move to the previous tab

Hey! thats pretty intuitive!

How about the `Alt` key?
Usually the `Alt` key is for user-programmed macros in your config file but i've provided some built-in macros (they come after this section)

 - `Alt+A`: Access special "vim command" mode to execute custom macros
 - `Alt+J`: Move cursor left
 - `Alt+K`: Move cursor up
 - `Alt+L`: Move cursor down
 - `Alt+;`: Move cursor right

Hey! thats pretty nice! So what about those built-in macros?
After pressing `Alt-A` you can access a special command mode where you can type the name of your macro and provide some arguments, here are the built-in ones:

 - `r [REGEX/TEXT] [TEXT]`: Replace the regex or text with other text

 - `d [REGEX/TEXT]`: Delete the first occurance of a regex / text
 - `da [REGEX/TEXT]`: Purge every occurance of a regex/text

 - `ya`: Copy the entire buffer into the clipboard
 - `xa`: Cut the entire buffer into the clipboard

 - `e`: Open up the config file in a new buffer
