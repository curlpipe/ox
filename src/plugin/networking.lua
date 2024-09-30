-- Networking library (for plug-ins to use)
-- Uses curl on unix based systems and powershell on windows

http = {
    backend = package.config:sub(1,1) == '\\' and 'powershell' or 'curl',
}

local function execute(cmd)
    local handle = io.popen(cmd)
    local result = handle:read("*a")
    handle:close()
    return result
end

function http.get(url)
    -- Using curl for the request
    local cmd
    if http.backend == 'curl' then
        cmd = "curl -s -X GET '" .. url .. "'"
    else
        cmd = table.concat({
            'powershell -Command "Invoke-WebRequest -Uri \'', url,
            '\' -UseBasicParsing | Select-Object -ExpandProperty Content"'
        })
    end
    return execute(cmd)
end

function http.post(url, data)
    local cmd
    if http.backend == 'curl' then
        cmd = "curl -s -X POST -d '" .. data .. "' '" .. url .. "'"
    else
        cmd = table.concat({
            'powershell -Command "Invoke-WebRequest -Uri \'', url,
            '\' -Method POST -Body \'', data,
            '\' -UseBasicParsing | Select-Object -ExpandProperty Content"'
        })
    end
    return execute(cmd)
end

function http.put(url, data)
    local cmd
    if http.backend == 'curl' then
        cmd = "curl -s -X PUT -d '" .. data .. "' '" .. url .. "'"
    else
        cmd = table.concat({
            'powershell -Command "Invoke-WebRequest -Uri \'', url,
            '\' -Method PUT -Body \'', data,
            '\' -UseBasicParsing | Select-Object -ExpandProperty Content"'
        })
    end
    return execute(cmd)
end

function http.delete(url)
    local cmd
    if http.backend == 'curl' then
        cmd = "curl -s -X DELETE '" .. url .. "'"
    else
        cmd = table.concat({
            'powershell -Command "Invoke-WebRequest -Uri \'', url,
            '\' -Method DELETE -UseBasicParsing | Select-Object -ExpandProperty Content"'
        })
    end
    return execute(cmd)
end

return http
