
-- Pallette --
black = 'black'
darkgrey = 'darkgrey'
red = 'red'
darkred = 'darkred'
green = 'green'
darkgreen = 'darkgreen'
yellow = 'yellow'
darkyellow = 'darkyellow'
blue = 'blue'
darkblue = 'darkblue'
magenta = 'magenta'
darkmagenta = 'darkmagenta'
cyan = 'cyan'
darkcyan = 'darkcyan'
white = 'white'
grey = 'grey'
transparent = 'transparent'


-- Configure Colours --
colors.editor_bg = black
colors.editor_fg = white
colors.line_number_fg = grey
colors.line_number_bg = black

colors.status_bg = darkblue
colors.status_fg = white

colors.highlight = cyan

colors.tab_inactive_bg = black
colors.tab_inactive_fg = white
colors.tab_active_bg = darkblue
colors.tab_active_fg = white

colors.split_bg = black
colors.split_fg = darkblue

colors.info_bg = black
colors.info_fg = cyan
colors.warning_bg = black
colors.warning_fg = yellow
colors.error_bg = black
colors.error_fg = red

colors.selection_bg = darkgrey
colors.selection_fg = cyan

colors.file_tree_bg = black
colors.file_tree_fg = white
colors.file_tree_selection_bg = darkgrey
colors.file_tree_selection_fg = cyan

colors.file_tree_red = red
colors.file_tree_orange = darkyellow
colors.file_tree_yellow = yellow
colors.file_tree_green = green
colors.file_tree_lightblue = blue
colors.file_tree_darkblue = darkblue
colors.file_tree_purple = darkmagenta
colors.file_tree_pink = magenta
colors.file_tree_brown = darkred
colors.file_tree_grey = grey

-- Configure Syntax Highlighting Colours --
syntax:set("string", green)  -- Strings, bright green
syntax:set("comment", darkgrey)  -- Comments, light purple/gray
syntax:set("digit", red)  -- Digits, cyan
syntax:set("keyword", darkmagenta)  -- Keywords, vibrant pink
syntax:set("attribute", blue)  -- Attributes, cyan
syntax:set("character", darkblue)  -- Characters, cyan
syntax:set("type", yellow)  -- Types, light purple
syntax:set("function", darkblue)  -- Function names, light purple
syntax:set("header", blue)  -- Headers, cyan
syntax:set("macro", red)  -- Macros, red
syntax:set("namespace", darkblue)  -- Namespaces, light purple
syntax:set("struct", magenta)  -- Structs, classes, and enums, light purple
syntax:set("operator", grey)  -- Operators, light purple/gray
syntax:set("boolean", green)  -- Booleans, bright green
syntax:set("table", darkmagenta)  -- Tables, light purple
syntax:set("reference", magenta)  -- References, vibrant pink
syntax:set("tag", darkblue)  -- Tags (e.g. HTML tags), cyan
syntax:set("heading", darkmagenta)  -- Headings, light purple
syntax:set("link", magenta)  -- Links, vibrant pink
syntax:set("key", magenta)  -- Keys, vibrant pink
syntax:set("quote", grey)  -- Quotes, light purple/gray
syntax:set("bold", red)  -- Bold text, cyan
syntax:set("italic", darkyellow)  -- Italic text, cyan
syntax:set("block", blue)  -- Code blocks, cyan
syntax:set("image", blue)  -- Images in markup languages, cyan
syntax:set("list", green)  -- Lists, bright green
syntax:set("insertion", green)  -- Insertions (e.g. diff highlight), bright green
syntax:set("deletion", red)  -- Deletions (e.g. diff highlight), red
