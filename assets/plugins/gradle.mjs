// Gradle builds (uses ./gradlew of the project): Android, Minecraft mods, Kotlin Multiplatform, Maven publishing.
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const local = ["gradlew", "../gradlew", "../../gradlew"].map((p) => resolve(p)).find(existsSync);
const bin = local || which(["gradle"]);
const [cmd] = args;
if (cmd === "status" || cmd === "connect") { if (!bin) fail("No gradlew in the project and gradle is not installed."); console.log("Gradle: " + bin); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any Gradle tasks/arguments, e.g.
  'build'  'runClient'  'runServer'  'assembleRelease'  'bundleRelease'  'test'  'publish'  'dependencies --configuration runtimeClasspath'
  'tasks --all' (list tasks)   Android: 'installDebug', 'connectedAndroidTest'   KMP: 'linkDebugFrameworkIosArm64'`); process.exit(0); }
if (!bin) fail("No gradlew found.");
passthrough(bin, ["--console=plain", ...args]);
