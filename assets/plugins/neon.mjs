// Neon serverless Postgres: projects, branches, connection strings.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: "https://console.neon.tech/api/v2", headers: { Authorization: `Bearer ${settings.api_key || ""}` }, need: [settings.api_key],
  status: async (req) => `Connected (${(await req("GET", "/projects")).projects.length} projects)`,
  commands: {
    projects: async (req) => (await req("GET", "/projects")).projects.map((p) => `${p.id}  ${p.name}  ${p.region_id}`).join("\n"),
    branches: async (req, [p]) => (await req("GET", `/projects/${p}/branches`)).branches.map((b) => `${b.id}  ${b.name}${b.default ? " (default)" : ""}`).join("\n"),
    branch: async (req, [p, name]) => (await req("POST", `/projects/${p}/branches`, { branch: { name }, endpoints: [{ type: "read_write" }] })).branch,
    uri: async (req, [p, branch]) => (await req("GET", `/projects/${p}/connection_uri?database_name=neondb&role_name=neondb_owner${branch ? "&branch_id=" + branch : ""}`)).uri,
  },
  help: "Usage:\n  projects | branches <project> | branch <project> <name> | uri <project> [branch_id]",
});
