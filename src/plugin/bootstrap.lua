-- Bootstrap code provides plug-ins and configuration with APIs and other utilities
home = os.getenv("HOME") or os.getenv("USERPROFILE")

if package.config:sub(1,1) == "\\" then
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
        path = file_win
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
    },
    ["Ada"] = {
        icon = "",
        files = {},
        extensions = {"ada"},
        modelines = {},
    },
    ["AutoHotkey"] = {
        icon = " ",
        files = {},
        extensions = {"ahk", "ahkl"},
        modelines = {},
    },
    ["AppleScript"] = {
        icon = "",
        files = {},
        extensions = {"applescript", "scpt"},
        modelines = {},
    },
    ["Arc"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"arc"},
        modelines = {},
    },
    ["ASP"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"asp", "asax", "ascx", "ashx", "asmx", "aspx", "axd"},
        modelines = {},
    },
    ["ActionScript"] = {
        icon = "󰑷 ",
        files = {},
        extensions = {"as"},
        modelines = {},
    },
    ["AGS Script"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"asc", "ash"},
        modelines = {},
    },
    ["Assembly"] = {
        icon = " ",
        files = {},
        extensions = {"asm", "nasm"},
        modelines = {},
    },
    ["Awk"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"awk", "auk", "gawk", "mawk", "nawk"},
        modelines = {"#!\\s*/usr/bin/(env )?awk"},
    },
    ["Batch"] = {
        icon = "󰆍 ",
        files = {},
        extensions = {"bat", "cmd"},
        modelines = {},
    },
    ["Brainfuck"] = {
        icon = " ",
        files = {},
        extensions = {"b", "bf"},
        modelines = {},
    },
    ["C"] = {
        icon = " ",
        files = {},
        extensions = {"c"},
        modelines = {},
    },
    ["CMake"] = {
        icon = " ",
        files = {},
        extensions = {"cmake"},
        modelines = {},
    },
    ["Cobol"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"cbl", "cobol", "cob"},
        modelines = {},
    },
    ["Java"] = {
        icon = " ",
        files = {},
        extensions = {"class", "java"},
        modelines = {},
    },
    ["Clojure"] = {
        icon = " ",
        files = {},
        extensions = {"clj", "cl2", "cljs", "cljx", "cljc"},
        modelines = {},
    },
    ["CoffeeScript"] = {
        icon = " ",
        files = {},
        extensions = {"coffee"},
        modelines = {},
    },
    ["Crystal"] = {
        icon = " ",
        files = {},
        extensions = {"cr"},
        modelines = {},
    },
    ["Cuda"] = {
        icon = " ",
        files = {},
        extensions = {"cu", "cuh"},
        modelines = {},
    },
    ["C++"] = {
        icon = " ",
        files = {},
        extensions = {"cpp", "cxx"},
        modelines = {},
    },
    ["C#"] = {
        icon = " ",
        files = {},
        extensions = {"cs", "cshtml", "csx"},
        modelines = {},
    },
    ["CSS"] = {
        icon = " ",
        files = {},
        extensions = {"css"},
        modelines = {},
    },
    ["CSV"] = {
        icon = " ",
        files = {},
        extensions = {"csv"},
        modelines = {},
    },
    ["D"] = {
        icon = " ",
        files = {},
        extensions = {"d", "di"},
        modelines = {},
    },
    ["Dart"] = {
        icon = " ",
        files = {},
        extensions = {"dart"},
        modelines = {},
    },
    ["Diff"] = {
        icon = " ",
        files = {},
        extensions = {"diff", "patch"},
        modelines = {},
    },
    ["Dockerfile"] = {
        icon = " ",
        files = {},
        extensions = {"dockerfile"},
        modelines = {},
    },
    ["Elixir"] = {
        icon = " ",
        files = {},
        extensions = {"ex", "exs"},
        modelines = {},
    },
    ["Elm"] = {
        icon = " ",
        files = {},
        extensions = {"elm"},
        modelines = {},
    },
    ["Emacs Lisp"] = {
        icon = " ",
        files = {},
        extensions = {"el"},
        modelines = {},
    },
    ["ERB"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"erb"},
        modelines = {},
    },
    ["Erlang"] = {
        icon = " ",
        files = {},
        extensions = {"erl", "es"},
        modelines = {},
    },
    ["F#"] = {
        icon = " ",
        files = {},
        extensions = {"fs", "fsi", "fsx"},
        modelines = {},
    },
    ["FORTRAN"] = {
        icon = "󱈚 ",
        files = {},
        extensions = {"f", "f90", "fpp", "for"},
        modelines = {},
    },
    ["Fish"] = {
        icon = " ",
        files = {},
        extensions = {"fish"},
        modelines = {"#!\\s*/usr/bin/(env )?fish"},
    },
    ["Forth"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"fth"},
        modelines = {},
    },
    ["ANTLR"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"g4"},
        modelines = {},
    },
    ["GDScript"] = {
        icon = " ",
        files = {},
        extensions = {"gd"},
        modelines = {},
    },
    ["GLSL"] = {
        icon = " ",
        files = {},
        extensions = {"glsl", "vert", "shader", "geo", "fshader", "vrx", "vsh", "vshader", "frag"},
        modelines = {},
    },
    ["Gnuplot"] = {
        icon = " ",
        files = {},
        extensions = {"gnu", "gp", "plot"},
        modelines = {},
    },
    ["Go"] = {
        icon = "",
        files = {},
        extensions = {"go"},
        modelines = {},
    },
    ["Groovy"] = {
        icon = " ",
        files = {},
        extensions = {"groovy", "gvy"},
        modelines = {},
    },
    ["HLSL"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"hlsl"},
        modelines = {},
    },
    ["C Header"] = {
        icon = " ",
        files = {},
        extensions = {"h"},
        modelines = {},
    },
    ["Haml"] = {
        icon = "",
        files = {},
        extensions = {"haml"},
        modelines = {},
    },
    ["Handlebars"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"handlebars", "hbs"},
        modelines = {},
    },
    ["Haskell"] = {
        icon = " ",
        files = {},
        extensions = {"hs"},
        modelines = {},
    },
    ["C++ Header"] = {
        icon = " ",
        files = {},
        extensions = {"hpp"},
        modelines = {},
    },
    ["HTML"] = {
        icon = " ",
        files = {},
        extensions = {"html", "htm", "xhtml"},
        modelines = {},
    },
    ["INI"] = {
        icon = " ",
        files = {},
        extensions = {"ini", "cfg"},
        modelines = {},
    },
    ["Arduino"] = {
        icon = " ",
        files = {},
        extensions = {"ino"},
        modelines = {},
    },
    ["J"] = {
        icon = " ",
        files = {},
        extensions = {"ijs"},
        modelines = {},
    },
    ["JSON"] = {
        icon = " ",
        files = {},
        extensions = {"json"},
        modelines = {},
    },
    ["JSX"] = {
        icon = " ",
        files = {},
        extensions = {"jsx"},
        modelines = {},
    },
    ["JavaScript"] = {
        icon = " ",
        files = {},
        extensions = {"js"},
        modelines = {"#!\\s*/usr/bin/(env )?node"},
    },
    ["Julia"] = {
        icon = " ",
        files = {},
        extensions = {"jl"},
        modelines = {},
    },
    ["Kotlin"] = {
        icon = " ",
        files = {},
        extensions = {"kt", "ktm", "kts"},
        modelines = {},
    },
    ["LLVM"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"ll"},
        modelines = {},
    },
    ["Lex"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"l", "lex"},
        modelines = {},
    },
    ["Lua"] = {
        icon = " ",
        files = {".oxrc"},
        extensions = {"lua"},
        modelines = {"#!\\s*/usr/bin/(env )?lua"},
    },
    ["LiveScript"] = {
        icon = " ",
        files = {},
        extensions = {"ls"},
        modelines = {},
    },
    ["LOLCODE"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"lol"},
        modelines = {},
    },
    ["Common Lisp"] = {
        icon = " ",
        files = {},
        extensions = {"lisp", "asd", "lsp"},
        modelines = {},
    },
    ["Log file"] = {
        icon = " ",
        files = {},
        extensions = {"log"},
        modelines = {},
    },
    ["M4"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"m4"},
        modelines = {},
    },
    ["Groff"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"man", "roff"},
        modelines = {},
    },
    ["Matlab"] = {
        icon = " ",
        files = {},
        extensions = {"matlab"},
        modelines = {},
    },
    ["Objective-C"] = {
        icon = " ",
        files = {},
        extensions = {"m"},
        modelines = {},
    },
    ["OCaml"] = {
        icon = " ",
        files = {},
        extensions = {"ml"},
        modelines = {},
    },
    ["Makefile"] = {
        icon = " ",
        files = {"Makefile"},
        extensions = {"mk", "mak"},
        modelines = {},
    },
    ["Markdown"] = {
        icon = " ",
        files = {},
        extensions = {"md", "markdown"},
        modelines = {},
    },
    ["Nix"] = {
        icon = " ",
        files = {},
        extensions = {"nix"},
        modelines = {},
    },
    ["NumPy"] = {
        icon = "󰘨 ",
        files = {},
        extensions = {"numpy"},
        modelines = {},
    },
    ["OpenCL"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"opencl", "cl"},
        modelines = {},
    },
    ["PHP"] = {
        icon = "󰌟 ",
        files = {},
        extensions = {"php"},
        modelines = {"#!\\s*/usr/bin/(env )?php"},
    },
    ["Pascal"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"pas"},
        modelines = {},
    },
    ["Perl"] = {
        icon = " ",
        files = {},
        extensions = {"pl"},
        modelines = {"#!\\s*/usr/bin/(env )?perl"},
    },
    ["PowerShell"] = {
        icon = "󰨊 ",
        files = {},
        extensions = {"psl"},
        modelines = {},
    },
    ["Prolog"] = {
        icon = " ",
        files = {},
        extensions = {"pro"},
        modelines = {},
    },
    ["Python"] = {
        icon = " ",
        files = {},
        extensions = {"py", "pyw"},
        modelines = {"#!\\s*/usr/bin/(env )?python3?"},
    },
    ["Cython"] = {
        icon = " ",
        files = {},
        extensions = {"pyx", "pxd", "pxi"},
        modelines = {},
    },
    ["R"] = {
        icon = " ",
        files = {},
        extensions = {"r"},
        modelines = {},
    },
    ["reStructuredText"] = {
        icon = "󰊄",
        files = {},
        extensions = {"rst"},
        modelines = {},
    },
    ["Racket"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"rkt"},
        modelines = {},
    },
    ["Ruby"] = {
        icon = " ",
        files = {},
        extensions = {"rb", "ruby"},
        modelines = {"#!\\s*/usr/bin/(env )?ruby"},
    },
    ["Rust"] = {
        icon = " ",
        files = {},
        extensions = {"rs"},
        modelines = {"#!\\s*/usr/bin/(env )?rust"},
    },
    ["Shell"] = {
        icon = " ",
        files = {},
        extensions = {"sh"},
        modelines = {
            "#!\\s*/bin/(sh|bash)",
            "#!\\s*/usr/bin/env bash",
        },
    },
    ["SCSS"] = {
        icon = " ",
        files = {},
        extensions = {"scss"},
        modelines = {},
    },
    ["SQL"] = {
        icon = " ",
        files = {},
        extensions = {"sql"},
        modelines = {},
    },
    ["Sass"] = {
        icon = " ",
        files = {},
        extensions = {"sass"},
        modelines = {},
    },
    ["Scala"] = {
        icon = "",
        files = {},
        extensions = {"scala"},
        modelines = {},
    },
    ["Scheme"] = {
        icon = "",
        files = {},
        extensions = {"scm"},
        modelines = {},
    },
    ["Smalltalk"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"st"},
        modelines = {},
    },
    ["Swift"] = {
        icon = " ",
        files = {},
        extensions = {"swift"},
        modelines = {},
    },
    ["TOML"] = {
        icon = " ",
        files = {},
        extensions = {"toml"},
        modelines = {},
    },
    ["Tcl"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"tcl"},
        modelines = {"#!\\s*/usr/bin/(env )?tcl"},
    },
    ["TeX"] = {
        icon = " ",
        files = {},
        extensions = {"tex"},
        modelines = {},
    },
    ["TypeScript"] = {
        icon = " ",
        files = {},
        extensions = {"ts", "tsx"},
        modelines = {},
    },
    ["Plain Text"] = {
        icon = " ",
        files = {},
        extensions = {"txt"},
        modelines = {},
    },
    ["Vala"] = {
        icon = " ",
        files = {},
        extensions = {"vala"},
        modelines = {},
    },
    ["Visual Basic"] = {
        icon = "󰯁 ",
        files = {},
        extensions = {"vb", "vbs"},
        modelines = {},
    },
    ["Vue"] = {
        icon = " ",
        files = {},
        extensions = {"vue"},
        modelines = {},
    },
    ["Logos"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"xm", "x", "xi"},
        modelines = {},
    },
    ["XML"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"xml"},
        modelines = {},
    },
    ["Yacc"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"y", "yacc"},
        modelines = {},
    },
    ["Yaml"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"yaml", "yml"},
        modelines = {},
    },
    ["Bison"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"yxx"},
        modelines = {},
    },
    ["Zsh"] = {
        icon = " ",
        files = {},
        extensions = {"zsh"},
        modelines = {},
    },
}
