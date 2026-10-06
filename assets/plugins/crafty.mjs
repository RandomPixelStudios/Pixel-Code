// Crafty Controller (Minecraft server manager) API v2.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "https://localhost:8443").replace(/\/$/, "") + "/api/v2";
process.env.NODE_TLS_REJECT_UNAUTHORIZED = settings.insecure === "no" ? "1" : "0";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({
  base, headers, need: [settings.token],
  status: async (req) => `Connected (${(await req("GET", "/servers")).data.length} servers)`,
  commands: {
    servers: async (req) => (await req("GET", "/servers")).data.map((s) => `${s.server_id}  ${s.server_name}`).join("\n"),
    action: async (req, [id, a]) => req("POST", `/servers/${id}/action/${a}`),
    cmd: async (req, [id, ...c]) => { await http("POST", `${base}/servers/${id}/stdin`, { headers: { ...headers, "Content-Type": "text/plain" }, body: c.join(" ") }); return "Sent."; },
    logs: async (req, [id]) => (await req("GET", `/servers/${id}/logs`)).data.slice(-80).join("\n"),
  },
  help: `Usage:
  servers | action <id> start_server|stop_server|restart_server|backup_server | cmd <id> <command> | logs <id>`,
});
