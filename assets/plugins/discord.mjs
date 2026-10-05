// Discord bot: the agent can post and read messages. Incoming messages in the control channel are
// forwarded to the agents by the Pixel Code app (like Telegram).
import { args, settings, fail, http, print } from "./common.mjs";

const base = "https://discord.com/api/v10";
const headers = { Authorization: `Bot ${settings.bot_token || ""}` };
const channel = settings.channel_id;
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.bot_token || !channel)) fail("Discord bot token and channel id are required.");

switch (cmd) {
  case "connect": case "status": {
    const me = await http("GET", `${base}/users/@me`, { headers });
    const ch = await http("GET", `${base}/channels/${channel}`, { headers });
    console.log(`${me.username} connected to #${ch.name}`);
    break;
  }
  case "send": print((await http("POST", `${base}/channels/${channel}/messages`, { headers, body: { content: rest.join(" ").slice(0, 2000) } })).id ? "Sent." : "Failed"); break;
  case "read": print((await http("GET", `${base}/channels/${rest[0] || channel}/messages?limit=${rest[1] || 20}`, { headers })).reverse().map((m) => `${m.author.username}: ${m.content}`).join("\n")); break;
  default: console.log(`Usage:
  send <text>                 post in the control channel
  read [channel_id] [count]   recent messages`);
}
