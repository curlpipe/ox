--[[
Git v0.1

A plug-in for git integration that provides features to: 
 - Choose which files to add to a commit
 - Do a commit
 - Push local commits
 - View diffs
 - See which branch you are on
 - Pull any changes upstream
]]--

git = {
    status = {},
    icons = false,
}

-- Verify that git is installed
local git_handle = io.popen("git --version 2>&1")
local git_output = git_handle:read("*a")
git_handle:close()
git.has_git = git_output:find("git version")

function git:ready()
    return self.has_git
end

function git:repo_path()
    local handle = io.popen("git rev-parse --show-toplevel 2>/dev/null")
    local repo_path_output = handle:read("*a")
    handle:close()
    return repo_path_output:gsub("[\r\n]+", "")
end

function git:refresh_status()
    local repo_path = self:repo_path()
    local handle = io.popen("git status --porcelain 2>/dev/null")
    local status_output = handle:read("*a")
    handle:close()
    local status = {}
    for line in status_output:gmatch("[^\r\n]+") do
        local staged_status = line:sub(1, 1)
        local unstaged_status = line:sub(2, 2)
        local file_name = repo_path .. "/" .. line:sub(4)
        local staged
        local modified
        if self.icons then
            staged = "󰸩 "
            modified = "󱇨 "
        else
            staged = "S"
            modified = "M"
        end
        -- M = modified, S = staged
        if staged_status ~= " " and staged_status ~= "?" then
            status[file_name] = staged
        elseif unstaged_status ~= " " or unstaged_status == "?" then
            status[file_name] = modified
        end
    end
    self.status = status
end

function git:get_stats()
    local handle = io.popen("git diff --stat")
    local result = handle:read("*a")
    handle:close()

    local files = {}
    local total_insertions = 0
    local total_deletions = 0

    for line in result:gmatch("[^\r\n]+") do
        local file, changes = line:match("(%S+)%s+|%s+(%d+)")
        if file ~= nil then
            local insertions = select(2, line:gsub("%+", ""))
            local deletions = select(2, line:gsub("%-", ""))
            table.insert(files, { file = file, insertions = insertions, deletions = deletions })
            total_insertions = total_insertions + insertions
            total_deletions = total_deletions + deletions
        end
    end

    return {
        files = files,
        total_insertions = total_insertions,
        total_deletions = total_deletions
    }
end

function git:diff(file)
    local repo_path = git:repo_path()
    local handle = io.popen("git diff " ..  file)
    local result = handle:read("*a")
    handle:close()
    return result
end

function git:diff_all()
    local repo_path = git:repo_path()
    local handle = io.popen("git diff " ..  repo_path)
    local result = handle:read("*a")
    handle:close()
    return result
end

function git_branch()
    local handle = io.popen("git rev-parse --abbrev-ref HEAD 2>/dev/null")
    local branch = handle:read("*a")
    handle:close()
    if branch == "" then
        return "N/A"
    else
        return branch:gsub("[\r\n]+", "")
    end
end

function git_status(tab)
    git:refresh_status()
    for file, state in pairs(git.status) do
        if file == tab then
            if state ~= nil then
                return state
            end
        end
    end
    if git.icons then
        return "󰈤 "
    else
        return "U"
    end
end

-- Export the git command
commands["git"] = function(args)
    -- Check if git is installed
    if not git:ready() then
        editor:display_error("Git: git installation not found")
    else
        local repo_path = git:repo_path()
        if args[1] == "commit" then
            local message = editor:prompt("Message")
            editor:display_info("Committing with message: " .. message)
            if select(3, os.execute(string.format("git commit -S -m \"%s\" > /dev/null", message))) ~= 0 then
                editor:display_error("Failed to commit")
            end
        elseif args[1] == "push" then
            if select(3, os.execute("git push")) ~= 0 then
                editor:display_error("Failed to push")
            end
        elseif args[1] == "pull" then
            if select(3, os.execute("git pull")) ~= 0 then
                editor:display_error("Failed to pull")
            end
        elseif args[1] == "add" and args[2] == "all" then
            if select(3, os.execute(string.format("git add %s", repo_path))) ~= 0 then
                editor:display_error("Failed to add all files")
            end
        elseif args[1] == "add" then
            if select(3, os.execute(string.format("git add %s", editor.file_path))) ~= 0 then
                editor:display_error("Failed to add file")
            end
        elseif args[1] == "reset" and args[2] == "all" then
            if select(3, os.execute(string.format("git reset %s > /dev/null", repo_path))) ~= 0 then
                editor:display_error("Failed to unstage all files")
            end
        elseif args[1] == "reset" then
            if select(3, os.execute(string.format("git reset %s > /dev/null", editor.file_path))) ~= 0 then
                editor:display_error("Failed to unstage file")
            end
        elseif args[1] == "stat" and args[2] == "all" then
            local stats = git:get_stats()
            editor:display_info(string.format(
                "%d files changed: %s insertions, %s deletions", 
                #stats.files, stats.total_insertions, stats.total_deletions
            ))
        elseif args[1] == "stat" then
            local stats = git:get_stats()
            for _, t in ipairs(stats.files) do
                if repo_path .. "/" .. t.file == editor.file_path then
                    editor:display_info(string.format(
                        "%s: %s insertions, %s deletions",
                        t.file, t.insertions, t.deletions
                    ))
                end
            end
        elseif args[1] == "diff" and args[2] == "all" then
            local diff = git:diff_all()
            editor:new()
            editor:insert(diff)
            editor:set_file_type("Diff")
            editor:set_read_only(true)
            editor:move_top()
        elseif args[1] == "diff" then
            local diff = git:diff(editor.file_path)
            editor:new()
            editor:insert(diff)
            editor:set_file_type("Diff")
            editor:set_read_only(true)
            editor:move_top()
        end
    end
end


