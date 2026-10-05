// Supabase: projects, SQL and edge functions via the Management API (personal access token).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.supabase.com/v1";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Supabase access token set.");

switch (cmd) {
  case "connect": case "status": { const p = await http("GET", `${base}/projects`, { headers }); console.log(`Connected (${p.length} projects)`); break; }
  case "projects": print((await http("GET", `${base}/projects`, { headers })).map((p) => `${p.id} ${p.name} ${p.region} ${p.status}`).join("\n")); break;
  case "sql": print(await http("POST", `${base}/projects/${rest[0]}/database/query`, { headers, body: { query: rest.slice(1).join(" ") } })); break;
  case "functions": print((await http("GET", `${base}/projects/${rest[0]}/functions`, { headers })).map((f) => `${f.slug} v${f.version} ${f.status}`).join("\n")); break;
  case "cli": process.exit(spawnSync("npx", ["-y", "supabase@latest", ...rest], { stdio: "inherit", env: { ...process.env, SUPABASE_ACCESS_TOKEN: settings.token } }).status ?? 1);
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  projects | sql <project_ref> <query> | functions <project_ref> | cli <supabase cli args> (e.g. 'functions deploy x --project-ref ..')
  raw <METHOD> </v1 path> [json]`);
}
