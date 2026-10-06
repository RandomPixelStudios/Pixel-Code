// Asana - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://app.asana.com/api/1.0";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("GET", "/users/me")).data.name}`,
  commands: {
    projects: async (req) => { const ws = (await req("GET", "/workspaces")).data[0].gid; return (await req("GET", `/projects?workspace=${ws}`)).data.map((p) => `${p.gid}  ${p.name}`).join("\n"); },
    tasks: async (req, a) => (await req("GET", `/projects/${a[0]}/tasks?opt_fields=name,completed`)).data.filter((t) => !t.completed).map((t) => `${t.gid}  ${t.name}`).join("\n"),
    add: async (req, a) => (await req("POST", "/tasks", { data: { name: a.slice(1).join(" "), projects: [a[0]] } })).data.gid,
    done: async (req, a) => { await req("PUT", `/tasks/${a[0]}`, { data: { completed: true } }); return "Done."; },
  },
  help: `Usage:\n  projects\n  tasks <project_id>\n  add <project_id> <name>\n  done <task_id>` });
