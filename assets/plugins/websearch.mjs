// Local web search via SearXNG. Without a configured URL, Pixel Code runs its own SearXNG in Docker
// (container "pixel-code-searxng" on 127.0.0.1:8888) and starts it on demand.
import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { randomBytes } from "node:crypto";
import { args, settings, fail, flags } from "./common.mjs";

const UA = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";
const CONTAINER = "pixel-code-searxng";
const PORT = Number(settings.port || 8888);
const BASE = (settings.searxng || `http://127.0.0.1:${PORT}`).replace(/\/$/, "");
const MANAGED = !settings.searxng;

function curl(url, timeout = 20) {
  const r = spawnSync("curl", ["-sfL", "-m", String(timeout), "-A", UA, "-H", "Accept-Language: en-US,en;q=0.8", url], { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 });
  if (r.error || r.status !== 0) throw new Error(r.error?.message || `HTTP request failed (curl exit ${r.status})`);
  return r.stdout;
}

const docker = (...a) => spawnSync("docker", a, { encoding: "utf8" });
const up = () => { try { JSON.parse(curl(`${BASE}/search?format=json&q=ping`, 8)); return true; } catch { return false; } };

/** Starts the local SearXNG container (creating it and its config on first use). */
function ensure() {
  if (up()) return;
  if (!MANAGED) fail(`SearXNG at ${BASE} is not reachable.`);
  if (docker("version").status !== 0) fail("Docker is needed to run the local SearXNG (or set a SearXNG URL in Pixel Code > Settings > Plugins).");
  const dir = join(homedir(), ".local/share/pixel-code/searxng");
  if (!existsSync(join(dir, "settings.yml"))) {
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "settings.yml"), `use_default_settings: true
server:
  secret_key: "${randomBytes(24).toString("hex")}"
  limiter: false
  image_proxy: false
  bind_address: "0.0.0.0"
search:
  safe_search: 0
  formats: [html, json]
`);
  }
  const exists = docker("container", "inspect", CONTAINER).status === 0;
  const r = exists
    ? docker("start", CONTAINER)
    : docker("run", "-d", "--name", CONTAINER, "--restart", "unless-stopped", "-p", `127.0.0.1:${PORT}:8080`,
        "-v", `${dir}:/etc/searxng`, "searxng/searxng:latest");
  if (r.status !== 0) fail(`Starting SearXNG failed: ${(r.stderr || "").trim()}`);
  for (let i = 0; i < 90; i++) {
    if (up()) return;
    spawnSync("sleep", ["1"]);
  }
  fail("SearXNG did not become ready within 90s (docker logs pixel-code-searxng).");
}

function search(q, n, f) {
  const params = new URLSearchParams({ q, format: "json" });
  if (f.lang) params.set("language", f.lang);
  if (f.time) params.set("time_range", f.time);
  if (f.cat) params.set("categories", f.cat);
  const j = JSON.parse(curl(`${BASE}/search?${params}`));
  return (j.results || []).slice(0, n).map((r) => ({ title: r.title, url: r.url, snippet: r.content || "" }));
}

const decode = (s) => s.replace(/<[^>]+>/g, "").replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">")
  .replace(/&quot;/g, '"').replace(/&#x27;|&#39;/g, "'").replace(/&nbsp;/g, " ").replace(/[ \t]+/g, " ").trim();

function page(url, max) {
  let text = curl(url, 30);
  if (/<html|<body|<div/i.test(text.slice(0, 5000))) {
    text = text.replace(/<(script|style|noscript|svg|head)[\s\S]*?<\/\1>/gi, " ")
      .replace(/<\/(p|div|h\d|li|tr|section|article)>|<br\s*\/?>/gi, "\n");
    text = decode(text).replace(/\n\s*/g, "\n").replace(/\n{3,}/g, "\n\n");
  }
  return `${url}\n\n${text.slice(0, max)}${text.length > max ? `\n\n[truncated, ${text.length} chars total]` : ""}`;
}

const [cmd, ...rest] = args;
const [f, words] = flags(rest);
try {
  switch (cmd) {
    case "status":
      // Nur prüfen, nicht starten: der Container startet bei der ersten Suche.
      if (up()) console.log(MANAGED ? `Local SearXNG running on ${BASE}` : `SearXNG at ${BASE} works`);
      else if (!MANAGED) fail(`SearXNG at ${BASE} is not reachable.`);
      else if (docker("version").status !== 0) fail("Docker is needed for the local SearXNG.");
      else console.log("Ready - local SearXNG starts with the first search");
      break;
    case "connect":
    case "start": ensure(); console.log(`SearXNG running on ${BASE}`); break;
    case "stop": docker("stop", CONTAINER); console.log("SearXNG stopped."); break;
    case "search": {
      const q = words.join(" ");
      if (!q) fail("Usage: search <query> [--n 8] [--time day|week|month|year] [--lang de] [--cat news|it|science]");
      ensure();
      const r = search(q, Number(f.n || 8), f);
      console.log(r.map((x, i) => `${i + 1}. ${x.title}\n   ${x.url}\n   ${x.snippet}`).join("\n\n") || "No results.");
      break;
    }
    case "fetch": console.log(page(words[0], Number(f.max || 20000))); break;
    default:
      console.log(`Usage:
  search <query> [--n 8] [--time day|week|month|year] [--lang de] [--cat news|it|science]
                              search the web with the local SearXNG
  fetch <url> [--max 20000]   download a page as plain text
  start | stop                start or stop the local SearXNG container`);
  }
} catch (e) {
  fail(`Web search failed: ${e.message}`);
}
