--[[
AI v0.1

If you need advice or code, this plug-in will help you

It has two different options:
- Advice, where it will answer questions about the opened code
- Code, where it will look at the comment above the cursor and 
  insert code based on the comment

You can select between different models, including
- gemini - Google's Gemini
- chatgpt - OpenAI's ChatGPT
- claude - Anthropic's Claude
]]--

ai = {
    model = (ai or { model = "gemini" }).model, -- Gemini is free by default!
    key = (ai or { key = nil }).key, -- API key
}

-- Entry point of the plug-in
function ai:run()
    -- Gather context information
    local file = editor:get()
    local language = editor.document_type
    local instruction = self:grab_comment()
    -- Find out the method the user would like to use
    local method = editor:prompt("Would you like advice or code")
    local prompt
    if method == "advice" then
        prompt = self:advice_prompt(file, language, instruction)
    elseif method == "code" then
        prompt = self:code_prompt(file, language, instruction)
    end
    local response
    if self.model == "gemini" then
        response = self:send_to_gemini(prompt)
    elseif self.model == "chatgpt" then
        response = self:send_to_chatgpt(prompt)
    elseif self.model == "claude" then
        response = self:send_to_claude(prompt)
    end
    for i = 1, #response do
        local char = response:sub(i, i)  -- Extract the character at position 'i'
        if char == "\n" then
            editor:insert_line()
        else
            editor:insert(char)
        end
    end
    editor:rerender()
end

event_mapping["alt_space"] = function()
    ai:run()
end

-- Grab any comments above the cursor
function ai:grab_comment()
    -- Move upwards from the cursor y position
    local lines = {}
    local y = editor.cursor.y

    -- While y is greater than 0
    while y > 0 do
        -- Get the current line
        local line = editor:get_line_at(y)
        -- Check if the line is empty or full of whitespace
        if line:match("^%s*$") and y ~= editor.cursor.y then
            break -- Stop processing if an empty line is encountered
        else
            table.insert(lines, line)
        end
        -- Move to the previous line
        y = y - 1
    end

    -- Reverse order
    local reversed = {}
    for i = #lines, 1, -1 do
        table.insert(reversed, lines[i])
    end
    local lines = reversed

    return table.concat(lines, "\n")
end

-- Create a prompt for advice on a code base
function ai:advice_prompt(file, language, instruction)
    return string.format(
        "Take the following code as context (language is %s):\n```\n%s\n```\n\n\
Answer the following question: %s\nYour response should ONLY include the answer \
for this question in comment format in the same language, use the above context if helpful\n\
Start the code with the marker `(OX START)` and end the code with the marker `(OX END)`, \
both uncommented but included in the code block, the rest of the answer should be commented in the language we're using",
        language,
        file,
        instruction
    )
end

-- Create a prompt for code
function ai:code_prompt(file, language, instruction)
    return string.format(
        "Take the following code as context (language is %s):\n```\n%s\n```\n\n\
Can you complete the code as the comment suggests, taking into account the above code if required?\n\
```\n%s\n```\nYour response should ONLY include the section of code you've written excluding the above comment\
Start the code with the marker `(OX START)` and end the code with the marker `(OX END)`, both inside the code",
        language,
        file,
        instruction
    )
end

-- Send prompt to Google Gemini
function ai:send_to_gemini(prompt)
    if self.key ~= nil then
        editor:display_info("Please wait while your request is processed...")
        editor:rerender()
    else
        editor:display_error("Please specify an API key in your configuration file")
        editor:rerender()
        return
    end
    prompt = prompt:gsub("\\", "\\\\")
                   :gsub('"', '\\"')
                   :gsub("'", "\\'")
                   :gsub("\n", "\\n")
                   :gsub("([$`!])", "\\%1")
    local url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key=" .. self.key
    local cmd = 'curl -s -H "Content-Type: application/json" -X POST -d "{\'contents\':[{\'parts\':[{\'text\': \'' .. prompt .. '\'}]}]}" "' .. url .. '"'
    local json = shell:output(cmd)

    -- Find the `text` field within the JSON string
    local text_start, text_end = json:find('"text"%s*:%s*"')
    if not text_start then
        return nil, "Could not find 'text' field"
    end

    -- Extract the substring containing the text value
    local text_value_start = text_end + 1
    local text_value_end = json:find('"', text_value_start)
    while text_value_end do
        -- Check if the quote is escaped
        if json:sub(text_value_end - 1, text_value_end - 1) ~= "\\" then
            break
        end
        -- Continue searching for the ending quote
        text_value_end = json:find('"', text_value_end + 1)
    end

    if not text_value_end then
        return nil, "Unterminated 'text' field"
    end

    -- Extract the raw text value and unescape escaped quotes
    local text = json:sub(text_value_start, text_value_end - 1)
    text = text:gsub('\\"', '"'):gsub('\\\\', '\\')
    text = text:gsub("\\n", "\n")
    
    text = text:match("%(OX START%)(.-)%(OX END%)")
    text = text:gsub("\n+$", "\n")
    text = text:gsub("^\n+", "\n")

    -- Convert any weird unicode stuff into their actual characters
    text = text:gsub("\\u(%x%x%x%x)", function(hex)
        local codepoint = tonumber(hex, 16)  -- Convert hex to a number
        return utf8.char(codepoint)         -- Convert number to a UTF-8 character
    end)

    editor:display_info("Request processed!")
    return text
end

-- Send prompt to OpenAI ChatGPT
function ai:send_to_chatgpt(prompt)
    if self.key ~= nil then
        editor:display_info("Please wait while your request is processed...")
        editor:rerender()
    else
        editor:display_error("Please specify an API key in your configuration file")
        editor:rerender()
        return
    end
    prompt = prompt:gsub("\\", "\\\\")
                   :gsub('"', '\\"')
                   :gsub("'", "\\'")
                   :gsub("\n", "\\n")
                   :gsub("([$`!])", "\\%1")
    local url = "https://api.openai.com/v1/chat/completions"
    local headers = '-H "Content-Type: application/json" -H "Authorization: Bearer ' .. self.key .. '"'
    local cmd = 'curl -s ' .. headers .. ' -d "{\'model\': \'gpt-4\', \'messages\':[{\'role\':\'user\', \'content\':\'' .. prompt .. '\'}], \'temprature\':0.7}" "' .. url .. '"'
    local json = shell:output(cmd)

    -- Find the `content` field within the JSON string
    local text_start, text_end = json:find('"content"%s*:%s*"')
    if not text_start then
        return nil, "Could not find 'content' field"
    end

    -- Extract the substring containing the text value
    local text_value_start = text_end + 1
    local text_value_end = json:find('"', text_value_start)
    while text_value_end do
        -- Check if the quote is escaped
        if json:sub(text_value_end - 1, text_value_end - 1) ~= "\\" then
            break
        end
        -- Continue searching for the ending quote
        text_value_end = json:find('"', text_value_end + 1)
    end

    if not text_value_end then
        return nil, "Unterminated 'content' field"
    end

    -- Extract the raw text value and unescape escaped quotes
    local text = json:sub(text_value_start, text_value_end - 1)
    text = text:gsub('\\"', '"'):gsub('\\\\', '\\')
    text = text:gsub("\\n", "\n")

    text = text:match("%(OX START%)(.-)%(OX END%)")
    text = text:gsub("\n+$", "\n")
    text = text:gsub("^\n+", "\n")

    -- Convert any weird unicode stuff into their actual characters
    text = text:gsub("\\u(%x%x%x%x)", function(hex)
        local codepoint = tonumber(hex, 16)  -- Convert hex to a number
        return utf8.char(codepoint)         -- Convert number to a UTF-8 character
    end)

    editor:display_info("Request processed!")
    return text
end

-- Send prompt to Anthropic Claude
function ai:send_to_claude(prompt)
    if self.key ~= nil then
        editor:display_info("Please wait while your request is processed...")
        editor:rerender()
    else
        editor:display_error("Please specify an API key in your configuration file")
        editor:rerender()
        return
    end
    prompt = prompt:gsub("\\", "\\\\")
                   :gsub('"', '\\"')
                   :gsub("'", "\\'")
                   :gsub("\n", "\\n")
                   :gsub("([$`!])", "\\%1")
    local url = "https://api.anthropic.com/v1/messages"
    local headers = '-H "Content-Type: application/json" -H "x-api-key: ' .. self.key .. '"'
    local cmd = 'curl -s ' .. headers .. ' -d "{\'model\': \'claude-3-5-sonnet-20241022\', \'messages\':[{\'role\':\'user\', \'content\':\'' .. prompt .. '\'}]}" "' .. url .. '"'
    local json = shell:output(cmd)

    -- Find the `text` field within the JSON string
    local text_start, text_end = json:find('"text"%s*:%s*"')
    if not text_start then
        return nil, "Could not find 'text' field"
    end

    -- Extract the substring containing the text value
    local text_value_start = text_end + 1
    local text_value_end = json:find('"', text_value_start)
    while text_value_end do
        -- Check if the quote is escaped
        if json:sub(text_value_end - 1, text_value_end - 1) ~= "\\" then
            break
        end
        -- Continue searching for the ending quote
        text_value_end = json:find('"', text_value_end + 1)
    end

    if not text_value_end then
        return nil, "Unterminated 'text' field"
    end

    -- Extract the raw text value and unescape escaped quotes
    local text = json:sub(text_value_start, text_value_end - 1)
    text = text:gsub('\\"', '"'):gsub('\\\\', '\\')
    text = text:gsub("\\n", "\n")

    text = text:match("%(OX START%)(.-)%(OX END%)")
    text = text:gsub("\n+$", "\n")
    text = text:gsub("^\n+", "\n")

    -- Convert any weird unicode stuff into their actual characters
    text = text:gsub("\\u(%x%x%x%x)", function(hex)
        local codepoint = tonumber(hex, 16)  -- Convert hex to a number
        return utf8.char(codepoint)         -- Convert number to a UTF-8 character
    end)

    editor:display_info("Request processed!")
    return text
end
