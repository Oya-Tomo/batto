-- web-search plugin: multi-engine web search
-- Usage: /gg <query>, /gh <query>, /so <query>, /ddg <query>

batto.on_query({
    prefix = "gg",
    description = "Google search",
    handler = function(query)
        if query == "" then
            return { { title = "Google Search", exec = "xdg-open https://google.com" } }
        end
        return {
            { title = "Search Google: " .. query,
              exec = "xdg-open 'https://google.com/search?q=" .. query .. "'" },
        }
    end,
})

batto.on_query({
    prefix = "gh",
    description = "GitHub search",
    handler = function(query)
        if query == "" then
            return { { title = "GitHub", exec = "xdg-open https://github.com" } }
        end
        return {
            { title = "Search GitHub: " .. query,
              exec = "xdg-open 'https://github.com/search?q=" .. query .. "'" },
            { title = "Open repo: " .. query,
              exec = "xdg-open 'https://github.com/" .. query .. "'" },
        }
    end,
})

batto.on_query({
    prefix = "so",
    description = "Stack Overflow search",
    handler = function(query)
        if query == "" then
            return { { title = "Stack Overflow", exec = "xdg-open https://stackoverflow.com" } }
        end
        return {
            { title = "Search Stack Overflow: " .. query,
              exec = "xdg-open 'https://stackoverflow.com/search?q=" .. query .. "'" },
        }
    end,
})

batto.on_query({
    prefix = "ddg",
    description = "DuckDuckGo search",
    handler = function(query)
        if query == "" then
            return { { title = "DuckDuckGo", exec = "xdg-open https://duckduckgo.com" } }
        end
        return {
            { title = "Search DuckDuckGo: " .. query,
              exec = "xdg-open 'https://duckduckgo.com/?q=" .. query .. "'" },
        }
    end,
})
