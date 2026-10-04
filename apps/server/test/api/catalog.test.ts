// The catalog and validator bindings (src/api/catalog.ts, src/api/loadout-validator.ts): the two
// places the server reaches data and rules that live in other packages (SPEC §9.4).
//
// Two SPEC §11 rulings live here as well:
//   * R163 — the catalog endpoint a client that ships none can read: whole, unprojected,
//     unauthenticated, carrying R105's version.
//   * R164 — where L6's ban list lives: server state, never a flag on a card definition, read
//     through the catalog handle.

import { readFile } from "node:fs/promises";

import { describe, expect, it } from "vitest";

import {
  CatalogUnavailableError,
  DEPLOYED_COMMIT_HEADER,
  catalogFrom,
  catalogUrl,
  createCatalogRoutes,
  loadCatalog,
  loadCurrentPatch,
  versionOf,
} from "../../src/api/catalog";
import { createRouter, ok, route } from "../../src/api/http";
import type { CardDefs } from "@jackioh/shared";
import type { CatalogInfo } from "../../src/api/ports";
import { sharedLoadoutValidator } from "../../src/api/loadout-validator";
import { createTestDeps, jsonRequest, readJson } from "../fakes/deps";

/** The catalog file the server loads, as raw defs, so the tests below count it and not a transcription. */
async function catalogFile(): Promise<CardDefs> {
  return JSON.parse(await readFile(catalogUrl(), "utf8")) as CardDefs;
}

describe("catalog", () => {
  it("loads packages/cards/catalog.json through the workspace link", async () => {
    expect(catalogUrl().pathname).toContain("packages/cards/catalog.json");
    const catalog = await loadCatalog();
    // §8, §7 and patch v0.2.0 (B2.1): every entry the shipped file holds, across its sets.
    expect([...catalog.cardIds].sort()).toEqual(Object.keys(await catalogFile()).sort());
    expect(catalog.defs["core-001"]?.name.length).toBeGreaterThan(0);
  });

  it("derives a version from the catalog's own bytes, so it cannot drift from the data", () => {
    expect(versionOf("{}")).toMatch(/^c1-[0-9a-f]{12}$/);
    expect(versionOf("{}")).toBe(versionOf("{}"));
    expect(versionOf("{}")).not.toBe(versionOf("{ }"));
  });

  it("lets the environment pin the version instead (§9.4: both halves must agree)", async () => {
    const catalog = await loadCatalog({ version: "core-2026-09" });
    expect(catalog.version).toBe("core-2026-09");
  });

  it("marks tokens as tokens (§9.4 L3: no Token-tagged cards in a deck)", async () => {
    const catalog = await loadCatalog();
    const raw = await catalogFile();
    const tokens = catalog.cardIds.filter((id) => catalog.isToken(id));
    expect(new Set(tokens)).toEqual(new Set(Object.keys(raw).filter((id) => raw[id]?.token === true)));
  });

  it("refuses to invent a catalog when the file is missing or malformed", async () => {
    await expect(loadCatalog({ url: new URL("file:///nope/catalog.json") })).rejects.toThrow(
      CatalogUnavailableError,
    );
  });

  it("catalogFrom keeps the ban hook available for L6", () => {
    const catalog = catalogFrom({}, "v0");
    expect(catalog.cardIds).toEqual([]);
    expect(catalog.isBanned("core-001")).toBe(false);
  });
});

describe("loadout validator binding (§9.4: one module, shared)", () => {
  it("adapts the shared module's verdict without restating a rule", async () => {
    const catalog = await loadCatalog();
    const legal = catalog.cardIds.filter((id) => !catalog.isToken(id)).slice(0, 60);
    const owned = new Map(legal.map((cardId) => [cardId, 1] as const));

    const issues = sharedLoadoutValidator({
      decks: [legal.slice(0, 20), legal.slice(20, 40), legal.slice(40, 60)],
      catalogVersion: catalog.version,
      catalog,
      owned,
    });
    expect(issues).toEqual([]);

    // One deck short of DECK_SIZE: the shared module names the rule, the deck and the card.
    const short = sharedLoadoutValidator({
      decks: [legal.slice(0, 19), legal.slice(20, 40), legal.slice(40, 60)],
      catalogVersion: catalog.version,
      catalog,
      owned,
    });
    expect(short.map((issue) => issue.rule)).toContain("L2");
    expect(short[0]?.message.length).toBeGreaterThan(0);
  });

  it("R253 checks one Best-of-1 deck on the one-deck rules when the port asks for `scope: \"deck\"`", async () => {
    const catalog = await loadCatalog();
    const legal = catalog.cardIds.filter((id) => !catalog.isToken(id)).slice(0, 20);
    const owned = new Map(legal.map((cardId) => [cardId, 1] as const));
    const input = { catalogVersion: catalog.version, catalog, owned };

    // One legal deck: as a trio it fails L1 (one deck, not three); as a deck it passes.
    expect(sharedLoadoutValidator({ ...input, decks: [legal] }).map((issue) => issue.rule)).toEqual(["L1"]);
    expect(sharedLoadoutValidator({ ...input, decks: [legal], scope: "deck" })).toEqual([]);

    // One card short: L2, naming the deck by the name the player gave it.
    const short = sharedLoadoutValidator({
      ...input,
      decks: [legal.slice(0, 19)],
      names: ["Midrange"],
      scope: "deck",
    });
    expect(short.map((issue) => issue.rule)).toEqual(["L2"]);
    expect(short[0]?.deck).toBe(1);
    expect(short[0]?.message).toContain("Midrange");
  });

  it("R253 names a trio's decks in its messages when the port passes their names", async () => {
    const catalog = await loadCatalog();
    const legal = catalog.cardIds.filter((id) => !catalog.isToken(id)).slice(0, 60);
    const owned = new Map(legal.map((cardId) => [cardId, 1] as const));
    const shared = legal[0] ?? "";
    const issues = sharedLoadoutValidator({
      decks: [legal.slice(0, 20), [shared, ...legal.slice(21, 40)], legal.slice(40, 60)],
      names: ["Aggro", "Control", "Tempo"],
      catalogVersion: catalog.version,
      catalog,
      owned,
    });
    const l4 = issues.find((issue) => issue.rule === "L4");
    expect(l4?.cardId).toBe(shared);
    expect(l4?.message).toContain("Aggro");
    expect(l4?.message).toContain("Control");
    expect(l4?.message).not.toContain("Deck 1");
  });

  it("reports a card the profile does not own (L5) rather than silently allowing it", async () => {
    const catalog = await loadCatalog();
    const legal = catalog.cardIds.filter((id) => !catalog.isToken(id)).slice(0, 60);
    const issues = sharedLoadoutValidator({
      decks: [legal.slice(0, 20), legal.slice(20, 40), legal.slice(40, 60)],
      catalogVersion: catalog.version,
      catalog,
      owned: new Map(),
    });
    expect(issues.map((issue) => issue.rule)).toContain("L5");
  });

  it("R141 makes L5 unreachable on its own, given R111's launch grant of one copy of each", async () => {
    const catalog = await loadCatalog();
    const legal = catalog.cardIds.filter((id) => !catalog.isToken(id)).slice(0, 60);
    // R111's launch grant, exactly: one copy of every non-token card, which is what every active
    // profile owns. The test above owns *nothing*, which R111 makes impossible — so L5 on its own
    // is only reachable there, never in a real collection.
    const owned = new Map(catalog.cardIds.filter((id) => !catalog.isToken(id)).map((id) => [id, 1] as const));
    const token = catalog.cardIds.find((id) => catalog.isToken(id)) ?? "";
    const decks = (first: readonly string[]): string[][] => [
      [...first],
      legal.slice(20, 40),
      legal.slice(40, 60),
    ];

    const ways = [
      // A second copy of a card: L3 (MAX_COPIES) is broken before L5 can be.
      { name: "a repeated card", decks: decks([...legal.slice(0, 19), legal[0] ?? ""]) },
      // The same card in two decks: L4.
      { name: "a card in two decks", decks: [legal.slice(0, 20), [legal[0] ?? "", ...legal.slice(21, 40)], legal.slice(40, 60)] },
      // A Token, and an id the catalog does not have: L3 and L6.
      { name: "a token", decks: decks([...legal.slice(0, 19), token]) },
      { name: "an unknown id", decks: decks([...legal.slice(0, 19), "core-does-not-exist"]) },
    ];

    let sawL5 = false;
    for (const way of ways) {
      const rules = sharedLoadoutValidator({
        decks: way.decks,
        catalogVersion: catalog.version,
        catalog,
        owned,
      }).map((issue) => issue.rule);

      expect(rules, `${way.name} should be refused`).not.toEqual([]);
      // The ruling itself: whenever L5 fires against an R111 collection, the rule that made it
      // reachable fired too. Change R111's quantity or MAX_COPIES and this is the test that goes red.
      if (rules.includes("L5")) {
        sawL5 = true;
        expect(rules.filter((rule) => rule !== "L5"), `${way.name}: L5 was the only rule`).not.toEqual([]);
      }
    }
    // Otherwise the loop above would prove the claim by never reaching L5 at all.
    expect(sawL5, "no case reached L5, so this proves nothing").toBe(true);
  });
});

// ---------------------------------------------------------------------------
// R163 — "The catalog a client that ships none can read"
// ---------------------------------------------------------------------------

type CatalogBody = { version: string; defs: CardDefs };

describe("R163 — the catalog endpoint (§9.1, §9.4, R105)", () => {
  it("R163 serves the whole, unprojected catalog to a caller with no account at all", async () => {
    const catalog = await loadCatalog();
    const deps = createTestDeps({ catalog });
    // A second, `active` route on the same router, so "the anonymous call worked" is not just
    // "this router lets everybody through": §9.4's gate has to be demonstrably awake.
    const router = createRouter(
      [...createCatalogRoutes(), route("GET", "/api/collection", "active", async () => ok({}))],
      deps,
    );

    // PREMISE: the gate is on. The same request with no token is refused by the guarded route.
    const gated = await router(jsonRequest("GET", "/api/collection"));
    expect(gated.status).toBe(401);

    const response = await router(jsonRequest("GET", "/api/catalog"));
    expect(response.status).toBe(200);
    const body = await readJson<CatalogBody>(response);

    // R105's version, so a stale client learns it is stale before it builds a deck.
    expect(body.version).toBe(catalog.version);
    expect(body.version).toMatch(/^c1-[0-9a-f]{12}$/);

    // Whole: every entry the catalog holds, every one of them.
    expect(body.defs).toEqual(catalog.defs);

    // Unprojected: not one field is trimmed off a card on the way out. A trimmed card would be a
    // second, weaker copy of the catalog, and the deckbuilder's verdict (UX) would stop being the
    // verdict the save runs (law).
    for (const cardId of Object.keys(catalog.defs)) {
      expect(Object.keys(body.defs[cardId] ?? {}).sort()).toEqual(
        Object.keys(catalog.defs[cardId] ?? {}).sort(),
      );
    }
  });

  it('R163 declares `auth: "none"`, like the file it stands in for', () => {
    const routes = createCatalogRoutes();
    expect(routes.map((r) => `${r.method} ${r.path}`)).toEqual(["GET /api/catalog", "GET /api/catalog/:version"]);
    // "The same bytes for everybody, naming no profile": §9.4's gate is about collection, loadout,
    // queue and match, and card data is none of those. A patch's snapshot is the same (R388).
    expect(routes.map((r) => r.auth)).toEqual(["none", "none"]);
  });

  it("R163 hands a pending account and an anonymous caller the identical bytes", async () => {
    const catalog = await loadCatalog();
    const deps = createTestDeps({ catalog });
    const router = createRouter(createCatalogRoutes(), deps);
    deps.store.seedProfile({ id: "pending", userId: "user-pending", status: "pending" });
    const token = deps.auth.addUser({ userId: "user-pending", email: "pending@example.test" });

    const anonymous = await router(jsonRequest("GET", "/api/catalog"));
    const pending = await router(jsonRequest("GET", "/api/catalog", undefined, { token }));

    expect(anonymous.status).toBe(200);
    expect(pending.status).toBe(200);
    // It names no profile, so it cannot differ by one.
    expect(await pending.text()).toBe(await anonymous.text());
  });
});

// ---------------------------------------------------------------------------
// R388 — card patch history: every patch's catalog, served by version
// ---------------------------------------------------------------------------

describe("R388 — GET /api/catalog/:version serves the catalog as each patch left it (B4.2)", () => {
  const PATCHES = new URL("../../../../packages/cards/patches/", import.meta.url);

  async function snapshotFile(version: string): Promise<CardDefs> {
    return JSON.parse(await readFile(new URL(`${version}.json`, PATCHES), "utf8")) as CardDefs;
  }

  it("R388 serves every patch in patches.json, whole, to a caller with no account", async () => {
<<<<<<< HEAD
    const catalog = await loadCatalog({ version: "v0.2.2" });
=======
    // The stamp is the newest patch, which is the version the loaded catalog holds: stamping an
    // older one would serve the current bytes under that version's name.
    const catalog = await loadCatalog({ version: await loadCurrentPatch() });
>>>>>>> origin/main
    const router = createRouter(createCatalogRoutes(), createTestDeps({ catalog }));
    const patches = JSON.parse(await readFile(new URL("patches.json", PATCHES), "utf8")) as { version: string }[];
<<<<<<< HEAD
<<<<<<< HEAD
    expect(patches.map((patch) => patch.version)).toEqual(["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0", "v0.2.4", "v0.2.2"]);
=======
=======
    // Promotions only ever append (R646): the six versions the brief checked stay the prefix.
    expect(patches.map((patch) => patch.version).slice(0, 6)).toEqual(["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0"]);
>>>>>>> origin/main
    expect(patches.length, "patches.json is the shipped history").toBeGreaterThan(0);
<<<<<<< HEAD
>>>>>>> origin/main
=======
    expect(patches.map((patch) => patch.version)).toEqual(
      expect.arrayContaining(["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0", "v0.2.4", "v0.2.10"]),
    );
>>>>>>> origin/main

    for (const { version } of patches) {
      const response = await router(jsonRequest("GET", `/api/catalog/${version}`));
      expect(response.status, version).toBe(200);
      const body = await readJson<CatalogBody>(response);
      expect(body.version).toBe(version);
      expect(body.defs, version).toEqual(await snapshotFile(version));
    }
  });

  it("R388 shows what a patch changed: v0.1.0 knew 109 entries and Hit Job at (2); v0.2.0 knows 317 and (3)", async () => {
    const router = createRouter(createCatalogRoutes(), createTestDeps({ catalog: await loadCatalog() }));
    const first = await readJson<CatalogBody>(await router(jsonRequest("GET", "/api/catalog/v0.1.0")));
    const now = await readJson<CatalogBody>(await router(jsonRequest("GET", "/api/catalog/v0.2.0")));
    expect(Object.keys(first.defs)).toHaveLength(109);
    expect(first.defs["core-016"]?.cost).toBe(2);
    expect(Object.keys(now.defs)).toHaveLength(317);
    expect(now.defs["core-016"]?.cost).toBe(3);
  });

  it("R388 serves the version this server runs from the catalog it loaded, whatever it is called", async () => {
    // An environment that still says an older name for today's catalog (CI's `core-1`) is answered
    // with the loaded catalog under that name, never a 404.
    const catalog = await loadCatalog({ version: "core-1" });
    const router = createRouter(createCatalogRoutes(), createTestDeps({ catalog }));
    const body = await readJson<CatalogBody>(await router(jsonRequest("GET", "/api/catalog/core-1")));
    expect(body.version).toBe("core-1");
    expect(body.defs).toEqual(catalog.defs);
  });

  it("R388 answers an unknown or malformed version with 404, and reads no file for it", async () => {
    const router = createRouter(createCatalogRoutes(), createTestDeps({ catalog: await loadCatalog() }));
    for (const version of ["v9.9.9", "patches", "..%2Fcatalog", "v0.2.0.json", "%00"]) {
      const response = await router(jsonRequest("GET", `/api/catalog/${version}`));
      expect(response.status, version).toBe(404);
    }
  });
});

// ---------------------------------------------------------------------------
// R164 — "Where L6's ban list lives"
// ---------------------------------------------------------------------------

/** A ban held as server state: the catalog data is untouched, only the handle answers differently. */
function withBan(catalog: CatalogInfo, bannedId: string): CatalogInfo {
  return { ...catalog, isBanned: (cardId) => cardId === bannedId };
}

describe("R164 — where L6's ban list lives (§9.4, R105)", () => {
  it("R164 reads bannedness through the catalog handle, never off a card definition", async () => {
    const catalog = await loadCatalog();
    const playable = catalog.cardIds.filter((cardId) => !catalog.isToken(cardId)).slice(0, 60);
    const victim = playable[0] ?? "";
    const owned = new Map(playable.map((cardId) => [cardId, 1] as const));
    const decks = [playable.slice(0, 20), playable.slice(20, 40), playable.slice(40, 60)];

    // PREMISE: the loadout is legal today, so the L6 below comes from the ban and nothing else.
    expect(
      sharedLoadoutValidator({ decks, catalogVersion: catalog.version, catalog, owned }),
    ).toEqual([]);

    const banned = withBan(catalog, victim);
    const issues = sharedLoadoutValidator({
      decks,
      catalogVersion: banned.version,
      catalog: banned,
      owned,
    });

    // L6: "every card exists in the current catalog version and is not banned".
    expect(issues.map((issue) => issue.rule)).toContain("L6");
    expect(issues.find((issue) => issue.rule === "L6")?.cardId).toBe(victim);
  });

  it("R164 keeps a ban out of the catalog data, so R105's version does not move", async () => {
    const catalog = await loadCatalog();
    const victim = catalog.cardIds[0] ?? "";
    const banned = withBan(catalog, victim);

    // The failure R164 exists to prevent: a flag on the card would mean a new R105 version, and
    // §9.4's stale-version rejection would invalidate every saved loadout in the game at once.
    expect(banned.version).toBe(catalog.version);
    expect(banned.defs).toEqual(catalog.defs);
    expect(banned.cardIds).toEqual(catalog.cardIds);
    // A card definition carries no ban flag for anything to have been written to.
    for (const def of Object.values(catalog.defs)) {
      const keys = Object.keys(def as unknown as Record<string, unknown>);
      expect(keys.filter((key) => /ban/iu.test(key))).toEqual([]);
    }
  });

  it("R164 never hands the client a copy of the list: the served bytes do not change", async () => {
    const catalog = await loadCatalog();
    const victim = catalog.cardIds[0] ?? "";

    const serve = async (info: CatalogInfo): Promise<string> => {
      const router = createRouter(createCatalogRoutes(), createTestDeps({ catalog: info }));
      return (await router(jsonRequest("GET", "/api/catalog"))).text();
    };

    // R163's route carries the catalog both sides ship; R164 keeps the ban list out of it, so the
    // client has no copy of a list it has no business being able to disagree with.
    expect(await serve(withBan(catalog, victim))).toBe(await serve(catalog));
  });

  it("R164 bans nothing in §8 at launch, and holds the hook open for when something is", async () => {
    const catalog = await loadCatalog();
    // "Nothing in §8 is banned at launch" — every one of the 111, not just a sample.
    expect(catalog.cardIds.filter((cardId) => catalog.isBanned(cardId))).toEqual([]);
    // The single hook, which reads the db agent's `cards` table once there is something to ban.
    expect(catalogFrom({}, "v0").isBanned("core-001")).toBe(false);
  });

  it("R163 reports the deployed commit in a header when one is known, and leaves the body alone", async () => {
    const catalog = await loadCatalog();
    const deps = createTestDeps({ catalog });
    const commit = "0123456789abcdef0123456789abcdef01234567";

    const known = await createRouter(createCatalogRoutes({ commit }), deps)(jsonRequest("GET", "/api/catalog"));
    const unknown = await createRouter(createCatalogRoutes(), deps)(jsonRequest("GET", "/api/catalog"));

    // deploy-watch.yml compares this header with the commit that was pushed.
    expect(DEPLOYED_COMMIT_HEADER).toBe("x-deployed-commit");
    expect(known.headers.get(DEPLOYED_COMMIT_HEADER)).toBe(commit);
    // No commit (a local server, anything but Render): no header, rather than an empty one.
    expect(unknown.headers.has(DEPLOYED_COMMIT_HEADER)).toBe(false);
    // The header is not the body: the bytes stay the same for everybody.
    expect(await known.text()).toBe(await unknown.text());
  });
});
