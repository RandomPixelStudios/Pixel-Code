// App Store Connect API: apps, TestFlight builds and testers, reviews, certificates, profiles, devices.
import { readFileSync } from "node:fs";
import { sign } from "node:crypto";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.appstoreconnect.apple.com/v1";
function jwt() {
  let key;
  try { key = readFileSync((settings.key_file || "").replace(/^~/, process.env.HOME), "utf8"); } catch { fail("API key file (.p8) not found."); }
  const b64 = (o) => Buffer.from(JSON.stringify(o)).toString("base64url");
  const now = Math.floor(Date.now() / 1000);
  const body = `${b64({ alg: "ES256", kid: settings.key_id, typ: "JWT" })}.${b64({ iss: settings.issuer_id, iat: now, exp: now + 1200, aud: "appstoreconnect-v1" })}`;
  return body + "." + sign("sha256", Buffer.from(body), { key, dsaEncoding: "ieee-p1363" }).toString("base64url");
}
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !(settings.key_id && settings.issuer_id && settings.key_file)) fail("Key ID, issuer ID and the .p8 key file are required.");
const api = (m, p, body) => http(m, p.startsWith("http") ? p : base + p, { headers: { Authorization: `Bearer ${jwt()}` }, body });
switch (cmd) {
  case "connect": case "status": { const a = await api("GET", "/apps?limit=200"); console.log(`Connected (${a.data.length} apps)`); break; }
  case "apps": print((await api("GET", "/apps?limit=200")).data.map((a) => `${a.id}  ${a.attributes.name}  ${a.attributes.bundleId}`).join("\n")); break;
  case "builds": print((await api("GET", `/builds?filter[app]=${rest[0]}&sort=-uploadedDate&limit=15`)).data.map((b) => `${b.id}  ${b.attributes.version}  ${b.attributes.processingState}  ${b.attributes.uploadedDate?.slice(0, 10)}${b.attributes.expired ? " (expired)" : ""}`).join("\n")); break;
  case "groups": print((await api("GET", `/apps/${rest[0]}/betaGroups`)).data.map((g) => `${g.id}  ${g.attributes.name}${g.attributes.isInternalGroup ? " (internal)" : ""}`).join("\n")); break;
  case "add-build": await api("POST", `/betaGroups/${rest[0]}/relationships/builds`, { data: [{ type: "builds", id: rest[1] }] }); console.log("Build added to group."); break;
  case "add-tester": print((await api("POST", "/betaTesters", { data: { type: "betaTesters", attributes: { email: rest[1], firstName: rest[2] || "", lastName: rest[3] || "" }, relationships: { betaGroups: { data: [{ type: "betaGroups", id: rest[0] }] } } } })).data.id); break;
  case "whats-new": { const l = (await api("GET", `/builds/${rest[0]}/betaBuildLocalizations`)).data[0]; await api("PATCH", `/betaBuildLocalizations/${l.id}`, { data: { type: "betaBuildLocalizations", id: l.id, attributes: { whatsNew: rest.slice(1).join(" ") } } }); console.log("Updated."); break; }
  case "submit-beta": await api("POST", "/betaAppReviewSubmissions", { data: { type: "betaAppReviewSubmissions", relationships: { build: { data: { type: "builds", id: rest[0] } } } } }); console.log("Submitted for beta review."); break;
  case "reviews": print((await api("GET", `/apps/${rest[0]}/customerReviews?sort=-createdDate&limit=20`)).data.map((r) => `${"★".repeat(r.attributes.rating)}  ${r.attributes.title}: ${r.attributes.body}`).join("\n\n")); break;
  case "certificates": print((await api("GET", "/certificates")).data.map((c) => `${c.id}  ${c.attributes.certificateType}  ${c.attributes.name}  expires ${c.attributes.expirationDate?.slice(0, 10)}`).join("\n")); break;
  case "profiles": print((await api("GET", "/profiles")).data.map((p) => `${p.id}  ${p.attributes.profileType}  ${p.attributes.name}  ${p.attributes.profileState}`).join("\n")); break;
  case "devices": print((await api("GET", "/devices?limit=200")).data.map((d) => `${d.attributes.udid}  ${d.attributes.name}  ${d.attributes.platform}  ${d.attributes.status}`).join("\n")); break;
  case "register-device": print((await api("POST", "/devices", { data: { type: "devices", attributes: { name: rest[0], udid: rest[1], platform: rest[2] || "IOS" } } })).data.id); break;
  case "raw": print(await api(rest[0] || "GET", rest[1], rest[2] ? JSON.parse(rest.slice(2).join(" ")) : undefined)); break;
  default: console.log(`Usage:
  apps | builds <app_id> | groups <app_id> | add-build <group_id> <build_id> | add-tester <group_id> <email> [first] [last]
  whats-new <build_id> <text> | submit-beta <build_id> | reviews <app_id>
  certificates | profiles | devices | register-device <name> <udid> [IOS|MAC_OS] | raw <METHOD> </v1 path> [json]`);
}
