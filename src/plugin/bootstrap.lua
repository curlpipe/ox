home = os.getenv("HOME") or os.getenv("USERPROFILE")

function file_exists(file_path)
    local file = io.open(file_path, "r")
    if file then
        file:close()
        return true
    else
        return false
    end
end

plugins = {}
builtins = {}
plugin_issues = false

-- Import plug-in api components
http = require('src/plugin/networking')

function load_plugin(base)
    path_cross = base
    path_unix = home .. "/.config/ox/" .. base
    path_win = home .. "/ox/" .. base
    if file_exists(path_cross) then
        path = path_cross
    elseif file_exists(path_unix) then
        path = path_unix
    elseif file_exists(path_win) then
        path = file_win
    else
        -- Prevent warning if plug-in is built-in
        local is_autoindent = base:match("autoindent.lua$") ~= nil
        local is_pairs = base:match("pairs.lua$") ~= nil
        if not is_pairs and not is_autoindent then 
            -- Issue warning if plug-in is builtin
            print("[WARNING] Failed to load plugin " .. base)
            plugin_issues = true
        else
            table.insert(builtins, base)
        end
    end
    plugins[#plugins + 1] = path
end
