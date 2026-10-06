// Todoist tasks and projects.
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = "https://api.todoist.com/rest/v2";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [f, rest] = flags(args);
const [cmd, ...a] = rest;
if (cmd && cmd !== "help" && !settings.token) fail("No Todoist token set.");

switch (cmd) {
  case "connect": case "status": { const p = await http("GET", `${base}/projects`, { headers }); console.log(`Connected, ${p.length} projects`); break; }
  case "projects": print((await http("GET", `${base}/projects`, { headers })).map((p) => `${p.id}  ${p.name}`).join("\n")); break;
  case "tasks": {
    const q = f.filter ? `?filter=${encodeURIComponent(f.filter)}` : a[0] ? `?project_id=${a[0]}` : "";
    print((await http("GET", `${base}/tasks${q}`, { headers })).map((t) => `${t.id}  ${t.content}${t.due ? "  (due " + t.due.string + ")" : ""}`).join("\n") || "No tasks.");
    break;
  }
  case "add": {
    const body = { content: a.join(" "), ...(f.due ? { due_string: f.due } : {}), ...(f.project ? { project_id: f.project } : {}), ...(f.priority ? { priority: Number(f.priority) } : {}) };
    const t = await http("POST", `${base}/tasks`, { headers, body });
    console.log(`Added ${t.id}: ${t.content}`);
    break;
  }
  case "done": await http("POST", `${base}/tasks/${a[0]}/close`, { headers, raw: true }); console.log("Done."); break;
  default: console.log(`Usage:
  projects
  tasks [project_id] [--filter "today | overdue"]
  add <text> [--due "tomorrow 9am"] [--project id] [--priority 1-4]
  done <task_id>`);
}
