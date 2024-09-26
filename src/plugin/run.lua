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

if plugin_issues then
    print("Various plug-ins failed to load")
    print("You may download these plug-ins from the ox git repository (in the plugins folder)")
    print("https://github.com/curlpipe/ox")
    print("")
    print("Alternatively, you may silence these warnings\nby removing the load_plugin() lines in your configuration file\nfor the missing plug-ins that are listed above")
end
