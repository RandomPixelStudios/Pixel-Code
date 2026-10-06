// Latest Minecraft, Fabric, Quilt, NeoForge and Forge versions (no account needed).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const get = (u) => http("GET", u);
const xmlVersions = async (u) => [...(await (await fetch(u)).text()).matchAll(/<version>([^<]+)<\/version>/g)].map((m) => m[1]);
const [cmd, mc] = args;
switch (cmd) {
  case "connect": case "status": { const m = await get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"); console.log(`Latest Minecraft ${m.latest.release} (snapshot ${m.latest.snapshot})`); break; }
  case "minecraft": { const m = await get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"); print(m.versions.filter((v) => v.type === "release").slice(0, 25).map((v) => v.id).join(", ")); break; }
  case "fabric": {
    const game = mc || (await get("https://meta.fabricmc.net/v2/versions/game")).find((v) => v.stable).version;
    const loader = (await get("https://meta.fabricmc.net/v2/versions/loader"))[0].version;
    const yarn = (await get(`https://meta.fabricmc.net/v2/versions/yarn/${game}`))[0]?.version;
    const api = (await xmlVersions("https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/maven-metadata.xml")).filter((v) => v.endsWith("+" + game)).pop();
    print(`minecraft_version=${game}\nloader_version=${loader}\nyarn_mappings=${yarn ?? "?"}\nfabric_version=${api ?? "(no Fabric API for this version yet)"}`);
    break;
  }
  case "quilt": { const l = await get("https://meta.quiltmc.org/v3/versions/loader"); print(`quilt_loader=${l[0].version}`); break; }
  case "neoforge": {
    const all = await xmlVersions("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml");
    const want = mc ? mc.replace(/^1\./, "") : null;
    const list = want ? all.filter((v) => v.startsWith(want + ".") || v.startsWith(want.split(".")[0] + "." + (want.split(".")[1] || "0") + ".")) : all;
    print(`neo_version=${list.pop() ?? "?"}`);
    break;
  }
  case "forge": { const p = await get("https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json"); const k = mc ? [`${mc}-recommended`, `${mc}-latest`] : Object.keys(p.promos).slice(-2); print(k.filter((x) => p.promos[x]).map((x) => `${x}: ${p.promos[x]}`).join("\n") || "No Forge build for " + mc); break; }
  case "parchment": { const v = await xmlVersions(`https://maven.parchmentmc.org/org/parchmentmc/data/parchment-${mc}/maven-metadata.xml`).catch(() => []); print(v.pop() ?? "none"); break; }
  default: console.log(`Usage:
  minecraft                   recent releases
  fabric [mc]                 loader, yarn and Fabric API for a Minecraft version (gradle.properties format)
  quilt | neoforge [mc] | forge [mc] | parchment <mc>`);
}
