// Docker Hub: log in and push images, list your repositories and tags.
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.username || !settings.token)) fail("Username and access token are required.");
const login = () => { const r = spawnSync("docker", ["login", "-u", settings.username, "--password-stdin"], { input: settings.token, encoding: "utf8" }); if (r.status !== 0) fail("docker login failed: " + r.stderr.trim()); };
switch (cmd) {
  case "connect": case "status": login(); console.log(`Logged in to Docker Hub as ${settings.username}`); break;
  case "repos": print((await http("GET", `https://hub.docker.com/v2/namespaces/${settings.username}/repositories?page_size=100`)).results.map((r) => `${r.namespace}/${r.name}  ${r.pull_count} pulls  ${r.last_updated?.slice(0, 10)}`).join("\n")); break;
  case "tags": print((await http("GET", `https://hub.docker.com/v2/namespaces/${settings.username}/repositories/${rest[0]}/tags?page_size=50`)).results.map((t) => `${t.name}  ${(t.full_size / 1e6).toFixed(0)} MB  ${t.last_updated?.slice(0, 10)}`).join("\n")); break;
  case "push": login(); passthrough("docker", ["push", rest[0]]); break;
  default: console.log("Usage:\n  repos | tags <repo> | push <user/image:tag>   (build with the Docker plugin first)");
}
