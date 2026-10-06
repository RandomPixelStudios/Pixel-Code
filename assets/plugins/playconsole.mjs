// Google Play Console (Android Publisher API) with a service account JSON key.
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { createSign } from "node:crypto";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const API = "https://androidpublisher.googleapis.com/androidpublisher/v3/applications";
const UP = "https://androidpublisher.googleapis.com/upload/androidpublisher/v3/applications";
let key;
try { key = JSON.parse(readFileSync((settings.key_file || "").replace(/^~/, process.env.HOME), "utf8")); } catch {}
async function token() {
  if (!key) fail("Service account key file not found.");
  const b64 = (o) => Buffer.from(JSON.stringify(o)).toString("base64url");
  const now = Math.floor(Date.now() / 1000);
  const body = `${b64({ alg: "RS256", typ: "JWT" })}.${b64({ iss: key.client_email, scope: "https://www.googleapis.com/auth/androidpublisher", aud: key.token_uri, iat: now, exp: now + 3600 })}`;
  const jwt = body + "." + createSign("RSA-SHA256").update(body).sign(key.private_key, "base64url");
  return { Authorization: `Bearer ${(await http("POST", key.token_uri, { form: { grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer", assertion: jwt } })).access_token}` };
}
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
const pkg = f.package || settings.package;
async function edit(fn) {
  const h = await token();
  const e = await http("POST", `${API}/${pkg}/edits`, { headers: h, body: {} });
  const out = await fn(h, `${API}/${pkg}/edits/${e.id}`, e.id);
  await http("POST", `${API}/${pkg}/edits/${e.id}:commit`, { headers: h });
  return out;
}
switch (cmd) {
  case "connect": case "status": { await token(); console.log(`Service account ${key.client_email} works${pkg ? " (app " + pkg + ")" : ""}`); break; }
  case "tracks": { const h = await token(); const e = await http("POST", `${API}/${pkg}/edits`, { headers: h, body: {} }); print((await http("GET", `${API}/${pkg}/edits/${e.id}/tracks`, { headers: h })).tracks.map((t) => `${t.track}: ${(t.releases || []).map((r) => `${r.name ?? ""} [${(r.versionCodes || []).join(",")}] ${r.status}`).join("; ")}`).join("\n")); break; }
  case "upload": {
    const file = resolve(w[0]); const track = f.track || "internal";
    const r = await edit(async (h, base, id) => {
      const b = await http("POST", `${UP}/${pkg}/edits/${id}/bundles?uploadType=media`, { headers: { ...h, "Content-Type": "application/octet-stream" }, body: readFileSync(file) });
      await http("PUT", `${base}/tracks/${track}`, { headers: h, body: { track, releases: [{ versionCodes: [String(b.versionCode)], status: f.status || "completed", ...(f.name ? { name: f.name } : {}), ...(f.notes ? { releaseNotes: [{ language: f.lang || "en-US", text: f.notes }] } : {}) }] } });
      return b.versionCode;
    });
    console.log(`Uploaded version code ${r} to the ${track} track`);
    break;
  }
  case "reviews": { const h = await token(); print(((await http("GET", `${API}/${pkg}/reviews?maxResults=30`, { headers: h })).reviews || []).map((r) => { const c = r.comments[0].userComment; return `${"★".repeat(c.starRating)}  ${r.authorName}: ${c.text.trim()}`; }).join("\n\n") || "No recent reviews."); break; }
  case "reply": { const h = await token(); await http("POST", `${API}/${pkg}/reviews/${w[0]}:reply`, { headers: h, body: { replyText: w.slice(1).join(" ") } }); console.log("Replied."); break; }
  default: console.log(`Usage (app from settings or --package com.example.app):
  tracks
  upload <app-release.aab> [--track internal|alpha|beta|production] [--status completed|draft] [--notes "What's new"]
  reviews | reply <review_id> <text>`);
}
