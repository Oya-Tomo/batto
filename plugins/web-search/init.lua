-- web-search plugin: multi-engine web search
-- Usage: /gg <query>, /gh <query>, /so <query>, /ddg <query>

batto.command({
    name = "gg",
    description = "Google search",
    args = {
        { name = "query", required = true, type = "string" },
    },
    handler = function(args)
        local q = args.query or ""
        if q == "" then
            return { { title = "Google Search", exec = "xdg-open https://google.com" } }
        end
        return {
            { title = "Search Google: " .. q,
              exec = "xdg-open 'https://google.com/search?q=" .. q .. "'" },
        }
    end,
})

batto.command({
    name = "gh",
    description = "GitHub search",
    args = {
        { name = "query", required = true, type = "string" },
    },
    handler = function(args)
        local q = args.query or ""
        if q == "" then
            return { { title = "GitHub", exec = "xdg-open https://github.com" } }
        end
        return {
            { title = "Search GitHub: " .. q,
              exec = "xdg-open 'https://github.com/search?q=" .. q .. "'" },
            { title = "Open repo: " .. q,
              exec = "xdg-open 'https://github.com/" .. q .. "'" },
        }
    end,
})

batto.command({
    name = "so",
    description = "Stack Overflow search",
    args = {
        { name = "query", required = true, type = "string" },
    },
    handler = function(args)
        local q = args.query or ""
        if q == "" then
            return { { title = "Stack Overflow", exec = "xdg-open https://stackoverflow.com" } }
        end
        return {
            { title = "Search Stack Overflow: " .. q,
              exec = "xdg-open 'https://stackoverflow.com/search?q=" .. q .. "'" },
        }
    end,
})

batto.command({
    name = "ddg",
    description = "DuckDuckGo search",
    args = {
        { name = "query", required = true, type = "string" },
    },
    handler = function(args)
        local q = args.query or ""
        if q == "" then
            return { { title = "DuckDuckGo", exec = "xdg-open https://duckduckgo.com" } }
        end
        return {
            { title = "Search DuckDuckGo: " .. q,
              exec = "xdg-open 'https://duckduckgo.com/?q=" .. q .. "'" },
        }
    end,
})
