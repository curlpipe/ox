--[[
Todo Lists v0.3

This plug-in will provide todo list functionality on files with the extension .todo
You can mark todos as done / not done by using the Ctrl + Enter key combination
Todos are in the format "- [ ] Your Todo Name"
]]--

-- Add language specific information for .todo files
file_types["Todo"] = {
    icon = " ",
    extensions = {"todo"},
    files = {".todo.md", ".todo"},
    modelines = {},
    color = "grey",
}

-- Add syntax highlighting to .todo files (done todos are comments)
syntax:new(
    "Todo",
    {syntax:keyword("comment", "\\s*(-\\s*\\[(?:X|x)\\].*)")}
)

-- Create the structure and behaviour for todo list files
todo_lists = {}

function todo_lists:how_complete()
    -- Work out how many todos are done vs not done
    local total = 0
    local complete = 0
    for y = 1, editor.document_length do
        local line = editor:get_line_at(y)
        if string.match(line, "^%s*%-%s*%[([Xx])%].*") then
            complete = complete + 1
            total = total + 1
        elseif string.match(line, "^%s*%-%s*%[ %].*") then
            total = total + 1
        end
    end
    local percentage = math.floor(complete / total * 100 + 0.5)
    -- Return a really nice message
    return complete .. "/" .. total .. " done" .. " - " .. percentage .. "% complete"
end

-- Shortcut to flip todos from not done to done and vice versa
event_mapping["ctrl_enter"] = function()
    if editor.document_type == "Todo" then
        -- Determine what kind of line we're dealing with
        local line = editor:get_line()
        if string.match(line, "^%s*%-%s*%[([Xx])%].*") then
            -- Mark this line as not done
            line = string.gsub(line, "^(%s*%-%s*)%[([Xx])%]", "%1[ ]")
            editor:insert_line_at(line, editor.cursor.y)
            editor:remove_line_at(editor.cursor.y + 1)
            -- Print handy statistics
            local stats = todo_lists:how_complete()
            editor:display_info(stats)
        elseif string.match(line, "^%s*%-%s*%[ %].*") then
            -- Mark this line as done
            line = string.gsub(line, "^(%s*%-%s*)%[ %]", "%1[X]")
            editor:insert_line_at(line, editor.cursor.y)
            editor:remove_line_at(editor.cursor.y + 1)
            -- Print handy statistics
            local stats = todo_lists:how_complete()
            editor:display_info(stats)
        else
            editor:display_error("This todo is incorrectly formatted")
        end
    end
end

-- Autoadd empty task when the user presses enter onto a new line
event_mapping["enter"] = function()
    if editor.document_type == "Todo" then
        editor:insert("- [ ] ")
    end
end
