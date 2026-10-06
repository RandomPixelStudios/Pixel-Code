// Twitch: stream info, title/category, clips and chat messages.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.twitch.tv/helix";
const headers = { Authorization: `Bearer ${settings.token || ""}`, "Client-Id": settings.client_id || "" };
const get = (p) => http("GET", base + p, { headers });
const me = async () => (await get("/users")).data[0];
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!settings.token || !settings.client_id)) fail("Client id and user access token are required.");
switch (cmd) {
  case "connect": case "status": { const u = await me(); console.log(`Connected as ${u.display_name}`); break; }
  case "stream": { const u = await me(); const s = (await get(`/streams?user_id=${u.id}`)).data[0]; print(s ? `LIVE: ${s.title} (${s.game_name}), ${s.viewer_count} viewers` : "Offline"); break; }
  case "title": { const u = await me(); const body = { ...(w.length ? { title: w.join(" ") } : {}) }; if (f.game) body.game_id = (await get(`/games?name=${encodeURIComponent(f.game)}`)).data[0]?.id; await http("PATCH", `${base}/channels?broadcaster_id=${u.id}`, { headers, body }); console.log("Updated."); break; }
  case "clips": { const u = await me(); print((await get(`/clips?broadcaster_id=${u.id}&first=20`)).data.map((c) => `${c.title}  ${c.view_count} views  ${c.url}`).join("\n")); break; }
  case "chat": { const u = await me(); await http("POST", `${base}/chat/messages`, { headers, body: { broadcaster_id: u.id, sender_id: u.id, message: w.join(" ") } }); console.log("Sent."); break; }
  default: console.log(`Usage:
  stream | title [new title] [--game "Minecraft"] | clips | chat <message>`);
}
