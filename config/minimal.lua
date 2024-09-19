-- This is a more minimal example to the full ox config
-- This just adds a few tweaks to specific areas while demonstrating the power of ox configuration

-- Disable cursor wrapping (which stops a cursor moving to the next line when it reaches the end a line) --
document.wrap_cursor = false

-- Colour both the status text colour and highlight colour as the colour pink --
colors.highlight = {150, 70, 200}
colors.status_fg = colors.highlight

-- Super minimal status line --
status_line:add_part("  {file_name}{modified}  │") -- The left side of the status line
status_line:add_part("│  {cursor_y} / {line_count}  ")  -- The right side of the status line

-- Enable bracket / quote pairs and autoindentation for a slick code editing experience!
load_plugin("pairs.lua")
load_plugin("autoindent.lua")
