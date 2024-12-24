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

-- Behaviour for compiling / running projects
runner = {
    ["Rust"] = {
        compile = "cargo build",
        run = "cargo run",
    },
    ["Python"] = {
        compile = nil,
        run = "python -i {file_path}",
    },
    ["Ruby"] = {
        compile = nil,
        run = "irb -r {file_path}",
    },
}

-- Add types for built-in file type detection
-- Colours are in the format of a string of:
file_types = {
    ["ABAP"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"abap"},
        modelines = {},
        color = "darkblue",
    },
    ["Ada"] = {
        icon = "",
        files = {},
        extensions = {"ada"},
        modelines = {},
        color = "green",
    },
    ["AutoHotkey"] = {
        icon = " ",
        files = {},
        extensions = {"ahk", "ahkl"},
        modelines = {},
        color = "green",
    },
    ["AppleScript"] = {
        icon = "",
        files = {},
        extensions = {"applescript", "scpt"},
        modelines = {},
        color = "grey",
    },
    ["Arc"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"arc"},
        modelines = {},
        color = "pink",
    },
    ["ASP"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"asp", "asax", "ascx", "ashx", "asmx", "aspx", "axd"},
        modelines = {},
        color = "lightblue",
    },
    ["ActionScript"] = {
        icon = "󰑷 ",
        files = {},
        extensions = {"as"},
        modelines = {},
        color = "orange",
    },
    ["AGS Script"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"asc", "ash"},
        modelines = {},
        color = "purple",
    },
    ["Assembly"] = {
        icon = " ",
        files = {},
        extensions = {"asm", "nasm"},
        modelines = {},
        color = "grey",
    },
    ["Awk"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"awk", "auk", "gawk", "mawk", "nawk"},
        modelines = {"#!\\s*/usr/bin/(env )?awk"},
        color = "red",
    },
    ["Batch"] = {
        icon = "󰆍 ",
        files = {},
        extensions = {"bat", "cmd"},
        modelines = {},
        color = "grey",
    },
    ["Brainfuck"] = {
        icon = " ",
        files = {},
        extensions = {"b", "bf"},
        modelines = {},
        color = "yellow",
    },
    ["C"] = {
        icon = " ",
        files = {},
        extensions = {"c"},
        modelines = {},
        color = "lightblue",
    },
    ["CMake"] = {
        icon = " ",
        files = {},
        extensions = {"cmake"},
        modelines = {},
        color = "green",
    },
    ["Cobol"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"cbl", "cobol", "cob"},
        modelines = {},
        color = "purple",
    },
    ["Java"] = {
        icon = " ",
        files = {},
        extensions = {"class", "java"},
        modelines = {},
        color = "orange",
    },
    ["Clojure"] = {
        icon = " ",
        files = {},
        extensions = {"clj", "cl2", "cljs", "cljx", "cljc"},
        modelines = {},
        color = "lightblue",
    },
    ["CoffeeScript"] = {
        icon = " ",
        files = {},
        extensions = {"coffee"},
        modelines = {},
        color = "brown",
    },
    ["Crystal"] = {
        icon = " ",
        files = {},
        extensions = {"cr"},
        modelines = {},
        color = "grey",
    },
    ["Cuda"] = {
        icon = " ",
        files = {},
        extensions = {"cu", "cuh"},
        modelines = {},
        color = "green",
    },
    ["C++"] = {
        icon = " ",
        files = {},
        extensions = {"cpp", "cxx"},
        modelines = {},
        color = "darkblue",
    },
    ["C#"] = {
        icon = " ",
        files = {},
        extensions = {"cs", "cshtml", "csx"},
        modelines = {},
        color = "purple",
    },
    ["CSS"] = {
        icon = " ",
        files = {},
        extensions = {"css"},
        modelines = {},
        color = "purple",
    },
    ["CSV"] = {
        icon = " ",
        files = {},
        extensions = {"csv"},
        modelines = {},
        color = "grey",
    },
    ["D"] = {
        icon = " ",
        files = {},
        extensions = {"d", "di"},
        modelines = {},
        color = "red",
    },
    ["Dart"] = {
        icon = " ",
        files = {},
        extensions = {"dart"},
        modelines = {},
        color = "lightblue",
    },
    ["Diff"] = {
        icon = " ",
        files = {},
        extensions = {"diff", "patch"},
        modelines = {},
        color = "green",
    },
    ["Dockerfile"] = {
        icon = " ",
        files = {},
        extensions = {"dockerfile"},
        modelines = {},
        color = "lightblue",
    },
    ["Elixir"] = {
        icon = " ",
        files = {},
        extensions = {"ex", "exs"},
        modelines = {},
        color = "purple",
    },
    ["Elm"] = {
        icon = " ",
        files = {},
        extensions = {"elm"},
        modelines = {},
        color = "lightblue",
    },
    ["Emacs Lisp"] = {
        icon = " ",
        files = {},
        extensions = {"el"},
        modelines = {},
        color = "purple",
    },
    ["ERB"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"erb"},
        modelines = {},
        color = "red",
    },
    ["Erlang"] = {
        icon = " ",
        files = {},
        extensions = {"erl", "es"},
        modelines = {},
        color = "red",
    },
    ["F#"] = {
        icon = " ",
        files = {},
        extensions = {"fs", "fsi", "fsx"},
        modelines = {},
        color = "lightblue",
    },
    ["FORTRAN"] = {
        icon = "󱈚 ",
        files = {},
        extensions = {"f", "f90", "fpp", "for"},
        modelines = {},
        color = "purple",
    },
    ["Fish"] = {
        icon = " ",
        files = {},
        extensions = {"fish"},
        modelines = {"#!\\s*/usr/bin/(env )?fish"},
        color = "orange",
    },
    ["Forth"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"fth"},
        modelines = {},
        color = "red",
    },
    ["ANTLR"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"g4"},
        modelines = {},
        color = "red",
    },
    ["GDScript"] = {
        icon = " ",
        files = {},
        extensions = {"gd"},
        modelines = {},
        color = "darkblue",
    },
    ["GLSL"] = {
        icon = " ",
        files = {},
        extensions = {"glsl", "vert", "shader", "geo", "fshader", "vrx", "vsh", "vshader", "frag"},
        modelines = {},
        color = "lightblue",
    },
    ["Gnuplot"] = {
        icon = " ",
        files = {},
        extensions = {"gnu", "gp", "plot"},
        modelines = {},
        color = "grey",
    },
    ["Go"] = {
        icon = "",
        files = {},
        extensions = {"go"},
        modelines = {},
        color = "lightblue",
    },
    ["Groovy"] = {
        icon = " ",
        files = {},
        extensions = {"groovy", "gvy"},
        modelines = {},
        color = "lightblue",
    },
    ["HLSL"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"hlsl"},
        modelines = {},
        color = "darkblue",
    },
    ["C Header"] = {
        icon = " ",
        files = {},
        extensions = {"h"},
        modelines = {},
        color = "lightblue",
    },
    ["Haml"] = {
        icon = "",
        files = {},
        extensions = {"haml"},
        modelines = {},
        color = "yellow",
    },
    ["Handlebars"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"handlebars", "hbs"},
        modelines = {},
        color = "brown",
    },
    ["Haskell"] = {
        icon = " ",
        files = {},
        extensions = {"hs"},
        modelines = {},
        color = "purple",
    },
    ["C++ Header"] = {
        icon = " ",
        files = {},
        extensions = {"hpp"},
        modelines = {},
        color = "darkblue",
    },
    ["HTML"] = {
        icon = " ",
        files = {},
        extensions = {"html", "htm", "xhtml"},
        modelines = {},
        color = "orange",
    },
    ["INI"] = {
        icon = " ",
        files = {},
        extensions = {"ini", "cfg"},
        modelines = {},
        color = "grey",
    },
    ["Arduino"] = {
        icon = " ",
        files = {},
        extensions = {"ino"},
        modelines = {},
        color = "lightblue",
    },
    ["J"] = {
        icon = " ",
        files = {},
        extensions = {"ijs"},
        modelines = {},
        color = "lightblue",
    },
    ["JSON"] = {
        icon = " ",
        files = {},
        extensions = {"json"},
        modelines = {},
        color = "grey",
    },
    ["JSX"] = {
        icon = " ",
        files = {},
        extensions = {"jsx"},
        modelines = {},
        color = "lightblue",
    },
    ["JavaScript"] = {
        icon = " ",
        files = {},
        extensions = {"js"},
        modelines = {"#!\\s*/usr/bin/(env )?node"},
        color = "yellow",
    },
    ["Julia"] = {
        icon = " ",
        files = {},
        extensions = {"jl"},
        modelines = {},
        color = "lightblue",
    },
    ["Kotlin"] = {
        icon = " ",
        files = {},
        extensions = {"kt", "ktm", "kts"},
        modelines = {},
        color = "purple",
    },
    ["LLVM"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"ll"},
        modelines = {},
        color = "lightblue",
    },
    ["Lex"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"l", "lex"},
        modelines = {},
        color = "grey",
    },
    ["Lua"] = {
        icon = " ",
        files = {".oxrc"},
        extensions = {"lua"},
        modelines = {"#!\\s*/usr/bin/(env )?lua"},
        color = "darkblue",
    },
    ["LiveScript"] = {
        icon = " ",
        files = {},
        extensions = {"ls"},
        modelines = {},
        color = "lightblue",
    },
    ["LOLCODE"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"lol"},
        modelines = {},
        color = "red",
    },
    ["Common Lisp"] = {
        icon = " ",
        files = {},
        extensions = {"lisp", "asd", "lsp"},
        modelines = {},
        color = "grey",
    },
    ["Log file"] = {
        icon = " ",
        files = {},
        extensions = {"log"},
        modelines = {},
        color = "grey",
    },
    ["M4"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"m4"},
        modelines = {},
        color = "darkblue",
    },
    ["Groff"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"man", "roff"},
        modelines = {},
        color = "grey",
    },
    ["Matlab"] = {
        icon = " ",
        files = {},
        extensions = {"matlab"},
        modelines = {},
        color = "orange",
    },
    ["Nushell"] = {
        icon = " ",
        files = {},
        extensions = {"nu"},
        modelines = {},
        color = "green",
    },
    ["Objective-C"] = {
        icon = " ",
        files = {},
        extensions = {"m"},
        modelines = {},
        color = "lightblue",
    },
    ["OCaml"] = {
        icon = " ",
        files = {},
        extensions = {"ml"},
        modelines = {},
        color = "orange",
    },
    ["Makefile"] = {
        icon = " ",
        files = {"Makefile"},
        extensions = {"mk", "mak"},
        modelines = {},
        color = "grey",
    },
    ["Markdown"] = {
        icon = " ",
        files = {},
        extensions = {"md", "markdown"},
        modelines = {},
        color = "grey",
    },
    ["Nix"] = {
        icon = " ",
        files = {},
        extensions = {"nix"},
        modelines = {},
        color = "lightblue",
    },
    ["NumPy"] = {
        icon = "󰘨 ",
        files = {},
        extensions = {"numpy"},
        modelines = {},
        color = "darkblue",
    },
    ["OpenCL"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"opencl", "cl"},
        modelines = {},
        color = "green",
    },
    ["PHP"] = {
        icon = "󰌟 ",
        files = {},
        extensions = {"php"},
        modelines = {"#!\\s*/usr/bin/(env )?php"},
        color = "lightblue",
    },
    ["Pascal"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"pas"},
        modelines = {},
        color = "darkblue",
    },
    ["Perl"] = {
        icon = " ",
        files = {},
        extensions = {"pl"},
        modelines = {"#!\\s*/usr/bin/(env )?perl"},
        color = "lightblue",
    },
    ["PowerShell"] = {
        icon = "󰨊 ",
        files = {},
        extensions = {"psl"},
        modelines = {},
        color = "lightblue",
    },
    ["Prolog"] = {
        icon = " ",
        files = {},
        extensions = {"pro"},
        modelines = {},
        color = "orange",
    },
    ["Python"] = {
        icon = " ",
        files = {},
        extensions = {"py", "pyw"},
        modelines = {"#!\\s*/usr/bin/(env )?python3?"},
        color = "lightblue",
    },
    ["Cython"] = {
        icon = " ",
        files = {},
        extensions = {"pyx", "pxd", "pxi"},
        modelines = {},
        color = "darkblue",
    },
    ["R"] = {
        icon = " ",
        files = {},
        extensions = {"r"},
        modelines = {},
        color = "darkblue",
    },
    ["reStructuredText"] = {
        icon = "󰊄",
        files = {},
        extensions = {"rst"},
        modelines = {},
        color = "grey",
    },
    ["Racket"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"rkt"},
        modelines = {},
        color = "red",
    },
    ["Ruby"] = {
        icon = " ",
        files = {},
        extensions = {"rb", "ruby"},
        modelines = {"#!\\s*/usr/bin/(env )?ruby"},
        color = "red",
    },
    ["Rust"] = {
        icon = " ",
        files = {},
        extensions = {"rs"},
        modelines = {"#!\\s*/usr/bin/(env )?rust"},
        color = "orange",
    },
    ["Shell"] = {
        icon = " ",
        files = {},
        extensions = {"sh"},
        modelines = {
            "#!\\s*/bin/(sh|bash)",
            "#!\\s*/usr/bin/env bash",
        },
        color = "grey",
    },
    ["SCSS"] = {
        icon = " ",
        files = {},
        extensions = {"scss"},
        modelines = {},
        color = "pink",
    },
    ["SQL"] = {
        icon = " ",
        files = {},
        extensions = {"sql"},
        modelines = {},
        color = "lightblue",
    },
    ["Sass"] = {
        icon = " ",
        files = {},
        extensions = {"sass"},
        modelines = {},
        color = "pink",
    },
    ["Scala"] = {
        icon = "",
        files = {},
        extensions = {"scala"},
        modelines = {},
        color = "red",
    },
    ["Scheme"] = {
        icon = "",
        files = {},
        extensions = {"scm"},
        modelines = {},
        color = "grey",
    },
    ["Smalltalk"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"st"},
        modelines = {},
        color = "lightblue",
    },
    ["Swift"] = {
        icon = " ",
        files = {},
        extensions = {"swift"},
        modelines = {},
        color = "orange",
    },
    ["TOML"] = {
        icon = " ",
        files = {},
        extensions = {"toml"},
        modelines = {},
        color = "orange",
    },
    ["Tcl"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"tcl"},
        modelines = {"#!\\s*/usr/bin/(env )?tcl"},
        color = "red",
    },
    ["TeX"] = {
        icon = " ",
        files = {},
        extensions = {"tex"},
        modelines = {},
        color = "grey",
    },
    ["TypeScript"] = {
        icon = " ",
        files = {},
        extensions = {"ts", "tsx"},
        modelines = {},
        color = "darkblue",
    },
    ["Plain Text"] = {
        icon = " ",
        files = {},
        extensions = {"txt"},
        modelines = {},
        color = "grey",
    },
    ["Vala"] = {
        icon = " ",
        files = {},
        extensions = {"vala"},
        modelines = {},
        color = "purple",
    },
    ["Visual Basic"] = {
        icon = "󰯁 ",
        files = {},
        extensions = {"vb", "vbs"},
        modelines = {},
        color = "purple",
    },
    ["Vue"] = {
        icon = " ",
        files = {},
        extensions = {"vue"},
        modelines = {},
        color = "green",
    },
    ["Logos"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"xm", "x", "xi"},
        modelines = {},
        color = "red",
    },
    ["XML"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"xml"},
        modelines = {},
        color = "lightblue",
    },
    ["Yacc"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"y", "yacc"},
        modelines = {},
        color = "grey",
    },
    ["Yaml"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"yaml", "yml"},
        modelines = {},
        color = "red",
    },
    ["Bison"] = {
        icon = "󰅩 ",
        files = {},
        extensions = {"yxx"},
        modelines = {},
        color = "grey",
    },
    ["Zsh"] = {
        icon = " ",
        files = {},
        extensions = {"zsh"},
        modelines = {},
        color = "orange",
    },
}
