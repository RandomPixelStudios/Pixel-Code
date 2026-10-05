// GitLab (gitlab.com or self-hosted) via REST API v4 and a personal access token.
import { args, settings, fail, http, print, rawCall, flags } from "./common.mjs";

const base = (settings.url || "https://gitlab.com").replace(/\/$/, "") + "/api/v4";
const headers = { "PRIVATE-TOKEN": settings.token || "" };
const get = (p) => http("GET", base + p, { headers });
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
const proj = (p) => encodeURIComponent(p || f.project || "");
if (cmd && cmd !== "help" && !settings.token) fail("No GitLab token set.");

switch (cmd) {
  case "connect": case "status": { const u = await get("/user"); console.log(`Connected as ${u.username} (${base.replace("/api/v4", "")})`); break; }
  case "projects": print((await get(`/projects?membership=true&per_page=50&order_by=last_activity_at`)).map((p) => `${p.path_with_namespace}  ${p.web_url}`).join("\n")); break;
  case "issues": print((await get(`/projects/${proj(w[0])}/issues?state=${f.state || "opened"}&per_page=50`)).map((i) => `#${i.iid} ${i.title} [${i.state}]`).join("\n")); break;
  case "issue-create": print(await http("POST", `${base}/projects/${proj(w[0])}/issues`, { headers, body: { title: w[1], description: w.slice(2).join(" ") } })); break;
  case "mrs": print((await get(`/projects/${proj(w[0])}/merge_requests?state=${f.state || "opened"}`)).map((m) => `!${m.iid} ${m.title} (${m.source_branch} -> ${m.target_branch}) ${m.web_url}`).join("\n")); break;
  case "mr-create": print(await http("POST", `${base}/projects/${proj(w[0])}/merge_requests`, { headers, body: { source_branch: w[1], target_branch: w[2] || "main", title: w.slice(3).join(" ") || w[1] } })); break;
  case "pipelines": print((await get(`/projects/${proj(w[0])}/pipelines?per_page=10`)).map((p) => `${p.id} ${p.status} ${p.ref} ${p.web_url}`).join("\n")); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage (project = "group/name"):
  projects | issues <project> [--state opened|closed] | issue-create <project> <title> [description]
  mrs <project> | mr-create <project> <source> [target] [title] | pipelines <project>
  raw <METHOD> </api/v4 path> [json]`);
}
