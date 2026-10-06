// Turso (libSQL): databases and auth tokens.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const org = settings.org;
await restPlugin({
  base: `https://api.turso.tech/v1/organizations/${org}`, headers: { Authorization: `Bearer ${settings.token || ""}` }, need: [settings.token, org],
  status: async (req) => `Connected (${(await req("GET", "/databases")).databases.length} databases)`,
  commands: {
    databases: async (req) => (await req("GET", "/databases")).databases.map((d) => `${d.Name}  libsql://${d.Hostname}`).join("\n"),
    create: async (req, [name, group]) => (await req("POST", "/databases", { name, group: group || "default" })).database,
    token: async (req, [db]) => (await req("POST", `/databases/${db}/auth/tokens?expiration=never`)).jwt,
  },
  help: "Usage:\n  databases | create <name> [group] | token <db>",
});
