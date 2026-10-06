// Weather - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const [cmd, ...a] = rest;
async function place(name) { const r = await http("GET", `https://geocoding-api.open-meteo.com/v1/search?count=1&name=${encodeURIComponent(name || settings.city || "Berlin")}`); if (!r.results?.length) fail("City not found."); return r.results[0]; }
if (cmd === "status" || cmd === "connect") console.log("Open-Meteo");
else if (cmd === "now") { const p = await place(a.join(" ")); const r = await http("GET", `https://api.open-meteo.com/v1/forecast?latitude=${p.latitude}&longitude=${p.longitude}&current=temperature_2m,apparent_temperature,precipitation,wind_speed_10m,weather_code`); print(`${p.name}, ${p.country}: ${JSON.stringify(r.current)}`); }
else if (cmd === "forecast") { const p = await place(a.join(" ")); const r = await http("GET", `https://api.open-meteo.com/v1/forecast?latitude=${p.latitude}&longitude=${p.longitude}&daily=temperature_2m_max,temperature_2m_min,precipitation_sum&forecast_days=${f.days || 7}&timezone=auto`); print(r.daily.time.map((t, i) => `${t}  ${r.daily.temperature_2m_min[i]}..${r.daily.temperature_2m_max[i]} C  rain ${r.daily.precipitation_sum[i]} mm`).join("\n")); }
else console.log("Usage:\n  now [city]\n  forecast [city] [--days 7]");
