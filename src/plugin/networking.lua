-- Networking library (for plug-ins to use)
-- Requires curl to be installed

http = {}

function http.get(url)
    -- Using curl for the request
    local handle = io.popen("curl -s -X GET '" .. url .. "'")
    local result = handle:read("*a")
    handle:close()
    return result
end

function http.post(url, data)
    local handle = io.popen("curl -s -X POST -d '" .. data .. "' '" .. url .. "'")
    local result = handle:read("*a")
    handle:close()
    return result
end

function http.put(url, data)
    local handle = io.popen("curl -s -X PUT -d '" .. data .. "' '" .. url .. "'")
    local result = handle:read("*a")
    handle:close()
    return result
end

function http.delete(url)
    local handle = io.popen("curl -s -X DELETE '" .. url .. "'")
    local result = handle:read("*a")
    handle:close()
    return result
end

return http
