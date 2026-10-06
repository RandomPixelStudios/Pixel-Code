// Chrome Web Store: upload and publish extensions (OAuth client + refresh token).
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
async function headers() { const t = await http("POST", "https://oauth2.googleapis.com/token", { form: { client_id: settings.client_id, client_secret: settings.client_secret, refresh_token: settings.refresh_token, grant_type: "refresh_token" } }); return { Authorization: `Bearer ${t.access_token}`, "x-goog-api-version": "2" }; }
const id = settings.extension_id;
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !(settings.client_id && settings.client_secret && settings.refresh_token && id)) fail("Client id/secret, refresh token and extension id are required.");
switch (cmd) {
  case "connect": case "status": { const r = await http("GET", `https://www.googleapis.com/chromewebstore/v1.1/items/${id}?projection=DRAFT`, { headers: await headers() }); console.log(`Connected to extension ${r.id} (${r.uploadState ?? "ok"})`); break; }
  case "upload": print(await http("PUT", `https://www.googleapis.com/upload/chromewebstore/v1.1/items/${id}`, { headers: { ...(await headers()), "Content-Type": "application/zip" }, body: readFileSync(resolve(rest[0])) })); break;
  case "publish": print(await http("POST", `https://www.googleapis.com/chromewebstore/v1.1/items/${id}/publish${rest[0] === "testers" ? "?publishTarget=trustedTesters" : ""}`, { headers: await headers() })); break;
  default: console.log("Usage:\n  upload <extension.zip> | publish [testers]");
}
