-- Code for running and processing plug-ins

global_event_mapping = {}

function merge_event_mapping()
    for key, f in pairs(event_mapping) do
        if global_event_mapping[key] ~= nil then
            table.insert(global_event_mapping[key], f)
        else
            global_event_mapping[key] = {f,}
        end
    end
    event_mapping = {}
end

for c, path in ipairs(plugins) do
    merge_event_mapping()
    dofile(path)
end
merge_event_mapping()

-- Function to remap keys if necessary
function remap_keys(from, to)
    local has_name = global_event_mapping[from] ~= nil
    local has_char = global_event_mapping[to] ~= nil
    if has_name then
        if has_char then
            -- Append name to char
            for i = 1, #global_event_mapping[from] do
                table.insert(global_event_mapping[to], global_event_mapping[from][i])
            end
            global_event_mapping[from] = nil
        else
            -- Transfer name to char
            global_event_mapping[to] = global_event_mapping[from]
            global_event_mapping[from] = nil
        end
    end
end

-- Remap space keys
remap_keys("space", " ")
remap_keys("ctrl_space", "ctrl_ ")
remap_keys("alt_space", "alt_ ")
remap_keys("ctrl_alt_space", "ctrl_alt_ ")
remap_keys("shift_tab", "shift_backtab")
remap_keys("before:space", "before: ")
remap_keys("before:ctrl_space", "before:ctrl_ ")
remap_keys("before:alt_space", "before:alt_ ")
remap_keys("before:ctrl_alt_space", "before:ctrl_alt_ ")
remap_keys("before:shift_tab", "before:shift_backtab")

-- Show warning if any plugins weren't able to be loaded
if plugin_issues then
    print("Various plug-ins failed to load")
    print("You may download these plug-ins by running the command `plugin install [plugin_name]`")
    print("")
    print("Alternatively, you may silence these warnings\nby removing the load_plugin() lines in your configuration file\nfor the missing plug-ins that are listed above")
end
