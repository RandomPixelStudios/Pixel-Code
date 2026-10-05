// Trello boards, lists and cards (API key + token).
import { args, settings, fail, http, print } from "./common.mjs";

const auth = `key=${settings.api_key}&token=${settings.token}`;
const api = (m, p, q = "") => http(m, `https://api.trello.com/1${p}?${auth}${q}`);
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.api_key || !settings.token)) fail("Trello API key and token are required.");

switch (cmd) {
  case "connect": case "status": { const u = await api("GET", "/members/me"); console.log(`Connected as ${u.username}`); break; }
  case "boards": print((await api("GET", "/members/me/boards", "&filter=open")).map((b) => `${b.id} ${b.name}`).join("\n")); break;
  case "lists": print((await api("GET", `/boards/${rest[0]}/lists`)).map((l) => `${l.id} ${l.name}`).join("\n")); break;
  case "cards": print((await api("GET", `/lists/${rest[0]}/cards`)).map((c) => `${c.id} ${c.name}${c.due ? " (due " + c.due + ")" : ""}`).join("\n")); break;
  case "add": print((await api("POST", "/cards", `&idList=${rest[0]}&name=${encodeURIComponent(rest[1])}&desc=${encodeURIComponent(rest.slice(2).join(" "))}`)).shortUrl); break;
  case "move": await api("PUT", `/cards/${rest[0]}`, `&idList=${rest[1]}`); console.log("Moved."); break;
  case "comment": await api("POST", `/cards/${rest[0]}/actions/comments`, `&text=${encodeURIComponent(rest.slice(1).join(" "))}`); console.log("Commented."); break;
  default: console.log(`Usage:
  boards | lists <board_id> | cards <list_id> | add <list_id> <name> [description] | move <card_id> <list_id> | comment <card_id> <text>`);
}
