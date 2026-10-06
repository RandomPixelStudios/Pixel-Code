// NuGet: pack and push .NET packages.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
const dotnet = which(["dotnet"], [process.env.HOME + "/.dotnet"]);
switch (cmd) {
  case "connect": case "status": if (!dotnet) fail("dotnet SDK not found."); if (!settings.api_key) fail("No NuGet API key set."); console.log("dotnet ready: " + dotnet); break;
  case "pack": passthrough(dotnet, ["pack", "-c", "Release", ...rest]); break;
  case "push": passthrough(dotnet, ["nuget", "push", rest[0], "--api-key", settings.api_key, "--source", settings.source || "https://api.nuget.org/v3/index.json", "--skip-duplicate"]); break;
  case "info": { const r = await http("GET", `https://api.nuget.org/v3-flatcontainer/${rest[0].toLowerCase()}/index.json`); print(r.versions.slice(-10).join(", ")); break; }
  default: console.log("Usage:\n  pack [project] | push <file.nupkg> | info <package>");
}
