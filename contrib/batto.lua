---@meta

---@class batto.WindowConfig
---@field width number Window width in pixels (default: 600)
---@field list_height number List area height in pixels (default: 300)
---@field icon_size number App icon size in pixels (default: 48)
---@field hide_on_blur boolean Quit when window loses focus (default: true)

---@class batto.KeysConfig
---@field accept string Key binding for accept (default: "enter")
---@field close string Key binding for close (default: "escape")
---@field up string Key binding for move up (default: "up")
---@field down string Key binding for move down (default: "down")
---@field tab_complete string Key binding for tab completion (default: "tab")

---@class batto.Config
---@field window batto.WindowConfig
---@field keys batto.KeysConfig

---@class batto.ArgChoice
---@field name string Display name
---@field value string Actual value

---@class batto.CommandArg
---@field name string Argument name
---@field required boolean Whether argument is required (default: false)
---@field type "string"|"path"|"url"|"number"|"literal" Argument type (default: "string")
---@field choices batto.ArgChoice[] Predefined choices

---@class batto.QueryResult
---@field title string Text displayed in the list
---@field exec string Command executed on selection

---@class batto.CommandDefinition
---@field name string Command name (invoked via /name)
---@field description string Description shown in command list
---@field exec string Static execution command (supports {{args}} template)
---@field args batto.CommandArg[] Argument definitions
---@field handler fun(args: table):batto.QueryResult[] Dynamic handler function

---@class batto.FetchOpts
---@field method string HTTP method ("GET", "POST", etc.)
---@field headers table<string, string> Request headers
---@field body string Request body

---@class batto
local batto = {}

---Configure batto UI and key bindings.
---@param config batto.Config
function batto.setup(config) end

---Define a custom command invoked with /name in the launcher.
---@param definition batto.CommandDefinition
function batto.command(definition) end

---Load a plugin from ~/.config/batto/plugins/<name>/init.lua.
---@param name string Plugin directory name
function batto.use(name) end

---Perform an HTTP request using curl.
---@param url string Request URL
---@param opts? batto.FetchOpts Request options
---@return string body Response body
function batto.fetch(url, opts) end

---Decode a JSON string to a Lua value.
---@param s string JSON string
---@return any
function batto.json_decode(s) end

---Encode a Lua value to a JSON string.
---@param val any
---@return string
function batto.json_encode(val) end

---Get an environment variable value. Also reads from ~/.config/batto/.env.
---@param name string Environment variable name
---@return string|nil
function batto.env(name) end

return batto
