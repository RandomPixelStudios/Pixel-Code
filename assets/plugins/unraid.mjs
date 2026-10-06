// Unraid (7.2+ GraphQL API): array, disks, Docker containers and VMs.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const url = (settings.url || "").replace(/\/$/, "") + "/graphql";
const gql = async (query) => { const r = await http("POST", url, { headers: { "x-api-key": settings.api_key || "" }, body: { query } }); if (r.errors) fail(r.errors[0].message); return r.data; };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.url || !settings.api_key)) fail("Unraid URL and API key are required.");
switch (cmd) {
  case "connect": case "status": { const d = await gql("{ info { os { hostname } } array { state } }"); console.log(`Connected to ${d.info.os.hostname}, array ${d.array.state}`); break; }
  case "array": print((await gql("{ array { state disks { name size temp status } } }")).array); break;
  case "docker": print((await gql("{ docker { containers { names state status } } }")).docker.containers.map((c) => `${c.names[0]}  ${c.state}  ${c.status}`).join("\n")); break;
  case "vms": print((await gql("{ vms { domains { name state } } }")).vms.domains.map((v) => `${v.name}  ${v.state}`).join("\n")); break;
  case "gql": print(await gql(rest.join(" "))); break;
  default: console.log("Usage:\n  array | docker | vms | gql <graphql query>");
}
