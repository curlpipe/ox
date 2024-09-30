-- Get the contents of the latest Cargo.toml
local cargo_latest = http.get("https://raw.githubusercontent.com/curlpipe/ox/refs/heads/master/Cargo.toml")
-- Extract the version from the build file
local version = cargo_latest:match("version%s*=%s*\"(%d+.%d+.%d+)\"")
-- Display it to the user
if version ~= editor.version and version ~= nil then
    editor:display_warning("Update to " .. version .. " is available (you have " .. editor.version .. ")")
end

