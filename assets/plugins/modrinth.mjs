// Modrinth: list projects and upload mod versions (personal access token).
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { args, settings, fail, http, print, rawCall, flags } from "./common.mjs";

const base = "https://api.modrinth.com/v2";
const headers = { Authorization: settings.token || "", "User-Agent": "PixelCode/0.1 (pixel-code plugin)" };
const get = (p) => http("GET", base + p, { headers });
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
const list = (x) => (x ? String(x).split(",").map((s) => s.trim()).filter(Boolean) : []);
if (cmd && cmd !== "help" && !settings.token) fail("No Modrinth token set.");

switch (cmd) {
  case "connect": case "status": { const u = await get("/user"); console.log(`Connected as ${u.username}`); break; }
  case "projects": { const u = await get("/user"); print((await get(`/user/${u.id}/projects`)).map((p) => `${p.slug}  ${p.title}  (${p.project_type}, ${p.downloads} downloads)`).join("\n")); break; }
  case "versions": print((await get(`/project/${w[0]}/version`)).map((v) => `${v.version_number}  ${v.version_type}  ${v.loaders.join("/")}  MC ${v.game_versions.join(", ")}  ${v.date_published.slice(0, 10)}`).join("\n")); break;
  case "game-versions": print((await get("/tag/game_version")).filter((v) => f.all || v.version_type === "release").slice(0, 60).map((v) => v.version).join(", ")); break;
  case "upload": {
    const [project, ...files] = w;
    if (!project || !files.length || !f.version) fail("Usage: upload <project> <file.jar> [more files] --version 1.2.0 --game-versions 1.21.1,1.21 --loaders fabric [--name ..] [--changelog ..] [--type release|beta|alpha] [--deps P7dR8mSH:required]");
    const p = await get(`/project/${project}`);
    const data = {
      project_id: p.id,
      name: f.name || `${p.title} ${f.version}`,
      version_number: f.version,
      changelog: f.changelog || "",
      game_versions: list(f["game-versions"]),
      loaders: list(f.loaders),
      version_type: f.type || "release",
      featured: true,
      dependencies: list(f.deps).map((d) => { const [id, type] = d.split(":"); return { project_id: id, dependency_type: type || "required" }; }),
      file_parts: files.map((_, i) => `file${i}`),
      primary_file: "file0",
    };
    if (!data.game_versions.length || !data.loaders.length) fail("--game-versions and --loaders are required");
    const fd = new FormData();
    fd.append("data", JSON.stringify(data));
    files.forEach((file, i) => fd.append(`file${i}`, new Blob([readFileSync(resolve(file))]), basename(file)));
    const res = await fetch(`${base}/version`, { method: "POST", headers, body: fd });
    const j = await res.json().catch(() => ({}));
    if (!res.ok) fail(`Modrinth ${res.status}: ${j.description || JSON.stringify(j)}`);
    console.log(`Uploaded ${j.version_number}: https://modrinth.com/project/${p.slug}/version/${j.id}`);
    break;
  }
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  projects                         your projects
  versions <project>               versions of a project (slug or id)
  game-versions [--all]            valid Minecraft versions
  upload <project> <file.jar> --version 1.2.0 --game-versions 1.21.1,1.21 --loaders fabric,quilt
         [--name ..] [--changelog "markdown"] [--type release|beta|alpha] [--deps <project_id>:required|optional]
  raw <METHOD> </v2 path> [json]`);
}
