// Tripo3D: text/image to 3D model.
import { writeFileSync, mkdirSync, readFileSync } from "node:fs";
import { join, resolve, extname } from "node:path";
import { args, settings, fail, http, flags } from "./common.mjs";

const base = "https://api.tripo3d.ai/v2/openapi";
const headers = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No Tripo API key set.");

async function finish(task) {
  for (let i = 0; i < 300; i++) {
    const t = (await http("GET", `${base}/task/${task}`, { headers })).data;
    if (t.status === "success") {
      const dir = resolve(f.out || "tripo");
      mkdirSync(dir, { recursive: true });
      const url = t.output?.pbr_model || t.output?.model;
      const file = join(dir, `${task}.glb`);
      writeFileSync(file, Buffer.from(await (await http("GET", url, { raw: true })).arrayBuffer()));
      console.log(file);
      return;
    }
    if (["failed", "cancelled", "banned", "expired"].includes(t.status)) fail(`Tripo task ${t.status}`);
    await new Promise((r) => setTimeout(r, 4000));
  }
  fail("Tripo task timed out");
}

switch (cmd) {
  case "connect": case "status": { const b = (await http("GET", `${base}/user/balance`, { headers })).data; console.log(`Connected (${b.balance} credits)`); break; }
  case "text": await finish((await http("POST", `${base}/task`, { headers, body: { type: "text_to_model", prompt: w.join(" ") } })).data.task_id); break;
  case "image": {
    const buf = readFileSync(resolve(w[0]));
    const fd = new FormData();
    fd.append("file", new Blob([buf]), w[0]);
    const up = await fetch(`${base}/upload`, { method: "POST", headers, body: fd }).then((r) => r.json());
    if (up.code !== 0) fail(`Upload failed: ${JSON.stringify(up)}`);
    await finish((await http("POST", `${base}/task`, { headers, body: { type: "image_to_model", file: { type: extname(w[0]).slice(1) || "png", file_token: up.data.image_token } } })).data.task_id);
    break;
  }
  default: console.log(`Usage:
  text <prompt> [--out dir]        text to 3D model (.glb)
  image <file> [--out dir]         image to 3D model (.glb)`);
}
