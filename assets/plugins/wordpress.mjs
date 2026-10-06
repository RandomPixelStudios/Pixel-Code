// WordPress - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + "/wp-json/wp/v2";
const headers = { Authorization: "Basic " + Buffer.from(`${settings.user || ""}:${settings.password || ""}`).toString("base64") };
await restPlugin({ base, headers, need: [settings.url, settings.password], status: async (req) => `Connected as ${(await req("GET", "/users/me")).name}`,
  commands: {
    posts: async (req) => (await req("GET", "/posts?per_page=20&status=any")).map((p) => `${p.id}  ${p.status}  ${p.title.rendered}`).join("\n"),
    pages: async (req) => (await req("GET", "/pages?per_page=50")).map((p) => `${p.id}  ${p.title.rendered}`).join("\n"),
    post: async (req, a) => { const [f, r] = flags(a); const p = await req("POST", "/posts", { title: r[0], content: r.slice(1).join(" "), status: f.status || "draft" }); return `Created ${p.id}: ${p.link}`; },
  },
  help: `Usage:\n  posts\n  pages\n  post <title> <html> [--status draft|publish]` });
