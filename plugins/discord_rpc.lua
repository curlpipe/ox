-- Verify whether the dependencies are installed
discord_rpc = {
    has_python = python_interop:installation() ~= nil,
    has_discord_rpc_module = python_interop:has_module("discordrpc"),
    pid = nil,
    doc = editor.current_document_id,
}

function discord_rpc:ready()
    return self.has_python and self.has_discord_rpc_module
end

function discord_rpc:show_rpc()
    if not self:ready() then
        editor:display_error("Discord RPC: missing python or discord-rpc python module")
    else
        -- Spawn an rpc process
        local name = editor.file_name or "Untitled"
        local kind = string.lower(editor.document_type:gsub("%+", "p"):gsub("#", "s"))
        local code = drpc:gsub("\n", "; ")
        local command = string.format("python -c \"%s\" 'Ox' 'Editing %s' '%s'", code, name, kind)
        local handler = io.popen(command .. " > /dev/null 2>&1 & echo $!")
        local pid = handler:read("*a")
        pid = pid:gsub("%s+", "")
        pid = pid:gsub("\\n", "")
        pid = pid:gsub("\\t", "")
        self.pid = pid
        handler:close()
    end
end

function run_discord_rpc() 
    discord_rpc:show_rpc()
    editor:rerender()
end

function kill_discord_rpc()
    if discord_rpc.pid ~= nil then
        os.execute("kill " .. discord_rpc.pid)
    end
end

function check_discord_rpc()
    -- Detect change in document
    if discord_rpc.doc ~= editor.current_document_id then
        -- Reload the rpc
        kill_discord_rpc()
        discord_rpc.doc = editor.current_document_id
        after(1, "run_discord_rpc")
    end
end

every(5, "check_discord_rpc")

event_mapping["exit"] = function()
    -- Kill the rpc process
    kill_discord_rpc()
end

after(0, "run_discord_rpc")

drpc = [[
import discordrpc
import sys
args = sys.argv[1:]
rpc = discordrpc.RPC(app_id=1294981983146868807, output=False)
rpc.set_activity(state=args[0], details=args[1], small_image=args[2])
rpc.run()
]]
