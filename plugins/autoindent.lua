--[[
Auto Indent v0.2

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

