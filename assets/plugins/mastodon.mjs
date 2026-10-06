// Mastodon: post toots with media and read notifications.
import { readFileSync } from "node:fs";
import { resolve, basename } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.instance || "https://mastodon.social").replace(/\/$/, "") + "/api";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.token) fail("No Mastodon access token set.");
switch (cmd) {
  case "connect": case "status": { const a = await http("GET", `${base}/v1/accounts/verify_credentials`, { headers }); console.log(`Connected as @${a.acct}`); break; }
  case "post": {
    const media_ids = [];
    for (const p of f.image ? String(f.image).split(",") : []) { const fd = new FormData(); fd.append("file", new Blob([readFileSync(resolve(p))]), basename(p)); if (f.alt) fd.append("description", f.alt); const r = await fetch(`${base}/v2/media`, { method: "POST", headers, body: fd }).then((x) => x.json()); media_ids.push(r.id); }
    const s = await http("POST", `${base}/v1/statuses`, { headers, body: { status: w.join(" "), media_ids, visibility: f.visibility || "public" } });
    console.log("Posted: " + s.url);
    break;
  }
  case "notifications": print((await http("GET", `${base}/v1/notifications?limit=20`, { headers })).map((n) => `${n.type} from @${n.account.acct}${n.status ? ": " + n.status.content.replace(/<[^>]+>/g, "") : ""}`).join("\n")); break;
  default: console.log(`Usage:
  post <text> [--image a.png] [--alt ..] [--visibility public|unlisted|private] | notifications`);
}
