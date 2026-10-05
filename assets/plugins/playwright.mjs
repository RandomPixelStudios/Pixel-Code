// Browser automation with Playwright (installed on first use in ~/.local/share/pixel-code/playwright).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { args, fail } from "./common.mjs";

const dir = join(homedir(), ".local/share/pixel-code/playwright");
const ready = () => existsSync(join(dir, "node_modules/playwright"));
function install() {
  mkdirSync(dir, { recursive: true });
  if (!existsSync(join(dir, "package.json"))) writeFileSync(join(dir, "package.json"), '{"private":true}');
  let r = spawnSync("npm", ["install", "playwright@latest"], { cwd: dir, stdio: "ignore" });
  if (r.status !== 0) fail("npm install playwright failed");
  r = spawnSync("npx", ["playwright", "install", "chromium"], { cwd: dir, stdio: "ignore" });
  if (r.status !== 0) fail("Installing Chromium failed");
}

const [cmd, ...rest] = args;
if (cmd === "status") { if (!ready()) fail("Not installed yet - press Connect"); console.log("Playwright + Chromium ready"); process.exit(0); }
if (cmd === "connect") { if (!ready()) install(); console.log("Playwright + Chromium ready"); process.exit(0); }
if (!cmd || cmd === "help") {
  console.log(`Usage:
  screenshot <url> [file.png] [--full]   screenshot of a page
  text <url>                             visible text of a page (after JavaScript)
  script <file.mjs>                      run your own script: export default async ({ page, browser }) => { ... }`);
  process.exit(0);
}
if (!ready()) fail("Playwright is not installed - press Connect on the Playwright plugin.");
const { chromium } = await import(join(dir, "node_modules/playwright/index.mjs"));
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
try {
  if (cmd === "screenshot") {
    await page.goto(rest[0], { waitUntil: "networkidle" });
    const file = resolve(rest.find((x, i) => i > 0 && !x.startsWith("--")) || "screenshot.png");
    await page.screenshot({ path: file, fullPage: rest.includes("--full") });
    console.log(file);
  } else if (cmd === "text") {
    await page.goto(rest[0], { waitUntil: "networkidle" });
    console.log((await page.innerText("body")).slice(0, 30000));
  } else if (cmd === "script") {
    const mod = await import(resolve(rest[0]));
    const out = await mod.default({ page, browser });
    if (out !== undefined) console.log(typeof out === "string" ? out : JSON.stringify(out, null, 2));
  } else fail(`Unknown command ${cmd}`);
} finally {
  await browser.close();
}
