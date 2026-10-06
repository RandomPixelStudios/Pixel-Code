// OneSignal: push notifications to your app users.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const app = settings.app_id;
const headers = { Authorization: `Key ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!app || !settings.api_key)) fail("App ID and REST API key are required.");
switch (cmd) {
  case "connect": case "status": { const a = await http("GET", `https://api.onesignal.com/apps/${app}`, { headers }); console.log(`Connected to ${a.name} (${a.players ?? "?"} subscribers)`); break; }
  case "send": { const r = await http("POST", "https://api.onesignal.com/notifications", { headers, body: { app_id: app, target_channel: "push", headings: { en: f.title || "Update" }, contents: { en: w.join(" ") }, included_segments: [f.segment || "Total Subscriptions"], ...(f.url ? { url: f.url } : {}) } }); console.log(`Sent (id ${r.id})`); break; }
  case "history": print((await http("GET", `https://api.onesignal.com/notifications?app_id=${app}&limit=15`, { headers })).notifications.map((n) => `${new Date(n.queued_at * 1000).toISOString().slice(0, 16)}  ${n.headings?.en ?? ""}: ${n.contents?.en}  delivered ${n.successful}`).join("\n")); break;
  default: console.log("Usage:\n  send <text> [--title ..] [--segment \"Active Users\"] [--url ..] | history");
}
