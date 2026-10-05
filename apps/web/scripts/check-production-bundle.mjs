// Refuses a production web build that cannot sign in or reach the server, before it is deployed.
//
//   node apps/web/scripts/check-production-bundle.mjs [dist]      (dist defaults to apps/web/dist)
//
// The client reads four public settings at BUILD time (apps/web/.env.example): a bundle built
// without the two Supabase ones answers every sign-in with "Sign-in isn't available on this site right
// now" (src/net/auth.ts), and one built without the two server ones calls localhost:8787. Nothing at
// run time can repair either, so promote-production.yml runs this between the build and the deploy.
//
// It resolves the settings exactly as `vite build` did, with Vite's own loadEnv over apps/web
// (apps/web/.env.production, overridden by any VITE_ variable in the environment), requires each one,
// and requires each value to appear in the bundle's JavaScript. It also requires the files Cloudflare
// serves from the assets root (wrangler.jsonc, public/_redirects and _headers).
//
// Exits 1, listing every problem, when anything is missing.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadEnv } from "vite";

const WEB = fileURLToPath(new URL("..", import.meta.url));
const dist = process.argv[2] ?? join(WEB, "dist");

const REQUIRED = ["VITE_SUPABASE_URL", "VITE_SUPABASE_PUBLISHABLE_KEY", "VITE_SERVER_HTTP_URL", "VITE_SERVER_WS_URL"];
const SERVED = ["index.html", "404.html", "_redirects", "_headers"];

const env = loadEnv("production", WEB, "VITE_");
const problems = [];

for (const file of SERVED) {
  if (!existsSync(join(dist, file))) problems.push(`${file} is missing from ${dist}`);
}

const assets = join(dist, "assets");
const scripts = existsSync(assets) ? readdirSync(assets).filter((name) => name.endsWith(".js")) : [];
if (scripts.length === 0) problems.push(`no JavaScript in ${assets}`);
const bundle = scripts.map((name) => readFileSync(join(assets, name), "utf8")).join("\n");

for (const name of REQUIRED) {
  const value = env[name];
  if (value === undefined || value.length === 0) {
    problems.push(
      `${name} is not set: add it to apps/web/.env.production (it is public) or to the build's environment`,
    );
  } else if (!bundle.includes(value)) {
    problems.push(`${name} is set but its value is not in the bundle: the build did not read it`);
  }
}

if (problems.length > 0) {
  console.error(`check-production-bundle: this build must not be deployed (${problems.length} problem(s)):`);
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}

console.log(`check-production-bundle: ${dist} carries all ${REQUIRED.length} settings and the ${SERVED.length} served files`);
