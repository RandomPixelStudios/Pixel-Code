// Blender via the Blender MCP add-on (socket server inside Blender, default localhost:9876).
import { connect } from "node:net";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail } from "./common.mjs";

const host = settings.host || "localhost";
const port = Number(settings.port || 9876);

function send(type, params = {}) {
  return new Promise((ok, bad) => {
    const sock = connect({ host, port }, () => sock.write(JSON.stringify({ type, params })));
    let buf = "";
    const timer = setTimeout(() => { sock.destroy(); bad(new Error("Blender did not answer within 180s")); }, 180000);
    sock.on("data", (d) => {
      buf += d;
      try {
        const res = JSON.parse(buf);
        clearTimeout(timer);
        sock.end();
        ok(res);
      } catch {}
    });
    sock.on("error", (e) => { clearTimeout(timer); bad(e); });
  });
}

const show = (res) => {
  if (res.status === "error") fail(`Blender error: ${res.message}`);
  const r = res.result ?? res;
  console.log(typeof r === "string" ? r : JSON.stringify(r, null, 2));
};

const [cmd, ...rest] = args;
try {
  switch (cmd) {
    case "connect":
    case "status": {
      const r = await send("get_scene_info");
      console.log(`Connected to Blender on ${host}:${port} (scene: ${r.result?.name ?? "?"})`);
      break;
    }
    case "scene": show(await send("get_scene_info")); break;
    case "object": show(await send("get_object_info", { name: rest.join(" ") })); break;
    case "exec": show(await send("execute_code", { code: rest.join(" ") })); break;
    case "exec-file": show(await send("execute_code", { code: readFileSync(resolve(rest[0]), "utf8") })); break;
    case "screenshot": {
      const file = resolve(rest[0] || "blender-viewport.png");
      show(await send("get_viewport_screenshot", { max_size: Number(rest[1] || 1200), filepath: file, format: "png" }));
      console.log(`Saved to ${file}`);
      break;
    }
    case "raw": show(await send(rest[0], rest[1] ? JSON.parse(rest.slice(1).join(" ")) : {})); break;
    default:
      console.log(`Usage:
  scene                     scene overview (objects, materials)
  object <name>             details of one object
  exec <python>             run Python (bpy) inside Blender
  exec-file <file.py>       run a Python file inside Blender
  screenshot [file] [size]  save a viewport screenshot
  raw <command> [json]      send any Blender MCP command, e.g. raw search_polyhaven_assets '{"asset_type":"hdris"}'`);
  }
} catch (e) {
  fail(`Cannot reach Blender on ${host}:${port} (${e.message}). Open Blender, enable the "Blender MCP" add-on and click "Connect to MCP server" in the sidebar (N > BlenderMCP).`);
}
