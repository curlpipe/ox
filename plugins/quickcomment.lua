--[[
Quickcomment v0.1

A plug-in to help you comment and uncomment lines quickly
]]--

quickcomment = {}

function quickcomment:comment(y)
    local line = editor:get_line_at(y)
    -- Find start of line
    local _, index = line:find("%S")
    index = index or 0
    -- Select a comment depending on the language
    local comment_start = self:comment_start() .. " "
    -- Insert the character
    local old_x = editor.cursor.x
    editor:move_to(index - 1, y)
    editor:insert(comment_start)
    editor:move_to(old_x + #comment_start, y)
end

function quickcomment:uncomment(y)
    local comment_start = self:comment_start() .. " "
    local line = editor:get_line_at(y)
    local old_x = editor.cursor.x
    if self:is_commented(y) then
        local index = line:find(comment_start)
        if index ~= nil then
            for i = 0, #comment_start - 1 do
                editor:remove_at(index - 1, y)
            end
        else
            comment_start = self:comment_start()
            local index = line:find(comment_start)
            for i = 0, #comment_start - 1 do
                editor:remove_at(index - 1, y)
            end
        end
        editor:move_to(old_x - #comment_start, y)
    end
end

function quickcomment:is_commented(y)
    local comment_start = self:comment_start()
    local line = editor:get_line_at(y)
    local _, index = line:find("%S")
    index = index or 0
    return string.sub(line, index, index + #comment_start - 1) == comment_start
end

function quickcomment:comment_start()
    if editor.document_type == "Shell" then
        comment_start = "#"
	elseif editor.document_type == "Python" then
        comment_start = "#"
	elseif editor.document_type == "Ruby" then
        comment_start = "#"
	elseif editor.document_type == "Lua" then
        comment_start = "--"
	elseif editor.document_type == "Haskell" then
        comment_start = "--"
    else
        comment_start = "//"
    end
    return comment_start
end

event_mapping["alt_c"] = function()
    if quickcomment:is_commented(editor.cursor.y) then
        quickcomment:uncomment(editor.cursor.y)
    else
        quickcomment:comment(editor.cursor.y)
    end
end
