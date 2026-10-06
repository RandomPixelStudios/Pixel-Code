// Figma - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.figma.com/v1";
const headers = { "X-Figma-Token": settings.token || "" };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("GET", "/me")).handle}`,
  commands: {
    file: async (req, a) => { const f = await req("GET", `/files/${a[0]}?depth=2`); return f.document.children.map((p) => `${p.name}: ${(p.children || []).map((c) => `${c.name} (${c.id})`).join(", ")}`).join("\n"); },
    export: async (req, a) => { const [f, r] = flags(a); return (await req("GET", `/images/${r[0]}?ids=${r[1]}&format=${f.format || "png"}&scale=${f.scale || 2}`)).images; },
    comments: async (req, a) => (await req("GET", `/files/${a[0]}/comments`)).comments.map((c) => `${c.user.handle}: ${c.message}`).join("\n"),
  },
  help: `Usage:\n  file <file_key>\n  export <file_key> <node_ids comma separated> [--format png|svg|pdf] [--scale 2]\n  comments <file_key>` });
