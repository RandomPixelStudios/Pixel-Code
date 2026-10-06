// Scenario.gg: game assets in your own trained style.
import { writeFileSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.cloud.scenario.com/v1";
const headers = { Authorization: "Basic " + Buffer.from(`${settings.api_key || ""}:${settings.api_secret || ""}`).toString("base64") };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!settings.api_key || !settings.api_secret)) fail("Scenario API key and secret are required.");
switch (cmd) {
  case "connect": case "status": { const m = await http("GET", `${base}/models?pageSize=1`, { headers }); console.log("Connected to Scenario"); break; }
  case "models": print((await http("GET", `${base}/models?pageSize=50`, { headers })).models.map((m) => `${m.id}  ${m.name}  ${m.status}`).join("\n")); break;
  case "generate": {
    const job = await http("POST", `${base}/generate/txt2img`, { headers, body: { modelId: f.model || w[0], prompt: w.slice(f.model ? 0 : 1).join(" "), numSamples: Number(f.num || 1), width: Number(f.width || 1024), height: Number(f.height || 1024) } });
    const id = job.job?.jobId || job.inference?.id;
    let j;
    for (let i = 0; i < 200; i++) { await new Promise((r) => setTimeout(r, 3000)); j = (await http("GET", `${base}/jobs/${id}`, { headers })).job; if (j.status === "success" || j.status === "failure") break; }
    if (j.status !== "success") fail("Generation failed");
    const dir = resolve(f.out || "scenario"); mkdirSync(dir, { recursive: true });
    for (const a of j.metadata?.assetIds || []) { const asset = (await http("GET", `${base}/assets/${a}`, { headers })).asset; const file = join(dir, a + ".png"); writeFileSync(file, Buffer.from(await (await fetch(asset.url)).arrayBuffer())); console.log(file); }
    break;
  }
  default: console.log(`Usage:
  models | generate --model <id> <prompt> [--num 4] [--width 1024 --height 1024] [--out dir]`);
}
