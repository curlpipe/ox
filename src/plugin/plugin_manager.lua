-- Plug-in management system code

plugin_manager = {}

-- Install a plug-in
function plugin_manager:install(plugin)
    -- Check if downloaded / in config
    local downloaded = self:plugin_downloaded(plugin)
    local in_config = self:plugin_in_config(plugin)
    local do_download = false
    local do_enabling = false
    if downloaded and in_config then
        -- Already installed
        local resp = editor:prompt("Plug-in is already installed, would you like to update it? (y/n)")
        if resp == "y" then
            do_download = true
        else
            return false
        end
    elseif not downloaded and not in_config then
        -- No evidence of plug-in on system, get installing
        do_download = true
        do_enabling = true
    elseif not downloaded and in_config then
        -- Somehow, the user has it enabled, but it isn't downloaded
        local resp = editor:prompt("Plugin already enabled, start download? (y/n)")
        if resp == "y" then
            do_download = true
        else
            return false
        end
    elseif downloaded and not in_config then
        -- The user has managed to download it, but they haven't enabled it
        local resp = editor:prompt("Plugin already downloaded, enable plug-in? (y/n)")
        if resp == "y" then
            do_enabling = true
        else
            return false
        end
    end
    -- Do the installing
    if do_download then 
        local result = plugin_manager:download_plugin(plugin)
        if result ~= nil then
            editor:display_error(result)
            return true
        end
    end
    if do_enabling then 
        local result = plugin_manager:append_to_config(plugin)
        if result ~= nil then
            editor:display_error(result)
            return true
        end
    end
    -- Reload configuration file and plugins just to be safe
    editor:reload_plugins()
    editor:reset_terminal()
    editor:display_info("Plugin was installed successfully")
    return true
end

-- Uninstall a plug-in
function plugin_manager:uninstall(plugin)
    -- Check if downloaded / in config
    local downloaded = self:plugin_downloaded(plugin)
    local in_config = self:plugin_in_config(plugin)
    local is_builtin = self:plugin_is_builtin(plugin)
    if not downloaded and not in_config then
        editor:display_error("Plugin is not installed")
        return
    end
    if downloaded and not is_builtin then
        local result = plugin_manager:remove_plugin(plugin)
        if result ~= nil then
            editor:display_error(result)
            return
        end
    end
    if in_config then
        local result = plugin_manager:remove_from_config(plugin)
        if result ~= nil then
            editor:display_error(result)
            return
        end
    end
    -- Reload configuration file and plugins just to be safe
    editor:reload_plugins()
    editor:reset_terminal()
    editor:display_info("Plugin was uninstalled successfully")
end

-- Get the status of the plug-ins including how many are installed and which ones
function plugin_manager:status()
    local count = 0
    local list = ""
    for _, v in ipairs(builtins) do
        count = count + 1
        list = list .. v:match("(.+).lua$") .. " "
    end
    for _, v in ipairs(plugins) do
        count = count + 1
        list = list .. v:match("^.+[\\/](.+).lua$") .. " "
    end
    editor:display_info(tostring(count) .. " plug-ins installed: " .. list)
end

-- Verify whether or not a plug-in is built-in
function plugin_manager:plugin_is_builtin(plugin)
    local base = plugin .. ".lua"
    local is_autoindent = base == "autoindent.lua"
    local is_pairs = base == "pairs.lua"
    local is_quickcomment = base == "quickcomment.lua"
    return is_autoindent or is_pairs or is_quickcomment
end

-- Verify whether or not a plug-in is downloaded
function plugin_manager:plugin_downloaded(plugin)
    local base = plugin .. ".lua"
    local path_cross = base
    local path_unix = home .. "/.config/ox/" .. base
    local path_win = home .. "/ox/" .. base
    local installed = file_exists(path_cross) or file_exists(path_unix) or file_exists(path_win)
    -- Return true if plug-ins are built in
    local builtin = self:plugin_is_builtin(plugin)
    return installed or builtin
end

-- Download a plug-in from the ox repository
function plugin_manager:download_plugin(plugin)
    -- Download the plug-in code
    local url = "https://raw.githubusercontent.com/curlpipe/ox/refs/heads/master/plugins/" .. plugin .. ".lua"
    local resp = http.get(url)
    if resp == "404: Not Found" then
        return "Plug-in not found in repository"
    end
    -- Find the path to download it to
    local path = plugin_path .. "/" .. plugin .. ".lua"
    -- Create the plug-in directory if it doesn't already exist
    if not dir_exists(plugin_path) then
        local command
        if package.config.sub(1,1) == '\\' then
            command = "mkdir " .. plugin_path
        else
            command = "mkdir -p " .. plugin_path
        end
        if shell:run(command) ~= 0 then
            return "Failed to make directory at " .. plugin_path
        end
    end
    -- Write it to a file
    file = io.open(path, "w")
    if not file then
        return "Failed to write to " .. path
    end
    file:write(resp)
    file:close()
    return nil
end

-- Remove a plug-in from the configuration directory
function plugin_manager:remove_plugin(plugin)
    -- Obtain the path
    local path = package.config:sub(1,1) == '\\' and home .. "/ox" or home .. "/.config/ox"
    path = path .. "/" .. plugin .. ".lua"
    -- Remove the file
    local success, err = os.remove(path)
    if not success then
        return "Failed to delete the plug-in: " .. err
    else
        return nil
    end
end

-- Verify whether the plug-in is being imported in the configuration file
function plugin_manager:plugin_in_config(plugin)
    -- Find the configuration file path
    local path = home .. "/.oxrc"
    -- Open the document
    local file = io.open(path, "r")
    if not file then return false end
    -- Check each line to see whether it is being loaded
    for line in file:lines() do
        local pattern1 = '^load_plugin%("' .. plugin .. '.lua"%)'
        local pattern2 = "^load_plugin%('" .. plugin .. ".lua'%)"
        if line:match(pattern1) or line:match(pattern2) then
            file:close()
            return true
        end
    end
    file:close()
    return false
end

-- Append the plug-in import code to the configuration file so it is loaded
function plugin_manager:append_to_config(plugin)
    local path = home .. "/.oxrc"
    local file = io.open(path, "a")
    if not file then
        return "Failed to open configuration file"
    end
    file:write('load_plugin("' .. plugin .. '.lua")\n')
    file:close()
    return nil
end

-- Remove plug-in import code from the configuration file
function plugin_manager:remove_from_config(plugin)
    -- Find the configuration file path
    local path = home .. "/.oxrc"
    -- Open the configuration file
    local file = io.open(path, "r")
    if not file then
        return "Failed to open configuration file"
    end
    local lines = {}
    for line in file:lines() do
        table.insert(lines, line)
    end
    file:close()
    -- Run through each line and only write back the non-offending lines
    local file = io.open(path, "w")
    for _, line in ipairs(lines) do
        local pattern1 = '^load_plugin%("' .. plugin .. '.lua"%)'
        local pattern2 = "^load_plugin%('" .. plugin .. ".lua'%)"
        if not line:match(pattern1) and not line:match(pattern2) then
            file:write(line .. "\n")
        end
    end
    file:close()
    return nil
end

-- Find the local version of a plug-in that is installed
function plugin_manager:local_version(plugin)
    -- Open the file
    local file = io.open(plugin_path .. path_sep .. plugin .. ".lua", "r")
    if not file then return nil end
    -- Attempt to find a version indicator in the first 10 lines of the file
    local version = nil
    for i = 1, 10 do
        -- Read the line
        local line = file:read("*line")
        if not line then break end
        -- See if there is a match
        local match = line:match("(v%d+%.%d+)")
        if match then
            version = match
            break
        end
    end
    file:close()
    return version
end

-- Find the latest online version of a plug-in
function plugin_manager:latest_version(plugin)
    -- Download the plug-in's source
    local url = "https://raw.githubusercontent.com/curlpipe/ox/refs/heads/master/plugins/" .. plugin .. ".lua"
    local resp = http.get(url)
    if resp == "404: Not Found" then return nil end
    -- Attempt to find a version indicator in the first 10 lines of the file
    local version = nil
    for line in resp:gmatch("[^\r\n]+") do
        -- See if there is a match
        local match = line:match("(v%d+%.%d+)")
        if match then
            version = match
            break
        end
    end
    return version
end

commands["plugin"] = function(arguments)
    if arguments[1] == "install" then
        local result = plugin_manager:install(arguments[2])
        if not result then
            editor:display_info("Plug-in installation cancelled")
        end
    elseif arguments[1] == "uninstall" then
        plugin_manager:uninstall(arguments[2])
    elseif arguments[1] == "status" then
        plugin_manager:status()
    elseif arguments[1] == "update" then
        -- editor:display_info(tostring(local_copy) .. " locally vs " .. tostring(latest_copy) .. " latest")
        editor:display_info("Please wait whilst versions are checked...")
        editor:rerender_feedback_line()
        local outdated = {}
        for _, plugin in ipairs(plugins) do
            local name = plugin:match("([^/\\]+)%.lua$")
            local local_copy = plugin_manager:local_version(name)
            local latest_copy = plugin_manager:latest_version(name)
            if local_copy ~= latest_copy then
                table.insert(outdated, {name, local_copy, latest_copy})
            end
        end
        for _, data in ipairs(outdated) do
            local name = data[1]
            local local_copy = data[2]
            local latest_copy = data[3]
            local response = editor:prompt(
                string.format(
                    "%s needs an update: you have %s, latest is %s, update plugin? (y/n)",
                    name,
                    local_copy,
                    latest_copy
                )
            )
            if response == "y" then
                editor:display_info("Updating " .. name .. ", please wait...")
                editor:rerender_feedback_line()
                local result = plugin_manager:download_plugin(name)
                if result ~= nil then
                    editor:display_error("Failed to download plug-in: " .. result)
                    return
                end
            end
        end
        editor:display_info("Update check-up completed, you're all set")
    end
end
