// Jenkins - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "");
const headers = { Authorization: "Basic " + Buffer.from(`${settings.user || ""}:${settings.token || ""}`).toString("base64") };
await restPlugin({ base, headers, need: [settings.url, settings.token], status: async (req) => `Connected, ${(await req("GET", "/api/json")).jobs.length} jobs`,
  commands: {
    jobs: async (req) => (await req("GET", "/api/json?tree=jobs[name,color]")).jobs.map((j) => `${j.name}  ${j.color}`).join("\n"),
    build: async (req, a) => { await req("POST", `/job/${a[0]}/build`); return "Build queued."; },
    log: async (_req, a) => (await http("GET", `${base}/job/${a[0]}/${a[1] || "lastBuild"}/consoleText`, { headers, raw: true }).then((r) => r.text())).slice(-20000),
  },
  help: `Usage:\n  jobs\n  build <job>\n  log <job> [build number]` });
