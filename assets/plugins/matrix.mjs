// Matrix - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.homeserver || "https://matrix.org").replace(/\/$/, "") + "/_matrix/client/v3";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("GET", "/account/whoami")).user_id}`,
  commands: {
    send: async (req, a) => { const [f, r] = flags(a); const room = encodeURIComponent(f.room || settings.room); await req("PUT", `/rooms/${room}/send/m.room.message/pc${Date.now()}`, { msgtype: "m.text", body: r.join(" ") }); return "Sent."; },
    read: async (req, a) => { const [f] = flags(a); const room = encodeURIComponent(f.room || settings.room); return (await req("GET", `/rooms/${room}/messages?dir=b&limit=20`)).chunk.filter((e) => e.type === "m.room.message").reverse().map((e) => `${e.sender}: ${e.content.body}`).join("\n"); },
  },
  help: `Usage:\n  send <text> [--room id]\n  read [--room id]` });
