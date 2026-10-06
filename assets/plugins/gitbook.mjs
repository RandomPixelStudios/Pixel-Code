// GitBook: spaces, pages and content search.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: "https://api.gitbook.com/v1", headers: { Authorization: `Bearer ${settings.token || ""}` }, need: [settings.token],
  status: async (req) => `Connected as ${(await req("GET", "/user")).displayName}`,
  commands: {
    spaces: async (req) => { const orgs = (await req("GET", "/orgs")).items; const out = []; for (const o of orgs) for (const s of (await req("GET", `/orgs/${o.id}/spaces`)).items) out.push(`${s.id}  ${o.title} / ${s.title}  ${s.urls?.published ?? ""}`); return out.join("\n"); },
    pages: async (req, [space]) => { const walk = (ps, d = 0) => ps.flatMap((p) => [`${"  ".repeat(d)}${p.id}  ${p.title}`, ...walk(p.pages || [], d + 1)]); return walk((await req("GET", `/spaces/${space}/content`)).pages).join("\n"); },
    search: async (req, [space, ...q]) => (await req("GET", `/spaces/${space}/search?query=${encodeURIComponent(q.join(" "))}`)).items.map((i) => `${i.title}  ${i.path}`).join("\n"),
  },
  help: "Usage:\n  spaces | pages <space_id> | search <space_id> <query>",
});
