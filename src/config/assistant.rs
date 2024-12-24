/// Code for the configuration set-up assistant
use crate::cli::VERSION;
use crate::config::{Color, Colors, Indentation, SyntaxHighlighting};
use crate::error::Result;
use crate::{PLUGIN_BOOTSTRAP, PLUGIN_MANAGER, PLUGIN_NETWORKING};
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::style::{SetBackgroundColor as Bg, SetForegroundColor as Fg};
use crossterm::terminal::{Clear, ClearType};
use mlua::prelude::*;
use std::io::{stdout, Write};

pub const TROPICAL: &str = include_str!("../../plugins/themes/tropical.lua");
pub const GALAXY: &str = include_str!("../../plugins/themes/galaxy.lua");
pub const TRANSPARENT: &str = include_str!("../../plugins/themes/transparent.lua");
pub const DEFAULT16: &str = include_str!("../../plugins/themes/default16.lua");
pub const OMNI: &str = include_str!("../../plugins/themes/omni.lua");

#[macro_export]
macro_rules! gets {
    () => {{
        let mut s = std::string::String::new();
        std::io::stdin().read_line(&mut s).unwrap();
        s.trim_end_matches(&['\n', '\r'][..]).to_owned()
    }};
    ( $($args:tt)* ) => {{
        use std::io::Write;
        print!("{}", format_args!($($args)*));
        std::io::stdout().flush().unwrap();
        $crate::gets!()
    }};
}

const TITLE: &str = r"
  ___          ____             __ _            _            _     _              _
 / _ \__  __  / ___|___  _ __  / _(_) __ _     / \   ___ ___(_)___| |_ __ _ _ __ | |_
| | | \ \/ / | |   / _ \| '_ \| |_| |/ _` |   / _ \ / __/ __| / __| __/ _` | '_ \| __|
| |_| |>  <  | |__| (_) | | | |  _| | (_| |  / ___ \\__ \__ \ \__ \ || (_| | | | | |_
 \___//_/\_\  \____\___/|_| |_|_| |_|\__, | /_/   \_\___/___/_|___/\__\__,_|_| |_|\__|
                                     |___/
";

const NO_CONFIG_MESSAGE: &str = r"
Thank you for installing Ox
We noticed you don't have a configuration file.
This set-up process will help you customise and configure Ox.
This way, you'll have a better user experience out of the box.
";

const INTRODUCTION: &str = r"
Welcome to the configuration assistant for the Ox Editor.
This is a tool that will help get Ox set up for you.
It will take no more than 3 minutes and the config assistant will not show again after set-up.
You can always re-access this tool using `ox --config-assist`

";

const PLUGIN_LIST: &str = r"
Ox has an ecosystem of plug-ins that you can make use of, they are as follows:

──────────────────── Code Helpers ────────────────────
AutoIndent - A plug-in that will insert and remove code indentation automatically
Pairs - A plug-in that will insert end brackets and end quotes automatically
QuickComment - A plug-in that will help you quickly comment and uncomment lines of code
AI - A plug-in that will provide advice and code within your code files

─────────────────── Web Development ──────────────────
Emmet - A neat language to help you write HTML quickly - requires python and the py-emmet module
Live HTML - Start a web server that previews your HTML site as you write the code - requires python and the Flask module

────────── Integration with Existing Tools ───────────
DiscordRPC - Have Ox interact with Discord and show your programming activity - requires python and the discord-rpc module
Git - View and manage your git repository - requires git to be installed

─────────────────── Miscellaneous ────────────────────
Pomodoro - A timer that helps you track your periods of work and breaks
Todo - Makes .todo files interactive todo lists
Typing Speed - Shows the rough speed that you're typing in the status line
Update Notification - Warns you if there is a new version of Ox - requires curl to be installed on unix based systems
";

const FINAL_WORDS: &str = r"
Configuration file was successfully generated.

Just a few things before you go:

See the documentation here: https://github.com/curlpipe/ox/wiki
Report any bugs or request new features here: https://github.com/curlpipe/ox/issues/new/choose

Remember: You can press Ctrl + H when you are in the editor to reveal a help message to get started

I hope you enjoy your Ox experience

Ready? Press the enter key to start Ox
";

const PLUGIN_INSTALL: &str = r#"
if not plugin_manager:plugin_downloaded('{name}') then
    print("Installing " .. '{name}')
    plugin_manager:download_plugin('{name}')
end
"#;

const AI_INTRO_GEMINI: &str = r"You'll need a Google Gemini API key.
Don't worry, as of 2024, you can use this API free of charge (but it is rate limited).
More information regarding pricing: https://ai.google.dev/pricing#1_5flash

Follow the instructions below to obtain your own free API key:

1. Navigate to the following link:
https://aistudio.google.com/app/apikey

2. Sign in with a google account if asked

3. Click 'Create API key'

4. Check that your API key is free of charge under the `Plan` field in the table (to avoid any surprise bills)

5. Copy the stream of letters and numbers underneath the `API Key` field in the table

6. Paste it below (without spaces)";

#[derive(PartialEq)]
pub enum Theme {
    Tropical,
    Galaxy,
    Transparent,
    Default,
    Default16,
    Omni,
}

impl Theme {
    pub fn to_config(&self) -> String {
        match self {
            Self::Tropical => TROPICAL,
            Self::Galaxy => GALAXY,
            Self::Transparent => TRANSPARENT,
            Self::Omni => OMNI,
            Self::Default16 => DEFAULT16,
            Self::Default => "",
        }
        .to_string()
    }
}

#[derive(PartialEq, Debug)]
pub enum Plugin {
    AutoIndent,
    Pairs,
    QuickComment,
    DiscordRPC,
    Emmet,
    Git,
    LiveHTML,
    Pomodoro,
    Todo,
    TypingSpeed,
    UpdateNotification,
    AI,
}

impl Plugin {
    pub fn to_config(&self) -> String {
        let plugin_name = self.name();
        format!("load_plugin(\"{plugin_name}.lua\")\n")
    }

    pub fn name(&self) -> &str {
        match self {
            Self::AutoIndent => "autoindent",
            Self::Pairs => "pairs",
            Self::QuickComment => "quickcomment",
            Self::DiscordRPC => "discord_rpc",
            Self::Emmet => "emmet",
            Self::Git => "git",
            Self::LiveHTML => "live_html",
            Self::Pomodoro => "pomodoro",
            Self::Todo => "todo",
            Self::TypingSpeed => "typing_speed",
            Self::UpdateNotification => "update_notification",
            Self::AI => "ai",
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
pub struct Assistant {
    pub theme: Theme,
    pub indentation: Indentation,
    pub tab_width: usize,
    pub mouse: bool,
    pub scroll_sensitivity: usize,
    pub cursor_wrap: bool,
    pub line_numbers: bool,
    pub line_number_padding: (usize, usize),
    pub icons: bool,
    pub tab_line: bool,
    pub tab_line_sep: bool,
    pub greeting_message: bool,
    pub file_tree_icons: bool,
    pub file_tree_language_icons: bool,
    pub ai_key: Option<String>,
    pub model: String,
    pub plugins: Vec<Plugin>,
}

impl Default for Assistant {
    fn default() -> Self {
        Self {
            // Colours and theme
            theme: Theme::Default,
            // Document
            indentation: Indentation::Tabs,
            tab_width: 4,
            // Line Numbers
            line_numbers: true,
            line_number_padding: (1, 1),
            // Tab Line
            tab_line: true,
            tab_line_sep: true,
            // Greeting Message
            greeting_message: true,
            // File Tree
            file_tree_icons: false,
            file_tree_language_icons: true,
            // Mouse and Cursor Behaviour
            mouse: true,
            scroll_sensitivity: 4,
            cursor_wrap: true,
            // AI
            ai_key: None,
            model: "gemini".to_string(),
            // Plug-ins
            plugins: vec![Plugin::AutoIndent, Plugin::Pairs, Plugin::QuickComment],
            // Misc
            icons: false,
        }
    }
}

impl Assistant {
    /// Run the configuration assistant
    pub fn run(because_no_config: bool) -> Result<()> {
        Self::reset()?;
        if because_no_config {
            println!("{NO_CONFIG_MESSAGE}");
        }
        println!("{INTRODUCTION}");
        if Self::confirmation("Do you wish to set-up the editor?", true) {
            let mut result = Self::default();
            // Theme
            Self::ask_theme(&mut result)?;
            // Document
            Self::ask_document(&mut result)?;
            // Line Numbers
            Self::ask_line_numbers(&mut result)?;
            // Tab line
            Self::ask_tab_line(&mut result)?;
            // Mouse and Cursor
            Self::ask_mouse_cursor(&mut result)?;
            // Icons
            Self::ask_icons(&mut result)?;
            // File tree
            Self::ask_file_tree(&mut result)?;
            // Plug-Ins
            Self::ask_plugins(&mut result)?;
            // Create the configuration file (and print it)
            Self::reset()?;
            println!("\nSet-up is complete!");
            if !because_no_config {
                let yellow = Fg(Color::Ansi(220).to_color()?);
                let reset = Fg(Color::Transparent.to_color()?);
                println!("{yellow}WARNING{reset}: config file already exists, it will be backed-up to ~/.oxrc-backup if you write");
            }
            let contents = result.to_config();
            if Self::confirmation(
                "Would you like to write the configuration file?",
                because_no_config,
            ) {
                Self::write_config(&result.plugins, &contents, because_no_config)?;
            } else {
                println!("Below is your newly generated configuration file:\n\n");
                println!("{contents}");
            }
            println!("{FINAL_WORDS}");
            let _ = gets!();
        }
        Ok(())
    }

    pub fn reset() -> Result<()> {
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
        println!("{TITLE}");
        Ok(())
    }

    pub fn write_config(
        plugins: &Vec<Plugin>,
        result: &str,
        because_no_config: bool,
    ) -> Result<()> {
        let config_path = format!("{}/.oxrc", shellexpand::tilde("~"));
        let backup_path = format!("{}/.oxrc-backup", shellexpand::tilde("~"));
        if !because_no_config {
            let _ = std::fs::rename(config_path.clone(), backup_path);
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_path)?;
        file.write_all(result.as_bytes())?;
        // Install plug-ins
        let lua = Lua::new();
        lua.load("commands = {}").exec()?;
        lua.load(PLUGIN_BOOTSTRAP).exec()?;
        lua.load(PLUGIN_NETWORKING).exec()?;
        lua.load(PLUGIN_MANAGER).exec()?;
        for plugin in plugins {
            let plugin_name = plugin.name();
            lua.load(PLUGIN_INSTALL.replace("{name}", plugin_name))
                .exec()?;
        }
        Ok(())
    }

    pub fn confirmation(msg: &str, default: bool) -> bool {
        let mut response = "#####################".to_string();
        while response != "y" && response != "n" && !response.is_empty() {
            response = gets!(
                "{msg}, default is {} (y/n)\n> ",
                if default { "yes" } else { "no" }
            )
            .to_lowercase();
        }
        println!();
        if response.is_empty() {
            default
        } else {
            response == "y"
        }
    }

    pub fn options(msg: &str, options: &[&str], default: &str) -> String {
        let options: Vec<String> = options
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        let mut response = "#####################".to_string();
        while !options.contains(&response) && !response.is_empty() {
            response =
                gets!("{msg}, default is {default} ({})\n> ", options.join("/")).to_lowercase();
        }
        println!();
        if response.is_empty() {
            default.to_string()
        } else {
            response
        }
    }

    pub fn integer(msg: &str, default: usize) -> usize {
        let mut response = "#####################".to_string();
        while response.parse::<usize>().is_err() && !response.is_empty() {
            response = gets!("{msg} (enter a number, default: {default})\n> ").to_lowercase();
        }
        println!();
        if response.is_empty() {
            default
        } else {
            response.parse::<usize>().unwrap()
        }
    }

    pub fn ask_theme(result: &mut Self) -> Result<()> {
        Self::reset()?;
        println!("Let's begin with what theme you'd like to use\n\n");
        // Prepare demonstration
        Self::demonstrate_themes()?;
        let choice = Self::options(
            "Please choose which theme you'd like",
            &[
                "default",
                "tropical",
                "galaxy",
                "transparent",
                "default16",
                "omni",
            ],
            "default",
        );
        result.theme = match choice.as_str() {
            "default" => Theme::Default,
            "default16" => Theme::Default16,
            "tropical" => Theme::Tropical,
            "galaxy" => Theme::Galaxy,
            "transparent" => Theme::Transparent,
            "omni" => Theme::Omni,
            _ => unreachable!(),
        };
        Ok(())
    }

    pub fn ask_document(result: &mut Self) -> Result<()> {
        let red = Fg(Color::Ansi(196).to_color()?);
        let orange = Fg(Color::Ansi(202).to_color()?);
        let yellow = Fg(Color::Ansi(220).to_color()?);
        let green = Fg(Color::Ansi(34).to_color()?);
        let blue = Fg(Color::Ansi(39).to_color()?);
        let purple = Fg(Color::Ansi(141).to_color()?);
        let pink = Fg(Color::Ansi(213).to_color()?);
        let reset = Fg(Color::Transparent.to_color()?);
        Self::reset()?;
        println!("Great choice, now let's move onto indentation\n");
        println!("{purple}_{blue}_{purple}_{blue}_{reset}spaces");
        println!("    tabs\n{purple}‾‾‾‾{reset}");
        result.indentation = Self::options(
            "Please choose how you'd like to represent indentation",
            &["spaces", "tabs"],
            "tabs",
        )
        .into();
        println!("{red}•{reset}1");
        println!("{orange}••{reset}2");
        println!("{yellow}•••{reset}3");
        println!("{green}••••{reset}4");
        println!("{blue}•••••{reset}5");
        println!("{purple}••••••{reset}6");
        println!("{pink}•••••••{reset}7");
        println!("••••••••8\n");
        result.tab_width = if result.indentation == Indentation::Tabs {
            Self::integer("How wide should tabs be rendered as", 4)
        } else {
            Self::integer("How many spaces should make up 1 indent", 4)
        };
        Ok(())
    }

    pub fn ask_mouse_cursor(result: &mut Self) -> Result<()> {
        let red = Fg(Color::Ansi(196).to_color()?);
        let yellow = Fg(Color::Ansi(220).to_color()?);
        let green = Fg(Color::Ansi(34).to_color()?);
        let blue = Fg(Color::Ansi(39).to_color()?);
        let purple = Fg(Color::Ansi(141).to_color()?);
        let pink = Fg(Color::Ansi(213).to_color()?);
        let reset = Fg(Color::Transparent.to_color()?);
        Self::reset()?;
        println!("Now for the mouse and cursor behaviour\n");
        println!("{blue}🖰 {reset}Clicking to move cursor, {purple}◅ 🖰 ▻ {reset} Dragging to select text\n");
        result.mouse = Self::confirmation(
            "Would you like to use your mouse cursor in the editor",
            true,
        );
        println!("{red}🖰 ⭥ {reset}  {yellow}🖰 ⭥ {reset}  {green}🖰 ⭥ {reset}\n");
        result.scroll_sensitivity = Self::integer(
            "How sensitive should scrolling be, 1 = least sensitive, 7 = very sensitive",
            4,
        );
        println!("    Cursor wraps{pink}|{reset}→ \n  ↳ {pink}|{reset}Onto new line\n");
        result.cursor_wrap = Self::confirmation(
            "Would you like the cursor to wrap around when at the edge of a line",
            true,
        );
        Ok(())
    }

    pub fn ask_tab_line(result: &mut Self) -> Result<()> {
        let orange = Fg(Color::Ansi(202).to_color()?);
        let yellow = Fg(Color::Ansi(220).to_color()?);
        let green = Fg(Color::Ansi(34).to_color()?);
        let purple = Fg(Color::Ansi(141).to_color()?);
        let reset = Fg(Color::Transparent.to_color()?);
        println!(
            "|  {purple}File 1{reset}  |  {green}File 2{reset}  |  {orange}File 3{reset}  |\n"
        );
        result.tab_line = Self::confirmation("Would you like the tab line to be visible", true);
        if result.tab_line {
            result.tab_line_sep = Self::confirmation(
                "Would you like the tab line to have separators (the | between tabs)",
                true,
            );
        }
        // Greeting message
        println!("   {yellow}Welcome to Ox Editor!{reset}   \n");
        result.greeting_message = Self::confirmation(
            "Would you like the greeting message to be visible on start-up",
            true,
        );
        Ok(())
    }

    pub fn ask_file_tree(result: &mut Self) -> Result<()> {
        if result.icons {
            let orange = Fg(Color::Ansi(202).to_color()?);
            let yellow = Fg(Color::Ansi(220).to_color()?);
            let green = Fg(Color::Ansi(34).to_color()?);
            let reset = Fg(Color::Transparent.to_color()?);
            println!("🖹  file1.txt \n🖹  file2.txt \n🖹  file3.txt \n");
            result.file_tree_icons =
                Self::confirmation("Would you like icons in the file tree?", false);
            if result.file_tree_icons {
                println!(
                    "\n {green}🎵{reset}  file1.mp3 \n {orange}{{}}{reset}  file2.css \n {yellow}</>{reset} file3.html \n"
                );
                result.file_tree_language_icons = Self::confirmation(
                    "Would you like the tab line to have language specific icons?",
                    true,
                );
            }
        }
        Ok(())
    }

    pub fn ask_line_numbers(result: &mut Self) -> Result<()> {
        let red = Fg(Color::Ansi(196).to_color()?);
        let orange = Fg(Color::Ansi(202).to_color()?);
        let yellow = Fg(Color::Ansi(220).to_color()?);
        let green = Fg(Color::Ansi(34).to_color()?);
        let reset = Fg(Color::Transparent.to_color()?);
        Self::reset()?;
        println!("Great, now for deciding which parts of the editor should be visible\n");
        println!("{green} 1 {reset}│");
        println!("{yellow} 2 {reset}│");
        println!("{red} 3 {reset}│\n");
        result.line_numbers = Self::confirmation("Would you like line numbers to be visible", true);
        if result.line_numbers {
            println!("{red}•{reset}1 │");
            println!("{orange}••{reset}2 │");
            println!("{yellow}•••{reset}3 │\n");
            let left_tab = Self::integer(
                "How much space should there be on the left hand side of the line numbers",
                1,
            );
            println!(" 1{red}•{reset}│");
            println!(" 2{orange}••{reset}│");
            println!(" 3{yellow}•••{reset}│\n");
            let right_tab = Self::integer(
                "How much space should there be on the right hand side of the line numbers",
                1,
            );
            result.line_number_padding = (left_tab, right_tab);
        }
        Ok(())
    }

    pub fn ask_icons(result: &mut Self) -> Result<()> {
        let yellow = Fg(Color::Ansi(220).to_color()?);
        let blue = Fg(Color::Ansi(39).to_color()?);
        let reset = Fg(Color::Transparent.to_color()?);
        Self::reset()?;
        println!("{blue}🖹 {yellow}🖉 {reset}");
        println!("Ox has support for icons, which can enhance the UI, if you choose to enable them, ensure you install nerd fonts\n");
        result.icons =
            Self::confirmation("Would you like to enable icons, yes is recommended", false);
        Ok(())
    }

    pub fn ask_plugins(result: &mut Self) -> Result<()> {
        Self::reset()?;
        let green = Fg(Color::Ansi(34).to_color()?);
        let reset = Fg(Color::Transparent.to_color()?);
        println!("{PLUGIN_LIST}");
        let mut adding = String::new();
        while adding != "exit" {
            println!("{green}Enabled plug-ins:{reset} {:?}\n", result.plugins);
            adding = Self::options(
                "Enter the name of a plug-in you'd like to enable / disable",
                &[
                    "autoindent",
                    "pairs",
                    "quickcomment",
                    "emmet",
                    "live_html",
                    "discord_rpc",
                    "git",
                    "pomodoro",
                    "todo",
                    "typing_speed",
                    "update_notification",
                    "ai",
                    "exit",
                ],
                "exit",
            );
            let plugin = match adding.as_str() {
                "autoindent" => Plugin::AutoIndent,
                "pairs" => Plugin::Pairs,
                "quickcomment" => Plugin::QuickComment,
                "emmet" => Plugin::Emmet,
                "live_html" => Plugin::LiveHTML,
                "discord_rpc" => Plugin::DiscordRPC,
                "git" => Plugin::Git,
                "pomodoro" => Plugin::Pomodoro,
                "todo" => Plugin::Todo,
                "typing_speed" => Plugin::TypingSpeed,
                "update_notification" => Plugin::UpdateNotification,
                "ai" => Plugin::AI,
                _ => continue,
            };
            if result.plugins.contains(&plugin) {
                result.plugins.retain(|p| *p != plugin);
            } else {
                result.plugins.push(plugin);
            }
        }
        // Plugin-specific configuration options
        Self::plugin_specific_config(result)?;
        Ok(())
    }

    pub fn plugin_specific_config(result: &mut Self) -> Result<()> {
        if result.plugins.contains(&Plugin::AI) {
            Self::reset()?;
            // AI specific questions
            println!("Let's set up the AI plug-in.");
            println!(
                "NOTE: you will need an API key, `gemini` is free, the other options are not\n"
            );
            result.model = Self::options(
                "Which AI model would you like to use?",
                &["gemini", "chatgpt", "claude"],
                "gemini",
            );
            if result.model == "gemini" {
                println!("{AI_INTRO_GEMINI}");
                result.ai_key = Some(gets!("\n> "));
            } else {
                println!("Please paste your API key for {} below:", result.model);
                result.ai_key = Some(gets!("\n> "));
            }
        }
        Ok(())
    }

    pub fn demonstrate_themes() -> Result<()> {
        println!(
            "{}",
            Self::demonstrate_theme_row(&["default", "default16"])?
        );
        println!("{}", Self::demonstrate_theme_row(&["tropical", "galaxy"])?);
        println!("{}", Self::demonstrate_theme_row(&["transparent", "omni"])?);
        Ok(())
    }

    pub fn demonstrate_theme_row(include: &[&str]) -> Result<String> {
        // Gather the list of theme previews
        let mut themes: Vec<Vec<String>> = vec![];
        for name in include {
            let code = match *name {
                "default" => "",
                "tropical" => TROPICAL,
                "galaxy" => GALAXY,
                "transparent" => TRANSPARENT,
                "default16" => DEFAULT16,
                "omni" => OMNI,
                _ => unreachable!(),
            };
            let theme = Self::demonstrate_theme(name, code)?
                .split('\n')
                .map(std::string::ToString::to_string)
                .collect();
            themes.push(theme);
        }
        // Put into row format
        let mut result = String::new();
        let mut at = 0;
        while at < 13 {
            for theme in &themes {
                result += &format!("{}   ", theme[at]);
            }
            result += "\n";
            at += 1;
        }
        // Return the result
        Ok(result)
    }

    pub fn demonstrate_theme(name: &str, code: &str) -> Result<String> {
        // Create an environment to capture all the values
        let lua = Lua::new();
        let colors = lua.create_userdata(Colors::default())?;
        let syntax_highlighting = lua.create_userdata(SyntaxHighlighting::default())?;
        lua.globals().set("syntax", syntax_highlighting.clone())?;
        lua.globals().set("colors", colors.clone())?;
        // Access all the values
        lua.load(code).exec()?;
        // Gather the editor colours
        let col: LuaUserDataRef<Colors> = colors.borrow()?;
        let editor = format!(
            "{}{}",
            Fg(col.editor_fg.to_color()?),
            Bg(col.editor_bg.to_color()?)
        );
        let reset = format!(
            "{}{}",
            Fg(crossterm::style::Color::Reset),
            Bg(crossterm::style::Color::Reset)
        );
        let active_tab = format!(
            "{}{}",
            Fg(col.tab_active_fg.to_color()?),
            Bg(col.tab_active_bg.to_color()?)
        );
        let inactive_tab = format!(
            "{}{}",
            Fg(col.tab_inactive_fg.to_color()?),
            Bg(col.tab_inactive_bg.to_color()?)
        );
        let line_number = format!(
            "{}{}",
            Fg(col.line_number_fg.to_color()?),
            Bg(col.line_number_bg.to_color()?)
        );
        let status_line = format!(
            "{}{}",
            Fg(col.status_fg.to_color()?),
            Bg(col.status_bg.to_color()?)
        );
        let error = format!(
            "{}{}",
            Fg(col.error_fg.to_color()?),
            Bg(col.error_bg.to_color()?)
        );
        let warning = format!(
            "{}{}",
            Fg(col.warning_fg.to_color()?),
            Bg(col.warning_bg.to_color()?)
        );
        let info = format!(
            "{}{}",
            Fg(col.info_fg.to_color()?),
            Bg(col.info_bg.to_color()?)
        );
        // Gather syntax highlighting colours
        let syn: LuaUserDataRef<SyntaxHighlighting> = syntax_highlighting.borrow()?;
        let string = Fg(syn.get_theme("string")?);
        let comment = Fg(syn.get_theme("comment")?);
        let digit = Fg(syn.get_theme("digit")?);
        let keyword = Fg(syn.get_theme("keyword")?);
        let character = Fg(syn.get_theme("character")?);
        let type_syn = Fg(syn.get_theme("type")?);
        let function = Fg(syn.get_theme("function")?);
        let macro_syn = Fg(syn.get_theme("macro")?);
        let block = Fg(syn.get_theme("block")?);
        let namespace = Fg(syn.get_theme("namespace")?);
        let header = Fg(syn.get_theme("header")?);
        let struct_syn = Fg(syn.get_theme("struct")?);
        let operator = Fg(syn.get_theme("operator")?);
        let boolean = Fg(syn.get_theme("boolean")?);
        let reference = Fg(syn.get_theme("reference")?);
        let tag = Fg(syn.get_theme("tag")?);
        let heading = Fg(syn.get_theme("heading")?);
        let link = Fg(syn.get_theme("link")?);
        let bold = Fg(syn.get_theme("bold")?);
        let italic = Fg(syn.get_theme("italic")?);
        let insertion = Fg(syn.get_theme("insertion")?);
        let deletion = Fg(syn.get_theme("deletion")?);
        // Render the preview
        let name = format!("  {name}  ");
        Ok(format!("{name:─^47}
{editor}┌─────────────────────────────────────────────┐{reset}
{editor}│{inactive_tab}  Inactive Tab  |{active_tab}  Active Tab  {inactive_tab}|             {editor}│{reset}
{editor}│{line_number}   1 │{editor}{function}print{editor}({string}\"hello\" {operator}+ {digit}3 {operator}+ {boolean}true {operator}+ {character}'c'{editor});       │{reset}
{editor}│{line_number}   2 │{editor}{keyword}let {editor}var{operator}: {type_syn}Type {operator}= {reference}&{struct_syn}Object{editor}({namespace}name::space{editor});  │{reset}
{editor}│{line_number}   3 │{editor}{tag}<html></html> {comment}// Comment{editor}               │{reset}
{editor}│{line_number}   4 │{editor}{header}import {editor}random;                         │{reset}
{editor}│{line_number}   5 │{editor}{macro_syn}macro!{editor}();   {insertion}+ insertion   {deletion}- deletion   {editor}│{reset}
{editor}│{line_number}   6 │{editor}{heading}# Title {italic}*italic* {bold}**bold** {block}`code`{editor}       │{reset}
{editor}│{line_number}   7 │{editor}{link}[link](example.com){editor}                    │{reset}
{editor}│{status_line}  Status Line                                {editor}│{reset}
{editor}│{error}      Error    {warning}   Warning   {info}   Information   {editor}│{reset}
{editor}└─────────────────────────────────────────────┘{reset}"))
    }

    /// Turn the configuration assistant details into a lua file
    #[allow(clippy::too_many_lines)]
    pub fn to_config(&self) -> String {
        let mut result = String::new();
        let (sections, fields) = self.diff();
        // Comment at the top
        result += &format!(
            "-- Configuration generated for Ox {VERSION} by the configuration assistant --\n"
        );
        // Configuration of colours and theme
        if sections.contains(&"theme") {
            result += &self.theme.to_config();
        }
        // Configuration of document
        if sections.contains(&"document") {
            result += "\n-- Document Configuration --\n";
            if fields.contains(&"indentation") {
                result += &format!("document.indentation = '{}'\n", self.indentation);
            }
            if fields.contains(&"tab_width") {
                result += &format!("document.tab_width = {}\n", self.tab_width);
            }
        }
        // Configuration of line numbers
        if sections.contains(&"line_numbers") {
            result += "\n-- Line Number Configuration --\n";
            if fields.contains(&"line_numbers") {
                result += &format!("line_numbers.enabled = {}\n", self.line_numbers);
            }
            if fields.contains(&"line_number_padding") {
                result += &format!(
                    "line_numbers.padding_left = {}\n",
                    self.line_number_padding.0
                );
                result += &format!(
                    "line_numbers.padding_right = {}\n",
                    self.line_number_padding.1
                );
            }
        }
        // Configuration of tab line
        if sections.contains(&"tab_line") {
            result += "\n-- Tab Line Configuration --\n";
            result += &format!("tab_line.enabled = {}\n", self.tab_line);
            if fields.contains(&"tab_line_sep") {
                result += &format!("tab_line.separators = {}\n", self.tab_line_sep);
            }
            let mut format = "  {file_name}{modified}  ".to_string();
            let mut format_changed = false;
            if self.icons {
                format = format.replace("{file_name}", "{icon} {file_name}");
                format_changed = true;
            }
            if self.plugins.contains(&Plugin::Git) {
                format = format.replace("{modified}", "{modified} {git_status}");
                format_changed = true;
            }
            if format_changed {
                result += &format!("tab_line.format = '{format}'\n");
            }
        }
        // Configuration of status line
        if sections.contains(&"status_line") {
            result += "\n-- Status Line Configuration --\n";
            let mut left = "  {file_name}{modified}  │  {file_type}  │".to_string();
            let mut right = "│  {cursor_y} / {line_count}  {cursor_x}  ".to_string();
            // Handle file type icons
            if self.icons {
                left = left.replace("{file_type}", "{icon} {file_type}");
            }
            // Handle git plug-in
            if self.plugins.contains(&Plugin::Git) && self.icons {
                right = format!("│    {{git_branch}}  {right}");
            } else if self.plugins.contains(&Plugin::Git) && !self.icons {
                right = format!("│  {{git_branch}}  {right}");
            }
            // Handle typing speed plug-in
            if self.plugins.contains(&Plugin::TypingSpeed) {
                right = format!("│  {{typing_speed_show}}  {right}");
            }
            // Handle pomodoro plug-in
            if self.plugins.contains(&Plugin::Pomodoro) {
                left = format!("{left}  {{pomodoro_show}}  │");
            }
            result += &format!("status_line.parts = {{\n\t'{left}',\n\t'{right}',\n}}\n");
        }
        // Configuration of greeting message
        if sections.contains(&"greeting_message") {
            result += "\n-- Greeting Message Configuration --\n";
            result += &format!("greeting_message.enabled = {}\n", self.greeting_message);
        }
        // Configuration of file tree
        if sections.contains(&"file_tree") {
            result += "\n-- File Tree Configuration --\n";
            if fields.contains(&"file_tree_icons") {
                result += &format!("file_tree.icons = {}\n", self.file_tree_icons);
            }
            if fields.contains(&"file_tree_language_icons") {
                result += &format!(
                    "file_tree.language_icons = {}\n",
                    self.file_tree_language_icons
                );
            }
        }
        // Configuration of mouse and cursor behaviour
        if sections.contains(&"cursors") {
            result += "\n-- Cursor Configuration --\n";
            if fields.contains(&"mouse") {
                result += &format!("terminal.mouse_enabled = {}\n", self.mouse);
            }
            if fields.contains(&"scroll_sensitivity") {
                result += &format!("terminal.scroll_amount = {}\n", self.scroll_sensitivity);
            }
            if fields.contains(&"cursor_wrap") {
                result += &format!("document.wrap_cursor = {}\n", self.cursor_wrap);
            }
        }
        // Configuration of plug-ins
        result += "\n-- Load Plug-Ins --\n";
        for plugin in &self.plugins {
            result += &plugin.to_config();
            if plugin == &Plugin::Git && self.icons {
                result += "git = { icons = true }\n";
            } else if plugin == &Plugin::AI {
                if let Some(api_key) = &self.ai_key {
                    result +=
                        &format!("ai = {{ model = \"{}\", key = \"{api_key}\" }}", self.model);
                }
            }
        }
        // Ready to go
        result
    }

    /// Find the difference between the default configuration and this one
    pub fn diff(&self) -> (Vec<&str>, Vec<&str>) {
        let def = Self::default();
        let fields = vec![
            ("theme", self.theme != def.theme),
            ("indentation", self.indentation != def.indentation),
            ("line_numbers", self.line_numbers != def.line_numbers),
            ("tab_line", self.tab_line != def.tab_line),
            ("tab_line_sep", self.tab_line_sep != def.tab_line_sep),
            (
                "greeting_message",
                self.greeting_message != def.greeting_message,
            ),
            ("mouse", self.mouse != def.mouse),
            ("cursor_wrap", self.cursor_wrap != def.cursor_wrap),
            ("icons", self.icons != def.icons),
            (
                "file_tree_icons",
                self.file_tree_icons != def.file_tree_icons,
            ),
            (
                "file_tree_language_icons",
                self.file_tree_language_icons != def.file_tree_language_icons,
            ),
            (
                "line_number_padding",
                self.line_number_padding != def.line_number_padding,
            ),
            (
                "scroll_sensitivity",
                self.scroll_sensitivity != def.scroll_sensitivity,
            ),
            ("tab_width", self.tab_width != def.tab_width),
        ];
        let fields = fields
            .iter()
            .filter_map(|(name, differs)| if *differs { Some(*name) } else { None })
            .collect::<Vec<&str>>();
        let sections = [
            ("theme", fields.contains(&"theme")),
            (
                "document",
                fields.contains(&"indentation") || fields.contains(&"tab_width"),
            ),
            (
                "line_numbers",
                fields.contains(&"line_numbers") || fields.contains(&"line_number_padding"),
            ),
            (
                "tab_line",
                fields.contains(&"tab_line")
                    || fields.contains(&"tab_line_sep")
                    || fields.contains(&"icons")
                    || self.plugins.contains(&Plugin::Git),
            ),
            (
                "status_line",
                fields.contains(&"icons")
                    || self.plugins.contains(&Plugin::Git)
                    || self.plugins.contains(&Plugin::Pomodoro)
                    || self.plugins.contains(&Plugin::TypingSpeed),
            ),
            ("greeting_message", fields.contains(&"greeting_message")),
            (
                "cursors",
                fields.contains(&"mouse")
                    || fields.contains(&"scroll_sensitivity")
                    || fields.contains(&"cursor_wrap"),
            ),
            (
                "file_tree",
                fields.contains(&"file_tree_icons") || fields.contains(&"file_tree_language_icons"),
            ),
        ];
        let sections = sections
            .iter()
            .filter_map(|(name, differs)| if *differs { Some(*name) } else { None })
            .collect::<Vec<&str>>();
        (sections, fields)
    }
}
