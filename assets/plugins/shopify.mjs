// Shopify - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = `https://${settings.store || ""}/admin/api/2025-07`;
const headers = { "X-Shopify-Access-Token": settings.token || "" };
await restPlugin({ base, headers, need: [settings.store, settings.token], status: async (req) => `Connected to ${(await req("GET", "/shop.json")).shop.name}`,
  commands: {
    products: async (req) => (await req("GET", "/products.json?limit=50")).products.map((p) => `${p.id}  ${p.title}  ${p.status}`).join("\n"),
    orders: async (req) => (await req("GET", "/orders.json?status=any&limit=20")).orders.map((o) => `${o.name}  ${o.total_price} ${o.currency}  ${o.financial_status}`).join("\n"),
    gql: async (req, a) => req("POST", "/graphql.json", { query: a.join(" ") }),
  },
  help: `Usage:\n  products\n  orders\n  gql <graphql query>` });
