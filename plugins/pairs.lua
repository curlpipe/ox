--[[
Bracket Pairs v0.2

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

-- Link up pairs to event mapping
for i, str in ipairs(pairings) do
    local start_pair = string.sub(str, 1, 1)
    local end_pair = string.sub(str, 2, 2)
    -- Determine which implementation to use
    if start_pair == end_pair then
        -- Handle hybrid start_pair and end_pair
        event_mapping[start_pair] = function()
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
            else
                just_paired = editor.cursor
                editor:insert(end_pair)
                editor:move_left()
            end
        end
        end
    else
        -- Handle traditional pairs
        event_mapping[end_pair] = function()
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
            just_paired = editor.cursor
            editor:insert(end_pair)
            editor:move_left()
        end
    end
end
