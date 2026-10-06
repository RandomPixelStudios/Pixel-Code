// Bluesky: post devlogs (with images) and read your timeline. Uses an app password.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const pds = (settings.pds || "https://bsky.social").replace(/\/$/, "");
async function session() { return http("POST", `${pds}/xrpc/com.atproto.server.createSession`, { body: { identifier: settings.handle, password: settings.app_password } }); }
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!settings.handle || !settings.app_password)) fail("Handle and app password are required.");
switch (cmd) {
  case "connect": case "status": { const s = await session(); console.log(`Connected as @${s.handle}`); break; }
  case "post": {
    const s = await session(); const headers = { Authorization: `Bearer ${s.accessJwt}` };
    const text = w.join(" ");
    const facets = [...text.matchAll(/https?:\/\/\S+/g)].map((m) => ({ index: { byteStart: Buffer.byteLength(text.slice(0, m.index)), byteEnd: Buffer.byteLength(text.slice(0, m.index + m[0].length)) }, features: [{ $type: "app.bsky.richtext.facet#link", uri: m[0] }] }));
    const record = { $type: "app.bsky.feed.post", text, createdAt: new Date().toISOString(), ...(facets.length ? { facets } : {}) };
    if (f.image) {
      const images = [];
      for (const p of String(f.image).split(",")) { const b = await http("POST", `${pds}/xrpc/com.atproto.repo.uploadBlob`, { headers: { ...headers, "Content-Type": p.endsWith(".png") ? "image/png" : "image/jpeg" }, body: readFileSync(resolve(p)) }); images.push({ alt: f.alt || "", image: b.blob }); }
      record.embed = { $type: "app.bsky.embed.images", images };
    }
    const r = await http("POST", `${pds}/xrpc/com.atproto.repo.createRecord`, { headers, body: { repo: s.did, collection: "app.bsky.feed.post", record } });
    console.log(`Posted: https://bsky.app/profile/${s.handle}/post/${r.uri.split("/").pop()}`);
    break;
  }
  case "timeline": { const s = await session(); print((await http("GET", `${pds}/xrpc/app.bsky.feed.getTimeline?limit=${f.n || 20}`, { headers: { Authorization: `Bearer ${s.accessJwt}` } })).feed.map(({ post: p }) => `@${p.author.handle}: ${p.record.text}  (♥ ${p.likeCount})`).join("\n\n")); break; }
  default: console.log(`Usage:
  post <text> [--image a.png,b.png] [--alt "description"]   (max 300 characters, 4 images)
  timeline [--n 20]`);
}
