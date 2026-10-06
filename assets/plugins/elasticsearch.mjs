// Elasticsearch - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "http://localhost:9200").replace(/\/$/, "");
const headers = settings.api_key ? { Authorization: `ApiKey ${settings.api_key}` } : {};
await restPlugin({ base, headers, need: [], status: async (req) => `Elasticsearch ${(await req("GET", "/")).version?.number}`,
  commands: {
    indices: async (req) => (await req("GET", "/_cat/indices?format=json")).map((i) => `${i.index}  ${i["docs.count"]} docs  ${i["store.size"]}`).join("\n"),
    search: async (req, a) => (await req("GET", `/${a[0]}/_search?q=${encodeURIComponent(a.slice(1).join(" "))}&size=10`)).hits.hits.map((h) => JSON.stringify(h._source)).join("\n"),
  },
  help: `Usage:\n  indices\n  search <index> <query>` });
