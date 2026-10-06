// Node-RED - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "http://localhost:1880").replace(/\/$/, "");
const headers = settings.token ? { Authorization: `Bearer ${settings.token}` } : {};
await restPlugin({ base, headers, need: [], status: async (req) => `Node-RED ${(await req("GET", "/settings")).version}`,
  commands: {
    flows: async (req) => (await req("GET", "/flows")).filter((n) => n.type === "tab").map((t) => `${t.id}  ${t.label}${t.disabled ? " (disabled)" : ""}`).join("\n"),
    export: async (req, a) => { const f = await req("GET", "/flows"); const { writeFileSync } = await import("node:fs"); const out = a[0] || "flows.json"; writeFileSync(out, JSON.stringify(f, null, 2)); return `Saved ${out}`; },
    deploy: async (req, a) => { const { readFileSync } = await import("node:fs"); await req("POST", "/flows", JSON.parse(readFileSync(a[0], "utf8"))); return "Deployed."; },
  },
  help: `Usage:\n  flows\n  export [file]\n  deploy <flows.json>` });
