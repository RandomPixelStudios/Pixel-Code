// Hangar (PaperMC): upload plugin versions for Paper, Velocity and Waterfall.
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://hangar.papermc.io/api/v1";
async function jwt() {
  const r = await http("POST", `${base}/authenticate?apiKey=${encodeURIComponent(settings.api_key || "")}`);
  return { Authorization: r.token };
}
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No Hangar API key set.");
switch (cmd) {
  case "connect": case "status": { await jwt(); console.log("Connected to Hangar"); break; }
  case "projects": print((await http("GET", `${base}/projects?owner=${encodeURIComponent(w[0] || settings.owner || "")}&limit=25`, { headers: await jwt() })).result.map((p) => `${p.namespace.owner}/${p.namespace.slug}  ${p.stats.downloads} downloads`).join("\n")); break;
  case "versions": print((await http("GET", `${base}/projects/${w[0]}/versions?limit=10`, { headers: await jwt() })).result.map((v) => `${v.name}  ${v.channel.name}  ${v.createdAt.slice(0, 10)}`).join("\n")); break;
  case "upload": {
    const [project, file] = w;
    if (!project || !file || !f.version) fail("Usage: upload <project> <file.jar> --version 1.2.0 --platform PAPER --mc 1.21,1.21.1 [--channel Release] [--description ..]");
    const platform = (f.platform || "PAPER").toUpperCase();
    const data = {
      version: f.version, channel: f.channel || "Release", description: f.description || "",
      files: [{ platforms: [platform] }],
      platformDependencies: { [platform]: String(f.mc || "").split(",").filter(Boolean) },
      pluginDependencies: {},
    };
    const fd = new FormData();
    fd.append("versionUpload", new Blob([JSON.stringify(data)], { type: "application/json" }));
    fd.append("files", new Blob([readFileSync(resolve(file))]), basename(file));
    const r = await fetch(`${base}/projects/${project}/upload`, { method: "POST", headers: await jwt(), body: fd });
    if (!r.ok) fail(`Hangar ${r.status}: ${await r.text()}`);
    console.log("Uploaded:", (await r.json()).url ?? "ok");
    break;
  }
  default: console.log(`Usage:
  projects [owner] | versions <project> | upload <project> <file.jar> --version 1.2.0 --platform PAPER|VELOCITY|WATERFALL --mc 1.21.1 [--channel Release]`);
}
