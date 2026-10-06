// Mattermost - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + "/api/v4";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.url, settings.token], status: async (req) => `Connected as ${(await req("GET", "/users/me")).username}`,
  commands: {
    send: async (req, a) => { const [f, r] = flags(a); await req("POST", "/posts", { channel_id: f.channel || settings.channel, message: r.join(" ") }); return "Sent."; },
    channels: async (req) => (await req("GET", "/users/me/channels")).map((c) => `${c.id}  ${c.display_name || c.name}`).join("\n"),
  },
  help: `Usage:\n  send <text> [--channel id]\n  channels` });
