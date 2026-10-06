// Stability AI: image generation, upscaling and background removal.
import { writeFileSync, readFileSync } from "node:fs";
import { resolve, basename } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.stability.ai";
const headers = { Authorization: `Bearer ${settings.api_key || ""}`, Accept: "image/*" };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No Stability API key set.");
async function form(path, fields, out) {
  const fd = new FormData();
  for (const [k, v] of Object.entries(fields)) if (v !== undefined) fd.append(k, v);
  const r = await fetch(base + path, { method: "POST", headers, body: fd });
  if (!r.ok) fail(`Stability ${r.status}: ${await r.text()}`);
  writeFileSync(resolve(out), Buffer.from(await r.arrayBuffer()));
  console.log(resolve(out));
}
const img = (p) => new Blob([readFileSync(resolve(p))]);
switch (cmd) {
  case "connect": case "status": { const r = await http("GET", `${base}/v1/user/balance`, { headers: { Authorization: headers.Authorization } }); console.log(`Connected (${r.credits} credits)`); break; }
  case "generate": await form(`/v2beta/stable-image/generate/${f.model || "core"}`, { prompt: w.join(" "), aspect_ratio: f.ratio || "1:1", output_format: "png", style_preset: f.style, negative_prompt: f.negative }, f.out || "stability.png"); break;
  case "upscale": await form("/v2beta/stable-image/upscale/fast", { image: img(w[0]), output_format: "png" }, f.out || "upscaled-" + basename(w[0])); break;
  case "remove-bg": await form("/v2beta/stable-image/edit/remove-background", { image: img(w[0]), output_format: "png" }, f.out || "nobg-" + basename(w[0]).replace(/\.[^.]+$/, ".png")); break;
  default: console.log(`Usage:
  generate <prompt> [--model core|ultra] [--ratio 16:9] [--style pixel-art] [--out file.png]
  upscale <image> [--out file] | remove-bg <image> [--out file]`);
}
