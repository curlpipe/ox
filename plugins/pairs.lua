--[[
Bracket Pairs v0.3

This will automatically insert a closing bracket or quote
when you type an opening one
]]--

-- The following pairs are in the form [start of pair][end of pair]
pairings = {
    -- Bracket pairs
    "()", "[]", "{}",
    -- Quote pairs
    '""', "''", "``",
    -- Other pairs you wish to define can be added below...
}

just_paired = { x = nil, y = nil }
was_pasting = editor.pasting
line_cache = { y = editor.cursor.y, line = editor:get_line() }

event_mapping["*"] = function()
    -- If the editor is pasting, try to determine the first character of the paste
    if editor.pasting and not was_pasting then
        local first_paste = editor:get_character_at(editor.cursor.x - 2, editor.cursor.y)
        local between_pasting = false
        for _, str in ipairs(pairings) do
            if string.sub(str, 1, 1) == first_paste then
                between_pasting = true
            end
        end
        if between_pasting then
            -- Fix rogue paste
            editor:remove_at(editor.cursor.x, editor.cursor.y)
        end
    end
    was_pasting = editor.pasting
    local changed_line = line_cache.y ~= editor.cursor.y;
    local potential_backspace = not changed_line and string.len(line_cache.line) - 1 == string.len(editor:get_line());
    if changed_line or not potential_backspace then
        line_cache = { y = editor.cursor.y, line = editor:get_line() }
    end
end

-- Link up pairs to event mapping
for i, str in ipairs(pairings) do
    local start_pair = string.sub(str, 1, 1)
    local end_pair = string.sub(str, 2, 2)
    -- Determine which implementation to use
    if start_pair == end_pair then
        -- Handle hybrid start_pair and end_pair
        event_mapping[start_pair] = function()
            -- Return if the user is currently pasting text
            if editor.pasting then return end
            -- Check if there is a matching start pair
            local at_char = ' '
            if editor.cursor.x > 1 then
                at_char = editor:get_character_at(editor.cursor.x - 2, editor.cursor.y)
            end
            local potential_dupe = at_char == start_pair
            -- Check if we're at the site of the last pairing
            local at_immediate_pair_x = just_paired.x == editor.cursor.x - 1
            local at_immediate_pair_y = just_paired.y == editor.cursor.y
            local at_immediate_pair = at_immediate_pair_x and at_immediate_pair_y
            if potential_dupe and at_immediate_pair then
                -- User just tried to add a closing pair despite us doing it for them!
                -- Undo it for them
                editor:remove_at(editor.cursor.x - 1, editor.cursor.y)
                just_paired = { x = nil, y = nil }
                line_cache = { y = editor.cursor.y, line = editor:get_line() }
            else
                just_paired = editor.cursor
                editor:insert(end_pair)
                editor:move_left()
                line_cache = { y = editor.cursor.y, line = editor:get_line() }
            end
        end
    else
        -- Handle traditional pairs
        event_mapping[end_pair] = function()
            -- Return if the user is currently pasting text
            if editor.pasting then return end
            -- Check if there is a matching start pair
            local at_char = editor:get_character_at(editor.cursor.x - 2, editor.cursor.y)
            local potential_dupe = at_char == start_pair
            -- Check if we're at the site of the last pairing
            local at_immediate_pair_x = just_paired.x == editor.cursor.x - 1
            local at_immediate_pair_y = just_paired.y == editor.cursor.y
            local at_immediate_pair = at_immediate_pair_x and at_immediate_pair_y
            if potential_dupe and at_immediate_pair then
                -- User just tried to add a closing pair despite us doing it for them!
                -- Undo it for them
                editor:remove_at(editor.cursor.x - 1, editor.cursor.y)
                just_paired = { x = nil, y = nil }
            end
        end
        event_mapping[start_pair] = function()
            -- Return if the user is currently pasting text
            if editor.pasting then return end
            just_paired = editor.cursor
            editor:insert(end_pair)
            editor:move_left()
            line_cache = { y = editor.cursor.y, line = editor:get_line() }
        end
    end
end

function includes(array, value)
    for _, v in ipairs(array) do
        if v == value then
            return true  -- Value found
        end
    end
    return false  -- Value not found
end

-- Automatically delete pairs
event_mapping["backspace"] = function()
    local old_line = line_cache.line
    local potential_pair = string.sub(old_line, editor.cursor.x + 1, editor.cursor.x + 2)
    if includes(pairings, potential_pair) then
        editor:remove_at(editor.cursor.x, editor.cursor.y)
        line_cache = { y = editor.cursor.y, line = editor:get_line() }
    end
end

-- Space out pairs when pressing space between pairs
event_mapping["space"] = function()
    local first = editor:get_character_at(editor.cursor.x - 2, editor.cursor.y)
    local second = editor:get_character_at(editor.cursor.x, editor.cursor.y)
    local potential_pair = first .. second
    if includes(pairings, potential_pair) then
        editor:insert(" ")
        editor:move_left()
    end
end
