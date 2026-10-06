// Crowdin: upload source strings (e.g. lang/en_us.json) and download translations.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { basename, resolve, join } from "node:path";
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.crowdin.com/api/v2";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const pid = settings.project_id;
const api = (m, p, body) => http(m, base + p, { headers, body });
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.token || !pid)) fail("Crowdin token and project id are required.");
switch (cmd) {
  case "connect": case "status": { const p = await api("GET", `/projects/${pid}`); console.log(`Connected to ${p.data.name} (${p.data.targetLanguageIds.length} languages)`); break; }
  case "progress": print((await api("GET", `/projects/${pid}/languages/progress?limit=100`)).data.map(({ data: d }) => `${d.languageId.padEnd(8)} ${String(d.translationProgress).padStart(3)}% translated  ${d.approvalProgress}% approved`).join("\n")); break;
  case "upload": {
    const file = resolve(rest[0]);
    const st = await http("POST", `${base}/storages`, { headers: { ...headers, "Crowdin-API-FileName": basename(file), "Content-Type": "application/octet-stream" }, body: readFileSync(file) });
    const files = (await api("GET", `/projects/${pid}/files?limit=500`)).data.map((x) => x.data);
    const existing = files.find((x) => x.name === basename(file));
    if (existing) await api("PUT", `/projects/${pid}/files/${existing.id}`, { storageId: st.data.id });
    else await api("POST", `/projects/${pid}/files`, { storageId: st.data.id, name: basename(file) });
    console.log(existing ? "Source file updated." : "Source file added.");
    break;
  }
  case "download": {
    const b = await api("POST", `/projects/${pid}/translations/builds`, {});
    let s = b.data;
    while (s.status !== "finished") { await new Promise((r) => setTimeout(r, 2000)); s = (await api("GET", `/projects/${pid}/translations/builds/${b.data.id}`)).data; if (s.status === "failed") fail("Build failed"); }
    const url = (await api("GET", `/projects/${pid}/translations/builds/${b.data.id}/download`)).data.url;
    const dir = resolve(rest[0] || "translations"); mkdirSync(dir, { recursive: true });
    const zip = join(dir, "crowdin.zip");
    writeFileSync(zip, Buffer.from(await (await fetch(url)).arrayBuffer()));
    spawnSync("unzip", ["-o", "-q", zip, "-d", dir]);
    console.log(`Translations extracted to ${dir}`);
    break;
  }
  default: console.log(`Usage:
  progress                    translation progress per language
  upload <file>               add/update a source file (e.g. src/main/resources/assets/mymod/lang/en_us.json)
  download [dir]              build and download all translations`);
}
