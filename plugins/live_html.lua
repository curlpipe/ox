--[[
Live HTML v0.2

As you develop a website, you can view it in your browser without needing to refresh with every change
]]--

live_html = {
    has_python = python_interop:installation() ~= nil,
    has_flask_module = python_interop:has_module("flask"),
    entry_point = nil,
    tracking = {},
    pid = nil,
    last_request = "",
    refresh_when = (live_html or { refresh_when = "save" }).refresh_when,
}

function live_html:ready()
    return self.has_python and self.has_flask_module
end

function live_html:start()
    -- Start up flask server
    live_html.entry_point = editor.file_path
    local command = string.format("python %s/livehtml.py '%s'", plugin_path, editor.file_path)
    self.pid = shell:spawn(command)
    -- Notify user of location
    editor:display_info("Running server on http://localhost:5000")
end

function live_html:stop()
    shell:kill(self.pid)
    self.entry_point = nil
    self.pid = nil
end

function live_html_refresh()
    local tracked_file_changed = false
    for _, v in ipairs(live_html.tracking) do
        if v == editor.file_name then
            tracked_file_changed = true
            break
        end
    end
    if editor.file_path == live_html.entry_point then
        local contents = editor:get():gsub('"', '\\"'):gsub("\n", ""):gsub("`", "\\`")
        live_html.last_request = contents
        http.post("localhost:5000/update", contents)
    elseif tracked_file_changed then
        http.post("localhost:5000/forceupdate", live_html.last_request)
    end
end

commands["html"] = function(args)
    -- Check dependencies
    if live_html:ready() then
        if args[1] == "start" then
            -- Prevent duplicate server
            live_html:stop()
            -- Run the server
            live_html:start()
            after(5, "live_html_refresh")
        elseif args[1] == "stop" then
            live_html:stop()
        elseif args[1] == "track" then
            local file = args[2]
            table.insert(live_html.tracking, file)
            editor:display_info("Now tracking file " .. file)
        end
    else
        editor:display_error("Live HTML: python or flask module not found")
    end
end

event_mapping["*"] = function()
    if live_html.pid ~= nil and live_html.refresh_when == "keypress" then
        after(1, "live_html_refresh")
    end
end

event_mapping["ctrl_s"] = function()
    if live_html.pid ~= nil and live_html.refresh_when == "save" then
        after(1, "live_html_refresh")
    end
end

event_mapping["exit"] = function()
    live_html:stop()
end

-- Code for creating a server to load code
live_html_start = [[
from flask import Flask, request, render_template_string, redirect, url_for, Response, send_from_directory
import logging
import queue
import time
import sys
import os

try:
    os.chdir(os.path.dirname(sys.argv[1]))
except:
    pass

app = Flask(__name__)

log = logging.getLogger('werkzeug')
log.disabled = True

# HTML code stored in a variable
reload_script = """
<script type="text/javascript">
    // EventSource to listen to the /reload endpoint
    var source = new EventSource("/reload");
    source.onmessage = function(event) {
        if (event.data === "reload") {
            location.reload();
        }
    };
</script>
"""

html_content = """
<style>
body {
    padding: 100px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: middle;
    gap: 10px;
}

h1, h3 {
    font-family: Helvetica;
    text-align: center;
    margin: 0;
    padding: 0;
}

.loader {
    border: 8px solid #f3f3f3;
    border-top: 8px solid #3498db;
    border-radius: 50%;
    width: 30px;
    height: 30px;
    animation: spin 1s linear infinite;
    margin-top: 30px;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}
</style>

<body>
	<h1>Welcome to Ox Live HTML Edit</h1>
    <h3>Please wait whilst we load your website...</h3>
    <div class="loader"></div>
</body>
"""

# A list to keep track of clients that are connected
clients = []

# Function to notify all clients to reload
def notify_clients():
    for client in clients:
        print("Reloading a client...")
        try:
            client.put("reload")
        except:
            clients.remove(client)  # Remove any disconnected clients

@app.route('/')
def serve_html():
    # Render the HTML stored in the variable
    return render_template_string(reload_script + html_content)

@app.route('/update', methods=['POST'])
def update_html():
    global html_content
    # Get the new HTML content from the POST request
    new_code = request.get_data().decode('utf-8')
    if new_code and new_code != html_content:
        # Update the HTML content with the new code
        html_content = new_code
        notify_clients()  # Notify all clients to reload
    # Return a 200 status on successful update
    return "Update successful", 200

@app.route('/forceupdate', methods=['POST'])
def force_update_html():
    global html_content
    # Get the new HTML content from the POST request
    new_code = request.get_data().decode('utf-8')
    # Update the HTML content with the new code
    html_content = new_code
    notify_clients()  # Notify all clients to reload
    # Return a 200 status on successful update
    return "Update successful", 200

@app.route('/reload')
def reload():
    def stream():
        client = queue.Queue()
        clients.append(client)
        try:
            while True:
                msg = client.get()
                yield f"data: {msg}\n\n"
        except GeneratorExit:  # Disconnected client
            clients.remove(client)

    return Response(stream(), content_type='text/event-stream')

@app.route('/<path:filename>', methods=['GET'])
def serve_file(filename):
    # Serve a specific file from the current working directory
    return send_from_directory(os.getcwd(), filename)

if __name__ == "__main__":
    app.run(debug=False, threaded=True)
]]

-- Write the livehtml script if not already there
if not file_exists(plugin_path .. "/livehtml.py") then
    local file = io.open(plugin_path .. "/livehtml.py", "w")
    file:write(live_html_start)
    file:close()    
end
