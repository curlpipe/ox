--[[
Auto Indent v0.7

Helps you when programming by guessing where indentation should go
and then automatically applying these guesses as you program
]]--

autoindent = {}

-- Determine if a line starts with a certain string
function autoindent:starts(y, starting)
    local line = editor:get_line_at(y)
    return line:match("^" .. starting)
end

-- Determine if a line ends with a certain string
function autoindent:ends(y, ending)
    local line = editor:get_line_at(y)
    return line:match(ending .. "$")
end

-- Determine if the line causes an indent when return is pressed on the end
function autoindent:causes_indent(y)
    -- Always indent on open brackets
    local is_bracket = self:ends(y, "%{") or self:ends(y, "%[") or self:ends(y, "%(")
    if is_bracket then return true end
    -- Language specific additions
    if editor.document_type == "Python" then
        if self:ends(y, ":") then return true end
    elseif editor.document_type == "Ruby" then
        if self:ends(y, "do") then return true end
    elseif editor.document_type == "Lua" then
        local func = self:ends(y, "%)") and (self:starts(y, "function") or self:starts(y, "local function"))
        if self:ends(y, "else") or self:ends(y, "do") or self:ends(y, "then") or func then return true end
    elseif editor.document_type == "Haskell" then
        if self:ends(y, "where") or self:ends(y, "let") or self:ends(y, "do") then return true end
    elseif editor.document_type == "Shell" then
        if self:ends(y, "then") or self:ends(y, "do") then return true end
    end
    return false
end

-- Determine if the line causes a dedent as soon as the pattern matches
function autoindent:causes_dedent(y)
    -- Always dedent after closing brackets
    local is_bracket = self:starts(y, "%s*%}") or self:starts(y, "%s*%]") or self:starts(y, "%s*%)")
    if is_bracket then return true end
    -- Check the line for token giveaways
	if editor.document_type == "Shell" then
        local end1 = self:starts(y, "%s*fi") or self:starts(y, "%s*done") or self:starts(y, "%s*esac")
        local end2 = self:starts(y, "%s*elif") or self:starts(y, "%s*else") or self:starts(y, "%s*;;")
        if end1 or end2 then return true end
	elseif editor.document_type == "Python" then
        local end1 = self:starts(y, "%s*else") or self:starts(y, "%s*elif")
        local end2 = self:starts(y, "%s*except") or self:starts(y, "%s*finally")
		if end1 or end2 then return true end
	elseif editor.document_type == "Ruby" then
        local end1 = self:starts(y, "%s*end") or self:starts(y, "%s*else") or self:starts(y, "%s*elseif")
        local end2 = self:starts(y, "%s*ensure") or self:starts(y, "%s*rescue") or self:starts(y, "%s*when")
		if end1 or end2 or self:starts(y, "%s*;;") then return true end
	elseif editor.document_type == "Lua" then
        local end1 = self:starts(y, "%s*end") or self:starts(y, "%s*else")
        local end2 = self:starts(y, "%s*elseif") or self:starts(y, "%s*until")
		if end1 or end2 then return true end
	elseif editor.document_type == "Haskell" then
		if self:starts(y, "%s*else") or self:starts(y, "%s*in") then return true end
    end
    return false
end

-- Set an indent at a certain y index
function autoindent:set_indent(y, new_indent)
    -- Handle awkward scenarios
    if new_indent < 0 then return end
    -- Find the indent of the line at the moment
    local line = editor:get_line_at(y)
    local current = autoindent:get_indent(y)
    -- Work out how much to change and what to change
    local indent_change = new_indent - current
    local tabs = line:match("^\t") ~= nil
    -- Prepare to form the new line contents
    local new_line = nil
    -- Work out if adding or removing
    local x = editor.cursor.x
    if indent_change > 0 then
        -- Insert indentation
        if tabs then
            -- Insert Tabs
            x = x + indent_change
            new_line = string.rep("\t", indent_change) .. line
        else
            -- Insert Spaces
            x = x + indent_change * document.tab_width
            new_line = string.rep(" ", indent_change * document.tab_width) .. line
        end
    elseif indent_change < 0 then
        -- Remove indentation
        if tabs then
            -- Remove Tabs
            x = x - -indent_change
            new_line = line:gsub("\t", "", -indent_change)
        else
            -- Remove Spaces
            x = x - -indent_change * document.tab_width
            new_line = line:gsub(string.rep(" ", document.tab_width), "", -indent_change)
        end
    else
        return
    end
    -- Perform the substitution with the new line
    editor:insert_line_at(new_line, y)
    editor:remove_line_at(y + 1)
    -- Place the cursor at a sensible position
    editor:move_to(x, y)
end

-- Get how indented a line is at a certain y index
function autoindent:get_indent(y)
    local line = editor:get_line_at(y)
    return #(line:match("^\t+") or "") + #(line:match("^ +") or "") / document.tab_width
end

-- Utilties for when moving lines around
function autoindent:fix_indent()
    -- Check the indentation of the line above this one (and match the line the cursor is currently on)
    local indents_above = autoindent:get_indent(editor.cursor.y - 1)
    local indents_below = autoindent:get_indent(editor.cursor.y + 1)
    local new_indent = nil
    if editor.cursor.y == 1 then
        -- Always remove all indent when on the first line
        new_indent = 0
    elseif indents_below >= indents_above then
        new_indent = indents_below
    else
        new_indent = indents_above
    end
    -- Give a boost when entering an empty block
    local indenting_above = autoindent:causes_indent(editor.cursor.y - 1)
    local dedenting_below = autoindent:causes_dedent(editor.cursor.y + 1)
    if indenting_above and dedenting_below then
        new_indent = new_indent + 1
    end
    -- Set the indent
    autoindent:set_indent(editor.cursor.y, new_indent)
end

-- Handle the case where the enter key is pressed between two brackets
function autoindent:disperse_block()
    local indenting_above = autoindent:causes_indent(editor.cursor.y - 1)
    local current_dedenting = autoindent:causes_dedent(editor.cursor.y)
    if indenting_above and current_dedenting then
        local starting_indent = autoindent:get_indent(editor.cursor.y - 1)
        local old_cursor = editor.cursor
        editor:insert_line()
        autoindent:set_indent(editor.cursor.y, starting_indent)
        editor:move_to(old_cursor.x, old_cursor.y)
    end
end

event_mapping["enter"] = function()
    -- Indent where appropriate
    if autoindent:causes_indent(editor.cursor.y - 1) then
        local new_level = autoindent:get_indent(editor.cursor.y) + 1
        autoindent:set_indent(editor.cursor.y, new_level)
    end
    -- Give newly created line a boost to match it up relatively with the line before it
    local added_level = autoindent:get_indent(editor.cursor.y) + autoindent:get_indent(editor.cursor.y - 1)
    autoindent:set_indent(editor.cursor.y, added_level)
    -- Handle the case where enter is pressed, creating a multi-line block that requires neatening up
    autoindent:disperse_block()
end

event_mapping["*"] = function()
    -- Dedent where appropriate
    if autoindent:causes_dedent(editor.cursor.y) then
        local new_level = autoindent:get_indent(editor.cursor.y) - 1
        autoindent:set_indent(editor.cursor.y, new_level)
    end
end
