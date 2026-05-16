-- docker plugin: manage Docker containers and images
-- Usage:
--   /dk <query>   -- list and filter running containers
--   /dkps         -- list all containers
--   /dki          -- list images

batto.command({
    name = "dkps",
    description = "List Docker containers",
    exec = "terminal -e sh -c 'docker ps -a; read'",
})

batto.command({
    name = "dki",
    description = "List Docker images",
    exec = "terminal -e sh -c 'docker images; read'",
})

batto.command({
    name = "dk",
    description = "Docker containers",
    args = {
        { name = "query", type = "string" },
    },
    handler = function(args)
        local query = args.query or ""
        -- List running containers and filter
        local output = batto.fetch("http://localhost/containers/json", {
            headers = {},
        })

        if not output then
            -- Fallback: use docker CLI
            return {
                { title = "List running containers", exec = "terminal -e sh -c 'docker ps; read'" },
                { title = "List all containers", exec = "terminal -e sh -c 'docker ps -a; read'" },
                { title = "List images", exec = "terminal -e sh -c 'docker images; read'" },
            }
        end

        local results = {}
        -- If we got Docker API response, parse it
        local ok, containers = pcall(batto.json_decode, output)
        if ok and type(containers) == "table" then
            for _, c in ipairs(containers) do
                local name = (c.Names and c.Names[1] or "unknown"):gsub("^/", "")
                local image = c.Image or ""
                local status = c.Status or ""
                local match = query == "" or name:lower():find(query:lower()) or image:lower():find(query:lower())
                if match then
                    local cid = c.Id and c.Id:sub(1, 12) or ""
                    table.insert(results, {
                        title = name .. " (" .. image .. ") - " .. status,
                        exec = "terminal -e sh -c 'docker attach " .. cid .. "; read'",
                    })
                end
            end
        end

        if #results == 0 then
            table.insert(results, { title = "List running containers", exec = "terminal -e sh -c 'docker ps; read'" })
            table.insert(results, { title = "List all containers", exec = "terminal -e sh -c 'docker ps -a; read'" })
            table.insert(results, { title = "List images", exec = "terminal -e sh -c 'docker images; read'" })
        end
        return results
    end,
})
