// Netlify sites and deploys via API token (CLI through npx for deploys).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print } from "./common.mjs";

const headers = { Authorization: `Bearer ${settings.token || ""}` };
const api = (p) => http("GET", "https://api.netlify.com/api/v1" + p, { headers });
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Netlify token set.");
const cli = (a) => { const r = spawnSync("npx", ["-y", "netlify-cli@latest", ...a], { stdio: "inherit", env: { ...process.env, NETLIFY_AUTH_TOKEN: settings.token } }); process.exit(r.status ?? 1); };

switch (cmd) {
  case "connect": case "status": { const u = await api("/user"); console.log(`Connected as ${u.email}`); break; }
  case "sites": print((await api("/sites")).map((s) => `${s.id} ${s.name} ${s.ssl_url || s.url}`).join("\n")); break;
  case "deploys": print((await api(`/sites/${rest[0]}/deploys?per_page=10`)).map((d) => `${d.state} ${d.created_at} ${d.deploy_ssl_url}`).join("\n")); break;
  case "deploy": cli(["deploy", "--dir", rest[0] || ".", ...(rest.includes("--prod") ? ["--prod"] : []), ...(rest.find((x) => x.startsWith("--site")) ? [] : [])]); break;
  case "cli": cli(rest); break;
  default: console.log(`Usage:
  sites | deploys <site_id> | deploy [dir] [--prod] | cli <args>`);
}
