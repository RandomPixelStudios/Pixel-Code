// X (Twitter): post with the API v2 (OAuth 1.0a user keys from developer.x.com).
import { createHmac, randomBytes } from "node:crypto";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const s = settings;
const enc = (v) => encodeURIComponent(v).replace(/[!'()*]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());
function auth(method, url) {
  const o = { oauth_consumer_key: s.api_key, oauth_nonce: randomBytes(16).toString("hex"), oauth_signature_method: "HMAC-SHA1", oauth_timestamp: Math.floor(Date.now() / 1000), oauth_token: s.access_token, oauth_version: "1.0" };
  const params = Object.keys(o).sort().map((k) => `${enc(k)}=${enc(o[k])}`).join("&");
  const base = [method, enc(url), enc(params)].join("&");
  o.oauth_signature = createHmac("sha1", `${enc(s.api_secret)}&${enc(s.access_secret)}`).update(base).digest("base64");
  return "OAuth " + Object.keys(o).map((k) => `${enc(k)}="${enc(o[k])}"`).join(", ");
}
const call = (m, url, body) => http(m, url, { headers: { Authorization: auth(m, url) }, body });
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !(s.api_key && s.api_secret && s.access_token && s.access_secret)) fail("API key/secret and access token/secret are required.");
switch (cmd) {
  case "connect": case "status": { const r = await call("GET", "https://api.x.com/2/users/me"); console.log(`Connected as @${r.data.username}`); break; }
  case "post": { const r = await call("POST", "https://api.x.com/2/tweets", { text: rest.join(" ") }); console.log(`Posted: https://x.com/i/status/${r.data.id}`); break; }
  case "delete": { await call("DELETE", `https://api.x.com/2/tweets/${rest[0]}`); console.log("Deleted."); break; }
  default: console.log(`Usage:
  post <text>      (free API tier: posting only)
  delete <id>`);
}
