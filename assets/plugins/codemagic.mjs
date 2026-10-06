// Codemagic: cloud builds for iOS and Android (on Macs, no Mac needed locally).
import { writeFileSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = "https://api.codemagic.io";
const headers = { "x-auth-token": settings.token || "" };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.token) fail("No Codemagic API token set.");
switch (cmd) {
  case "connect": case "status": { const a = await http("GET", base + "/apps", { headers }); console.log(`Connected (${a.applications.length} apps)`); break; }
  case "apps": print((await http("GET", base + "/apps", { headers })).applications.map((a) => `${a._id}  ${a.appName}  workflows: ${Object.keys(a.workflows || {}).join(", ")}`).join("\n")); break;
  case "build": { const b = await http("POST", base + "/builds", { headers, body: { appId: w[0], workflowId: w[1], branch: f.branch || "main" } }); console.log("Build started: " + b.buildId); if (!f.wait) break; let s; do { await new Promise((r) => setTimeout(r, 15000)); s = (await http("GET", `${base}/builds/${b.buildId}`, { headers })).build; console.log(s.status); } while (!["finished", "failed", "canceled", "timeout"].includes(s.status)); break; }
  case "builds": print((await http("GET", `${base}/builds?appId=${w[0]}`, { headers })).builds.slice(0, 15).map((b) => `${b._id}  ${b.status}  ${b.workflowId}  ${b.branch}  ${b.startedAt?.slice(0, 16) ?? ""}`).join("\n")); break;
  case "artifacts": { const b = (await http("GET", `${base}/builds/${w[0]}`, { headers })).build; const dir = resolve(f.out || "codemagic"); mkdirSync(dir, { recursive: true }); for (const a of b.artefacts || []) { const file = join(dir, a.name); writeFileSync(file, Buffer.from(await (await fetch(a.url, { headers })).arrayBuffer())); console.log(file); } break; }
  case "cancel": await http("POST", `${base}/builds/${w[0]}/cancel`, { headers }); console.log("Canceled."); break;
  default: console.log(`Usage:
  apps | build <app_id> <workflow> [--branch main] [--wait] | builds <app_id> | artifacts <build_id> [--out dir] | cancel <build_id>`);
}
