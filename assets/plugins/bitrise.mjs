// Bitrise: mobile CI builds.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: "https://api.bitrise.io/v0.1", headers: { Authorization: settings.token || "" }, need: [settings.token],
  status: async (req) => `Connected as ${(await req("GET", "/me")).data.username}`,
  commands: {
    apps: async (req) => (await req("GET", "/apps")).data.map((a) => `${a.slug}  ${a.title}  ${a.project_type}`).join("\n"),
    builds: async (req, [app]) => (await req("GET", `/apps/${app}/builds?limit=15`)).data.map((b) => `${b.slug}  ${b.status_text}  ${b.triggered_workflow}  ${b.branch}  #${b.build_number}`).join("\n"),
    build: async (req, [app, workflow, branch]) => (await req("POST", `/apps/${app}/builds`, { hook_info: { type: "bitrise" }, build_params: { workflow_id: workflow, branch: branch || "main" } })).build_url,
    artifacts: async (req, [app, build]) => { const a = (await req("GET", `/apps/${app}/builds/${build}/artifacts`)).data; const out = []; for (const x of a) out.push(`${x.title}  ${(await req("GET", `/apps/${app}/builds/${build}/artifacts/${x.slug}`)).data.expiring_download_url}`); return out.join("\n"); },
    log: async (req, [app, build]) => { const l = await req("GET", `/apps/${app}/builds/${build}/log`); return l.expiring_raw_log_url ? await (await fetch(l.expiring_raw_log_url)).text().then((t) => t.slice(-15000)) : l; },
  },
  help: "Usage:\n  apps | builds <app> | build <app> <workflow> [branch] | artifacts <app> <build> | log <app> <build>",
});
