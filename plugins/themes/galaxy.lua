
-- Pallette --
black = '#1e1e2e'
grey1 = '#24273a'
grey2 = '#303446'
grey3 = '#7f849c'
white = '#cdd6f4'
brown = '#f2cdcd'
red = '#f38ba8'
orange = '#fab387'
yellow = '#f9e2af'
green = '#a6e3a1'
lightblue = '#89dceb'
darkblue = '#89b4fa'
purple = '#cba6f7'
pink = '#f5c2e7'

-- Configure Colours --
colors.editor_bg = black
colors.editor_fg = white
colors.line_number_fg = grey2
colors.line_number_bg = black

colors.status_bg = grey1
colors.status_fg = purple

colors.highlight = purple

colors.tab_inactive_bg = grey1
colors.tab_inactive_fg = white
colors.tab_active_bg = grey2
colors.tab_active_fg = purple

colors.split_bg = black
colors.split_fg = purple

colors.info_bg = black
colors.info_fg = darkblue
colors.warning_bg = black
colors.warning_fg = yellow
colors.error_bg = black
colors.error_fg = red

colors.selection_bg = grey1
colors.selection_fg = lightblue

colors.file_tree_bg = black
colors.file_tree_fg = white
colors.file_tree_selection_bg = purple
colors.file_tree_selection_fg = black

colors.file_tree_red = {247, 156, 156}
colors.file_tree_orange = {247, 165, 156}
colors.file_tree_yellow = {247, 226, 156}
colors.file_tree_green = {191, 247, 156}
colors.file_tree_lightblue = {156, 214, 247}
colors.file_tree_darkblue = {156, 163, 247}
colors.file_tree_purple = {197, 156, 247}
colors.file_tree_pink = {246, 156, 247}
colors.file_tree_brown = {163, 118, 118}
colors.file_tree_grey = {160, 157, 191}

-- Configure Syntax Highlighting Colours --
syntax:set("string", green)  -- Strings, bright green
syntax:set("comment", grey3)  -- Comments, light purple/gray
syntax:set("digit", red)  -- Digits, cyan
syntax:set("keyword", purple)  -- Keywords, vibrant pink
syntax:set("attribute", lightblue)  -- Attributes, cyan
syntax:set("character", darkblue)  -- Characters, cyan
syntax:set("type", yellow)  -- Types, light purple
syntax:set("function", darkblue)  -- Function names, light purple
syntax:set("header", lightblue)  -- Headers, cyan
syntax:set("macro", red)  -- Macros, red
syntax:set("namespace", darkblue)  -- Namespaces, light purple
syntax:set("struct", pink)  -- Structs, classes, and enums, light purple
syntax:set("operator", grey3)  -- Operators, light purple/gray
syntax:set("boolean", green)  -- Booleans, bright green
syntax:set("table", purple)  -- Tables, light purple
syntax:set("reference", pink)  -- References, vibrant pink
syntax:set("tag", darkblue)  -- Tags (e.g. HTML tags), cyan
syntax:set("heading", purple)  -- Headings, light purple
syntax:set("link", pink)  -- Links, vibrant pink
syntax:set("key", pink)  -- Keys, vibrant pink
syntax:set("quote", grey3)  -- Quotes, light purple/gray
syntax:set("bold", red)  -- Bold text, cyan
syntax:set("italic", orange)  -- Italic text, cyan
syntax:set("block", lightblue)  -- Code blocks, cyan
syntax:set("image", lightblue)  -- Images in markup languages, cyan
syntax:set("list", green)  -- Lists, bright green
syntax:set("insertion", green)  -- Insertions (e.g. diff highlight), bright green
syntax:set("deletion", red)  -- Deletions (e.g. diff highlight), red
