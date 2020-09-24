0.2.3 (Interface improvements)
- [ ] Replace
  - [ ] Fix X offset jumping
  - [ ] Create replace all command
  - [ ] Create replace some command
  - [ ] Allow Regex expressions to be used?
- [X] CLAP cli
  - [X] Update documentation
- [X] Config file
  - [X] Add a default config path
  - [X] Allow a config argument
  - [X] Add hardcoded backup config file
  - [X] Read a config file and populate values
  - [X] Have a few example config files

0.2.4 (Syntax highlighting)
- [ ] Syntax Highlighting
  - [ ] Set up basic syntax highlighting regex
  - [ ] Add external file reading
  - [ ] Add basic Rust syntax
  - [ ] Allow for multiline syntax highlighting
  - [ ] Finish Rust highlighting
  - [ ] Add Javascript
  - [ ] Add Python
  - [ ] Add Ruby
  - [ ] Add C

0.2.5 (Multitasking)
- [ ] Tabs
  - [ ] Allow holding several documents
  - [ ] Set up current doc variable
  - [ ] Rewrite editor to use documents from current doc
  - [ ] Allow editor to move between different documents
  - [ ] Add tab line
- [ ] Save all
  - [ ] Write function
  - [ ] Set up keybinding

0.2.6 (Extensibility)
- [ ] Macro system
  - [ ] Allow special command mode
  - [ ] Have a few example macros
  - [ ] Allow binding of macros to some keys

0.2.7 (Mouse support)
  - [ ] Mouse selection support
    - [ ] Read mouse events
    - [ ] Move the cursor when clicking with mouse
    - [ ] Add selection mode to document
    - [ ] Allow text selection with the mouse cursor

0.3.0 (IDE level features)
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

0.3.1 (IDE level features #2)
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

0.3.2 (IDE level features #3)
- [ ] Auto complete
  - [ ] Get information from racer and display it in a menu
  - [ ] Add configuration entries for the autocomplete
  - [ ] Add support for file autocomplete too

0.3.4 (Navigation)
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

0.3.5 (Start up experience improvements)
- [ ] Start page
  - [ ] Store recently used documents
  - [ ] List them out
- [ ] Add ability to save sessions and load them from cli and start page

Further ideas
- [ ] Split editors
- [ ] Terminal integration
- [ ] Package manager
- [ ] Stack overflow searcher
- [ ] Cheatsheet downloader
- [ ] Discord rich presence
- [ ] Live HTML editor
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

