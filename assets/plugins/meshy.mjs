// Meshy: text/image to 3D model (GLB/FBX/OBJ), downloaded into the project.
import { writeFileSync, mkdirSync, readFileSync } from "node:fs";
import { join, resolve, extname } from "node:path";
import { args, settings, fail, http, flags } from "./common.mjs";

const base = "https://api.meshy.ai/openapi";
const headers = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No Meshy API key set.");

async function wait(path) {
  for (let i = 0; i < 300; i++) {
    const t = await http("GET", base + path, { headers });
    if (t.status === "SUCCEEDED") return t;
    if (t.status === "FAILED" || t.status === "CANCELED") fail(`Meshy task ${t.status}: ${t.task_error?.message ?? ""}`);
    await new Promise((r) => setTimeout(r, 5000));
  }
  fail("Meshy task timed out");
}
async function download(t) {
  const dir = resolve(f.out || "meshy");
  mkdirSync(dir, { recursive: true });
  for (const [fmt, url] of Object.entries(t.model_urls || {})) {
    if (!url || (f.format && fmt !== f.format)) continue;
    const file = join(dir, `${t.id}.${fmt}`);
    writeFileSync(file, Buffer.from(await (await http("GET", url, { raw: true })).arrayBuffer()));
    console.log(file);
  }
}

switch (cmd) {
  case "connect": case "status": { const b = await http("GET", `${base}/v1/balance`, { headers }); console.log(`Connected (${b.balance} credits)`); break; }
  case "text": {
    const preview = await http("POST", `${base}/v2/text-to-3d`, { headers, body: { mode: "preview", prompt: w.join(" "), art_style: f.style || "realistic" } });
    const p = await wait(`/v2/text-to-3d/${preview.result}`);
    const refine = await http("POST", `${base}/v2/text-to-3d`, { headers, body: { mode: "refine", preview_task_id: p.id } });
    await download(await wait(`/v2/text-to-3d/${refine.result}`));
    break;
  }
  case "image": {
    const img = w[0].startsWith("http") ? w[0] : `data:image/${extname(w[0]).slice(1) || "png"};base64,${readFileSync(resolve(w[0])).toString("base64")}`;
    const t = await http("POST", `${base}/v1/image-to-3d`, { headers, body: { image_url: img, enable_pbr: true } });
    await download(await wait(`/v1/image-to-3d/${t.result}`));
    break;
  }
  default: console.log(`Usage:
  text <prompt> [--style realistic|sculpture] [--format glb|fbx|obj] [--out dir]   text to textured 3D model
  image <file-or-url> [--format glb] [--out dir]                                     image to 3D model`);
}
