// Leonardo.ai image generation (REST API v1).
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, flags } from "./common.mjs";

const API = "https://cloud.leonardo.ai/api/rest/v1";
const key = settings.api_key;

async function call(path, body) {
  const res = await fetch(API + path, {
    method: body ? "POST" : "GET",
    headers: { Authorization: `Bearer ${key}`, "Content-Type": "application/json", Accept: "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const j = await res.json().catch(() => ({}));
  if (!res.ok) fail(`Leonardo API ${res.status}: ${j.error || j.message || JSON.stringify(j)}`);
  return j;
}

const [cmd, ...rest] = args;
if (!key && cmd !== undefined && cmd !== "help") fail("No Leonardo.ai API key. Add it in Pixel Code > Settings > Plugins.");
const [f, words] = flags(rest);

switch (cmd) {
  case "connect":
  case "status": {
    const me = await call("/me");
    const u = me.user_details?.[0];
    console.log(`Connected as ${u?.user?.username ?? "?"} (API credits: ${u?.apiSubscriptionTokens ?? u?.apiPaidTokens ?? "?"})`);
    break;
  }
  case "models": {
    const j = await call("/platformModels");
    console.log((j.custom_models || []).map((m) => `${m.id}  ${m.name}`).join("\n"));
    break;
  }
  case "generate": {
    const prompt = words.join(" ");
    if (!prompt) fail("Usage: generate <prompt> [--width 1024] [--height 1024] [--num 1] [--model <id>] [--negative <text>] [--out dir]");
    const body = {
      prompt,
      width: Number(f.width || 1024),
      height: Number(f.height || 1024),
      num_images: Number(f.num || 1),
      ...(f.model ? { modelId: f.model } : {}),
      ...(f.negative ? { negative_prompt: f.negative } : {}),
    };
    const id = (await call("/generations", body)).sdGenerationJob?.generationId;
    if (!id) fail("Leonardo did not return a generation id");
    let gen;
    for (let i = 0; i < 90; i++) {
      await new Promise((r) => setTimeout(r, 2000));
      gen = (await call(`/generations/${id}`)).generations_by_pk;
      if (gen?.status === "COMPLETE" || gen?.status === "FAILED") break;
    }
    if (gen?.status !== "COMPLETE") fail(`Generation ${id} status: ${gen?.status ?? "timeout"}`);
    const dir = resolve(f.out || "leonardo");
    mkdirSync(dir, { recursive: true });
    for (const [i, img] of gen.generated_images.entries()) {
      const file = join(dir, `${id}-${i + 1}.${img.url.split(".").pop().split("?")[0] || "png"}`);
      writeFileSync(file, Buffer.from(await (await fetch(img.url)).arrayBuffer()));
      console.log(`${file}\n  ${img.url}`);
    }
    break;
  }
  case "raw": console.log(JSON.stringify(await call(words[0], words[1] ? JSON.parse(words.slice(1).join(" ")) : undefined), null, 2)); break;
  default:
    console.log(`Usage:
  generate <prompt> [--width 1024 --height 1024 --num 1 --model <id> --negative <text> --out dir]
                       generate images and save them (default folder ./leonardo)
  models               list platform models (ids for --model)
  status               account and credits
  raw <path> [json]    call any API endpoint, e.g. raw /me`);
}
