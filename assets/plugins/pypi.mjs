// PyPI: build and upload Python packages (twine through pipx/uvx).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
const env = { ...process.env, TWINE_USERNAME: "__token__", TWINE_PASSWORD: settings.token || "", ...(settings.test === "yes" ? { TWINE_REPOSITORY: "testpypi" } : {}) };
const tool = which(["uvx", "pipx"]);
const run = (pkg, a) => process.exit(spawnSync(tool, [...(tool.endsWith("pipx") ? ["run"] : []), pkg, ...a], { stdio: "inherit", env }).status ?? 1);
switch (cmd) {
  case "connect": case "status": if (!tool) fail("uv or pipx is needed."); if (!settings.token) fail("No PyPI API token set."); console.log("Ready to upload with " + tool); break;
  case "build": run("build", rest); break;
  case "upload": run("twine", ["upload", ...(rest.length ? rest : ["dist/*"])]); break;
  case "info": { const p = (await http("GET", `https://pypi.org/pypi/${rest[0]}/json`)).info; print(`${p.name} ${p.version}\n${p.summary}`); break; }
  default: console.log("Usage:\n  build | upload [files] | info <package>");
}
