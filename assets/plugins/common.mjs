// Shared helpers for Pixel Code plugin scripts. Settings arrive as JSON in PC_SETTINGS.
import { existsSync, readdirSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { delimiter, join } from "node:path";
import { homedir } from "node:os";

export const settings = (() => {
  try { return JSON.parse(process.env.PC_SETTINGS || "{}"); } catch { return {}; }
})();

export const args = process.argv.slice(2);

export function fail(msg) {
  console.log(msg);
  process.exit(1);
}

/** Finds an executable by name in PATH and a few extra directories. */
export function which(names, extra = []) {
  const dirs = [...(process.env.PATH || "").split(delimiter), ...extra, join(homedir(), ".local/bin"), "/snap/bin", "/usr/local/bin", "/opt"];
  for (const n of names) for (const d of dirs) {
    const p = join(d, n);
    try { if (statSync(p).isFile()) return p; } catch {}
  }
  return null;
}

/** Newest subdirectory (by name, version-like sort). */
export function newest(dir) {
  try {
    return readdirSync(dir).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }))[0] ?? null;
  } catch { return null; }
}

/** Runs a program and forwards its output; exits with its exit code. */
export function passthrough(bin, argv) {
  const r = spawnSync(bin, argv, { stdio: "inherit", timeout: 30 * 60 * 1000 });
  if (r.error) fail(`Failed to run ${bin}: ${r.error.message}`);
  process.exit(r.status ?? 1);
}

/** Parses `--key value` flags; returns [flags, rest]. */
export function flags(list) {
  const f = {}, rest = [];
  for (let i = 0; i < list.length; i++) {
    if (list[i].startsWith("--")) f[list[i].slice(2)] = list[i + 1]?.startsWith("--") || list[i + 1] === undefined ? true : list[++i];
    else rest.push(list[i]);
  }
  return [f, rest];
}

export const exists = existsSync;

// ---------------------------------------------------------------- HTTP

/** JSON request; exits with a readable error on HTTP errors. */
export async function http(method, url, { headers = {}, body, raw = false, form } = {}) {
  const init = { method, headers: { Accept: "application/json", ...headers } };
  if (form) {
    init.body = new URLSearchParams(form);
    init.headers["Content-Type"] = "application/x-www-form-urlencoded";
  } else if (body !== undefined) {
    init.body = typeof body === "string" || body instanceof Uint8Array ? body : JSON.stringify(body);
    if (!init.headers["Content-Type"]) init.headers["Content-Type"] = "application/json";
  }
  let res;
  try { res = await fetch(url, init); } catch (e) { fail(`Request to ${url} failed: ${e.cause?.message || e.message}`); }
  if (raw) {
    if (!res.ok) fail(`HTTP ${res.status}: ${(await res.text()).slice(0, 500)}`);
    return res;
  }
  const text = await res.text();
  let data;
  try { data = text ? JSON.parse(text) : {}; } catch { data = text; }
  if (!res.ok) fail(`HTTP ${res.status}: ${typeof data === "string" ? data.slice(0, 500) : JSON.stringify(data).slice(0, 800)}`);
  return data;
}

export const print = (x) => console.log(typeof x === "string" ? x : JSON.stringify(x, null, 2));

/** `raw <METHOD> <path> [json]` helper shared by API plugins. */
export async function rawCall(base, headers, list) {
  const [method, path, ...json] = list;
  if (!method || !path) fail("Usage: raw <GET|POST|PUT|PATCH|DELETE> <path> [json body]");
  print(await http(method.toUpperCase(), path.startsWith("http") ? path : base + path, { headers, body: json.length ? JSON.parse(json.join(" ")) : undefined }));
}
