// RENDER_GIT_COMMIT -> ServerEnv.DEPLOYED_COMMIT (src/env.ts): the commit `GET /api/catalog`
// reports so deploy-watch.yml can compare it with the push. It is optional, never a problem, and
// only a git SHA survives, because the value ends up in a response header.

import { describe, expect, it } from "vitest";

import { SERVER_ONLY_ENV_VARS, loadEnv } from "../src/env";

/** A complete, valid environment for the server (apps/server/README.md's table). */
function validEnv(): Record<string, string> {
  return {
    SUPABASE_URL: "https://project.supabase.test",
    SUPABASE_SECRET_KEY: "sb_secret_0123456789abcdefghijklmnopqrstuv",
    DATABASE_URL: "postgres://postgres:postgres@localhost:5432/jackioh",
    CODE_PEPPER: "a-pepper-of-at-least-thirty-two-characters",
    CATALOG_VERSION: "core-1",
    PUBLIC_ORIGINS: "https://play.jackioh.test",
    NODE_ENV: "test",
  };
}

describe("RENDER_GIT_COMMIT", () => {
  it("is unset, not a problem, when the server is not on Render", () => {
    expect(loadEnv(validEnv()).DEPLOYED_COMMIT).toBeUndefined();
  });

  it("is kept, lower-cased, when it is a git SHA", () => {
    const sha = "71DFDB6CF14B66670D88D6561EC50B5783C4F998";
    expect(loadEnv({ ...validEnv(), RENDER_GIT_COMMIT: ` ${sha}\n` }).DEPLOYED_COMMIT).toBe(sha.toLowerCase());
  });

  it("drops anything that is not a git SHA instead of failing the boot", () => {
    for (const value of ["", "main", "not a sha", "abc123", "71dfdb6\r\nx-injected: 1", "g".repeat(40)]) {
      expect(loadEnv({ ...validEnv(), RENDER_GIT_COMMIT: value }).DEPLOYED_COMMIT, JSON.stringify(value)).toBeUndefined();
    }
  });

  it("is listed with the server-only variables loadEnv reads", () => {
    expect(SERVER_ONLY_ENV_VARS).toContain("RENDER_GIT_COMMIT");
  });
});
