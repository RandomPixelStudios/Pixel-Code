// RSS / Atom feeds: watch changelogs (Minecraft, Fabric, mods, blogs).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const feeds = (settings.feeds || "").split(/[\s,]+/).filter(Boolean);
const tag = (s, t) => (s.match(new RegExp(`<${t}[^>]*>([\\s\\S]*?)</${t}>`)) || [])[1]?.replace(/<!\[CDATA\[|\]\]>/g, "").replace(/<[^>]+>/g, "").trim() || "";
async function read(url, n) {
  const b = new Uint8Array(await (await fetch(url, { headers: { "User-Agent": "PixelCode" } })).arrayBuffer());
  const utf16 = (b[0] === 0xff && b[1] === 0xfe) || (b[1] === 0 && b[3] === 0);
  const x = new TextDecoder(utf16 ? "utf-16le" : "utf-8").decode(b);
  const items = x.match(/<(item|entry)[\s>][\s\S]*?<\/(item|entry)>/g) || [];
  return items.slice(0, n).map((it) => ({ title: tag(it, "title"), date: tag(it, "pubDate") || tag(it, "updated") || tag(it, "published"), link: tag(it, "link") || (it.match(/<link[^>]*href="([^"]+)"/) || [])[1] || "" }));
}
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": case "status": if (!feeds.length) fail("Add feed URLs."); await read(feeds[0], 1); console.log(`${feeds.length} feeds`); break;
  case "latest": for (const f of rest.length ? rest : feeds) { console.log(`## ${f}`); for (const i of await read(f, 5)) console.log(`  ${i.date.slice(0, 16)}  ${i.title}\n    ${i.link}`); } break;
  default: console.log("Usage:\n  latest [feed url...]   newest 5 entries of each feed");
}
