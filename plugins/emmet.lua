--[[
Emmet v0.4

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
    unexpanded = unexpanded:gsub("^%s+", "")
    unexpanded = unexpanded:gsub("%s+$", "")
    -- Request the expanded equivalent
    local command = string.format("python %s/oxemmet.py \"%s\"", plugin_path, unexpanded)
    local expanded = shell:output(command)
    expanded = expanded:gsub("\n$", "")
    -- Keep track of the level of indentation
    local indent_level = autoindent:get_indent(editor.cursor.y)
    -- Delete the existing line
    editor:remove_line_at(editor.cursor.y)
    editor:insert_line_at("", editor.cursor.y)
    local old_cursor = editor.cursor
    -- Insert the expanded equivalent
    local lines = {}
    for line in expanded:gmatch("[^\r\n]+") do
        table.insert(lines, line)
    end
    for i, line in ipairs(lines) do
        -- Ensure correct indentation
        autoindent:set_indent(editor.cursor.y, indent_level)
        old_cursor.x = editor.cursor.x
        -- Insert rest of line
        editor:insert(line)
        -- Press return
        if i ~= #lines then
            editor:insert_line()
        end
    end
    -- Move to suggested cursor position
    editor:move_to(old_cursor.x, old_cursor.y)
    editor:move_next_match("\\|")
    editor:remove_at(editor.cursor.x, editor.cursor.y)
end

event_mapping["ctrl_m"] = function()
    if emmet:ready() then
        emmet:expand()
    else
        editor:display_error("Emmet: can't find python or py-emmet module")
    end
end

emmet_expand = [[
import emmet
import sys
import re

def place_cursor(expansion):
    def find_cursor_index(pattern, attribute):
        try:
            match = re.search(pattern, expansion)
            if match:
                attr_start = match.start() + expansion[match.start():].index(attribute) + len(attribute) + 1
                return attr_start + len(match.group(1)) + 1
        except IndexError:
            pass
        return None
    if expansion.split('\n')[0].lower().startswith("<!doctype html>"):
        match = re.search(r"<body[^>]*>(.*?)</body>", expansion, re.DOTALL)
        if match:
            before_body = match.start(1)
            after_body = match.end(1)
            mid_point = (before_body + after_body) // 2
            return mid_point
        return None
    a_match = find_cursor_index(r'<a[^>]*href="()"></a>', 'href')
    img_match = find_cursor_index(r'<img[^>]*src="()"[^>]*>', 'src')
    input_match = find_cursor_index(r'<input[^>]*type="()"[^>]*>', 'type')
    label_match = find_cursor_index(r'<label[^>]*for="()"[^>]*>', 'for')
    form_match = find_cursor_index(r'<form[^>]*action="()"[^>]*>', 'action')
    empty_tag_match = re.search(r"<([a-zA-Z0-9]+)([^>]*)></\1>", expansion)
    if empty_tag_match is not None:
        empty_tag_match = empty_tag_match.end(2) + 1
    alone_tags = [a_match, img_match, input_match, label_match, form_match, empty_tag_match]
    try:
        best_alone = min(filter(lambda x: x is not None, alone_tags))
        return best_alone
    except ValueError:
        return 0
contents = sys.argv[1]
expansion = emmet.expand(contents)
cursor_loc = place_cursor(expansion)
expansion = expansion[:cursor_loc] + "|" + expansion[cursor_loc:]
print(expansion)
]]

-- Write the emmet script if not already there
if not file_exists(plugin_path .. "/oxemmet.py") then
    local file = io.open(plugin_path .. "/oxemmet.py", "w")
    file:write(emmet_expand)
    file:close()
end
