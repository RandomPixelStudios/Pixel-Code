// DaVinci Resolve scripting API (Resolve must be running; Studio needed for external scripting).
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve as abs } from "node:path";
import { args, settings, fail } from "./common.mjs";

const root = settings.path || "/opt/resolve";
const env = {
  ...process.env,
  RESOLVE_SCRIPT_API: `${root}/Developer/Scripting`,
  RESOLVE_SCRIPT_LIB: `${root}/libs/Fusion/fusionscript.so`,
  PYTHONPATH: `${root}/Developer/Scripting/Modules`,
};
const py = (code) => spawnSync("python3", ["-c", `import DaVinciResolveScript as dvr\nresolve = dvr.scriptapp("Resolve")\nif resolve is None: raise SystemExit("Resolve is not running (or external scripting is off: Preferences > System > General > External scripting = Local)")\npm = resolve.GetProjectManager()\nproject = pm.GetCurrentProject()\n${code}`], { encoding: "utf8", env });
const out = (r) => { if (r.status !== 0) fail((r.stderr || r.stdout).trim().split("\n").pop()); console.log(r.stdout.trim()); };
const [cmd, ...rest] = args;

switch (cmd) {
  case "connect": case "status": out(py(`print("Connected to", resolve.GetProductName(), resolve.GetVersionString(), "- project:", project.GetName() if project else "none")`)); break;
  case "info": out(py(`tl = project.GetCurrentTimeline()\nprint("Project:", project.GetName())\nprint("Timelines:", [project.GetTimelineByIndex(i+1).GetName() for i in range(project.GetTimelineCount())])\nprint("Current:", tl.GetName() if tl else None)\nprint("Render presets:", project.GetRenderPresetList())`)); break;
  case "render": out(py(`project.LoadRenderPreset(${JSON.stringify(rest[0] || "YouTube - 1080p")})\nproject.SetRenderSettings({"TargetDir": ${JSON.stringify(abs(rest[1] || "render"))}})\njob = project.AddRenderJob()\nproject.StartRendering(job)\nimport time\nwhile project.IsRenderingInProgress(): time.sleep(2)\nprint("Render finished:", project.GetRenderJobStatus(job))`)); break;
  case "import": out(py(`ms = resolve.GetMediaStorage()\nitems = ms.AddItemListToMediaPool(${JSON.stringify(rest.map((x) => abs(x)))})\nprint("Imported", len(items or []), "items")`)); break;
  case "python": out(py(rest[0]?.endsWith(".py") ? readFileSync(abs(rest[0]), "utf8") : rest.join(" "))); break;
  default: console.log(`Usage:
  info                          project, timelines and render presets
  import <files...>             add media to the media pool
  render [preset] [target_dir]  render the current timeline
  python <code|file.py>         run Python with resolve, pm, project predefined`);
}
