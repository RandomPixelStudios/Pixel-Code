// Confluence - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + "/wiki/rest/api";
const headers = { Authorization: "Basic " + Buffer.from(`${settings.email || ""}:${settings.token || ""}`).toString("base64") };
await restPlugin({ base, headers, need: [settings.url, settings.token], status: async (req) => `Connected as ${(await req("GET", "/user/current")).displayName}`,
  commands: {
    search: async (req, a) => (await req("GET", `/search?cql=${encodeURIComponent(`text ~ "${a.join(" ")}"`)}&limit=15`)).results.map((r) => `${r.content?.id}  ${r.title}`).join("\n"),
    page: async (req, a) => (await req("GET", `/content/${a[0]}?expand=body.storage`)).body.storage.value.replace(/<[^>]+>/g, " ").slice(0, 20000),
  },
  help: `Usage:\n  search <text>\n  page <id>` });
