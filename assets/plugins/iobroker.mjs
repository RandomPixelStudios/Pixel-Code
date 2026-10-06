// ioBroker via the simple-api adapter (default port 8087).
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = (settings.url || "http://iobroker.local:8087").replace(/\/$/, "");
const auth = settings.user ? `user=${encodeURIComponent(settings.user)}&pass=${encodeURIComponent(settings.password || "")}` : "";
const url = (path, query = "") => `${base}${path}${query || auth ? "?" : ""}${[query, auth].filter(Boolean).join("&")}`;
const enc = (id) => encodeURIComponent(id);
const [f, rest] = flags(args);
const [cmd, ...a] = rest;

function lines(states) {
  return Object.entries(states).map(([id, s]) => `${id} = ${s && typeof s === "object" ? JSON.stringify(s.val) : JSON.stringify(s)}`).join("\n");
}

switch (cmd) {
  case "connect": case "status": {
    const hosts = await http("GET", url("/objects", "pattern=system.host.*&type=host"));
    const host = Object.values(hosts)[0];
    console.log(host ? `Connected to ioBroker host ${host.common?.name || "?"} (js-controller ${host.common?.installedVersion || "?"})` : "Connected to ioBroker");
    break;
  }
  case "states": print(lines(await http("GET", url("/states", `pattern=${enc(a[0] || "*")}`)))); break;
  case "get": print(await http("GET", url(`/get/${enc(a[0])}`))); break;
  case "value": print(String(await (await http("GET", url(`/getPlainValue/${enc(a[0])}`), { raw: true })).text())); break;
  case "set": {
    if (!a[0] || a[1] === undefined) fail("Usage: set <id> <value>");
    const q = `value=${enc(a.slice(1).join(" "))}${f.ack ? "&ack=true" : ""}`;
    print(await http("GET", url(`/set/${enc(a[0])}`, q)));
    break;
  }
  case "toggle": print(await http("GET", url(`/toggle/${enc(a[0])}`))); break;
  case "objects": {
    const q = `pattern=${enc(a[0] || "*")}${f.type ? `&type=${enc(f.type)}` : ""}`;
    const objs = await http("GET", url("/objects", q));
    print(Object.entries(objs).map(([id, o]) => `${id}  [${o.type}${o.common?.role ? ", " + o.common.role : ""}]  ${typeof o.common?.name === "object" ? o.common.name.en || o.common.name.de || "" : o.common?.name || ""}`).join("\n"));
    break;
  }
  case "search": {
    const term = a.join(" ").toLowerCase();
    if (!term) fail("Usage: search <text>");
    const objs = await http("GET", url("/objects", "pattern=*&type=state"));
    const name = (o) => typeof o.common?.name === "object" ? Object.values(o.common.name).join(" ") : String(o.common?.name || "");
    print(Object.entries(objs).filter(([id, o]) => id.toLowerCase().includes(term) || name(o).toLowerCase().includes(term)).slice(0, 200).map(([id, o]) => `${id}  ${name(o)}`).join("\n") || "Nothing found.");
    break;
  }
  case "rooms": case "functions": {
    const objs = await http("GET", url("/objects", `pattern=enum.${cmd}.*&type=enum`));
    print(Object.entries(objs).map(([id, o]) => `${id}: ${(o.common?.members || []).join(", ")}`).join("\n"));
    break;
  }
  default: console.log(`Usage:
  states [pattern]          e.g. states hm-rpc.0.*   (current values)
  get <id>                  state with its object
  value <id>                plain value
  set <id> <value> [--ack]  write a state, e.g. set hue.0.desk.on true
  toggle <id>               toggle a boolean state
  objects [pattern] [--type state|channel|device]
  search <text>             find states by id or name
  rooms | functions         list rooms / functions and their members`);
}
