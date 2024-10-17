-- Define user-defined commands
commands = {
    ["reload"] = function(arguments)
        editor:reload_config()
        editor:display_info("Configuration file and plugins reloaded")
    end,
}

-- Pallette --
black = '#1e1e2e'
darkgrey = '#24273a'
lightgrey = '#303446'
verylightgrey = '#7f849c'
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
colors.line_number_fg = lightgrey
colors.line_number_bg = black

colors.status_bg = darkgrey
colors.status_fg = purple

colors.highlight = purple

colors.tab_inactive_bg = darkgrey
colors.tab_inactive_fg = white
colors.tab_active_bg = lightgrey
colors.tab_active_fg = purple

colors.info_bg = black
colors.info_fg = lightblue
colors.warning_bg = black
colors.warning_fg = yellow
colors.error_bg = black
colors.error_fg = red

colors.selection_bg = darkgrey
colors.selection_fg = lightblue

-- Configure Syntax Highlighting Colours --
syntax:set("string", green)  -- Strings, bright green
syntax:set("comment", verylightgrey)  -- Comments, light purple/gray
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
syntax:set("operator", verylightgrey)  -- Operators, light purple/gray
syntax:set("boolean", green)  -- Booleans, bright green
syntax:set("table", purple)  -- Tables, light purple
syntax:set("reference", pink)  -- References, vibrant pink
syntax:set("tag", darkblue)  -- Tags (e.g. HTML tags), cyan
syntax:set("heading", purple)  -- Headings, light purple
syntax:set("link", pink)  -- Links, vibrant pink
syntax:set("key", pink)  -- Keys, vibrant pink
syntax:set("quote", verylightgrey)  -- Quotes, light purple/gray
syntax:set("bold", red)  -- Bold text, cyan
syntax:set("italic", orange)  -- Italic text, cyan
syntax:set("block", lightblue)  -- Code blocks, cyan
syntax:set("image", lightblue)  -- Images in markup languages, cyan
syntax:set("list", green)  -- Lists, bright green
syntax:set("insertion", green)  -- Insertions (e.g. diff highlight), bright green
syntax:set("deletion", red)  -- Deletions (e.g. diff highlight), red

-- Import plugins (must be at the bottom of this file)
load_plugin("pairs.lua")
load_plugin("autoindent.lua")
