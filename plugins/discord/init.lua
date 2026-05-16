-- discord plugin: open Discord and send messages via webhook
-- Usage:
--   /discord         -- send message via Discord webhook (select channel)


batto.command({
  name = "discord",
  description = "Send Discord message via webhook",
  args = {
    {
      name = "channel",
      required = true,
      type = "literal",
      choices = {
        -- Add your webhook channels here:
        -- { name = "General", value = "https://discord.com/api/webhooks/..." },
        -- { name = "Bot-log", value = "https://discord.com/api/webhooks/..." },
      },
    },
    { name = "message", required = true, type = "string" },
  },
  exec = "curl -s -X POST '{{channel}}' -H 'Content-Type: application/json' -d '{\"content\":\"{{message}}\"}'",
})
