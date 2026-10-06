// Algolia search: indices, search and record upload.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const appId = settings.app_id;
await restPlugin({
  base: `https://${appId}.algolia.net/1`, headers: { "X-Algolia-Application-Id": appId || "", "X-Algolia-API-Key": settings.api_key || "" }, need: [appId, settings.api_key],
  status: async (req) => `Connected (${(await req("GET", "/indexes")).items.length} indices)`,
  commands: {
    indices: async (req) => (await req("GET", "/indexes")).items.map((i) => `${i.name}  ${i.entries} records  updated ${i.updatedAt.slice(0, 10)}`).join("\n"),
    search: async (req, [index, ...q]) => (await req("POST", `/indexes/${index}/query`, { params: `query=${encodeURIComponent(q.join(" "))}&hitsPerPage=10` })).hits.map((h) => JSON.stringify(h).slice(0, 300)).join("\n"),
    upload: async (req, [index, file]) => { const recs = JSON.parse(readFileSync(resolve(file), "utf8")); const r = await req("POST", `/indexes/${index}/batch`, { requests: recs.map((body) => ({ action: body.objectID ? "updateObject" : "addObject", body })) }); return `Uploaded ${recs.length} records (task ${r.taskID})`; },
    clear: async (req, [index]) => req("POST", `/indexes/${index}/clear`),
  },
  help: "Usage:\n  indices | search <index> <query> | upload <index> <records.json> | clear <index>",
});
