// fal.ai: run image, video and 3D models (Flux, Kling, Hunyuan3D, ...) by model id.
import { writeFileSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const headers = { Authorization: `Key ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No fal.ai API key set.");
const urls = (o) => JSON.stringify(o).match(/https:\/\/[^"]+\.(png|jpe?g|webp|mp4|glb|obj|wav|mp3)/g) || [];
switch (cmd) {
  case "connect": case "status": { const r = await fetch("https://queue.fal.run/fal-ai/flux/schnell/requests/x/status", { headers }); if (r.status === 401 || r.status === 403) fail("fal.ai rejected the key"); console.log("fal.ai key accepted"); break; }
  case "run": {
    const model = w[0];
    const input = f.json ? JSON.parse(f.json) : { prompt: w.slice(1).join(" ") };
    const q = await http("POST", `https://queue.fal.run/${model}`, { headers, body: input });
    let s;
    for (let i = 0; i < 600; i++) { s = await http("GET", q.status_url, { headers }); if (s.status === "COMPLETED") break; await new Promise((r) => setTimeout(r, 2000)); }
    const res = await http("GET", q.response_url, { headers });
    const dir = resolve(f.out || "fal"); mkdirSync(dir, { recursive: true });
    for (const [i, u] of urls(res).entries()) { const file = join(dir, `${q.request_id}-${i}.${u.split(".").pop()}`); writeFileSync(file, Buffer.from(await (await fetch(u)).arrayBuffer())); console.log(file); }
    if (!urls(res).length) print(res);
    break;
  }
  default: console.log(`Usage:
  run <model> <prompt> [--out dir]            e.g. run fal-ai/flux/dev "pixel art sword, transparent background"
  run <model> --json '{"prompt":"..","image_url":".."}'   any input (see fal.ai/models)`);
}
