// Lemon Squeezy: stores, products, orders and license keys.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: "https://api.lemonsqueezy.com/v1", headers: { Authorization: `Bearer ${settings.api_key || ""}`, Accept: "application/vnd.api+json" }, need: [settings.api_key],
  status: async (req) => `Connected as ${(await req("GET", "/users/me")).data.attributes.name}`,
  commands: {
    products: async (req) => (await req("GET", "/products")).data.map((p) => `${p.id}  ${p.attributes.name}  ${p.attributes.price_formatted}`).join("\n"),
    orders: async (req) => (await req("GET", "/orders?page[size]=25")).data.map((o) => `${o.attributes.created_at.slice(0, 10)}  ${o.attributes.first_order_item?.product_name}  ${o.attributes.total_formatted}  ${o.attributes.user_email}`).join("\n"),
    licenses: async (req) => (await req("GET", "/license-keys?page[size]=25")).data.map((l) => `${l.attributes.key_short}  ${l.attributes.status}  ${l.attributes.user_email}`).join("\n"),
  },
  help: "Usage:\n  products | orders | licenses",
});
