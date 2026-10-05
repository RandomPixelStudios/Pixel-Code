// Notion: search, read and write pages (internal integration token).
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.notion.com/v1";
const headers = { Authorization: `Bearer ${settings.token || ""}`, "Notion-Version": "2022-06-28" };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Notion token set.");
const title = (p) => Object.values(p.properties || {}).find((x) => x.type === "title")?.title?.map((t) => t.plain_text).join("") || p.title?.[0]?.plain_text || "(untitled)";
const text = (b) => (b[b.type]?.rich_text || []).map((t) => t.plain_text).join("");

switch (cmd) {
  case "connect": case "status": { const u = await http("GET", `${base}/users/me`, { headers }); console.log(`Connected as ${u.name} (share pages with the integration to give access)`); break; }
  case "search": print((await http("POST", `${base}/search`, { headers, body: { query: rest.join(" "), page_size: 20 } })).results.map((p) => `${p.object} ${p.id}  ${title(p)}  ${p.url}`).join("\n")); break;
  case "read": {
    const blocks = await http("GET", `${base}/blocks/${rest[0]}/children?page_size=100`, { headers });
    print(blocks.results.map((b) => `${b.type === "heading_1" ? "# " : b.type === "heading_2" ? "## " : b.type === "bulleted_list_item" ? "- " : b.type === "to_do" ? (b.to_do.checked ? "[x] " : "[ ] ") : ""}${text(b)}`).join("\n"));
    break;
  }
  case "append": await http("PATCH", `${base}/blocks/${rest[0]}/children`, { headers, body: { children: rest.slice(1).join(" ").split("\\n").map((l) => ({ object: "block", type: "paragraph", paragraph: { rich_text: [{ type: "text", text: { content: l } }] } })) } }); console.log("Appended."); break;
  case "create": print((await http("POST", `${base}/pages`, { headers, body: { parent: { page_id: rest[0] }, properties: { title: { title: [{ text: { content: rest[1] } }] } }, children: rest.slice(2).join(" ") ? [{ object: "block", type: "paragraph", paragraph: { rich_text: [{ type: "text", text: { content: rest.slice(2).join(" ") } }] } }] : [] } })).url); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  search <query> | read <page_id> | append <page_id> <text (\\n = new line)> | create <parent_page_id> <title> [text]
  raw <METHOD> </v1 path> [json]`);
}
