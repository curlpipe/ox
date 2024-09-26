--[[
Auto Indent v0.4

You will be able to press return at the start of a block and have
Ox automatically indent for you.

By default, it will indent whenever you press the enter key with
the character to the left of the cursor being an opening bracket or
other syntax that indicates a block has started e.g. ":" in python
]]--

-- Automatic Indentation
event_mapping["enter"] = function()
    -- Get line the cursor was on
    y = editor.cursor.y - 1
    line = editor:get_line_at(y)
    -- Work out what the last character on the line was
    sline = line:gsub("^%s*(.-)%s*$", "%1")
    local function starts(prefix)
        return sline:sub(1, #prefix) == prefix
	end
    local function ends(suffix) 
        return suffix == "" or sline:sub(-#suffix) == suffix 
    end
    -- Work out how indented the line was
    indents = #(line:match("^\t+") or "") + #(line:match("^ +") or "") / document.tab_width
    -- Account for common groups of block starting characters
    is_bracket = ends("{") or ends("[") or ends("(")
    if is_bracket then indents = indents + 1 end
    -- Language specific additions
    if editor.document_type == "Python" then
        if ends(":") then indents = indents + 1 end
    elseif editor.document_type == "Ruby" then
        if ends("do") then indents = indents + 1 end
    elseif editor.document_type == "Lua" then
        func = ends(")") and (starts("function") or starts("local function"))
        if ends("else") or ends("do") or ends("then") or func then indents = indents + 1 end
    elseif editor.document_type == "Haskell" then
        if ends("where") or ends("let") or ends("do") then indents = indents + 1 end
    elseif editor.document_type == "Shell" then
        if ends("then") or ends("do") then indents = indents + 1 end
    end
    -- Indent the correct number of times
    for i = 1, indents do
        editor:insert("\t")
    end
    -- Handle the case where enter is pressed between two brackets
    local last_char = string.sub(line, string.len(line), string.len(line))
    local current_char = editor:get_character()
    local potential_pair = last_char .. current_char
    local old_cursor = editor.cursor
    if potential_pair == "{}" or potential_pair == "[]" or potential_pair == "()" then
        editor:insert_line()
        editor:move_to(old_cursor.x, old_cursor.y)
    end
end

-- Automatic Dedenting
local function do_dedent()
	local current_line = editor:get_line()
    if current_line:match("\t") ~= nil then
        editor:insert_line_at(current_line:gsub("\t", "", 1), editor.cursor.y)
        editor:remove_line_at(editor.cursor.y + 1)
    else
        editor:insert_line_at(current_line:gsub(string.rep(" ", document.tab_width), "", 1), editor.cursor.y)
        editor:remove_line_at(editor.cursor.y + 1)
    end
end

event_mapping["*"] = function()
	line = editor:get_line()
    local function ends(suffix) 
        return line:match("^%s*" .. suffix .. "$") ~= nil
    end
	if editor.document_type == "Shell" then
        if ends("fi") or ends("done") or ends("esac") or ends("}") or ends("elif") or ends("else") or ends(";;") then do_dedent() end
	elseif editor.document_type == "Python" then
		if ends("else") or ends("elif") or ends("except") or ends("finally") then do_dedent() end
	elseif editor.document_type == "Ruby" then
		if ends("end") or ends("else") or ends("elseif") or ends("ensure") or ends("rescue") or ends("when") or ends(";;") then do_dedent() end
	elseif editor.document_type == "Lua" then
		if ends("end") or ends("else") or ends("elseif") or ends("until") then do_dedent() end
	elseif editor.document_type == "Haskell" then
		if ends("else") or ends("in") then do_dedent() end
    end
end

-- Utilties for when moving lines around
autoindent = {}

function autoindent:fix_indent()
    -- Check the indentation of the line above this one (and match the line the cursor is currently on)
    local line_above = editor:get_line_at(editor.cursor.y - 1)
    local indents_above = #(line_above:match("^\t+") or "") + #(line_above:match("^ +") or "") / document.tab_width
    local line_below = editor:get_line_at(editor.cursor.y + 1)
    local indents_below = #(line_below:match("^\t+") or "") + #(line_below:match("^ +") or "") / document.tab_width
    local new_indent = nil
    if editor.cursor.y == 1 then
        -- Always remove all indent when on the first line
        new_indent = 0
    elseif indents_below == indents_above then
        new_indent = indents_below
    elseif indents_below > indents_above then
        new_indent = indents_below
    else
        new_indent = indents_above
    end
    -- Give a boost when entering empty blocks
    if line_above:match("{%s*$") ~= nil and line_below:match("^%s*}") ~= nil then
        new_indent = new_indent + 1;
    end
    -- Work out the contents of the new line
    local line = editor:get_line()
    local indents = #(line:match("^\t+") or "") + #(line:match("^ +") or "") / document.tab_width
    local indent_change = new_indent - indents
    local new_line = nil
    if indent_change > 0 then
        -- Insert indentation
        if line:match("\t") ~= nil then
            -- Insert Tabs
            new_line = string.rep("\t", indent_change) .. line
        else
            -- Insert Spaces
            new_line = string.rep(" ", indent_change * document.tab_width) .. line
        end
    elseif indent_change < 0 then
        -- Remove indentation
        if line:match("\t") ~= nil then
            -- Remove Tabs
            new_line = line:gsub("\t", "", -indent_change)
        else
            -- Remove Spaces
            new_line = line:gsub(string.rep(" ", document.tab_width), "", -indent_change)
        end
    end
    -- Perform replacement
    editor:insert_line_at(new_line, editor.cursor.y)
    editor:remove_line_at(editor.cursor.y + 1)
end
