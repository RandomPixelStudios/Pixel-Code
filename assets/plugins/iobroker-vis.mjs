// ioBroker VIS 2 via the rest-api adapter (default port 8093). Projects live in the file storage
// under vis-2.0/<project>/vis-views.json (+ vis-user.css).
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { resolve, join } from "node:path";
import { tmpdir } from "node:os";
import { spawn } from "node:child_process";
import { args, settings, fail, http, print, flags, which } from "./common.mjs";

const base = (settings.url || "http://iobroker.local:8093").replace(/\/$/, "") + "/v1";
const adapter = settings.instance || "vis-2.0";
// VIS-Laufzeit (web-Adapter): eigenes Feld, sonst gleicher Host wie die rest-api auf Port 8082
const web = (settings.web || (() => { const u = new URL(settings.url || "http://iobroker.local:8093"); u.port = "8082"; return u.origin; })()).replace(/\/$/, "");
const headers = settings.user ? { Authorization: "Basic " + Buffer.from(`${settings.user}:${settings.password || ""}`).toString("base64") } : {};
// rest-api: /file/{objectId}/{fileName}, der Dateiname mit "/" als %2F in einem Segment
const fileUrl = (name) => `${base}/file/${encodeURIComponent(adapter)}/${encodeURIComponent(name)}`;
const [f, rest] = flags(args);
const [cmd, ...a] = rest;

const BACKUP = "vis-views.pixelcode-backup.json";

async function readFile(name) {
  return http("GET", fileUrl(name), { headers });
}

// Upload über writeFile64 (der Multipart-Upload unter /file nimmt keine Unterordner an)
async function writeFile(name, content) {
  await http("POST", `${base}/command/writeFile64`, { headers, body: { adapter, fileName: name, data64: Buffer.from(content).toString("base64") } });
}

/** Liest eine JSON-Datei, ohne bei Fehlern abzubrechen. */
async function tryRead(name) {
  try {
    const res = await fetch(fileUrl(name), { headers });
    return res.ok ? await res.json() : null;
  } catch { return null; }
}

/** Widget-Typen aus allen (oder den angegebenen) Projekten; der erste Fund gewinnt. */
async function allTemplates(list) {
  const all = {};
  for (const p of list || await projects()) {
    const views = await tryRead(`${p}/vis-views.json`);
    if (views && typeof views === "object") for (const [k, v] of Object.entries(templates(views))) all[k] ??= v;
  }
  return all;
}

async function projects() {
  const list = await http("GET", `${base}/dir/${encodeURIComponent(adapter)}`, { headers });
  return (Array.isArray(list) ? list : []).filter((e) => e.isDir).map((e) => e.file);
}

function need(project) {
  if (!project) fail("Project name missing. Run 'projects' to list them.");
  return project;
}

async function load(project) {
  const views = await readFile(`${need(project)}/vis-views.json`);
  if (!views || typeof views !== "object") fail(`Project "${project}" has no readable vis-views.json.`);
  return views;
}

// Vor jedem Schreiben den alten Stand sichern, damit 'restore' ihn zurückholen kann
async function save(project, views) {
  try { await writeFile(`${project}/${BACKUP}`, JSON.stringify(await readFile(`${project}/vis-views.json`), null, 2)); } catch {}
  await writeFile(`${project}/vis-views.json`, JSON.stringify(views, null, 2));
}

const viewNames = (views) => Object.keys(views).filter((k) => k !== "___settings");

function view(views, name) {
  if (!views[name] || name === "___settings") fail(`View "${name}" not found. Views: ${viewNames(views).join(", ")}`);
  views[name].widgets ||= {};
  return views[name];
}

const json = (s, what) => {
  if (s === undefined || s === true) return {};
  try { return JSON.parse(s); } catch (e) { fail(`--${what} is not valid JSON: ${e.message}`); }
};

const emptyView = (f) => ({
  settings: { style: { background_class: "" }, theme: "redmond", sizex: f.width, sizey: f.height, useAsDefault: false },
  widgets: {},
  activeWidgets: {},
});

const px = (v) => (v === undefined ? undefined : /^\d+(\.\d+)?$/.test(String(v)) ? `${v}px` : String(v));

/** Nächste freie Widget-ID (projektweit eindeutig, Format w000001 bzw. g000001 für Gruppen). */
function nextId(views, prefix = "w") {
  let max = 0;
  for (const v of viewNames(views)) for (const id of Object.keys(views[v].widgets || {})) {
    const n = Number(id.slice(1));
    if (id[0] === prefix && n > max) max = n;
  }
  return prefix + String(max + 1).padStart(6, "0");
}

/** Alle Widget-Typen eines Projekts mit Widget-Set und einem Beispiel. */
function templates(views) {
  const out = {};
  for (const v of viewNames(views)) for (const w of Object.values(views[v].widgets || {})) {
    if (w?.tpl && !out[w.tpl]) out[w.tpl] = { widgetSet: w.widgetSet, data: w.data, style: w.style };
  }
  return out;
}

function describe(views, name) {
  const v = views[name];
  const ws = Object.entries(v.widgets || {});
  const st = v.settings?.style || {};
  return [
    `## ${name}  (${ws.length} widgets${st.width ? `, ${st.width}x${st.height}` : ""})`,
    ...ws.map(([id, w]) => {
      const s = w.style || {};
      const d = w.data || {};
      return `  ${id}  ${w.tpl}${d.oid ? `  oid=${d.oid}` : ""}${d.name ? `  "${d.name}"` : ""}  @${s.left ?? "?"},${s.top ?? "?"}  ${s.width ?? "?"} x ${s.height ?? "?"}`;
    }),
  ].join("\n");
}

// ---------------------------------------------------------------- States

const command = (name, body) => http("POST", `${base}/command/${name}`, { headers, body });

/** Wert aus der Kommandozeile: Zahl, true/false, JSON oder Text. */
function parseValue(s) {
  if (s === "true" || s === "false") return s === "true";
  if (s !== "" && !isNaN(Number(s))) return Number(s);
  if (/^[\[{]/.test(s)) { try { return JSON.parse(s); } catch {} }
  return s;
}

// ---------------------------------------------------------------- Screenshot

/** Öffnet eine VIS-Seite in Chrome/Chromium (headless, über DevTools) und speichert ein PNG. */
async function screenshot(url, file, { width = 1440, height = 900, wait = 8000, dark = false, eval: js } = {}) {
  const bin = which(["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"]);
  if (!bin) fail("Chrome or Chromium is required for screenshots.");
  if (typeof WebSocket === "undefined") fail("Node.js 22 or newer is required for screenshots.");
  const profile = mkdtempSync(join(tmpdir(), "pc-vis-"));
  const proc = spawn(bin, ["--headless=new", "--disable-gpu", "--no-sandbox", "--no-first-run", `--user-data-dir=${profile}`, "--remote-debugging-port=0", "about:blank"], { stdio: ["ignore", "ignore", "pipe"] });
  const cleanup = () => { try { proc.kill(); } catch {} setTimeout(() => { try { rmSync(profile, { recursive: true, force: true }); } catch {} }, 500); };
  try {
    const port = await new Promise((ok, bad) => {
      let err = "";
      const t = setTimeout(() => bad(new Error("Chrome did not start")), 20000);
      proc.stderr.on("data", (d) => { err += d; const m = err.match(/DevTools listening on ws:\/\/[^:]+:(\d+)\//); if (m) { clearTimeout(t); ok(m[1]); } });
      proc.on("exit", () => bad(new Error("Chrome exited: " + err.slice(-300))));
    });
    const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
    const ws = new WebSocket(target.webSocketDebuggerUrl);
    let n = 0; const pend = {}; const errors = [];
    const call = (method, params = {}) => new Promise((ok) => { const id = ++n; pend[id] = ok; ws.send(JSON.stringify({ id, method, params })); });
    ws.onmessage = (e) => {
      const m = JSON.parse(e.data);
      if (m.id) pend[m.id]?.(m.result ?? m.error);
      else if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description?.split("\n")[0] || m.params.exceptionDetails.text);
      else if (m.method === "Runtime.consoleAPICalled" && m.params.type === "error") errors.push(m.params.args.map((a) => a.value ?? a.description).join(" ").split("\n")[0]);
    };
    await new Promise((r) => (ws.onopen = r));
    await call("Runtime.enable");
    await call("Page.enable");
    await call("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width < 700 });
    if (dark) await call("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: "dark" }] });
    await call("Page.navigate", { url });
    await new Promise((r) => setTimeout(r, wait));
    let result;
    if (js) {
      const r = await call("Runtime.evaluate", { expression: js, awaitPromise: true, returnByValue: true });
      result = r?.result?.value ?? r?.exceptionDetails?.exception?.description ?? r?.exceptionDetails?.text;
      await new Promise((r) => setTimeout(r, 1500));
    }
    const shot = await call("Page.captureScreenshot", { format: "png" });
    writeFileSync(file, Buffer.from(shot.data, "base64"));
    ws.close();
    return { errors, result };
  } finally {
    cleanup();
  }
}

switch (cmd) {
  case "connect": case "status": {
    const p = await projects();
    console.log(`Connected to ${adapter} (${p.length} project${p.length === 1 ? "" : "s"}${p.length ? ": " + p.join(", ") : ""})`);
    break;
  }
  case "projects": print((await projects()).join("\n") || "No VIS 2 projects found."); break;
  case "views": {
    const views = await load(a[0]);
    print(viewNames(views).map((n) => describe(views, n)).join("\n\n"));
    break;
  }
  case "get": {
    const views = await load(a[0]);
    print(a[1] ? view(views, a[1]) : views);
    break;
  }
  case "put": {
    // Ganzes Projekt oder mit --view nur eine View ersetzen
    if (!a[1]) fail("Usage: put <project> <file.json> [--view <name>]");
    const data = JSON.parse(readFileSync(resolve(a[1]), "utf8"));
    const views = await load(a[0]);
    if (typeof f.view === "string") views[f.view] = data;
    else Object.assign(views, data);
    await save(a[0], views);
    print(`Saved ${typeof f.view === "string" ? `view "${f.view}"` : "project"} in ${a[0]}. Backup: ${BACKUP}`);
    break;
  }
  case "create-project": {
    if (!a[0]) fail("Usage: create-project <name> [--view <first view>]");
    if ((await projects()).includes(a[0])) fail(`Project "${a[0]}" already exists.`);
    const first = typeof f.view === "string" ? f.view : "Main";
    await writeFile(`${a[0]}/vis-views.json`, JSON.stringify({ ___settings: { folders: [], openedViews: [first] }, [first]: emptyView(f) }, null, 2));
    await writeFile(`${a[0]}/vis-user.css`, "/* Project CSS */\n");
    print(`Created project "${a[0]}" with view "${first}".`);
    break;
  }
  case "add-view": {
    if (!a[1]) fail("Usage: add-view <project> <name> [--from <view>] [--width 1280 --height 800]");
    const views = await load(a[0]);
    if (views[a[1]]) fail(`View "${a[1]}" already exists.`);
    if (typeof f.from === "string") {
      // Kopie mit neuen Widget-IDs
      const src = JSON.parse(JSON.stringify(view(views, f.from)));
      const widgets = {};
      for (const w of Object.values(src.widgets)) {
        const id = nextId({ ...views, [a[1]]: { widgets } }, "w");
        widgets[id] = w;
      }
      views[a[1]] = { ...src, widgets };
    } else {
      views[a[1]] = emptyView(f);
    }
    await save(a[0], views);
    print(`Added view "${a[1]}".`);
    break;
  }
  case "del-view": {
    const views = await load(a[0]);
    view(views, a[1]);
    delete views[a[1]];
    await save(a[0], views);
    print(`Deleted view "${a[1]}".`);
    break;
  }
  case "add-widget": {
    if (!a[2]) fail("Usage: add-widget <project> <view> <tpl> [--oid <state>] [--x --y --w --h] [--data json] [--style json] [--set <widgetSet>]");
    const views = await load(a[0]);
    const v = view(views, a[1]);
    const known = templates(views)[a[2]];
    const id = nextId(views);
    const data = { ...json(f.data, "data") };
    if (typeof f.oid === "string") data.oid = f.oid;
    const style = { left: px(f.x ?? 10), top: px(f.y ?? 10), width: px(f.w ?? 100), height: px(f.h ?? 40), ...json(f.style, "style") };
    v.widgets[id] = { tpl: a[2], widgetSet: f.set || known?.widgetSet || "basic", data, style };
    await save(a[0], views);
    print(`Added ${id} (${a[2]}) to "${a[1]}".${known ? "" : " Note: this widget type is not used anywhere in the project yet, check it with 'widgets' or a screenshot."}`);
    break;
  }
  case "set-widget": {
    if (!a[2]) fail("Usage: set-widget <project> <view> <widgetId> [--data json] [--style json] [--x --y --w --h] [--oid <state>]");
    const views = await load(a[0]);
    const w = view(views, a[1]).widgets[a[2]];
    if (!w) fail(`Widget ${a[2]} not found in "${a[1]}".`);
    w.data = { ...w.data, ...json(f.data, "data"), ...(typeof f.oid === "string" ? { oid: f.oid } : {}) };
    const pos = Object.fromEntries([["left", f.x], ["top", f.y], ["width", f.w], ["height", f.h]].filter(([, v]) => v !== undefined).map(([k, v]) => [k, px(v)]));
    w.style = { ...w.style, ...json(f.style, "style"), ...pos };
    await save(a[0], views);
    print(`Updated ${a[2]}.`);
    break;
  }
  case "del-widget": {
    const views = await load(a[0]);
    const v = view(views, a[1]);
    if (!v.widgets[a[2]]) fail(`Widget ${a[2]} not found in "${a[1]}".`);
    delete v.widgets[a[2]];
    await save(a[0], views);
    print(`Deleted ${a[2]}.`);
    break;
  }
  case "widgets": {
    // Widget-Typen aus allen (oder einem) Projekt(en) als Vorlage
    const all = await allTemplates(a[0] ? [a[0]] : null);
    const entries = Object.entries(all);
    if (!entries.length) print("No widgets found in any project. Create one widget in the VIS 2 editor first, then use it as a template.");
    else print(f.full ? all : entries.map(([tpl, t]) => `${tpl}  [${t.widgetSet}]  data keys: ${Object.keys(t.data || {}).filter((k) => !k.startsWith("g_")).join(", ")}`).join("\n"));
    break;
  }
  case "widget": {
    // Beispiel eines Widget-Typs vollständig anzeigen
    const all = await allTemplates();
    if (!all[a[0]]) fail(`Widget type "${a[0]}" is not used in any project.`);
    print(all[a[0]]);
    break;
  }
  case "css": {
    const name = `${need(a[0])}/vis-user.css`;
    if (!a[1]) {
      const res = await fetch(fileUrl(name), { headers }).catch(() => null);
      print(res?.ok ? await res.text() : "(no vis-user.css yet)");
      break;
    }
    await writeFile(name, readFileSync(resolve(a[1]), "utf8"));
    print(`Saved ${name}.`);
    break;
  }
  case "restore": {
    const backup = await readFile(`${need(a[0])}/${BACKUP}`);
    await writeFile(`${a[0]}/vis-views.json`, JSON.stringify(backup, null, 2));
    print(`Restored ${a[0]} from ${BACKUP}.`);
    break;
  }
  case "states": {
    if (!a[0]) fail("Usage: states <pattern>, e.g. states 0_userdata.0.*");
    const st = await http("GET", `${base}/states?filter=${encodeURIComponent(a[0])}`, { headers });
    print(Object.entries(st || {}).map(([id, s]) => `${id} = ${JSON.stringify(s?.val)}`).join("\n") || "No states found.");
    break;
  }
  case "get-state": {
    if (!a[0]) fail("Usage: get-state <id>");
    print(await http("GET", `${base}/state/${encodeURIComponent(a[0])}`, { headers }));
    break;
  }
  case "set-state": {
    if (!a[0] || a[1] === undefined) fail("Usage: set-state <id> <value> [--ack]");
    await command("setState", { id: a[0], state: { val: parseValue(a.slice(1).join(" ")), ack: !!f.ack } });
    print(`${a[0]} = ${a.slice(1).join(" ")}`);
    break;
  }
  case "create-state": {
    // Datenpunkt anlegen (nur unter 0_userdata.0 oder javascript.X, damit keine Adapter-Objekte überschrieben werden)
    if (!a[0]) fail("Usage: create-state <id> [value] [--type number|boolean|string|json] [--name ..] [--unit ..] [--role ..] [--min ..] [--max ..] [--readonly]");
    if (!/^(0_userdata\.0|javascript\.\d+)\./.test(a[0])) fail("Only states under 0_userdata.0.* or javascript.<n>.* can be created.");
    const value = a[1] !== undefined ? parseValue(a.slice(1).join(" ")) : undefined;
    const type = typeof f.type === "string" ? f.type : value === undefined ? "mixed" : typeof value === "object" ? "json" : typeof value;
    const common = { name: typeof f.name === "string" ? f.name : a[0].split(".").pop(), type, role: typeof f.role === "string" ? f.role : "state", read: true, write: !f.readonly };
    if (typeof f.unit === "string") common.unit = f.unit;
    if (f.min !== undefined) common.min = Number(f.min);
    if (f.max !== undefined) common.max = Number(f.max);
    await command("setObject", { id: a[0], obj: { type: "state", common, native: {} } });
    if (value !== undefined) await command("setState", { id: a[0], state: { val: type === "json" && typeof value === "object" ? JSON.stringify(value) : value, ack: true } });
    print(`Created ${a[0]} (${type})${value !== undefined ? ` = ${JSON.stringify(value)}` : ""}`);
    break;
  }
  case "create-states": {
    // Viele Datenpunkte aus einer JSON-Datei: [{ "id", "value", "type", "name", "unit", "role" }, ...]
    if (!a[0]) fail("Usage: create-states <file.json>");
    const list = JSON.parse(readFileSync(resolve(a[0]), "utf8"));
    let n = 0;
    for (const s of list) {
      if (!/^(0_userdata\.0|javascript\.\d+)\./.test(s.id)) { console.log(`Skipped ${s.id} (only 0_userdata.0.* / javascript.<n>.*)`); continue; }
      const type = s.type || (s.value === undefined ? "mixed" : typeof s.value === "object" ? "json" : typeof s.value);
      const common = { name: s.name || s.id.split(".").pop(), type, role: s.role || "state", read: true, write: s.write !== false };
      if (s.unit) common.unit = s.unit;
      await command("setObject", { id: s.id, obj: { type: "state", common, native: {} } });
      if (s.value !== undefined) await command("setState", { id: s.id, state: { val: typeof s.value === "object" ? JSON.stringify(s.value) : s.value, ack: true } });
      n++;
    }
    print(`Created ${n} states.`);
    break;
  }
  case "screenshot": {
    // Laufzeitansicht rendern, damit der Agent sein Ergebnis prüfen kann
    if (!a[0]) fail("Usage: screenshot <project> [view] [file.png] [--width 1440 --height 900] [--wait 8000] [--dark] [--eval <js>]");
    const file = resolve(a[2] || `vis-${a[0]}${a[1] ? "-" + a[1] : ""}.png`);
    const url = `${web}/vis-2/?${encodeURIComponent(a[0])}${a[1] ? "#" + encodeURIComponent(a[1]) : ""}`;
    const { errors, result } = await screenshot(url, file, {
      width: Number(f.width || 1440), height: Number(f.height || 900), wait: Number(f.wait || 8000), dark: !!f.dark, eval: typeof f.eval === "string" ? f.eval : undefined,
    });
    print([`Saved ${file} (${url})`, result !== undefined ? `eval: ${JSON.stringify(result)}` : "", errors.length ? `Page errors:\n  ${[...new Set(errors)].slice(0, 10).join("\n  ")}` : "No page errors."].filter(Boolean).join("\n"));
    break;
  }
  case "reload": {
    // Alle offenen VIS-2-Seiten neu laden
    const st = (id, value) => http("GET", `${base}/state/${encodeURIComponent(`${adapter}.control.${id}`)}?value=${encodeURIComponent(value)}`, { headers });
    await st("instance", "FFFFFFFF");
    await st("command", "refresh");
    print("Sent refresh to all VIS 2 clients.");
    break;
  }
  default: console.log(`Usage (VIS 2 projects in the ioBroker file storage, via the rest-api adapter):
  projects                                   list projects
  create-project <name> [--view Main]        new project with one empty view
  views <project>                            views with their widgets (id, type, state, position)
  get <project> [view]                       raw JSON of the project or one view
  put <project> <file.json> [--view <name>]  write the whole project or replace one view
  add-view <project> <name> [--from <view>] [--width 1280 --height 800]
  del-view <project> <view>
  widgets [project] [--full]                 widget types used in the projects (use them as templates)
  widget <tpl>                               full example of one widget type
  add-widget <project> <view> <tpl> [--oid <state>] [--x 10 --y 10 --w 100 --h 40] [--data json] [--style json] [--set <widgetSet>]
  set-widget <project> <view> <id> [--oid ..] [--x ..] [--data json] [--style json]   (merges)
  del-widget <project> <view> <id>
  css <project> [file.css]                   read or write vis-user.css
  restore <project>                          undo the last change (backup is written before every write)
  reload                                     reload all open VIS 2 pages
  screenshot <project> [view] [file.png]     render the runtime view in headless Chrome and save a PNG
             [--width 1440 --height 900] [--wait 8000] [--dark] [--eval <js>]   (also lists page errors)

States (for example values and controls):
  states <pattern>                           e.g. states 0_userdata.0.*
  get-state <id>   set-state <id> <value> [--ack]
  create-state <id> [value] [--type number|boolean|string|json] [--name ..] [--unit ..] [--role ..] [--min --max] [--readonly]
  create-states <file.json>                  [{"id","value","type","name","unit","role"}, ...]
                                             (only under 0_userdata.0.* or javascript.<n>.*)

Tips: find states with the 'iobroker' plugin ('search', 'rooms'). Look at existing widgets with 'widgets'
and 'widget <tpl>' before creating new ones; widget types depend on the installed widget sets.`);
}
