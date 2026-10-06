// Spotify: control playback while coding. Needs your own app (developer.spotify.com) with the
// redirect URI http://127.0.0.1:8899/callback. Connect opens the browser once.
import { createServer } from "node:http";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const TOKENS = join(homedir(), ".config/pixel-code/tokens/spotify.json");
const REDIRECT = "http://127.0.0.1:8899/callback";
const basic = "Basic " + Buffer.from(`${settings.client_id}:${settings.client_secret}`).toString("base64");
const load = () => { try { return JSON.parse(readFileSync(TOKENS, "utf8")); } catch { return null; } };
const save = (t) => { mkdirSync(join(homedir(), ".config/pixel-code/tokens"), { recursive: true }); writeFileSync(TOKENS, JSON.stringify(t), { mode: 0o600 }); };
async function login() {
  const scope = "user-read-playback-state user-modify-playback-state user-read-currently-playing playlist-read-private";
  const url = `https://accounts.spotify.com/authorize?${new URLSearchParams({ client_id: settings.client_id, response_type: "code", redirect_uri: REDIRECT, scope })}`;
  const code = await new Promise((ok, bad) => {
    const srv = createServer((req, res) => { const q = new URL(req.url, REDIRECT).searchParams; res.end(q.get("code") ? "Connected to Pixel Code - you can close this tab." : "Failed: " + q.get("error")); srv.close(); q.get("code") ? ok(q.get("code")) : bad(new Error(q.get("error"))); });
    srv.listen(8899, "127.0.0.1", () => spawnSync("xdg-open", [url], { stdio: "ignore" }));
    setTimeout(() => { srv.close(); bad(new Error("timeout")); }, 300000);
  }).catch((e) => fail("Spotify sign-in failed: " + e.message));
  const t = await http("POST", "https://accounts.spotify.com/api/token", { headers: { Authorization: basic }, form: { grant_type: "authorization_code", code, redirect_uri: REDIRECT } });
  save({ ...t, at: Date.now() });
}
async function token() {
  let t = load();
  if (!t) fail("Not signed in - press Connect on the Spotify plugin.");
  if (Date.now() - t.at > (t.expires_in - 60) * 1000) { const n = await http("POST", "https://accounts.spotify.com/api/token", { headers: { Authorization: basic }, form: { grant_type: "refresh_token", refresh_token: t.refresh_token } }); t = { ...t, ...n, at: Date.now() }; save(t); }
  return { Authorization: `Bearer ${t.access_token}` };
}
const api = async (m, p, body) => { const r = await fetch("https://api.spotify.com/v1" + p, { method: m, headers: { ...(await token()), "Content-Type": "application/json" }, body: body ? JSON.stringify(body) : undefined }); if (r.status === 204) return null; const j = await r.json().catch(() => null); if (!r.ok) fail(`Spotify ${r.status}: ${j?.error?.message ?? ""}`); return j; };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.client_id || !settings.client_secret)) fail("Spotify client id and secret are required.");
switch (cmd) {
  case "connect": if (!load()) await login(); // fall through
  case "status": { const me = await api("GET", "/me"); console.log(`Connected as ${me.display_name}`); break; }
  case "now": { const p = await api("GET", "/me/player/currently-playing"); print(p?.item ? `${p.is_playing ? "▶" : "⏸"} ${p.item.name} - ${p.item.artists.map((a) => a.name).join(", ")}` : "Nothing playing"); break; }
  case "play": if (rest.length) { const t = (await api("GET", `/search?type=track,playlist&limit=1&q=${encodeURIComponent(rest.join(" "))}`)); const pl = t.playlists?.items?.find(Boolean); const tr = t.tracks?.items?.[0]; await api("PUT", "/me/player/play", rest[0] === "playlist" && pl ? { context_uri: pl.uri } : { uris: [tr.uri] }); console.log("Playing " + (tr?.name ?? pl?.name)); } else { await api("PUT", "/me/player/play"); console.log("Resumed."); } break;
  case "pause": await api("PUT", "/me/player/pause"); console.log("Paused."); break;
  case "next": await api("POST", "/me/player/next"); console.log("Skipped."); break;
  case "previous": await api("POST", "/me/player/previous"); console.log("Back."); break;
  case "volume": await api("PUT", `/me/player/volume?volume_percent=${rest[0]}`); console.log("Volume " + rest[0]); break;
  default: console.log("Usage:\n  now | play [song or 'playlist <name>'] | pause | next | previous | volume <0-100>   (needs Spotify Premium for control)");
}
