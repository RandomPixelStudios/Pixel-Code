// CurseForge: upload Minecraft mod files (upload API token from your CurseForge account).
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = "https://minecraft.curseforge.com/api";
const headers = { "X-Api-Token": settings.token || "" };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.token) fail("No CurseForge API token set.");
const list = (x) => (x ? String(x).split(",").map((s) => s.trim()).filter(Boolean) : []);

switch (cmd) {
  case "connect": case "status": { const v = await http("GET", `${base}/game/versions`, { headers }); console.log(`Token valid (${v.length} game versions available)`); break; }
  case "game-versions": {
    const types = await http("GET", `${base}/game/version-types`, { headers });
    const v = await http("GET", `${base}/game/versions`, { headers });
    const t = Object.fromEntries(types.map((x) => [x.id, x.name]));
    print(v.filter((x) => !w[0] || x.name.toLowerCase().includes(w[0].toLowerCase())).map((x) => `${x.id}  ${x.name}  (${t[x.gameVersionTypeID] ?? x.gameVersionTypeID})`).join("\n"));
    break;
  }
  case "upload": {
    const [project, file] = w;
    if (!project || !file) fail("Usage: upload <project_id> <file.jar> --game-versions 1.21.1 --loaders Fabric [--java 21] [--name ..] [--changelog ..] [--type release|beta|alpha] [--deps slug:requiredDependency]");
    const all = await http("GET", `${base}/game/versions`, { headers });
    const wanted = [...list(f["game-versions"]), ...list(f.loaders), ...(f.java ? [`Java ${f.java}`] : []), ...list(f.env)];
    const ids = wanted.map((n) => {
      const hits = all.filter((v) => v.name.toLowerCase() === n.toLowerCase());
      if (!hits.length) fail(`Unknown CurseForge game version "${n}" (see game-versions)`);
      return hits.map((h) => h.id);
    }).flat();
    const metadata = {
      changelog: f.changelog || "", changelogType: "markdown", displayName: f.name || basename(file),
      gameVersions: [...new Set(ids)], releaseType: f.type || "release",
      ...(f.deps ? { relations: { projects: list(f.deps).map((d) => { const [slug, type] = d.split(":"); return { slug, type: type || "requiredDependency" }; }) } } : {}),
    };
    const fd = new FormData();
    fd.append("metadata", JSON.stringify(metadata));
    fd.append("file", new Blob([readFileSync(resolve(file))]), basename(file));
    const res = await fetch(`${base}/projects/${project}/upload-file`, { method: "POST", headers, body: fd });
    const j = await res.json().catch(() => ({}));
    if (!res.ok) fail(`CurseForge ${res.status}: ${j.errorMessage || JSON.stringify(j)}`);
    console.log(`Uploaded file id ${j.id} (it appears after CurseForge's review)`);
    break;
  }
  default: console.log(`Usage:
  game-versions [filter]     names/ids of Minecraft versions, loaders (Fabric, NeoForge, Forge, Quilt) and Java versions
  upload <project_id> <file.jar> --game-versions 1.21.1,1.21 --loaders Fabric [--java 21] [--env Client,Server]
         [--name ..] [--changelog "markdown"] [--type release|beta|alpha] [--deps fabric-api:requiredDependency]`);
}
