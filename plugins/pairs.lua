--[[
Bracket Pairs v0.1

This will automatically insert a closing bracket or quote
when you type an opening one
]]--

event_mapping["("] = function()
    editor:insert(")")
    editor:move_left()
end

event_mapping["["] = function()
    editor:insert("]")
    editor:move_left()
end

event_mapping["{"] = function()
    editor:insert("}")
    editor:move_left()
end

event_mapping["\""] = function()
    editor:insert("\"")
    editor:move_left()
end

event_mapping["'"] = function()
    editor:insert("'")
    editor:move_left()
end

event_mapping["`"] = function()
    editor:insert("`")
    editor:move_left()
end

