# Lua API Reference

batto features a Lua 5.4 based configuration and plugin system.
The config file is located at `~/.config/batto/init.lua` and is auto-generated on first launch.

---

## Table of Contents

- [batto.setup(config)](#battosetupconfig)
- [batto.command(definition)](#battocommanddefinition)
- [batto.use(name)](#battousename)
- [batto.fetch(url, opts)](#battofetchurl-opts)
- [batto.json_decode(s)](#battojson_decodes)
- [batto.json_encode(val)](#battojson_encodeval)
- [batto.env(name)](#battoenvname)
- [Type Reference](#type-reference)
- [Writing Plugins](#writing-plugins)

---

## batto.setup(config)

Configures UI display settings and key bindings. Call once in `init.lua`.
All fields are optional — defaults are applied for omitted values.

### Parameters

```lua
batto.setup({
  window = {
    width       = 600,     -- Window width in pixels
    list_height = 300,     -- List area height in pixels
    icon_size   = 48,      -- App icon size in pixels
  },
  keys = {
    accept       = "enter",   -- Accept/confirm key
    close        = "escape",  -- Close launcher key
    up           = "up",      -- Move up
    down         = "down",    -- Move down
    tab_complete = "tab",     -- Tab completion
  },
})
```

### Key Binding Modifiers

Combine modifiers with `+`:

```
ctrl+enter
shift+up
alt+space
ctrl+shift+t
```

Supported modifiers: `ctrl` / `control`, `shift`, `alt`

### Defaults

| Field | Default |
|---|---|
| `window.width` | `600` |
| `window.list_height` | `300` |
| `window.icon_size` | `48` |
| `keys.accept` | `"enter"` |
| `keys.close` | `"escape"` |
| `keys.up` | `"up"` |
| `keys.down` | `"down"` |
| `keys.tab_complete` | `"tab"` |

---

## batto.command(definition)

Defines a custom command. Type `/` in the launcher to enter command mode and invoke defined commands.

### Parameters

```lua
batto.command({
  name        = "mycmd",          -- (required) Command name. Invoked via /mycmd
  description = "Description",    -- Description shown in command list
  exec        = "shell command",  -- Static execution command
  args        = { ... },          -- Argument definitions (optional)
  handler     = function(args) end, -- Dynamic handler (optional)
})
```

Specify either `exec` or `handler`. If both are provided, `handler` takes precedence.

### Static Commands (exec)

`exec` accepts a shell command string. Embed arguments using `{{arg_name}}`:

```lua
batto.command({
  name = "search",
  description = "Search the web",
  args = {
    { name = "query", required = true, type = "string" },
  },
  exec = "xdg-open 'https://google.com/search?q={{query}}'",
})
```

Launcher input: `/search rust lang` → opens Google search for "rust lang"

### Dynamic Commands (handler)

`handler` accepts a Lua function that receives an argument table and returns an array of result tables.
Results appear in the command mode list — selecting one executes its `exec`.

```lua
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
```

**Handler signature:**

```lua
function(args: table) -> table
```

- `args` — Table of command arguments, keyed by argument `name`.
- Returns — Array of `{ title, exec }` tables. Each element becomes one row in the list.

### Argument Definitions (args)

Each argument has the following fields:

```lua
{
  name     = "query",       -- (required) Argument name
  required = true,          -- Optional (default: false)
  type     = "string",      -- Optional (default: "string")
  choices  = { ... },       -- Optional: predefined choices
}
```

**Valid type values:**

| type | Description |
|---|---|
| `"string"` | String (default) |
| `"path"` | File path |
| `"url"` | URL |
| `"number"` | Number |
| `"literal"` | Must choose from predefined choices only |

**choices:**

When `choices` is specified, the argument is selected from predefined options at input time.

```lua
batto.command({
  name = "discord",
  description = "Send Discord message",
  args = {
    {
      name     = "channel",
      required = true,
      type     = "literal",
      choices  = {
        { name = "General",  value = "https://discord.com/api/webhooks/..." },
        { name = "Bot-log",  value = "https://discord.com/api/webhooks/..." },
      },
    },
    { name = "message", required = true, type = "string" },
  },
  handler = function(args)
    return {
      { title = "Send: " .. args.message,
        exec = "curl -s -X POST '" .. args.channel
            .. "' -H 'Content-Type: application/json'"
            .. " -d '{\"content\":\"" .. args.message .. "\"}'" },
    }
  end,
})
```

---

## batto.use(name)

Loads a plugin. Duplicate loads of the same plugin name are skipped.

Plugins are loaded from `~/.config/batto/plugins/<name>/init.lua`.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` | Plugin directory name |

### Example

```lua
batto.use("web-search")
batto.use("notion")
batto.use("docker")
batto.use("discord")
```

---

## batto.fetch(url, opts)

Performs an HTTP request using `curl` internally.

### Parameters

| Parameter | Type | Required | Description |
|---|---|---|---|
| `url` | `string` | **required** | Request URL |
| `opts` | `table` | optional | Request options |

### opts

| Field | Type | Description |
|---|---|---|
| `method` | `string` | HTTP method (`"GET"`, `"POST"`, etc.) |
| `headers` | `table` | Request headers (`{ ["Key"] = "Value" }`) |
| `body` | `string` | Request body |

### Return Value

- On success: response body as a string
- On failure: raises a Lua error (catchable with `pcall`)

### Details

- Timeout: 5 seconds
- Executes as `curl -s --max-time 5`

### Example

```lua
-- GET request
local body = batto.fetch("https://api.example.com/data")

-- POST request with headers and body
local resp = batto.fetch("https://api.notion.com/v1/search", {
  method = "POST",
  headers = {
    ["Authorization"] = "Bearer " .. token,
    ["Content-Type"]  = "application/json",
    ["Notion-Version"] = "2022-06-28",
  },
  body = batto.json_encode({ query = "hello", page_size = 5 }),
})

-- Error handling
local ok, resp = pcall(batto.fetch, "https://api.example.com/data")
if not ok then
  -- request failed
end
```

---

## batto.json_decode(s)

Converts a JSON string to a Lua value.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `s` | `string` | JSON string |

### Return Value

A Lua value. JSON types map as follows:

| JSON | Lua |
|---|---|
| object | `table` |
| array | `table` (1-indexed) |
| string | `string` |
| number | `number` |
| boolean | `boolean` |
| null | `nil` |

### Errors

Raises a Lua error for invalid JSON. Use `pcall` to catch errors.

### Example

```lua
local ok, data = pcall(batto.json_decode, '{"name": "batto", "version": 1}')
if ok then
  print(data.name)    -- "batto"
  print(data.version) -- 1
end
```

---

## batto.json_encode(val)

Converts a Lua value to a JSON string.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `val` | any | Lua value to encode |

### Return Value

- On success: JSON string
- On failure: raises a Lua error

### Example

```lua
local json = batto.json_encode({
  query = "search term",
  page_size = 5,
})
-- '{"page_size":5,"query":"search term"}'
```

---

## batto.env(name)

Retrieves an environment variable.

### Parameters

| Parameter | Type | Description |
|---|---|---|
| `name` | `string` | Environment variable name |

### Return Value

| Condition | Return |
|---|---|
| Variable is set | `string` |
| Variable is unset | `nil` |

### Example

```lua
local token = batto.env("NOTION_TOKEN")
if not token then
  -- environment variable not set
  return { { title = "Please set NOTION_TOKEN", exec = "true" } }
end
```

---

## Type Reference

### Config

```lua
{
  window = {
    width       = number,  -- Window width in px (default: 600)
    list_height = number,  -- List height in px (default: 300)
    icon_size   = number,  -- Icon size in px (default: 48)
  },
  keys = {
    accept       = string,  -- (default: "enter")
    close        = string,  -- (default: "escape")
    up           = string,  -- (default: "up")
    down         = string,  -- (default: "down")
    tab_complete = string,  -- (default: "tab")
  },
}
```

### CommandDefinition

```lua
{
  name        = string,       -- (required) Command name
  description = string,       -- Description (default: "")
  exec        = string,       -- Execution command (default: "")
  args        = CommandArg[], -- Argument definitions (default: {})
  handler     = function,     -- Dynamic handler (optional)
}
```

### CommandArg

```lua
{
  name     = string,     -- (required) Argument name
  required = boolean,    -- (default: false)
  type     = string,     -- "string"|"path"|"url"|"number"|"literal" (default: "string")
  choices  = ArgChoice[], -- (default: {})
}
```

### ArgChoice

```lua
{
  name  = string,  -- (required) Display name
  value = string,  -- (required) Actual value
}
```

### QueryResult (handler return element)

```lua
{
  title = string,  -- (required) Text displayed in the list
  exec  = string,  -- (required) Command executed on selection
}
```

---

## Writing Plugins

Plugins are placed at `~/.config/batto/plugins/<name>/init.lua`.
All APIs (`batto.command()`, `batto.setup()`, etc.) are available inside plugins.

### Directory Structure

```
~/.config/batto/
├── init.lua                    -- Main config
└── plugins/
    ├── web-search/
    │   └── init.lua
    ├── notion/
    │   └── init.lua
    └── my-plugin/
        └── init.lua
```

### Loading Plugins

Call `batto.use()` in `init.lua`:

```lua
batto.use("my-plugin")
```

### Plugin Example

A practical example combining multiple APIs:

```lua
-- ~/.config/batto/plugins/weather/init.lua

batto.command({
  name = "weather",
  description = "Show weather forecast",
  args = {
    { name = "city", required = true, type = "string" },
  },
  handler = function(args)
    local city = args.city or ""
    if city == "" then
      return { { title = "Enter city name...", exec = "true" } }
    end

    local ok, resp = pcall(batto.fetch,
      "https://wttr.in/" .. city .. "?format=j1")
    if not ok then
      return { { title = "Failed to fetch weather", exec = "true" } }
    end

    local data = batto.json_decode(resp)
    local results = {}
    if data.current_condition then
      local c = data.current_condition[1]
      local temp = c.temp_C
      local desc = c.weatherDesc[1].value
      table.insert(results, {
        title = city .. ": " .. desc .. " " .. temp .. "C",
        exec = "xdg-open 'https://wttr.in/" .. city .. "'",
      })
    end
    return results
  end,
})
```

In `init.lua`:

```lua
batto.use("weather")
```

Type `/weather Tokyo` in the launcher to display the weather.
