// Better Stack Uptime: monitors and incidents.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: "https://uptime.betterstack.com/api/v2", headers: { Authorization: `Bearer ${settings.token || ""}` }, need: [settings.token],
  status: async (req) => `Connected (${(await req("GET", "/monitors")).data.length} monitors)`,
  commands: {
    monitors: async (req) => (await req("GET", "/monitors")).data.map((m) => `${m.attributes.status.padEnd(8)} ${m.attributes.pronounceable_name}  ${m.attributes.url}`).join("\n"),
    incidents: async (req) => (await req("GET", "/incidents")).data.slice(0, 20).map((i) => `${i.attributes.started_at}  ${i.attributes.name}  ${i.attributes.resolved_at ? "resolved" : "OPEN"}`).join("\n"),
    pause: async (req, [id]) => req("PATCH", `/monitors/${id}`, { paused: true }),
    resume: async (req, [id]) => req("PATCH", `/monitors/${id}`, { paused: false }),
  },
  help: "Usage:\n  monitors | incidents | pause <id> | resume <id>",
});
