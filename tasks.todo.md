0.2.5 (Multitasking) { Manage multiple buffers in one instance }
- [ ] Tabs
  - [ ] Allow holding several documents
  - [ ] Set up current doc variable
  - [ ] Rewrite editor to use documents from current doc
  - [ ] Allow editor to move between different documents
  - [ ] Add tab line
- [ ] Save all
  - [ ] Write function
  - [ ] Set up keybinding
- [ ] Update hardcoded config file

0.2.6 (Macros) { Allow for more keybindings and operations }
- [ ] Macro system
  - [ ] Allow special command mode
  - [ ] Have a few example macros
    - [ ] Goto line number
    - [ ] Move forward a word
    - [ ] Move backward a word
    - [ ] Delete line
    - [ ] Move line
    - [ ] Move cursor
  - [ ] Allow binding of macros to some keys

0.2.7 (Small patches) { Small tweaks to make Ox more comfy }
- [ ] General Editing
  - [ ] File overwrite prevention
  - [ ] Better file save error messages
  - [ ] Ctrl + Z for undo
  - [ ] Save as sudo / read only files
  - [ ] Backup
  - [ ] Fix (0, 0) deletion issues
- [ ] Searching
  - [ ] Exit search when typing characters and catch up with events
- [ ] Undoing
  - [ ] Undoing to origin makes file not dirty
  - [ ] Undo / Redo patch limit to prevent high memory usage
  - [ ] Have a goto call for Undo / Redo

0.2.8 (Small patches #2) { Larger tweaks to make Ox more efficient and compatible }
- [ ] Themes
  - [ ] Small line specific retokenization for performance
  - [ ] Highlight search and replace messages
  - [ ] Transparent background
  - [ ] Improved language syntax highlighting support
  - [ ] Colour fallbacks
- [ ] Rewrite using crossterm for windows support and efficiency
  - [ ] Build RGB ansi code function
  - [ ] Fix unwrap on terminal size
  - [ ] Properly implement terminal resizing

0.2.9 (Mouse support) { To allow the mouse cursor to move the editor cursor & select text }
- [ ] Mouse selection support
  - [ ] Read mouse events
  - [ ] Move the cursor when clicking with mouse
  - [ ] Add selection mode to document
  - [ ] Allow text selection with the mouse cursor

0.3.0 (IDE level features) { Allow for IDE level features to smooth out development experience }
- [ ] Auto indentation 
  - [ ] Detect when to auto indent
  - [ ] Find the amount of tabs needed
  - [ ] Insert tabs there
- [ ] Prettier
  - [ ] Find a way to access the prettier API
  - [ ] Add a confirmation
- [ ] Linting
  - [ ] Read output from cargo's JSON
  - [ ] Display issues in the command line
  - [ ] highlight different colors for errors and warnings
  - [ ] Add support for Pylint readings

0.3.1 (IDE level features #2) { More IDE level features }
- [ ] Auto brackets
  - [ ] Automatically insert brackets on opening pair
    - [ ] <
    - [ ] (
    - [ ] [
    - [ ] {
    - [ ] "
    - [ ] '
    - [ ] `
    - [ ] |
  - [ ] Move them around when pressing enter

0.3.2 (IDE level features #3) { Even more IDE features }
- [ ] Auto complete
  - [ ] Get information from racer and display it in a menu
  - [ ] Add configuration entries for the autocomplete
  - [ ] Add support for file autocomplete too

0.3.3 (Navigation) { To help with navigating and managing your project from within the editor }
- [ ] File tree
  - [ ] Allow the document to be shifted up a bit
  - [ ] Render random text to the left of the document
  - [ ] List directory
  - [ ] Add cursor focus variable
  - [ ] Add mutable flags
  - [ ] Allow opening of files
  - [ ] Allow collapse and expand of files
  - [ ] Add sorting
  - [ ] Add file operations
    - [ ] New directory
    - [ ] New file
    - [ ] Delete directory
    - [ ] Delete file
    - [ ] Move file
    - [ ] Copy file

0.3.4 (Start up experience improvements) { Add a help menu and start menu and session saver }
- [ ] Start page
  - [ ] Store recently used documents
  - [ ] List them out
- [ ] Add ability to save sessions and load them from cli and start page
- [ ] Add detailed help menu / document / mode

Further ideas { Further fun ideas to look at }
- [ ] Automatically closing status line
- [ ] Split editors
- [ ] Terminal integration
- [ ] Package manager
- [ ] Stack overflow searcher
- [ ] Cheatsheet downloader
- [ ] Discord rich presence
- [ ] Live HTML editor
- [ ] HTML expansion like emmet
- [ ] Documentation viewer
- [ ] Todo list
- [ ] Pomodoro timer for work / rest balance
- [ ] Easter eggs
- [ ] Automated tests
- [ ] Theme builder
- [ ] Typing speed tests / statistics

0.1.1
- [X] Go to the next line at end of line
- [X] Go to the previous line at start of line
- [X] Solve unicode width issues
  - [X] Fix unicode cursor issues
  - [X] Add Home / End / PageUp / PageDown support
  - [X] Fix offset up/down issues
  - [X] Fix dodgy up/down unicode issues
- [X] Insertion of characters
- [X] Deletion of characters
  - [X] Deletion in middle of line
  - [X] Deletion at start of line
  - [X] Deletion at end of line
- [X] The enter key
  - [X] Enter at the start of a line
  - [X] Enter at the end of a line
  - [X] Enter in the middle of a line
- [X] Render tabs (4 spaces)
- [X] Save
- [X] Save as
- [X] Dirty files
- [X] Quit confirmation
- [X] Improve status bar
  - [X] File identification
  - [X] Cursor position
  - [X] File name
  - [X] File edited
  - [X] Current line
- [X] Revamp theme
- [X] Thorough commenting
- [X] Privatisation

0.2.0
- [X] Line numbers
- [X] Open document
- [X] New document
- [X] Search feature
  - [X] Searching on the same line as the cursor
    - [X] Backwards
    - [X] Forwards
  - [X] Searching forward by default
  - [X] Scroll offset with search
    - [X] Ensure the initial offset is saved
    - [X] Move the offset

0.2.1 (Undo & Redo)
- [X] Undo / Redo
  - [X] Undo
    - [X] Add event executor
  - [X] Set up EventStack
    - [X] Read Insertion
    - [X] Read Deletion
    - [X] Add reverse event lookup
    - [X] Read NewLine
      - [X] End of line
      - [X] Start of line
      - [X] Middle of line
    - [X] Support for offsets
    - [X] Read DeleteLine
      - [X] Middle of line
      - [X] Start of line
  - [X] Redo
    - [X] Set up seperate redo stack
    - [X] Clear redo stack on change
    - [X] Link up operations
  - [X] Set up smart undoing / redoing to undo by groups of common events
  - [X] Commit changes after inactivity period
  - [X] Refactor

0.2.2 (Input bug solving)
- [X] Fix clipboard bug

0.2.3 (Interface improvements)
- [X] CLAP cli
  - [X] Update documentation
- [X] Config file
  - [X] Add a default config path
  - [X] Allow a config argument
  - [X] Add hardcoded backup config file
  - [X] Read a config file and populate values
  - [X] Have a few example config files
  - [X] Left line number padding
- [X] Change default theme
- [X] Updated logo
- [X] Performance optimizations
- [X] No unwrap calls to reduce runtime errors
- [X] Proper status line and welcome message wrapping
- [X] Added left line number padding
- [X] Improved search command to show results in the middle of screen
- [X] Replace
  - [X] Fix X offset jumping
  - [X] Create replace all command
  - [X] Create replace some command
  - [X] Allow Regex expressions to be used?

0.2.4 (Syntax highlighting)
- [X] Add support for reading XDG config variable
- [X] Fix blank file runtime error
- [X] Use RON format instead
- [X] Syntax Highlighting
  - [X] Add basic Rust syntax
  - [X] Create a theme and regex definitions
  - [X] Implement basic colourization
  - [X] Fix overlapping tokens
  - [X] Fix fallout with unicode and trimming
    - [X] Trimming start
    - [X] Unicode
    - [X] Trimming end
  - [X] Finish Rust highlighting
  - [X] Add syntax to config file
  - [X] Allow for file type specific syntax highlighting
  - [X] Optimize
  - [X] Allow for multiline syntax highlighting
  - [X] Add Python
  - [X] Add Javascript
  - [X] Add C
  - [X] Add Ruby
  - [X] Add Crystal

