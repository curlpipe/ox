
-- Pallette --
black = '#232336'
grey1 = '#353552'
grey2 = '#484863'
grey3 = '#A1A7C7'
white = '#cdd6f4'
brown = '#dd7878'
red = '#ed8796'
orange = '#f5a97f'
yellow = '#eed49f'
green = '#a6da95'
lightblue = '#7dc4e4'
darkblue = '#8aadf4'
purple = '#c6a0f6'
pink = '#f5bde6'

-- Configure Colours --
colors.editor_bg = black
colors.editor_fg = white
colors.line_number_fg = grey2
colors.line_number_bg = black

colors.status_bg = grey1
colors.status_fg = orange

colors.highlight = orange

colors.tab_inactive_bg = grey1
colors.tab_inactive_fg = white
colors.tab_active_bg = grey2
colors.tab_active_fg = orange

colors.info_bg = black
colors.info_fg = darkblue
colors.warning_bg = black
colors.warning_fg = yellow
colors.error_bg = black
colors.error_fg = red

colors.selection_bg = grey1
colors.selection_fg = lightblue

-- Configure Syntax Highlighting Colours --
syntax:set("string", lightblue)  -- Strings, bright green
syntax:set("comment", grey3)  -- Comments, light purple/gray
syntax:set("digit", lightblue)  -- Digits, cyan
syntax:set("keyword", orange)  -- Keywords, vibrant pink
syntax:set("attribute", darkblue)  -- Attributes, cyan
syntax:set("character", orange)  -- Characters, cyan
syntax:set("type", pink)  -- Types, light purple
syntax:set("function", red)  -- Function names, light purple
syntax:set("header", darkblue)  -- Headers, cyan
syntax:set("macro", darkblue)  -- Macros, red
syntax:set("namespace", pink)  -- Namespaces, light purple
syntax:set("struct", yellow)  -- Structs, classes, and enums, light purple
syntax:set("operator", darkblue)  -- Operators, light purple/gray
syntax:set("boolean", pink)  -- Booleans, bright green
syntax:set("table", yellow)  -- Tables, light purple
syntax:set("reference", yellow)  -- References, vibrant pink
syntax:set("tag", orange)  -- Tags (e.g. HTML tags), cyan
syntax:set("heading", red)  -- Headings, light purple
syntax:set("link", darkblue)  -- Links, vibrant pink
syntax:set("key", yellow)  -- Keys, vibrant pink
syntax:set("quote", grey3)  -- Quotes, light purple/gray
syntax:set("bold", red)  -- Bold text, cyan
syntax:set("italic", orange)  -- Italic text, cyan
syntax:set("block", red)  -- Code blocks, cyan
syntax:set("image", red)  -- Images in markup languages, cyan
syntax:set("list", red)  -- Lists, bright green
syntax:set("insertion", green)  -- Insertions (e.g. diff highlight), bright green
syntax:set("deletion", red)  -- Deletions (e.g. diff highlight), red
