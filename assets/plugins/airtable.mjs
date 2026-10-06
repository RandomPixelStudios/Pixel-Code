// Airtable - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.airtable.com/v0";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected, ${(await req("GET", "/meta/bases")).bases.length} bases`,
  commands: {
    bases: async (req) => (await req("GET", "/meta/bases")).bases.map((b) => `${b.id}  ${b.name}`).join("\n"),
    records: async (req, a) => (await req("GET", `/${a[0]}/${encodeURIComponent(a[1])}?maxRecords=50`)).records.map((r) => `${r.id}  ${JSON.stringify(r.fields)}`).join("\n"),
    add: async (req, a) => req("POST", `/${a[0]}/${encodeURIComponent(a[1])}`, { fields: JSON.parse(a.slice(2).join(" ")) }),
  },
  help: `Usage:\n  bases\n  records <base_id> <table>\n  add <base_id> <table> <json fields>` });
