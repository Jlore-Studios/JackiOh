/**
 * `src/db/seed-accounts.ts`'s gate. The accounts it makes skip the invite gate, so it must refuse
 * unless the caller names the one project it is meant for, and its password must come from the
 * environment: a password written in this public repo let anyone sign in to the seeded accounts.
 */

import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { AUTH_PASSWORD_MIN_LENGTH } from "../../src/config";
import {
  SEED_PASSWORD_VAR,
  SEED_PROJECT_VAR,
  seedAccountsSettings,
} from "../../src/db/seed-accounts";

const ENV = { SUPABASE_URL: "https://dev-project.supabase.co", NODE_ENV: "development" } as const;
const PASSWORD = "p".repeat(AUTH_PASSWORD_MIN_LENGTH);

describe("seedAccountsSettings", () => {
  it("reads the password from the environment once the named project matches", () => {
    const source = { [SEED_PROJECT_VAR]: "dev-project.supabase.co", [SEED_PASSWORD_VAR]: PASSWORD };
    expect(seedAccountsSettings(source, ENV)).toEqual({ password: PASSWORD });
  });

  it("refuses without the opt-in, and names the host it expects", () => {
    expect(() => seedAccountsSettings({ [SEED_PASSWORD_VAR]: PASSWORD }, ENV)).toThrow(
      /SEED_ACCOUNTS_PROJECT must equal SUPABASE_URL's host \(dev-project\.supabase\.co\)/,
    );
  });

  it("refuses when the opt-in names a different project than SUPABASE_URL", () => {
    const source = { [SEED_PROJECT_VAR]: "prod-project.supabase.co", [SEED_PASSWORD_VAR]: PASSWORD };
    expect(() => seedAccountsSettings(source, ENV)).toThrow(/it is "prod-project\.supabase\.co"/);
  });

  it("refuses a missing or short password", () => {
    const project = { [SEED_PROJECT_VAR]: "dev-project.supabase.co" };
    expect(() => seedAccountsSettings(project, ENV)).toThrow(/SEED_ACCOUNTS_PASSWORD .*it is not set/);
    expect(() =>
      seedAccountsSettings({ ...project, [SEED_PASSWORD_VAR]: "p".repeat(AUTH_PASSWORD_MIN_LENGTH - 1) }, ENV),
    ).toThrow(/at least 12 characters/);
  });

  it("refuses under NODE_ENV=production even with the opt-in", () => {
    const source = { [SEED_PROJECT_VAR]: "dev-project.supabase.co", [SEED_PASSWORD_VAR]: PASSWORD };
    expect(() => seedAccountsSettings(source, { ...ENV, NODE_ENV: "production" })).toThrow(
      /NODE_ENV is production/,
    );
  });

  it("carries no password literal in the script", () => {
    const script = readFileSync(new URL("../../src/db/seed-accounts.ts", import.meta.url), "utf8");
    expect(script).not.toMatch(/const PASSWORD = "/);
    expect(script).not.toMatch(/password: "/);
  });
});
