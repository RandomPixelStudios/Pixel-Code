// Stripe via its REST API (use a restricted or test key).
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = "https://api.stripe.com/v1";
const headers = { Authorization: `Bearer ${settings.api_key || ""}` };
const [f, rest] = flags(args);
const [cmd, ...a] = rest;
if (cmd && cmd !== "help" && !settings.api_key) fail("No Stripe API key set.");
const list = async (what) => (await http("GET", `${base}/${what}?limit=${f.n || 20}`, { headers })).data;
const money = (x, c) => `${(x / 100).toFixed(2)} ${String(c).toUpperCase()}`;

switch (cmd) {
  case "connect": case "status": { const b = await http("GET", `${base}/balance`, { headers }); console.log(`Connected (${b.livemode ? "live" : "test"} mode)`); break; }
  case "balance": print((await http("GET", `${base}/balance`, { headers })).available.map((b) => money(b.amount, b.currency)).join("\n")); break;
  case "payments": print((await list("payment_intents")).map((p) => `${p.id}  ${money(p.amount, p.currency)}  ${p.status}  ${new Date(p.created * 1000).toISOString().slice(0, 10)}`).join("\n")); break;
  case "customers": print((await list("customers")).map((c) => `${c.id}  ${c.email || ""}  ${c.name || ""}`).join("\n")); break;
  case "products": print((await list("products")).map((p) => `${p.id}  ${p.name}  ${p.active ? "active" : "archived"}`).join("\n")); break;
  case "subscriptions": print((await list("subscriptions")).map((s) => `${s.id}  ${s.customer}  ${s.status}`).join("\n")); break;
  case "raw": print(await http(a[0] || "GET", base + a[1], { headers, form: a[2] ? JSON.parse(a[2]) : undefined })); break;
  default: console.log(`Usage:
  balance | payments | customers | products | subscriptions  [--n 20]
  raw <METHOD> </path> [json form]   e.g. raw GET /invoices`);
}
