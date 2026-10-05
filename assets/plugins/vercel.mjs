// Vercel deployments via the Vercel CLI (installed on demand with npx) and API token.
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print } from "./common.mjs";

const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Vercel token set.");
const cli = (a) => { const r = spawnSync("npx", ["-y", "vercel@latest", ...a, "--token", settings.token, "--yes"], { stdio: "inherit" }); process.exit(r.status ?? 1); };

switch (cmd) {
  case "connect": case "status": { const u = await http("GET", "https://api.vercel.com/v2/user", { headers }); console.log(`Connected as ${u.user.username}`); break; }
  case "deployments": print((await http("GET", "https://api.vercel.com/v6/deployments?limit=15", { headers })).deployments.map((d) => `${d.state} ${d.name} https://${d.url} ${new Date(d.created).toISOString()}`).join("\n")); break;
  case "deploy": cli(rest.includes("--prod") ? ["deploy", "--prod"] : ["deploy"]); break;
  case "logs": cli(["logs", rest[0]]); break;
  case "cli": cli(rest); break;
  default: console.log(`Usage:
  deployments            recent deployments
  deploy [--prod]        deploy the current directory
  logs <url>             logs of a deployment
  cli <args>             any Vercel CLI command`);
}
