-- Define our pomodoro state
pomodoro = {
    -- Configuration values
    work_time = 25,
    rest_time = 5,
    -- Plug-in state
    current = "none",
    started = os.date("*t"),
}

-- Utility function to show a user-friendly time
function dec2mmss(decimal_seconds)
    local minutes = math.floor(decimal_seconds / 60)
    local seconds = decimal_seconds % 60
    
    -- Format seconds to always have two digits
    return string.format("%02d:%02d", minutes, seconds)
end

-- Define a function to display the countdown in the status line
function pomodoro_show()
    local current = os.date("*t")
    local elapsed = os.time(current) - os.time(pomodoro.started)
    local minutes = 0
    if pomodoro.current == "work" then
        minutes = pomodoro.work_time * 60 - elapsed
    elseif pomodoro.current == "rest" then
        minutes = pomodoro.rest_time * 60 - elapsed
    end
    if minutes < 0 then
        if pomodoro.current == "work" then
            pomodoro.current = "rest"
        elseif pomodoro.current == "rest" then
            pomodoro.current = "work"
        end
        pomodoro.started = os.date("*t")
        return "Time is up!"
    elseif pomodoro.current == "none" then
        return "No Pomodoro Active"
    else
        return pomodoro.current .. " for " .. dec2mmss(minutes)
    end
end

-- Add the pomodoro command to interface with the user
commands["pomodoro"] = function(arguments)
    subcmd = arguments[1]
    if subcmd == "start" then
        if pomodoro.current ~= "none" then
            editor:display_error("Pomodoro timer is already active")
        else
            pomodoro.current = "work"
            pomodoro.started = os.date("*t")
        end
    elseif subcmd == "stop" then
        pomodoro.current = "none"
        editor:display_info("Stopped pomodoro timer")
    end
end