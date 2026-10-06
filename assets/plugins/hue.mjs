// Philips Hue - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = `http://${settings.bridge || ""}/api/${settings.username || ""}`;
const headers = {};
await restPlugin({ base, headers, need: [settings.bridge, settings.username], status: async (req) => `Connected to ${(await req("GET", "/config")).name}`,
  commands: {
    lights: async (req) => Object.entries(await req("GET", "/lights")).map(([id, l]) => `${id}  ${l.state.on ? "on " : "off"}  ${l.name}`).join("\n"),
    on: async (req, a) => req("PUT", `/lights/${a[0]}/state`, { on: true }),
    off: async (req, a) => req("PUT", `/lights/${a[0]}/state`, { on: false }),
    bri: async (req, a) => req("PUT", `/lights/${a[0]}/state`, { on: true, bri: Number(a[1]) }),
    color: async (req, a) => req("PUT", `/lights/${a[0]}/state`, { on: true, hue: Number(a[1]), sat: 254 }),
    scenes: async (req) => Object.entries(await req("GET", "/scenes")).map(([id, s]) => `${id}  ${s.name}`).join("\n"),
  },
  help: `Usage:\n  lights\n  on <id> | off <id>\n  bri <id> <0-254>\n  color <id> <hue 0-65535>\n  scenes` });
