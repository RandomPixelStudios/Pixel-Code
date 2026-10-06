// Dropbox - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.dropboxapi.com/2";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("POST", "/users/get_current_account", null)).name.display_name}`,
  commands: {
    ls: async (req, a) => (await req("POST", "/files/list_folder", { path: a[0] || "" })).entries.map((e) => `${e[".tag"] === "folder" ? "d" : "-"}  ${e.path_display}`).join("\n"),
    get: async (_req, a) => { const r = await http("POST", "https://content.dropboxapi.com/2/files/download", { headers: { ...headers, "Dropbox-API-Arg": JSON.stringify({ path: a[0] }) }, raw: true }); const { writeFileSync } = await import("node:fs"); const out = a[1] || a[0].split("/").pop(); writeFileSync(out, Buffer.from(await r.arrayBuffer())); return `Saved ${out}`; },
    put: async (_req, a) => { const { readFileSync } = await import("node:fs"); return http("POST", "https://content.dropboxapi.com/2/files/upload", { headers: { ...headers, "Content-Type": "application/octet-stream", "Dropbox-API-Arg": JSON.stringify({ path: a[1], mode: "overwrite" }) }, body: readFileSync(a[0]) }); },
  },
  help: `Usage:\n  ls [path]\n  get <path> [out]\n  put <file> </dropbox/path>` });
