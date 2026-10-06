// Bitbucket - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.bitbucket.org/2.0";
const headers = { Authorization: "Basic " + Buffer.from(`${settings.user || ""}:${settings.token || ""}`).toString("base64") };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("GET", "/user")).display_name}`,
  commands: {
    repos: async (req) => (await req("GET", `/repositories/${settings.workspace || settings.user}?pagelen=50`)).values.map((r) => r.full_name).join("\n"),
    prs: async (req, a) => (await req("GET", `/repositories/${a[0]}/pullrequests`)).values.map((p) => `#${p.id} ${p.title} (${p.state})`).join("\n"),
  },
  help: `Usage:\n  repos\n  prs <workspace/repo>` });
