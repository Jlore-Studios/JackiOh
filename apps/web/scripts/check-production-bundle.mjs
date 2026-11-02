// Refuses a production or staging web build that cannot sign in or reach the server, before it is deployed.
//
//   node apps/web/scripts/check-production-bundle.mjs [--staging] [dist]      (dist defaults to apps/web/dist)
//
// The client reads four public settings at BUILD time (apps/web/.env.example): a bundle built
// without the two Supabase ones answers every sign-in with "Sign-in isn't available on this site right
// now" (src/net/auth.ts), and one built without the two server ones calls localhost:8787. Nothing at
// run time can repair either, so promote-production.yml runs this between the build and the deploy,
// and deploy-staging.yml runs it with --staging.
//
// It resolves the settings exactly as the build did: `build`'s with Vite's own loadEnv over apps/web
// (apps/web/.env.production, overridden by any VITE_ variable in the environment); `build:staging`'s
// (--staging) the same, with apps/web/.env.staging's values over the file's, read with the parser
// Node's --env-file uses, and any VITE_ variable the environment already held over both. It requires
// each one, and requires each value to appear in the bundle's JavaScript. It also requires the files
// Cloudflare serves from the assets root (wrangler.jsonc, public/_redirects and _headers), and the
// CSP the build wrote into _headers to let the page reach Supabase and the server's sockets.
//
// Exits 1, listing every problem, when anything is missing.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseEnv } from "node:util";

import { loadEnv } from "vite";

const WEB = fileURLToPath(new URL("..", import.meta.url));
const args = process.argv.slice(2);
const staging = args.includes("--staging");
const dist = args.find((arg) => !arg.startsWith("--")) ?? join(WEB, "dist");
const file = staging ? ".env.staging" : ".env.production";

const REQUIRED = ["VITE_SUPABASE_URL", "VITE_SUPABASE_PUBLISHABLE_KEY", "VITE_SERVER_HTTP_URL", "VITE_SERVER_WS_URL"];
const SERVED = ["index.html", "404.html", "_redirects", "_headers", "sitemap.xml", "almanac/index.html"];

const preset = Object.fromEntries(Object.entries(process.env).filter(([name]) => name.startsWith("VITE_")));
const env = {
  ...loadEnv("production", WEB, "VITE_"),
  ...(staging ? parseEnv(readFileSync(join(WEB, file), "utf8")) : {}),
  ...preset,
};
const problems = [];

for (const served of SERVED) {
  if (!existsSync(join(dist, served))) problems.push(`${served} is missing from ${dist}`);
}

const assets = join(dist, "assets");
const scripts = existsSync(assets) ? readdirSync(assets).filter((name) => name.endsWith(".js")) : [];
if (scripts.length === 0) problems.push(`no JavaScript in ${assets}`);
const bundle = scripts.map((name) => readFileSync(join(assets, name), "utf8")).join("\n");

for (const name of REQUIRED) {
  const value = env[name];
  if (value === undefined || value.length === 0) {
    problems.push(`${name} is not set: add it to apps/web/${file} (it is public) or to the build's environment`);
  } else if (!bundle.includes(value)) {
    problems.push(`${name} is set but its value is not in the bundle: the build did not read it`);
  }
}

// static-pages.ts fills the CSP's connect-src in for the build; a page it does not allow to reach
// Supabase or the sockets cannot sign in or play.
const headers = existsSync(join(dist, "_headers")) ? readFileSync(join(dist, "_headers"), "utf8") : "";
const connect = (/connect-src ([^;]+)/u.exec(headers)?.[1] ?? "").split(" ");
for (const name of ["VITE_SUPABASE_URL", "VITE_SERVER_WS_URL"]) {
  const value = env[name];
  if (headers === "" || value === undefined || value.length === 0) continue;
  const origin = new URL(value).origin;
  if (!connect.includes(origin)) problems.push(`_headers' connect-src does not allow ${origin} (${name})`);
}
const left = /%[A-Z][A-Z0-9_]*%/u.exec(headers)?.[0];
if (left !== undefined) problems.push(`_headers still holds ${left}, which the build did not fill`);

if (problems.length > 0) {
  console.error(`check-production-bundle: this build must not be deployed (${problems.length} problem(s)):`);
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}

console.log(
  `check-production-bundle: ${dist} carries all ${REQUIRED.length} settings of apps/web/${file}, the ${SERVED.length} served files and a CSP that reaches them`,
);
