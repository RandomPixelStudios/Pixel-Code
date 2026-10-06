// Pterodactyl panel (client API key): control your game servers.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + "/api/client";
const headers = { Authorization: `Bearer ${settings.api_key || ""}`, Accept: "Application/vnd.pterodactyl.v1+json" };
await restPlugin({
  base, headers, need: [settings.url, settings.api_key],
  status: async (req) => `Connected (${(await req("GET", "")).data.length} servers)`,
  commands: {
    servers: async (req) => (await req("GET", "")).data.map((s) => `${s.attributes.identifier}  ${s.attributes.name}`).join("\n"),
    power: async (req, [id, signal]) => { await req("POST", `/servers/${id}/power`, { signal }); return "Sent " + signal; },
    cmd: async (req, [id, ...c]) => { await req("POST", `/servers/${id}/command`, { command: c.join(" ") }); return "Sent."; },
    resources: async (req, [id]) => (await req("GET", `/servers/${id}/resources`)).attributes,
    files: async (req, [id, dir]) => (await req("GET", `/servers/${id}/files/list?directory=${encodeURIComponent(dir || "/")}`)).data.map((f) => (f.attributes.is_file ? "  " : "D ") + f.attributes.name).join("\n"),
    upload: async (req, [id, file, dir]) => {
      const { readFileSync } = await import("node:fs"); const { basename, resolve } = await import("node:path");
      const u = (await req("GET", `/servers/${id}/files/upload`)).attributes.url;
      const fd = new FormData(); fd.append("files", new Blob([readFileSync(resolve(file))]), basename(file));
      const r = await fetch(`${u}&directory=${encodeURIComponent(dir || "/")}`, { method: "POST", body: fd });
      return r.ok ? "Uploaded." : "Upload failed: HTTP " + r.status;
    },
  },
  help: `Usage:
  servers | power <id> start|stop|restart|kill | cmd <id> <console command> | resources <id>
  files <id> [dir] | upload <id> <file> [dir]   (e.g. upload a mod jar to /mods)`,
});
