// ComfyUI (local Stable Diffusion / Flux server): run workflows and download the images.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = (settings.url || "http://127.0.0.1:8188").replace(/\/$/, "");
const [cmd, ...rest] = args;
const [f, w] = flags(rest);

async function run(workflow) {
  const { prompt_id } = await http("POST", `${base}/prompt`, { body: { prompt: workflow } });
  for (let i = 0; i < 900; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    const h = (await http("GET", `${base}/history/${prompt_id}`))[prompt_id];
    if (!h) continue;
    if (h.status?.status_str === "error") fail(`ComfyUI error: ${JSON.stringify(h.status.messages).slice(0, 800)}`);
    const dir = resolve(f.out || "comfyui");
    mkdirSync(dir, { recursive: true });
    for (const node of Object.values(h.outputs || {})) for (const img of [...(node.images || []), ...(node.gifs || [])]) {
      const res = await http("GET", `${base}/view?${new URLSearchParams({ filename: img.filename, subfolder: img.subfolder, type: img.type })}`, { raw: true });
      const file = join(dir, img.filename);
      writeFileSync(file, Buffer.from(await res.arrayBuffer()));
      console.log(file);
    }
    return;
  }
  fail("ComfyUI did not finish within 15 minutes");
}

switch (cmd) {
  case "connect": case "status": { const s = await http("GET", `${base}/system_stats`); console.log(`ComfyUI ${s.system?.comfyui_version ?? ""} on ${base} (${s.devices?.[0]?.name ?? "?"})`); break; }
  case "run": {
    let wf = JSON.parse(readFileSync(resolve(w[0]), "utf8"));
    // Optional: Text des ersten positiven Prompts ersetzen
    if (f.prompt) { const n = Object.values(wf).find((x) => x.class_type === "CLIPTextEncode"); if (n) n.inputs.text = f.prompt; }
    if (f.seed) for (const n of Object.values(wf)) if (n.inputs && "seed" in n.inputs) n.inputs.seed = Number(f.seed);
    await run(wf);
    break;
  }
  case "models": print(await http("GET", `${base}/object_info/CheckpointLoaderSimple`).then((o) => o.CheckpointLoaderSimple.input.required.ckpt_name[0].join("\n"))); break;
  case "queue": print(await http("GET", `${base}/queue`)); break;
  default: console.log(`Usage:
  run <workflow_api.json> [--prompt "text"] [--seed 42] [--out dir]
        run a workflow (export it in ComfyUI with "Save (API Format)") and save the images
  models | queue`);
}
