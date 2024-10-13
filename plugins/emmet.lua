--[[
Emmet v0.1

Implementation of Emmet for Ox for rapid web development
]]--

-- Verify whether the dependencies are installed
emmet = {
    has_python = python_interop:installation() ~= nil,
    has_emmet_module = python_interop:has_module("emmet"),
}

function emmet:ready()
    return self.has_python and self.has_emmet_module
end

function emmet:expand()
    -- Get the emmet code
    local unexpanded = editor:get_line()
    unexpanded = unexpanded:gsub("%s+", "")
    -- Request the expanded equivalent
    local code = emmet_expand:gsub("\n", "; ")
    local command = string.format("python -c \"%s\" '%s'", code, unexpanded)
	local handler = io.popen(command)
    local expanded = handler:read("*a")
    expanded = expanded:gsub("\n$", "")
    handler:close()
    -- Keep track of the level of indentation
    local indent_level = autoindent:get_indent(editor.cursor.y)
    -- Delete the existing line
	editor:remove_line_at(editor.cursor.y)
    editor:insert_line_at("", editor.cursor.y)
    local old_cursor = editor.cursor
    -- Insert the expanded equivalent
    for line in expanded:gmatch("[^\r\n]+") do
    	-- Ensure correct indentation
    	autoindent:set_indent(editor.cursor.y, indent_level)
    	old_cursor.x = editor.cursor.x
    	-- Insert rest of line
    	editor:insert(line)
    	-- Press return
    	editor:insert_line()
    end
    -- Restore cursor position
    editor:move_to(old_cursor.x, old_cursor.y)
end

emmet_expand = [[
import emmet
import sys
contents = sys.argv[1]
print(emmet.expand(contents))
]]

event_mapping["ctrl_m"] = function()
	if emmet:ready() then
		emmet:expand()
    else
        editor:display_error("Emmet: can't find python or py-emmet module")
    end
end
