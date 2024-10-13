-- Verify whether the dependencies are installed
discord_rpc = {
    has_python = python_interop:installation() ~= nil,
    has_discord_rpc_module = python_interop:has_module("discordrpc"),
    code = [[
        import discordrpc
        rpc = discordrpc.RPC(app_id=1294981983146868807)
        rpc.set_activity(
            state = "Ox Editor",
            details = "Editing Files...",
        )
        rpc.run()
    ]],
}

function discord_rpc:ready()
    return self.has_python and self.has_discord_rpc_module
end

function discord_rpc:show_rpc()
    if not self:ready() then
        editor:display_error("Discord RPC: missing python or discord-rpc python module")
    else
        editor:display_info("Ready to go")
    end
end

function run_discord_rpc() 
    editor:display_info("Die")
end

event_mapping["ctrl_m"] = function()
    after(0, run_discord_rpc)
end
