// GitHub Container Registry: push images with your GitHub token.
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
const token = settings.token || spawnSync(which(["gh"]) || (process.env.HOME + "/.local/share/pixel-code/bin/gh"), ["auth", "token"], { encoding: "utf8" }).stdout?.trim();
const user = settings.username || spawnSync(which(["gh"]) || (process.env.HOME + "/.local/share/pixel-code/bin/gh"), ["api", "user", "--jq", ".login"], { encoding: "utf8" }).stdout?.trim();
const login = () => { if (!token || !user) fail("Connect GitHub in Pixel Code or set a token with write:packages."); const r = spawnSync("docker", ["login", "ghcr.io", "-u", user, "--password-stdin"], { input: token, encoding: "utf8" }); if (r.status !== 0) fail("docker login failed: " + r.stderr.trim()); };
switch (cmd) {
  case "connect": case "status": login(); console.log(`Logged in to ghcr.io as ${user}`); break;
  case "push": login(); passthrough("docker", ["push", rest[0]]); break;
  case "packages": print((await http("GET", "https://api.github.com/user/packages?package_type=container", { headers: { Authorization: `Bearer ${token}` } })).map((p) => `ghcr.io/${p.owner.login}/${p.name}  ${p.visibility}`).join("\n")); break;
  default: console.log("Usage:\n  push ghcr.io/<owner>/<image>:<tag> | packages   (the token needs write:packages / read:packages)");
}
