// Gumroad: products and sales.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const tok = `access_token=${settings.token || ""}`;
await restPlugin({
  base: "https://api.gumroad.com/v2", need: [settings.token],
  status: async (req) => `Connected as ${(await req("GET", `/user?${tok}`)).user.name}`,
  commands: {
    products: async (req) => (await req("GET", `/products?${tok}`)).products.map((p) => `${p.id}  ${p.name}  ${p.formatted_price}  ${p.sales_count} sales`).join("\n"),
    sales: async (req, [after]) => (await req("GET", `/sales?${tok}${after ? "&after=" + after : ""}`)).sales.map((s) => `${s.created_at.slice(0, 10)}  ${s.product_name}  ${s.formatted_display_price}  ${s.email}`).join("\n"),
  },
  help: "Usage:\n  products | sales [after YYYY-MM-DD]",
});
