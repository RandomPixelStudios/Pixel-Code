// Jira Cloud via REST API (email + API token).
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = `${(settings.url || "").replace(/\/$/, "")}/rest/api/3`;
const headers = { Authorization: `Basic ${Buffer.from(`${settings.email}:${settings.token}`).toString("base64")}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.url || !settings.email || !settings.token)) fail("Jira URL, e-mail and API token are required.");
const adf = (t) => ({ type: "doc", version: 1, content: [{ type: "paragraph", content: [{ type: "text", text: t }] }] });
const plain = (n) => !n ? "" : n.text ?? (n.content || []).map(plain).join(n.type === "paragraph" ? "\n" : "");

switch (cmd) {
  case "connect": case "status": { const u = await http("GET", `${base}/myself`, { headers }); console.log(`Connected as ${u.displayName}`); break; }
  case "search": print((await http("POST", `${base}/search/jql`, { headers, body: { jql: rest.join(" ") || "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC", maxResults: 30, fields: ["summary", "status", "assignee"] } })).issues.map((i) => `${i.key} [${i.fields.status.name}] ${i.fields.summary}`).join("\n")); break;
  case "issue": { const i = await http("GET", `${base}/issue/${rest[0]}?fields=summary,status,description,comment`, { headers }); print(`${i.key} ${i.fields.summary} [${i.fields.status.name}]\n\n${plain(i.fields.description)}\n\n${i.fields.comment.comments.map((c) => `${c.author.displayName}: ${plain(c.body)}`).join("\n")}`); break; }
  case "create": print((await http("POST", `${base}/issue`, { headers, body: { fields: { project: { key: rest[0] }, summary: rest[1], issuetype: { name: "Task" }, description: adf(rest.slice(2).join(" ") || rest[1]) } } })).key); break;
  case "comment": await http("POST", `${base}/issue/${rest[0]}/comment`, { headers, body: { body: adf(rest.slice(1).join(" ")) } }); console.log("Commented."); break;
  case "move": { const t = (await http("GET", `${base}/issue/${rest[0]}/transitions`, { headers })).transitions; const tr = t.find((x) => x.name.toLowerCase() === rest.slice(1).join(" ").toLowerCase()); if (!tr) fail(`Transitions: ${t.map((x) => x.name).join(", ")}`); await http("POST", `${base}/issue/${rest[0]}/transitions`, { headers, body: { transition: { id: tr.id } } }); console.log("Moved."); break; }
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  search [JQL] | issue <KEY-1> | create <PROJECT> <summary> [description] | comment <KEY-1> <text> | move <KEY-1> <transition>
  raw <METHOD> </rest/api/3 path> [json]`);
}
