
-- Palette --
background = '#191622'  -- Background Color
background2 = '#201B2D' -- Secondary background
background3 = '#15121E' -- Tertiary background
selection = '#41414D'  -- Selection Color
foreground = '#E1E1E6'  -- Foreground Color
comment = '#483C67'  -- Comment Color
cyan = '#78D1E1'  -- Cyan Color
green = '#67E480'  -- Green Color
orange = '#E89E64'  -- Orange Color
pink = '#FF79C6'  -- Pink Color
purple = '#988BC7'  -- Purple Color
red = '#E96379'  -- Red Color
yellow = '#E7DE79'  -- Yellow Color
white = '#FFFFFF'

-- Configure Colours --
colors.editor_bg = background
colors.editor_fg = foreground
colors.line_number_fg = comment
colors.line_number_bg = background

colors.status_bg = background3
colors.status_fg = white

colors.highlight = pink

colors.tab_inactive_bg = background3
colors.tab_inactive_fg = foreground
colors.tab_active_bg = background
colors.tab_active_fg = pink

colors.split_bg = background
colors.split_fg = white

colors.info_bg = background
colors.info_fg = cyan
colors.warning_bg = background
colors.warning_fg = yellow
colors.error_bg = background
colors.error_fg = red

colors.selection_bg = selection
colors.selection_fg = foreground

colors.file_tree_bg = background
colors.file_tree_fg = foreground
colors.file_tree_selection_bg = pink
colors.file_tree_selection_fg = background

colors.file_tree_red = {255, 128, 128}
colors.file_tree_orange = {255, 155, 128}
colors.file_tree_yellow = {255, 204, 128}
colors.file_tree_green = {196, 255, 128}
colors.file_tree_lightblue = {128, 236, 255}
colors.file_tree_darkblue = {128, 147, 255}
colors.file_tree_purple = {204, 128, 255}
colors.file_tree_pink = {255, 128, 200}
colors.file_tree_brown = {163, 108, 108}
colors.file_tree_grey = {155, 153, 176}

-- Configure Syntax Highlighting Colours --
syntax:set("string", yellow)  -- Strings, fresh green
syntax:set("comment", comment)  -- Comments, muted and subtle
syntax:set("digit", cyan)  -- Digits, cool cyan
syntax:set("keyword", pink)  -- Keywords, vibrant pink
syntax:set("attribute", orange)  -- Attributes, warm orange
syntax:set("character", yellow)  -- Characters, cheerful yellow
syntax:set("type", purple)  -- Types, elegant purple
syntax:set("function", green)  -- Function names, clean cyan
syntax:set("header", yellow)  -- Headers, bright yellow
syntax:set("macro", red)  -- Macros, bold red
syntax:set("namespace", purple)  -- Namespaces, subtle purple
syntax:set("struct", orange)  -- Structs, warm orange
syntax:set("operator", pink)  -- Operators, striking pink
syntax:set("boolean", green)  -- Booleans, fresh green
syntax:set("table", purple)  -- Tables, structured purple
syntax:set("reference", pink)  -- References, vibrant orange
syntax:set("tag", cyan)  -- Tags (e.g., HTML tags), calming cyan
syntax:set("heading", pink)  -- Headings, vibrant pink
syntax:set("link", cyan)  -- Links, attention-grabbing cyan
syntax:set("key", green)  -- Keys, fresh green
syntax:set("quote", comment)  -- Quotes, subtle comment color
syntax:set("bold", yellow)  -- Bold text, cheerful yellow
syntax:set("italic", purple)  -- Italic text, elegant purple
syntax:set("block", cyan)  -- Code blocks, cool cyan
syntax:set("image", orange)  -- Images in markup languages, warm orange
syntax:set("list", green)  -- Lists, structured green
syntax:set("insertion", green)  -- Insertions (e.g., diff highlight), vibrant green
syntax:set("deletion", red)  -- Deletions (e.g., diff highlight), bold red
