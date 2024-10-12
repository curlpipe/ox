-- Bootstrap plug-ins
home = os.getenv("HOME") or os.getenv("USERPROFILE")

function file_exists(file_path)
    local file = io.open(file_path, "r")
    if file then
        file:close()
        return true
    else
        return false
    end
end

plugins = {}
builtins = {}
plugin_issues = false

function load_plugin(base)
    path_cross = base
    path_unix = home .. "/.config/ox/" .. base
    path_win = home .. "/ox/" .. base
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
        if not is_pairs and not is_autoindent then 
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

-- Populate the document object with built-in file type detection
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
    ["Elixr"] = {
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
        files = {},
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
