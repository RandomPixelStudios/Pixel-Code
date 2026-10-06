// PocketBase: collections and records (superuser login).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "http://127.0.0.1:8090").replace(/\/$/, "") + "/api";
async function headers() {
  for (const p of ["/collections/_superusers/auth-with-password", "/admins/auth-with-password"]) {
    const r = await fetch(base + p, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ identity: settings.email, password: settings.password }) });
    if (r.ok) return { Authorization: (await r.json()).token };
  }
  fail("PocketBase login failed");
}
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!settings.email || !settings.password)) fail("Superuser e-mail and password are required.");
switch (cmd) {
  case "connect": case "status": { await headers(); console.log("Connected to PocketBase at " + base); break; }
  case "collections": print((await http("GET", `${base}/collections?perPage=200`, { headers: await headers() })).items.map((c) => `${c.name}  (${c.type})  ${(c.fields || c.schema || []).map((x) => x.name).join(", ")}`).join("\n")); break;
  case "list": print((await http("GET", `${base}/collections/${w[0]}/records?perPage=${f.n || 30}${f.filter ? "&filter=" + encodeURIComponent(f.filter) : ""}`, { headers: await headers() })).items); break;
  case "create": print(await http("POST", `${base}/collections/${w[0]}/records`, { headers: await headers(), body: JSON.parse(w.slice(1).join(" ")) })); break;
  case "update": print(await http("PATCH", `${base}/collections/${w[0]}/records/${w[1]}`, { headers: await headers(), body: JSON.parse(w.slice(2).join(" ")) })); break;
  case "delete": await http("DELETE", `${base}/collections/${w[0]}/records/${w[1]}`, { headers: await headers(), raw: true }); console.log("Deleted."); break;
  default: console.log("Usage:\n  collections | list <collection> [--filter \"x>1\"] [--n 30] | create <collection> <json> | update <collection> <id> <json> | delete <collection> <id>");
}
