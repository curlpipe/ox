-- Bootstrap code provides plug-ins and configuration with APIs and other utilities
home = os.getenv("HOME") or os.getenv("USERPROFILE")
path_sep = package.config:sub(1,1)

if path_sep == "\\" then
    plugin_path = home .. "\\ox"
else
    plugin_path = home .. "/.config/ox"
end

function file_exists(file_path)
    local file = io.open(file_path, "r")
    if file then
        file:close()
        return true
    else
        return false
    end
end

function dir_exists(dir_path)
    -- Check if the directory exists using the appropriate command
    local is_windows = package.config:sub(1, 1) == '\\'  -- Check if Windows
    local command
    if is_windows then
        command = "if exist \"" .. dir_path .. "\" (exit 0) else (exit 1)"
    else
        command = "if [ -d \"" .. dir_path .. "\" ]; then exit 0; else exit 1; fi"
    end
    -- Execute the command
    local result = shell:run(command)
    return result == 0
end

plugins = {}
builtins = {}
plugin_issues = false

function load_plugin(base)
    path_cross = base
    path_unix = home .. "/.config/ox/" .. base
    path_win = home .. "\\ox\\" .. base
    if file_exists(path_cross) then
        path = path_cross
    elseif file_exists(path_unix) then
        path = path_unix
    elseif file_exists(path_win) then
        path = path_win
    else
        path = nil
        -- Prevent warning if plug-in is built-in
        local is_autoindent = base:match("autoindent.lua$") ~= nil
        local is_pairs = base:match("pairs.lua$") ~= nil
        local is_quickcomment = base:match("quickcomment.lua$") ~= nil
        if not is_pairs and not is_autoindent and not is_quickcomment then
            -- Issue warning if plug-in is builtin
            print("[WARNING] Failed to load plugin " .. base)
            plugin_issues = true
        else
            table.insert(builtins, base)
        end
    end
    if path ~= nil then
        plugins[#plugins + 1] = path
    end
end

-- Python interoperability tools
python_interop = {}

function python_interop:installation()
    -- Try to capture Python version output using io.popen
    local python_handle = io.popen("python --version 2>&1")
    local python_output = python_handle:read("*a")
    python_handle:close()
    if python_output:find("Python") then
        return 2
    end

    -- If 'python' didn't work, try 'python3'
    local python3_handle = io.popen("python3 --version 2>&1")
    local python3_output = python3_handle:read("*a")
    python3_handle:close()
    if python3_output:find("Python") then
        return 3
    end

    return nil
end

function python_interop:has_module(module_name)
    -- Use python -c "import <module>"
    local command = "python -c \"import " .. module_name .. "\" 2>&1"
    local handle = io.popen(command)
    local result = handle:read("*a")
    handle:close()

    if result == "" then  -- No output means successful import
        return true
    end

    -- Try with python3 in case 'python' is python 2.x
    command = "python3 -c \"import " .. module_name .. "\" 2>&1"
    local handle_python3 = io.popen(command)
    local result_python3 = handle_python3:read("*a")
    handle_python3:close()

    return result_python3 == ""
end

-- Command line interaction
shell = {
    is_windows = os.getenv("OS") and os.getenv("OS"):find("Windows") ~= nil,
}

function shell:run(cmd)
    -- Runs a command (silently) and return the exit code
    if self.is_windows then
        return select(3, os.execute(cmd .. " > NUL 2>&1"))
    else
        return select(3, os.execute(cmd .. " > /dev/null 2>&1"))
    end
end

function shell:output(cmd)
    -- Runs a command (silently) and returns stdout and stderr together in a single string
    local command = cmd .. " 2>&1"
    local handle = io.popen(command)
    local result = handle:read("*a")
    handle:close()
    return result
end

function shell:spawn(cmd)
    -- Spawns a command (silently), and have it run in the background
    -- Returns PID so process can be killed later
    if self.is_windows then
        editor:display_error("Shell spawning is unavailable on Windows")
        editor:rerender_feedback_line()
        return nil
    else
        local command = cmd .. " > /dev/null 2>&1 & echo $!"
        local pid = shell:output(command)
        pid = pid:gsub("%s+", "")
        pid = pid:gsub("\\n", "")
        pid = pid:gsub("\\t", "")
        return pid
    end
end

function shell:kill(pid)
    if self.is_windows then
        editor:display_error("Shell spawning is unavailable on Windows")
        editor:rerender_feedback_line()
        return nil
    elseif pid ~= nil then
        shell:run("kill " .. tostring(pid))
    end
end

-- Add types for built-in file type detection
file_types = {
    ["ABAP"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"abap"},
        modelines = {},
        color = 21,
    },
    ["Ada"] = {
        icon = "",
        files = {},
        extensions = {"ada"},
        modelines = {},
        color = 28,
    },
    ["AutoHotkey"] = {
        icon = " ",
        files = {},
        extensions = {"ahk", "ahkl"},
        modelines = {},
        color = 157,
    },
    ["AppleScript"] = {
        icon = "",
        files = {},
        extensions = {"applescript", "scpt"},
        modelines = {},
        color = 252,
    },
    ["Arc"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"arc"},
        modelines = {},
        color = 125,
    },
    ["ASP"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"asp", "asax", "ascx", "ashx", "asmx", "aspx", "axd"},
        modelines = {},
        color = 33,
    },
    ["ActionScript"] = {
        icon = "󰑷 ",
        files = {},
        extensions = {"as"},
        modelines = {},
        color = 202,
    },
    ["AGS Script"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"asc", "ash"},
        modelines = {},
        color = 69,
    },
    ["Assembly"] = {
        icon = " ",
        files = {},
        extensions = {"asm", "nasm"},
        modelines = {},
        color = 250,
    },
    ["Awk"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"awk", "auk", "gawk", "mawk", "nawk"},
        modelines = {"#!\\s*/usr/bin/(env )?awk"},
        color = 160,
    },
    ["Batch"] = {
        icon = "󰆍 ",
        files = {},
        extensions = {"bat", "cmd"},
        modelines = {},
        color = 250,
    },
    ["Brainfuck"] = {
        icon = " ",
        files = {},
        extensions = {"b", "bf"},
        modelines = {},
        color = 226,
    },
    ["C"] = {
        icon = " ",
        files = {},
        extensions = {"c"},
        modelines = {},
        color = 33,
    },
    ["CMake"] = {
        icon = " ",
        files = {},
        extensions = {"cmake"},
        modelines = {},
        color = 40,
    },
    ["Cobol"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"cbl", "cobol", "cob"},
        modelines = {},
        color = 57,
    },
    ["Java"] = {
        icon = " ",
        files = {},
        extensions = {"class", "java"},
        modelines = {},
        color = 202,
    },
    ["Clojure"] = {
        icon = " ",
        files = {},
        extensions = {"clj", "cl2", "cljs", "cljx", "cljc"},
        modelines = {},
        color = 37,
    },
    ["CoffeeScript"] = {
        icon = " ",
        files = {},
        extensions = {"coffee"},
        modelines = {},
        color = 130,
    },
    ["Crystal"] = {
        icon = " ",
        files = {},
        extensions = {"cr"},
        modelines = {},
        color = 241,
    },
    ["Cuda"] = {
        icon = " ",
        files = {},
        extensions = {"cu", "cuh"},
        modelines = {},
        color = 76,
    },
    ["C++"] = {
        icon = " ",
        files = {},
        extensions = {"cpp", "cxx"},
        modelines = {},
        color = 26,
    },
    ["C#"] = {
        icon = " ",
        files = {},
        extensions = {"cs", "cshtml", "csx"},
        modelines = {},
        color = 63,
    },
    ["CSS"] = {
        icon = " ",
        files = {},
        extensions = {"css"},
        modelines = {},
        color = 99,
    },
    ["CSV"] = {
        icon = " ",
        files = {},
        extensions = {"csv"},
        modelines = {},
        color = 248,
    },
    ["D"] = {
        icon = " ",
        files = {},
        extensions = {"d", "di"},
        modelines = {},
        color = 197,
    },
    ["Dart"] = {
        icon = " ",
        files = {},
        extensions = {"dart"},
        modelines = {},
        color = 33,
    },
    ["Diff"] = {
        icon = " ",
        files = {},
        extensions = {"diff", "patch"},
        modelines = {},
        color = 28,
    },
    ["Dockerfile"] = {
        icon = " ",
        files = {},
        extensions = {"dockerfile"},
        modelines = {},
        color = 33,
    },
    ["Elixir"] = {
        icon = " ",
        files = {},
        extensions = {"ex", "exs"},
        modelines = {},
        color = 56,
    },
    ["Elm"] = {
        icon = " ",
        files = {},
        extensions = {"elm"},
        modelines = {},
        color = 26,
    },
    ["Emacs Lisp"] = {
        icon = " ",
        files = {},
        extensions = {"el"},
        modelines = {},
        color = 63,
    },
    ["ERB"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"erb"},
        modelines = {},
        color = 196,
    },
    ["Erlang"] = {
        icon = " ",
        files = {},
        extensions = {"erl", "es"},
        modelines = {},
        color = 196,
    },
    ["F#"] = {
        icon = " ",
        files = {},
        extensions = {"fs", "fsi", "fsx"},
        modelines = {},
        color = 39,
    },
    ["FORTRAN"] = {
        icon = "󱈚 ",
        files = {},
        extensions = {"f", "f90", "fpp", "for"},
        modelines = {},
        color = 129,
    },
    ["Fish"] = {
        icon = " ",
        files = {},
        extensions = {"fish"},
        modelines = {"#!\\s*/usr/bin/(env )?fish"},
        color = 203,
    },
    ["Forth"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"fth"},
        modelines = {},
        color = 196,
    },
    ["ANTLR"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"g4"},
        modelines = {},
        color = 196,
    },
    ["GDScript"] = {
        icon = " ",
        files = {},
        extensions = {"gd"},
        modelines = {},
        color = 27,
    },
    ["GLSL"] = {
        icon = " ",
        files = {},
        extensions = {"glsl", "vert", "shader", "geo", "fshader", "vrx", "vsh", "vshader", "frag"},
        modelines = {},
        color = 33,
    },
    ["Gnuplot"] = {
        icon = " ",
        files = {},
        extensions = {"gnu", "gp", "plot"},
        modelines = {},
        color = 249,
    },
    ["Go"] = {
        icon = "",
        files = {},
        extensions = {"go"},
        modelines = {},
        color = 33,
    },
    ["Groovy"] = {
        icon = " ",
        files = {},
        extensions = {"groovy", "gvy"},
        modelines = {},
        color = 39,
    },
    ["HLSL"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"hlsl"},
        modelines = {},
        color = 21,
    },
    ["C Header"] = {
        icon = " ",
        files = {},
        extensions = {"h"},
        modelines = {},
        color = 33,
    },
    ["Haml"] = {
        icon = "",
        files = {},
        extensions = {"haml"},
        modelines = {},
        color = 228,
    },
    ["Handlebars"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"handlebars", "hbs"},
        modelines = {},
        color = 94,
    },
    ["Haskell"] = {
        icon = " ",
        files = {},
        extensions = {"hs"},
        modelines = {},
        color = 93,
    },
    ["C++ Header"] = {
        icon = " ",
        files = {},
        extensions = {"hpp"},
        modelines = {},
        color = 26,
    },
    ["HTML"] = {
        icon = " ",
        files = {},
        extensions = {"html", "htm", "xhtml"},
        modelines = {},
        color = 208,
    },
    ["INI"] = {
        icon = " ",
        files = {},
        extensions = {"ini", "cfg"},
        modelines = {},
        color = 248,
    },
    ["Arduino"] = {
        icon = " ",
        files = {},
        extensions = {"ino"},
        modelines = {},
        color = 75,
    },
    ["J"] = {
        icon = " ",
        files = {},
        extensions = {"ijs"},
        modelines = {},
        color = 45,
    },
    ["JSON"] = {
        icon = " ",
        files = {},
        extensions = {"json"},
        modelines = {},
        color = 247,
    },
    ["JSX"] = {
        icon = " ",
        files = {},
        extensions = {"jsx"},
        modelines = {},
        color = 33,
    },
    ["JavaScript"] = {
        icon = " ",
        files = {},
        extensions = {"js"},
        modelines = {"#!\\s*/usr/bin/(env )?node"},
        color = 220,
    },
    ["Julia"] = {
        icon = " ",
        files = {},
        extensions = {"jl"},
        modelines = {},
        color = 27,
    },
    ["Kotlin"] = {
        icon = " ",
        files = {},
        extensions = {"kt", "ktm", "kts"},
        modelines = {},
        color = 129,
    },
    ["LLVM"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"ll"},
        modelines = {},
        color = 39,
    },
    ["Lex"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"l", "lex"},
        modelines = {},
        color = 249,
    },
    ["Lua"] = {
        icon = " ",
        files = {".oxrc"},
        extensions = {"lua"},
        modelines = {"#!\\s*/usr/bin/(env )?lua"},
        color = 27,
    },
    ["LiveScript"] = {
        icon = " ",
        files = {},
        extensions = {"ls"},
        modelines = {},
        color = 39,
    },
    ["LOLCODE"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"lol"},
        modelines = {},
        color = 160,
    },
    ["Common Lisp"] = {
        icon = " ",
        files = {},
        extensions = {"lisp", "asd", "lsp"},
        modelines = {},
        color = 243,
    },
    ["Log file"] = {
        icon = " ",
        files = {},
        extensions = {"log"},
        modelines = {},
        color = 248,
    },
    ["M4"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"m4"},
        modelines = {},
        color = 21,
    },
    ["Groff"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"man", "roff"},
        modelines = {},
        color = 249,
    },
    ["Matlab"] = {
        icon = " ",
        files = {},
        extensions = {"matlab"},
        modelines = {},
        color = 202,
    },
    ["Objective-C"] = {
        icon = " ",
        files = {},
        extensions = {"m"},
        modelines = {},
        color = 27,
    },
    ["OCaml"] = {
        icon = " ",
        files = {},
        extensions = {"ml"},
        modelines = {},
        color = 208,
    },
    ["Makefile"] = {
        icon = " ",
        files = {"Makefile"},
        extensions = {"mk", "mak"},
        modelines = {},
        color = 249,
    },
    ["Markdown"] = {
        icon = " ",
        files = {},
        extensions = {"md", "markdown"},
        modelines = {},
        color = 243,
    },
    ["Nix"] = {
        icon = " ",
        files = {},
        extensions = {"nix"},
        modelines = {},
        color = 33,
    },
    ["NumPy"] = {
        icon = "󰘨 ",
        files = {},
        extensions = {"numpy"},
        modelines = {},
        color = 27,
    },
    ["OpenCL"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"opencl", "cl"},
        modelines = {},
        color = 76,
    },
    ["PHP"] = {
        icon = "󰌟 ",
        files = {},
        extensions = {"php"},
        modelines = {"#!\\s*/usr/bin/(env )?php"},
        color = 69,
    },
    ["Pascal"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"pas"},
        modelines = {},
        color = 21,
    },
    ["Perl"] = {
        icon = " ",
        files = {},
        extensions = {"pl"},
        modelines = {"#!\\s*/usr/bin/(env )?perl"},
        color = 39,
    },
    ["PowerShell"] = {
        icon = "󰨊 ",
        files = {},
        extensions = {"psl"},
        modelines = {},
        color = 33,
    },
    ["Prolog"] = {
        icon = " ",
        files = {},
        extensions = {"pro"},
        modelines = {},
        color = 202,
    },
    ["Python"] = {
        icon = " ",
        files = {},
        extensions = {"py", "pyw"},
        modelines = {"#!\\s*/usr/bin/(env )?python3?"},
        color = 33,
    },
    ["Cython"] = {
        icon = " ",
        files = {},
        extensions = {"pyx", "pxd", "pxi"},
        modelines = {},
        color = 27,
    },
    ["R"] = {
        icon = " ",
        files = {},
        extensions = {"r"},
        modelines = {},
        color = 27,
    },
    ["reStructuredText"] = {
        icon = "󰊄",
        files = {},
        extensions = {"rst"},
        modelines = {},
        color = 243,
    },
    ["Racket"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"rkt"},
        modelines = {},
        color = 160,
    },
    ["Ruby"] = {
        icon = " ",
        files = {},
        extensions = {"rb", "ruby"},
        modelines = {"#!\\s*/usr/bin/(env )?ruby"},
        color = 196,
    },
    ["Rust"] = {
        icon = " ",
        files = {},
        extensions = {"rs"},
        modelines = {"#!\\s*/usr/bin/(env )?rust"},
        color = 166,
    },
    ["Shell"] = {
        icon = " ",
        files = {},
        extensions = {"sh"},
        modelines = {
            "#!\\s*/bin/(sh|bash)",
            "#!\\s*/usr/bin/env bash",
        },
        color = 250,
    },
    ["SCSS"] = {
        icon = " ",
        files = {},
        extensions = {"scss"},
        modelines = {},
        color = 200,
    },
    ["SQL"] = {
        icon = " ",
        files = {},
        extensions = {"sql"},
        modelines = {},
        color = 75,
    },
    ["Sass"] = {
        icon = " ",
        files = {},
        extensions = {"sass"},
        modelines = {},
        color = 200,
    },
    ["Scala"] = {
        icon = "",
        files = {},
        extensions = {"scala"},
        modelines = {},
        color = 196,
    },
    ["Scheme"] = {
        icon = "",
        files = {},
        extensions = {"scm"},
        modelines = {},
        color = 243,
    },
    ["Smalltalk"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"st"},
        modelines = {},
        color = 33,
    },
    ["Swift"] = {
        icon = " ",
        files = {},
        extensions = {"swift"},
        modelines = {},
        color = 202,
    },
    ["TOML"] = {
        icon = " ",
        files = {},
        extensions = {"toml"},
        modelines = {},
        color = 209,
    },
    ["Tcl"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"tcl"},
        modelines = {"#!\\s*/usr/bin/(env )?tcl"},
        color = 196,
    },
    ["TeX"] = {
        icon = " ",
        files = {},
        extensions = {"tex"},
        modelines = {},
        color = 243,
    },
    ["TypeScript"] = {
        icon = " ",
        files = {},
        extensions = {"ts", "tsx"},
        modelines = {},
        color = 27,
    },
    ["Plain Text"] = {
        icon = " ",
        files = {},
        extensions = {"txt"},
        modelines = {},
        color = 250,
    },
    ["Vala"] = {
        icon = " ",
        files = {},
        extensions = {"vala"},
        modelines = {},
        color = 135,
    },
    ["Visual Basic"] = {
        icon = "󰯁 ",
        files = {},
        extensions = {"vb", "vbs"},
        modelines = {},
        color = 69,
    },
    ["Vue"] = {
        icon = " ",
        files = {},
        extensions = {"vue"},
        modelines = {},
        color = 40,
    },
    ["Logos"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"xm", "x", "xi"},
        modelines = {},
        color = 196,
    },
    ["XML"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"xml"},
        modelines = {},
        color = 33,
    },
    ["Yacc"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"y", "yacc"},
        modelines = {},
        color = 249,
    },
    ["Yaml"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"yaml", "yml"},
        modelines = {},
        color = 161,
    },
    ["Bison"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"yxx"},
        modelines = {},
        color = 249,
    },
    ["Zsh"] = {
        icon = " ",
        files = {},
        extensions = {"zsh"},
        modelines = {},
        color = 208,
    },
}
