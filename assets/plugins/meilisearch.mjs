// Meilisearch - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "http://localhost:7700").replace(/\/$/, "");
const headers = settings.api_key ? { Authorization: `Bearer ${settings.api_key}` } : {};
await restPlugin({ base, headers, need: [], status: async (req) => `Meilisearch ${(await req("GET", "/version")).pkgVersion}`,
  commands: {
    indexes: async (req) => (await req("GET", "/indexes")).results.map((i) => `${i.uid}  (key ${i.primaryKey})`).join("\n"),
    search: async (req, a) => (await req("POST", `/indexes/${a[0]}/search`, { q: a.slice(1).join(" "), limit: 10 })).hits,
  },
  help: `Usage:\n  indexes\n  search <index> <query>` });
