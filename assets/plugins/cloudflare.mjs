// Cloudflare: DNS, Workers and Pages via API token (wrangler through npx for deploys).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.cloudflare.com/client/v4";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const get = async (p) => (await http("GET", base + p, { headers })).result;
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Cloudflare API token set.");

switch (cmd) {
  case "connect": case "status": { await get("/user/tokens/verify"); const z = await get("/zones"); console.log(`Token valid (${z.length} zones)`); break; }
  case "zones": print((await get("/zones")).map((z) => `${z.id} ${z.name} ${z.status}`).join("\n")); break;
  case "dns": print((await get(`/zones/${rest[0]}/dns_records?per_page=100`)).map((r) => `${r.id} ${r.type} ${r.name} -> ${r.content}${r.proxied ? " (proxied)" : ""}`).join("\n")); break;
  case "dns-add": print(await http("POST", `${base}/zones/${rest[0]}/dns_records`, { headers, body: { type: rest[1], name: rest[2], content: rest[3], proxied: rest[4] === "proxied" } })); break;
  case "wrangler": { const r = spawnSync("npx", ["-y", "wrangler@latest", ...rest], { stdio: "inherit", env: { ...process.env, CLOUDFLARE_API_TOKEN: settings.token } }); process.exit(r.status ?? 1); }
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  zones | dns <zone_id> | dns-add <zone_id> <TYPE> <name> <content> [proxied]
  wrangler <args>        e.g. 'deploy', 'pages deploy dist'
  raw <METHOD> <path> [json]`);
}
