-- notion plugin: search Notion pages
-- Requires: NOTION_TOKEN environment variable
-- Usage: /notion <query>

batto.command({
    name = "notion",
    description = "Search Notion pages",
    args = {
        { name = "query", required = true, type = "string" },
    },
    handler = function(args)
        local query = args.query or ""
        local token = batto.env("NOTION_TOKEN")
        if not token then
            return { { title = "Set NOTION_TOKEN env var to use Notion search", exec = "true" } }
        end
        if query == "" then
            return { { title = "Type to search Notion...", exec = "true" } }
        end

        local body = batto.json_encode({ query = query, page_size = 5 })
        local resp = batto.fetch("https://api.notion.com/v1/search", {
            method = "POST",
            headers = {
                ["Authorization"] = "Bearer " .. token,
                ["Content-Type"] = "application/json",
                ["Notion-Version"] = "2022-06-28",
            },
            body = body,
        })

        if not resp then
            return { { title = "Failed to connect to Notion API", exec = "true" } }
        end

        local data = batto.json_decode(resp)
        local results = {}
        if data.results then
            for _, page in ipairs(data.results) do
                local title = "Untitled"
                if page.properties and page.properties.title and page.properties.title.title then
                    local parts = {}
                    for _, part in ipairs(page.properties.title.title) do
                        table.insert(parts, part.plain_text or "")
                    end
                    title = table.concat(parts, "")
                end
                table.insert(results, {
                    title = title,
                    exec = "xdg-open '" .. (page.url or "") .. "'",
                })
            end
        end

        if #results == 0 then
            return { { title = "No results for: " .. query, exec = "true" } }
        end
        return results
    end,
})
