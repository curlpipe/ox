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

-- Remap ctrl_space to ctrl_ 
local has_name = global_event_mapping["ctrl_space"] ~= nil
local has_char = global_event_mapping["ctrl_ "] ~= nil
if has_name then
    if has_char then
        -- Append name to char
        for i = 1, #global_event_mapping["ctrl_space"] do
            table.insert(global_event_mapping["ctrl_ "], global_event_mapping["ctrl_space"][i])
        end
        global_event_mapping["ctrl_space"] = nil
    else
        -- Transfer name to char
        global_event_mapping["ctrl_ "] = global_event_mapping["ctrl_space"]
        global_event_mapping["ctrl_space"] = nil
    end
end

if plugin_issues then
    print("Various plug-ins failed to load")
    print("You may download these plug-ins from the ox git repository (in the plugins folder)")
    print("https://github.com/curlpipe/ox")
    print("")
    print("Alternatively, you may silence these warnings\nby removing the load_plugin() lines in your configuration file\nfor the missing plug-ins that are listed above")
end
