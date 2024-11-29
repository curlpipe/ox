--[[
Quickcomment v0.3

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
    if index - 1 <= 0 then
        editor:move_to(0, y)
    else
        editor:move_to(index - 1, y)
    end
    editor:insert(comment_start)
    if old_x + #comment_start <= 0 then
        editor:move_to(0, y)
    else
        editor:move_to(old_x + #comment_start, y)
    end
end

function quickcomment:uncomment(y)
    local comment_start = self:comment_start() .. " "
    local line = editor:get_line_at(y)
    local old_x = editor.cursor.x
    if self:is_commented(y) then
        local index = line:find(comment_start, 1, true)
        if index ~= nil then
            -- Existing comment has a space after it
            for i = 0, #comment_start - 1 do
                editor:remove_at(index - 1, y)
            end
        else
            -- Existing comment doesn't have a space after it
            comment_start = self:comment_start()
            local index = line:find(comment_start, 1, true)
            for i = 0, #comment_start - 1 do
                editor:remove_at(index - 1, y)
            end
        end
        if old_x - #comment_start <= 0 then
            editor:move_to(0, y)
        else
            editor:move_to(old_x - #comment_start, y)
        end
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
    elseif editor.document_type == "TOML" then
        comment_start = "#"
    elseif editor.document_type == "Lua" then
        comment_start = "--"
    elseif editor.document_type == "Haskell" then
        comment_start = "--"
    elseif editor.document_type == "Assembly" then
        comment_start = ";"
    elseif editor.document_type == "Ada" then
        comment_start = "--"
    elseif editor.document_type == "Crystal" then
        comment_start = "#"
    elseif editor.document_type == "Makefile" then
        comment_start = "#"
    elseif editor.document_type == "Julia" then
        comment_start = "#"
    elseif editor.document_type == "Lisp" then
        comment_start = ";"
    elseif editor.document_type == "Perl" then
        comment_start = "#"
    elseif editor.document_type == "R" then
        comment_start = "#"
    elseif editor.document_type == "Racket" then
        comment_start = ";"
    elseif editor.document_type == "SQL" then
        comment_start = "--"
    elseif editor.document_type == "Zsh" then
        comment_start = "#"
    elseif editor.document_type == "Yaml" then
        comment_start = "#"
    elseif editor.document_type == "Clojure" then
        comment_start = ";"
    elseif editor.document_type == "Zsh" then
        comment_start = "#"
    else
        comment_start = "//"
    end
    return comment_start
end

function quickcomment:toggle_comment(y)
    if self:is_commented(y) then
        self:uncomment(y)
    else
        self:comment(y)
    end
end

event_mapping["alt_c"] = function()
    editor:commit()
    local cursor = editor.cursor
    local select = editor.selection
    local no_select = select.x == cursor.x and select.y == cursor.y
    if no_select then
        quickcomment:toggle_comment(editor.cursor.y)
    else
        -- toggle comments on a group of lines
        if cursor.y > select.y then
            for line = select.y, cursor.y do
                editor:move_to(0, line)
                quickcomment:toggle_comment(editor.cursor.y)
            end
        else
            for line = cursor.y, select.y do
                editor:move_to(0, line)
                quickcomment:toggle_comment(editor.cursor.y)
            end
        end
        editor:move_to(cursor.x, cursor.y)
        editor:select_to(select.x, select.y)
    end
    -- Avoid weird behaviour with cursor moving up and down
    editor:cursor_snap()
end
