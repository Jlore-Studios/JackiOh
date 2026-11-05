// apps/admin/.env.production and .env.staging ship to every browser that loads the dashboard, so each
// holds only the two public Supabase settings, and the same project as the web's build of its
// environment: the server checks every token against that project (wrangler.jsonc's SUPABASE_URL).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const ADMIN = join(dirname(fileURLToPath(import.meta.url)), "..");
const WEB = join(ADMIN, "../web");

const KEYS = ["VITE_SUPABASE_URL", "VITE_SUPABASE_PUBLISHABLE_KEY"];

/** What a secret looks like: Supabase's secret key, a service role, a database URL, a key file, Google's client secret. */
const SECRET = /sb_secret_|service_role|postgres(ql)?:\/\/|-----BEGIN|GOCSPX-/u;

function entries(file: string): [string, string][] {
  return readFileSync(file, "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"))
    .map((line) => {
      const at = line.indexOf("=");
      return [line.slice(0, at), line.slice(at + 1)];
    });
}

describe.each([".env.production", ".env.staging"])("apps/admin/%s", (file) => {
  const values = entries(join(ADMIN, file));

  it("holds the two public Supabase settings and nothing else", () => {
    expect(values.map(([key]) => key).sort()).toEqual([...KEYS].sort());
    for (const [key, value] of values) expect(value, key).not.toMatch(SECRET);
    expect(Object.fromEntries(values)["VITE_SUPABASE_PUBLISHABLE_KEY"]).toMatch(/^sb_publishable_[A-Za-z0-9_-]+$/u);
  });

  it("names the web's project for the same build, the one its server checks tokens against", () => {
    const web = Object.fromEntries(entries(join(WEB, file)));
    for (const [key, value] of values) expect(value, key).toBe(web[key]);
  });
});

describe("the dashboard's two builds", () => {
  it("sign in to two projects, so staging's dashboard reaches no production account", () => {
    const url = (file: string): string | undefined => Object.fromEntries(entries(join(ADMIN, file)))["VITE_SUPABASE_URL"];
    expect(url(".env.production")).not.toBe(url(".env.staging"));
  });
});
