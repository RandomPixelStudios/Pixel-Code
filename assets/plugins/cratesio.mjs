// crates.io: publish Rust crates and look up versions.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": case "status": { if (!which(["cargo"], [process.env.HOME + "/.cargo/bin"])) fail("cargo not found."); if (!settings.token) fail("No crates.io token set."); const r = await http("GET", "https://crates.io/api/v1/me", { headers: { Authorization: settings.token, "User-Agent": "PixelCode" } }); console.log("Connected as " + r.user.login); break; }
  case "publish": process.env.CARGO_REGISTRY_TOKEN = settings.token || ""; passthrough(which(["cargo"], [process.env.HOME + "/.cargo/bin"]), ["publish", ...rest]); break;
  case "info": { const c = (await http("GET", `https://crates.io/api/v1/crates/${rest[0]}`, { headers: { "User-Agent": "PixelCode" } })).crate; print(`${c.name} ${c.max_stable_version}  ${c.downloads} downloads\n${c.description}`); break; }
  default: console.log("Usage:\n  publish [--dry-run] [-p crate] | info <crate>");
}
