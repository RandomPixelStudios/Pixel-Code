// Tasmota - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "";
const headers = {};
await restPlugin({ base, headers, need: [], status: async () => `${(settings.devices || "").split(",").filter(Boolean).length} devices`,
  commands: {
    cmd: async (_req, a) => { const map = Object.fromEntries((settings.devices || "").split(",").map((x) => x.split("=").map((s) => s.trim())).filter((x) => x[1])); const ip = map[a[0]] || a[0]; const auth = settings.password ? `user=admin&password=${encodeURIComponent(settings.password)}&` : ""; return http("GET", `http://${ip}/cm?${auth}cmnd=${encodeURIComponent(a.slice(1).join(" "))}`); },
    status: async (_req, a) => { const map = Object.fromEntries((settings.devices || "").split(",").map((x) => x.split("=").map((s) => s.trim())).filter((x) => x[1])); const ip = map[a[0]] || a[0]; return http("GET", `http://${ip}/cm?cmnd=Status%200`); },
  },
  help: `Usage:\n  cmd <device> <command>   e.g. cmd plug Power TOGGLE, cmd lamp Dimmer 40\n  status <device>` });
