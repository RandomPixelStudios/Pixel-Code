// CurseForge: upload Minecraft mod files (upload API token from your CurseForge account).
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = "https://minecraft.curseforge.com/api";
const headers = { "X-Api-Token": settings.token || "" };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
const CORE = "https://api.curseforge.com/v1";
const core = (path) => {
  if (!settings.api_key) fail("Listing projects needs a CurseForge Core API key (console.curseforge.com > API keys) in the plugin settings.");
  return http("GET", CORE + path, { headers: { "x-api-key": settings.api_key } });
};
// Autor-ID aus dem Nutzernamen: Suche findet Mods auch über den Autorennamen
async function authorId(name) {
  const r = await core(`/mods/search?gameId=432&searchFilter=${encodeURIComponent(name)}&pageSize=50`);
  const a = r.data.flatMap((m) => m.authors).find((x) => x.name.toLowerCase() === name.toLowerCase());
  if (!a) fail(`No CurseForge author "${name}" found (check the username in the plugin settings).`);
  return a.id;
}
const modId = async (x) => /^\d+$/.test(x) ? x : (await core(`/mods/search?gameId=432&slug=${encodeURIComponent(x)}`)).data[0]?.id ?? fail("Project not found: " + x);
if (cmd && !["help", "projects", "files", "info"].includes(cmd) && !settings.token) fail("No CurseForge upload token set.");
const list = (x) => (x ? String(x).split(",").map((s) => s.trim()).filter(Boolean) : []);

switch (cmd) {
  case "connect": case "status": {
    const v = await http("GET", `${base}/game/versions`, { headers });
    let extra = "";
    if (settings.api_key && settings.author) { const id = await authorId(settings.author); const r = await core(`/mods/search?gameId=432&authorId=${id}&pageSize=50`); extra = `, ${r.pagination.totalCount} projects by ${settings.author}`; }
    console.log(`Upload token valid${extra}`);
    break;
  }
  case "projects": {
    const name = w[0] || settings.author;
    if (!name) fail("Set your CurseForge username in the plugin settings or pass it: projects <username>");
    const id = await authorId(name);
    const out = [];
    for (let i = 0; ; i += 50) {
      const r = await core(`/mods/search?gameId=${f.game || 432}&authorId=${id}&pageSize=50&index=${i}&sortField=6&sortOrder=desc`);
      out.push(...r.data);
      if (i + 50 >= r.pagination.totalCount || i >= 950) break;
    }
    print(out.map((m) => `${m.id}  ${m.name}  (${m.slug})  ${m.downloadCount.toLocaleString("en")} downloads  latest: ${m.latestFilesIndexes?.[0]?.gameVersion ?? "-"}  ${m.links.websiteUrl}`).join("\n") || "No projects found.");
    break;
  }
  case "files": { const id = await modId(w[0]); print((await core(`/mods/${id}/files?pageSize=${f.n || 15}`)).data.map((x) => `${x.id}  ${x.displayName}  ${["", "release", "beta", "alpha"][x.releaseType]}  ${x.gameVersions.join(", ")}  ${x.downloadCount} downloads  ${x.fileDate.slice(0, 10)}`).join("\n")); break; }
  case "info": { const id = await modId(w[0]); const m = (await core(`/mods/${id}`)).data; print(`${m.name} (${m.slug}, id ${m.id})\n${m.summary}\n${m.downloadCount.toLocaleString("en")} downloads, ${m.thumbsUpCount ?? 0} likes, status ${m.status}\nauthors: ${m.authors.map((a) => a.name).join(", ")}\n${m.links.websiteUrl}`); break; }
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
  projects [username] [--game 432]  your projects with downloads (needs the Core API key)
  files <project id|slug> [--n 15]  latest files of a project
  info <project id|slug>            project details
  game-versions [filter]     names/ids of Minecraft versions, loaders (Fabric, NeoForge, Forge, Quilt) and Java versions
  upload <project_id> <file.jar> --game-versions 1.21.1,1.21 --loaders Fabric [--java 21] [--env Client,Server]
         [--name ..] [--changelog "markdown"] [--type release|beta|alpha] [--deps fabric-api:requiredDependency]`);
}
