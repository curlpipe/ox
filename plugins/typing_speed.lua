--[[
Typing Speed v0.1

Lets you know of your typing speed as you type
--]]

-- Program state
typing_speed = {
    time_between_words = {},
    last_capture = os.date("*t"),
}

event_mapping["space"] = function()
    if #typing_speed.time_between_words > 10 then
        typing_speed:pop()
    end
    local current = os.date("*t")
    local elapsed = os.time(current) - os.time(typing_speed.last_capture)
    typing_speed.last_capture = current
    typing_speed:push(elapsed)
end

function typing_speed:push(value)
    table.insert(self.time_between_words, value)
end

function typing_speed:pop()
    local result = table[1]
    table.remove(self.time_between_words, 1)
    return result
end

function typing_speed_show()
    -- Work out the average seconds taken to type each word
    local sum = 0
    local count = #typing_speed.time_between_words
    for i = 1, count do
        sum = sum + typing_speed.time_between_words[i]
    end
    local avg = 0
    if count > 0 then
        avg = sum / count
    end
    local wpm = 60 / avg
    if count <= 0 then
        wpm = 0
    end
    return tostring(math.floor(wpm + 0.5)) .. " wpm"
end
