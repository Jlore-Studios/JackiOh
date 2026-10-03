/**
 * Player settings on the account (`src/api/settings.ts`, SPEC §9.1, R633, R634).
 *
 * The questions here are the endpoints' own: who may call them (an active account, and only about
 * itself), what a body must look like, what a write does to what is stored (a group replaced only by
 * a strictly later one, the others left alone), and that a time from a clock running ahead is taken
 * as now. The merge itself is the store's and is asserted against both stores in
 * `test/db/contract.ts`; the database's half — RLS, the grants and the SQL function — is
 * `test/sql/11_player_settings.sql` and `02_rls_as_client.sql`.
 */

import { beforeEach, describe, expect, it } from "vitest";

import { createRouter, type Router } from "../../src/api/http";
import { createSettingsRoutes, type PlayerSettingsView } from "../../src/api/settings";
import {
  PLAYER_SETTINGS_BYTES_MAX,
  PLAYER_SETTINGS_GROUPS_MAX,
  PLAYER_SETTINGS_KEYS_MAX,
  PLAYER_SETTINGS_NAME_MAX_LENGTH,
  PLAYER_SETTINGS_TEXT_MAX_LENGTH,
} from "../../src/config";
import { createTestDeps, jsonRequest, readJson, type TestDeps } from "../fakes/deps";

type SettingsBody = { settings: PlayerSettingsView };
type ErrorBody = { error: { code: string; message: string; details?: unknown } };

const PROFILE = "p1";
const OTHER = "p2";

let deps: TestDeps;
let router: Router;
let token: string;
let otherToken: string;

function profileWith(target: TestDeps, id: string, status: "active" | "pending" | "banned"): string {
  const userId = `user-${id}`;
  target.store.seedProfile({ id, userId, status });
  return target.auth.addUser({ userId, email: `${id}@example.test` });
}

function get(bearer = token): Promise<Response> {
  return router(jsonRequest("GET", "/api/settings", undefined, { token: bearer }));
}

function put(body: unknown, bearer = token): Promise<Response> {
  return router(jsonRequest("PUT", "/api/settings", body, { token: bearer }));
}

async function settingsOf(response: Response): Promise<PlayerSettingsView> {
  expect(response.status).toBe(200);
  return (await readJson<SettingsBody>(response)).settings;
}

beforeEach(() => {
  deps = createTestDeps();
  router = createRouter(createSettingsRoutes(), deps);
  token = profileWith(deps, PROFILE, "active");
  otherToken = profileWith(deps, OTHER, "active");
});

describe("R633 the routes: an active account, and only about itself", () => {
  it("R633 declares GET and PUT /api/settings, both `active`", () => {
    const routes = createSettingsRoutes();
    expect(routes.map((entry) => `${entry.method} ${entry.path}`)).toEqual(["GET /api/settings", "PUT /api/settings"]);
    expect(routes.every((entry) => entry.auth === "active")).toBe(true);
  });

  it("R633 refuses a caller with no token (401), and a pending or banned account (403), writing nothing", async () => {
    const pending = profileWith(deps, "pending", "pending");
    const banned = profileWith(deps, "banned", "banned");
    const body = { groups: { audio: { at: 1, values: { master: 0.5 } } } };

    const noToken = await router(jsonRequest("PUT", "/api/settings", body));
    expect(noToken.status).toBe(401);
    for (const bearer of [pending, banned]) {
      expect((await get(bearer)).status).toBe(403);
      expect((await put(body, bearer)).status).toBe(403);
    }
    expect(deps.store.tables.playerSettings).toEqual([]);
  });

  it("R633 reads an account with no settings yet as none, and writes land on the caller's own row only", async () => {
    expect(await settingsOf(await get())).toEqual({ groups: {} });

    // A body naming another profile changes nothing about whose row is written: the profile is the
    // verified token's, never a field of the request.
    await settingsOf(await put({ groups: { audio: { at: 5, values: { master: 0.5 } } }, profileId: OTHER }));
    expect(await settingsOf(await get())).toEqual({ groups: { audio: { at: 5, values: { master: 0.5 } } } });
    expect(await settingsOf(await get(otherToken))).toEqual({ groups: {} });
    expect(deps.store.tables.playerSettings.map((row) => row.profileId)).toEqual([PROFILE]);
  });
});

describe("R634 a write replaces a group only with a strictly later one", () => {
  it("R634 a later group replaces the stored one whole; an older or tied one changes nothing; the answer is what is stored", async () => {
    const t = deps.timers.now() - 60_000;
    await settingsOf(await put({ groups: { audio: { at: t, values: { master: 0.5, muted: false } } } }));

    const later = await settingsOf(await put({ groups: { audio: { at: t + 1000, values: { master: 0.9 } } } }));
    expect(later.groups["audio"]).toEqual({ at: t + 1000, values: { master: 0.9 } });

    // A device that last changed the audio before that, and one that did so at the same moment.
    const older = await settingsOf(await put({ groups: { audio: { at: t + 500, values: { master: 0.1 } } } }));
    expect(older).toEqual(later);
    const tied = await settingsOf(await put({ groups: { audio: { at: t + 1000, values: { master: 0.2 } } } }));
    expect(tied).toEqual(later);
    expect(await settingsOf(await get())).toEqual(later);
  });

  it("R634 a group a write does not name stays, and one write can win one group and lose another", async () => {
    const t = deps.timers.now() - 60_000;
    await settingsOf(
      await put({ groups: { gameplay: { at: t + 100, values: { dragToPlay: false } }, audio: { at: t, values: { master: 0.4 } } } }),
    );

    const merged = await settingsOf(
      await put({ groups: { audio: { at: t + 200, values: { master: 0.7 } }, gameplay: { at: t, values: { dragToPlay: true } } } }),
    );

    expect(merged.groups["audio"]).toEqual({ at: t + 200, values: { master: 0.7 } });
    expect(merged.groups["gameplay"]).toEqual({ at: t + 100, values: { dragToPlay: false } });
    // And a write that names neither leaves both.
    expect(await settingsOf(await put({ groups: { fx: { at: t, values: { speed: 2 } } } }))).toMatchObject({
      groups: { audio: { at: t + 200 }, gameplay: { at: t + 100 }, fx: { at: t } },
    });
    expect(await settingsOf(await put({ groups: {} }))).toMatchObject({ groups: { audio: {}, gameplay: {}, fx: {} } });
  });

  it("R634 takes a group timed after the server's clock as made now, so a clock running ahead cannot pin it", async () => {
    const now = deps.timers.now();
    const ahead = await settingsOf(await put({ groups: { audio: { at: now + 86_400_000, values: { master: 0.1 } } } }));
    expect(ahead.groups["audio"]).toEqual({ at: now, values: { master: 0.1 } });

    // A minute later, a change from a device whose clock is right wins over it.
    deps.timers.advance(60_000);
    const later = await settingsOf(await put({ groups: { audio: { at: now + 60_000, values: { master: 0.8 } } } }));
    expect(later.groups["audio"]).toEqual({ at: now + 60_000, values: { master: 0.8 } });
  });

  it("R634 keeps a group the client no longer has, and hands back every value it was given", async () => {
    const values = { on: true, level: 0.25, station: "lofi", count: 3, negative: -2 };
    const answer = await settingsOf(await put({ groups: { "old-store": { at: 1, values } } }));
    expect(answer.groups["old-store"]).toEqual({ at: 1, values });
  });
});

describe("R633 the body is checked before anything is stored", () => {
  async function refused(body: unknown): Promise<ErrorBody> {
    const response = await put(body);
    expect(response.status).toBe(400);
    return readJson<ErrorBody>(response);
  }

  it("R633 refuses a body whose groups are not an object", async () => {
    for (const body of [{}, { groups: null }, { groups: [] }, { groups: "audio" }, { groups: 1 }]) {
      expect((await refused(body)).error.code).toBe("bad_request");
    }
    expect(deps.store.tables.playerSettings).toEqual([]);
  });

  it("R633 refuses a group whose id is not a lower-case slug of a bounded length", async () => {
    const group = { at: 1, values: {} };
    const tooLong = "a".repeat(PLAYER_SETTINGS_NAME_MAX_LENGTH + 1);
    for (const id of ["Audio", "a b", "1a", "-a", "a--b", "a-", "", tooLong]) {
      await refused({ groups: { [id]: group } });
    }
    expect((await settingsOf(await put({ groups: { ["a".repeat(PLAYER_SETTINGS_NAME_MAX_LENGTH)]: group } }))).groups).toBeDefined();
  });

  it("R633 refuses a group that is not { at, values } with a whole-millisecond time", async () => {
    for (const group of [null, 1, [], "x", {}, { at: 1 }, { values: {} }, { at: "1", values: {} }, { at: -1, values: {} }, { at: 1.5, values: {} }, { at: Number.MAX_SAFE_INTEGER + 2, values: {} }, { at: 1, values: [] }, { at: 1, values: null }]) {
      expect((await refused({ groups: { audio: group } })).error.code).toBe("bad_request");
    }
    expect(deps.store.tables.playerSettings).toEqual([]);
  });

  it("R633 refuses values that are not flat booleans, finite numbers and short texts, and names that are not words", async () => {
    const tooLong = "x".repeat(PLAYER_SETTINGS_TEXT_MAX_LENGTH + 1);
    for (const value of [null, [], {}, [1], { a: 1 }, tooLong]) {
      await refused({ groups: { audio: { at: 1, values: { master: value } } } });
    }
    for (const name of ["", "1a", "a b", "a-b", "a.b", "__proto__x!", "x".repeat(PLAYER_SETTINGS_NAME_MAX_LENGTH + 1)]) {
      await refused({ groups: { audio: { at: 1, values: { [name]: 1 } } } });
    }
    // JSON cannot carry NaN or Infinity, so a number the parser keeps is finite; the longest text is fine.
    const longest = "x".repeat(PLAYER_SETTINGS_TEXT_MAX_LENGTH);
    expect((await settingsOf(await put({ groups: { audio: { at: 1, values: { station: longest } } } }))).groups["audio"]?.values).toEqual({
      station: longest,
    });
  });

  it("R633 refuses more groups, and more settings in a group, than an account holds", async () => {
    const many = Object.fromEntries(
      Array.from({ length: PLAYER_SETTINGS_GROUPS_MAX + 1 }, (_, at) => [`group-${String(at)}`, { at: 1, values: {} }]),
    );
    expect((await refused({ groups: many })).error.message).toContain(String(PLAYER_SETTINGS_GROUPS_MAX));
    const values = Object.fromEntries(Array.from({ length: PLAYER_SETTINGS_KEYS_MAX + 1 }, (_, at) => [`k${String(at)}`, true]));
    expect((await refused({ groups: { audio: { at: 1, values } } })).error.message).toContain(String(PLAYER_SETTINGS_KEYS_MAX));
    const fits = Object.fromEntries(Array.from({ length: PLAYER_SETTINGS_KEYS_MAX }, (_, at) => [`k${String(at)}`, true]));
    expect(Object.keys((await settingsOf(await put({ groups: { audio: { at: 1, values: fits } } }))).groups["audio"]?.values ?? {})).toHaveLength(
      PLAYER_SETTINGS_KEYS_MAX,
    );
  });

  it("R633 answers a result past the account's caps with 409 and writes nothing", async () => {
    // Groups: fill the account to its group cap, then name one more.
    const filled = Object.fromEntries(
      Array.from({ length: PLAYER_SETTINGS_GROUPS_MAX }, (_, at) => [`group-${String(at)}`, { at: 1, values: { on: true } }]),
    );
    await settingsOf(await put({ groups: filled }));
    const response = await put({ groups: { "one-more": { at: 1, values: {} } } });
    expect(response.status).toBe(409);
    expect((await readJson<ErrorBody>(response)).error.details).toEqual({
      groups: PLAYER_SETTINGS_GROUPS_MAX,
      bytes: PLAYER_SETTINGS_BYTES_MAX,
    });
    expect(Object.keys((await settingsOf(await get())).groups)).not.toContain("one-more");
  });

  it("R633 answers a result past the byte cap with 409, and a group already held can still be replaced", async () => {
    const text = "x".repeat(PLAYER_SETTINGS_TEXT_MAX_LENGTH);
    const wide = (prefix: string): Record<string, unknown> =>
      Object.fromEntries(Array.from({ length: PLAYER_SETTINGS_KEYS_MAX }, (_, at) => [`${prefix}${String(at)}`, text]));
    // Each of these groups is about 1.7 kB of text, so the third pushes the account past 4 kB.
    await settingsOf(await put({ groups: { first: { at: 1, values: wide("a") } } }));
    await settingsOf(await put({ groups: { second: { at: 1, values: wide("b") } } }));
    expect((await put({ groups: { third: { at: 1, values: wide("c") } } })).status).toBe(409);
    // Replacing a held group with a smaller one is fine.
    const shrunk = await settingsOf(await put({ groups: { first: { at: 2, values: { a0: true } } } }));
    expect(shrunk.groups["first"]).toEqual({ at: 2, values: { a0: true } });
  });
});
