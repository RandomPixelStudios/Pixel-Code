// ClickUp - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.clickup.com/api/v2";
const headers = { Authorization: settings.token || "" };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("GET", "/user")).user.username}`,
  commands: {
    teams: async (req) => (await req("GET", "/team")).teams.map((t) => `${t.id}  ${t.name}`).join("\n"),
    tasks: async (req, a) => (await req("GET", `/list/${a[0]}/task`)).tasks.map((t) => `${t.id}  ${t.status?.status}  ${t.name}`).join("\n"),
    add: async (req, a) => (await req("POST", `/list/${a[0]}/task`, { name: a.slice(1).join(" ") })).url,
  },
  help: `Usage:\n  teams\n  tasks <list_id>\n  add <list_id> <name>` });
