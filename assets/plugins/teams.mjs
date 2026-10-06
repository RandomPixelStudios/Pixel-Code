// Microsoft Teams - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [cmd, ...a] = args;
if (cmd === "status" || cmd === "connect") { if (!settings.webhook) fail("No webhook URL set."); console.log("Webhook set"); }
else if (cmd === "send") { await http("POST", settings.webhook, { body: { type: "message", attachments: [{ contentType: "application/vnd.microsoft.card.adaptive", content: { type: "AdaptiveCard", version: "1.4", body: [{ type: "TextBlock", text: a.join(" "), wrap: true }] } }] }, raw: true }); console.log("Sent."); }
else console.log("Usage: send <text>");
