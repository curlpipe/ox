--[[
Bracket Pairs v0.7

Automatically insert and delete brackets and quotes where appropriate
Also helps when you want to pad out brackets and quotes with whitespace
]]--

autopairs = {}

-- The following pairs are in the form [start of pair][end of pair]
autopairs.pairings = {
    -- Bracket pairs
    "()", "[]", "{}",
    -- Quote pairs
    '""', "''", "``",
}

autopairs.just_paired = { x = nil, y = nil }

-- Determine whether we are currently inside a pair
function autopairs:in_pair()
    if editor.cursor == nil then return false end
    -- Get first candidate for a pair
    local first
    if editor.cursor.x == 0 then
        first = ""
    else
        first = editor:get_character_at(editor.cursor.x - 1, editor.cursor.y)
    end
    -- Get second candidate for a pair
    local second = editor:get_character_at(editor.cursor.x, editor.cursor.y)
    -- See if there are any matches
    local potential_pair = first .. second
    for _, v in ipairs(autopairs.pairings) do
        if v == potential_pair then
            return true
        end
    end
    return false
end

-- Automatically delete end pair if user deletes corresponding start pair
event_mapping["before:backspace"] = function()
    if autopairs:in_pair() then
        editor:remove_at(editor.cursor.x, editor.cursor.y)
    end
end

-- Automatically insert an extra space if the user presses space between pairs
event_mapping["before:space"] = function()
    if autopairs:in_pair() then
        editor:insert(" ")
        editor:move_left()
    end
end

-- Link up pairs to event mapping
for i, str in ipairs(autopairs.pairings) do
    local start_pair = string.sub(str, 1, 1)
    local end_pair = string.sub(str, 2, 2)
    -- Determine which implementation to use
    if start_pair == end_pair then
        -- Handle hybrid start_pair and end_pair
        event_mapping[start_pair] = function()
            if editor.cursor == nil then return end
            -- Check if there is a matching start pair
            local at_char = ' '
            if editor.cursor.x > 1 then
                at_char = editor:get_character_at(editor.cursor.x - 2, editor.cursor.y)
            end
            local potential_dupe = at_char == start_pair
            -- Check if we're at the site of the last pairing
            local at_immediate_pair_x = autopairs.just_paired.x == editor.cursor.x - 1
            local at_immediate_pair_y = autopairs.just_paired.y == editor.cursor.y
            local at_immediate_pair = at_immediate_pair_x and at_immediate_pair_y
            if potential_dupe and at_immediate_pair then
                -- User just tried to add a closing pair despite us doing it for them!
                -- Undo it for them
                editor:remove_at(editor.cursor.x - 1, editor.cursor.y)
                autopairs.just_paired = { x = nil, y = nil }
            else
                autopairs.just_paired = editor.cursor
                editor:insert(end_pair)
                editor:move_left()
            end
        end
    else
        -- Handle traditional pairs
        event_mapping[end_pair] = function()
            if editor.cursor == nil then return end
            -- Check if there is a matching start pair
            local at_char = editor:get_character_at(editor.cursor.x - 2, editor.cursor.y)
            local potential_dupe = at_char == start_pair
            -- Check if we're at the site of the last pairing
            local at_immediate_pair_x = autopairs.just_paired.x == editor.cursor.x - 1
            local at_immediate_pair_y = autopairs.just_paired.y == editor.cursor.y
            local at_immediate_pair = at_immediate_pair_x and at_immediate_pair_y
            if potential_dupe and at_immediate_pair then
                -- User just tried to add a closing pair despite us doing it for them!
                -- Undo it for them
                editor:remove_at(editor.cursor.x - 1, editor.cursor.y)
                autopairs.just_paired = { x = nil, y = nil }
            end
        end
        event_mapping[start_pair] = function()
            if editor.cursor == nil then return end
            autopairs.just_paired = editor.cursor
            editor:insert(end_pair)
            editor:move_left()
        end
    end
end
