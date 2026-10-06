// Patreon: your campaign and patrons (creator access token).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: "https://www.patreon.com/api/oauth2/v2", headers: { Authorization: `Bearer ${settings.token || ""}` }, need: [settings.token],
  status: async (req) => { const c = (await req("GET", "/campaigns?fields%5Bcampaign%5D=patron_count,creation_name")).data[0]; return `Connected: ${c.attributes.creation_name ?? "campaign"} with ${c.attributes.patron_count} patrons`; },
  commands: {
    patrons: async (req) => { const c = (await req("GET", "/campaigns")).data[0].id; return (await req("GET", `/campaigns/${c}/members?fields%5Bmember%5D=full_name,patron_status,currently_entitled_amount_cents,last_charge_date&page%5Bcount%5D=100`)).data.map((m) => `${m.attributes.full_name}  ${m.attributes.patron_status}  ${(m.attributes.currently_entitled_amount_cents / 100).toFixed(2)}`).join("\n"); },
    posts: async (req) => { const c = (await req("GET", "/campaigns")).data[0].id; return (await req("GET", `/campaigns/${c}/posts?fields%5Bpost%5D=title,published_at,url`)).data.map((p) => `${p.attributes.published_at?.slice(0, 10)}  ${p.attributes.title}  ${p.attributes.url}`).join("\n"); },
  },
  help: "Usage:\n  patrons | posts",
});
