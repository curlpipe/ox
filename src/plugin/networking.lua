-- Networking library (for plug-ins to use)
-- Uses curl

http = {
    backend = "curl",
}

local function execute(cmd)
    local handle = io.popen(cmd)
    local result = handle:read("*a")
    handle:close()
    return result
end

function http.get(url)
	local cmd = 'curl -s -X GET "' .. url .. '"'
    return execute(cmd)
end

function http.post(url, data)
    local cmd = 'curl -s -X POST -d "' .. data .. '" "' .. url .. '"'
    return execute(cmd)
end

function http.put(url, data)
    local cmd = 'curl -s -X PUT -d "' .. data .. '"  "' .. url .. '"'
    return execute(cmd)
end

function http.delete(url)
    local cmd = 'curl -s -X DELETE "' .. url .. '"'
    return execute(cmd)
end
