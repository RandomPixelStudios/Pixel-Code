// npm registry: package info, publishing and versions.
import { args, settings, fail, http, print, passthrough, which } from "./common.mjs";

const [cmd, ...a] = args;
switch (cmd) {
  case "connect": case "status": {
    const npm = which(["npm"]);
    if (!npm) fail("npm is not installed.");
    console.log(settings.token ? "npm with access token" : "npm (public registry)");
    break;
  }
  case "info": { const p = await http("GET", `https://registry.npmjs.org/${a[0].replace("/", "%2F")}`); print(`${p.name}@${p["dist-tags"]?.latest}\n${p.description || ""}\nversions: ${Object.keys(p.versions || {}).slice(-10).join(", ")}`); break; }
  case "search": print((await http("GET", `https://registry.npmjs.org/-/v1/search?text=${encodeURIComponent(a.join(" "))}&size=15`)).objects.map((o) => `${o.package.name}@${o.package.version}  ${o.package.description || ""}`).join("\n")); break;
  case "downloads": print(await http("GET", `https://api.npmjs.org/downloads/point/last-month/${a[0]}`)); break;
  case undefined: case "help": console.log(`Usage:
  info <package> | search <words> | downloads <package>
  <any npm arguments>   e.g. 'publish --access public', 'version patch', 'outdated'`); break;
  default: {
    if (settings.token) process.env["npm_config_//registry.npmjs.org/:_authToken"] = settings.token;
    passthrough(which(["npm"]) || "npm", args);
  }
}
