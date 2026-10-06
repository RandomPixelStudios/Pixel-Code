// Hyper3D Rodin: high-quality text/image to 3D.
import { writeFileSync, mkdirSync, readFileSync } from "node:fs";
import { join, resolve, basename } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://hyperhuman.deemos.com/api/v2";
const headers = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No Rodin API key set.");
async function run(fd) {
  fd.append("geometry_file_format", f.format || "glb");
  fd.append("quality", f.quality || "medium");
  const r = await fetch(`${base}/rodin`, { method: "POST", headers, body: fd }).then((x) => x.json());
  if (!r.uuid) fail("Rodin: " + JSON.stringify(r));
  for (let i = 0; i < 300; i++) {
    await new Promise((x) => setTimeout(x, 5000));
    const s = await http("POST", `${base}/status`, { headers, body: { subscription_key: r.jobs.subscription_key } });
    if (s.jobs.every((j) => j.status === "Done")) break;
    if (s.jobs.some((j) => j.status === "Failed")) fail("Rodin job failed");
  }
  const d = await http("POST", `${base}/download`, { headers, body: { task_uuid: r.uuid } });
  const dir = resolve(f.out || "rodin"); mkdirSync(dir, { recursive: true });
  for (const it of d.list) { const file = join(dir, it.name); writeFileSync(file, Buffer.from(await (await fetch(it.url)).arrayBuffer())); console.log(file); }
}
switch (cmd) {
  case "connect": case "status": { const r = await http("POST", `${base}/check_balance`, { headers }); console.log(`Connected (balance ${r.balance})`); break; }
  case "text": { const fd = new FormData(); fd.append("prompt", w.join(" ")); await run(fd); break; }
  case "image": { const fd = new FormData(); for (const p of w) fd.append("images", new Blob([readFileSync(resolve(p))]), basename(p)); if (f.prompt) fd.append("prompt", f.prompt); await run(fd); break; }
  default: console.log(`Usage:
  text <prompt> | image <img...> [--prompt ..]    [--format glb|fbx|obj|usdz] [--quality low|medium|high] [--out dir]`);
}
