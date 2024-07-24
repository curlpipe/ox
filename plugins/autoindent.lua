--[[
Auto Indent v0.1

You will be able to press return at the start of a block and have
Ox automatically indent for you.

By default, it will indent whenever you press the enter key with
the character to the left of the cursor being an opening bracket or
other syntax that indicates a block has started e.g. ":" in python
]]--

event_mapping["enter"] = function()
    -- Get line the cursor was on
    y = editor.cursor.y - 1
    line = editor:get_line_at(y)
    -- Work out what the last character on the line was
    sline = line:gsub("^%s*(.-)%s*$", "%1")
    local function ends(suffix) 
        return suffix == "" or sline:sub(-#suffix) == suffix 
    end
    local function starts(prefix)
        return sline:sub(1, #prefix) == prefix
    end
    -- Work out how indented the line was
    indents = #(line:match("^\t+") or "") + #(line:match("^ +") or "") / 4
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
        if ends("do") or ends("then") or func then indents = indents + 1 end
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

-- Helper function to check string endings
local function ends_with(str, ending)
end
