// Gitea / Forgejo / Codeberg via REST API and an access token.
import { args, settings, fail, http, print, rawCall, flags } from "./common.mjs";

const base = (settings.url || "https://codeberg.org").replace(/\/$/, "") + "/api/v1";
const headers = { Authorization: `token ${settings.token || ""}` };
const get = (p) => http("GET", base + p, { headers });
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.token) fail("No Gitea/Forgejo token set.");

switch (cmd) {
  case "connect": case "status": { const u = await get("/user"); console.log(`Connected as ${u.login} (${base.replace("/api/v1", "")})`); break; }
  case "repos": print((await get("/user/repos?limit=50")).map((r) => `${r.full_name}  ${r.html_url}`).join("\n")); break;
  case "issues": print((await get(`/repos/${w[0]}/issues?state=${f.state || "open"}&type=issues`)).map((i) => `#${i.number} ${i.title}`).join("\n")); break;
  case "issue-create": print(await http("POST", `${base}/repos/${w[0]}/issues`, { headers, body: { title: w[1], body: w.slice(2).join(" ") } })); break;
  case "prs": print((await get(`/repos/${w[0]}/pulls?state=${f.state || "open"}`)).map((p) => `#${p.number} ${p.title} ${p.html_url}`).join("\n")); break;
  case "pr-create": print(await http("POST", `${base}/repos/${w[0]}/pulls`, { headers, body: { head: w[1], base: w[2] || "main", title: w.slice(3).join(" ") || w[1] } })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage (repo = "owner/name"):
  repos | issues <repo> | issue-create <repo> <title> [body] | prs <repo> | pr-create <repo> <head> [base] [title]
  raw <METHOD> </api/v1 path> [json]`);
}
