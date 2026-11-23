// #1578, R1578000, #3729, R3729000: the Play hub is one screen, four groups of cards. jsdom cannot measure
// layout, so this mounts the route in a browser at a desktop and a phone size, with the reads it makes served
// from fixtures. The page never scrolls: the hub's own region does, and an Online card opens one panel in view.

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import { DECK_NAME_MAX_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "@jackioh/server-config";

import type { DecksResponse } from "../../../apps/web/src/net/api.ts";
import PlayRoute, { playTestid } from "../../../apps/web/src/routes/play.tsx";
import { playGroupTestid } from "../../../apps/web/src/routes/playTestids.ts";
import { setShips } from "../../../apps/web/src/wire/catalog.ts";
import { loadWasm } from "../../../apps/web/src/wasm/index.ts";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 800 },
  { label: "phone", width: 390, height: 844 },
] as const;

type Viewport = (typeof VIEWPORTS)[number];

/** Subpixel rounding between two boxes that start on the same line. */
const EDGE_SLACK_PX = 1;

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** Validator L3's deckable cards: no token by flag or tag, none of an unshipped set (R1420). */
const DECKABLE = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token") && setShips(def.set));

const DECK_ID = "00000000-0000-4000-8000-000000000001";
const SAVED_AT = 0;

const DECKS: DecksResponse = {
  catalogVersion: CATALOG_VERSION,
  decks: [
    {
      id: DECK_ID,
      name: "Deck 1",
      cards: DECKABLE.slice(0, DECK_SIZE).map((def) => def.id),
      portrait: null,
      catalogVersion: CATALOG_VERSION,
      createdAt: SAVED_AT,
      updatedAt: SAVED_AT,
    },
  ],
  trios: [],
  limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
};

/** Answers each read the lobby makes; other requests go on to the network. */
function serveFixtures(): void {
  const bodies: Record<string, unknown> = {
    "/api/decks": DECKS,
    "/api/collection": {
      catalogVersion: CATALOG_VERSION,
      entries: DECKABLE.map((def) => ({ cardId: def.id, quantity: 1 })),
    },
    "/api/auth/me": {
      profile: { id: "profile-1", status: "active" },
      needsInviteCode: false,
      emailVerified: true,
      currentMatchId: null,
      currentSeriesId: null,
      email: "player@example.test",
    },
    "/api/queue/population": { gamesLastHour: 12, expectedWaitSeconds: { bo1: 30, bo3: null, random: 90 } },
    "/api/ranked": {
      season: "v0.2",
      ladder: "constructed",
      profileId: "5d0c7a1e-0b9f-4c55-9a3e-6f2d8b1c4e70",
      username: "Max#3",
      rank: { tier: "normal", division: 3, pips: 1, pipsPerDivision: 3, floor: "rotten" },
      streak: 2,
      record: { games: 10, wins: 7, losses: 2, draws: 1 },
      badges: [],
    },
    "/api/lobbies": { lobbies: [] },
  };
  cy.window({ log: false }).then((win) => {
    const passThrough = win.fetch.bind(win);
    cy.stub(win, "fetch").callsFake((input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof win.Request ? input.url : String(input);
      const body = bodies[new URL(url, win.location.href).pathname];
      if (body === undefined) return passThrough(input, init);
      return Promise.resolve(
        new win.Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } }),
      );
    });
  });
}

function expectNoPageScroll(viewport: Viewport): void {
  cy.window({ log: false }).should((win) => {
    const where = `${String(viewport.width)}x${String(viewport.height)}`;
    expect(win.innerWidth, "the viewport is the one asked for").to.eq(viewport.width);
    expect(win.innerHeight, "the viewport is the one asked for").to.eq(viewport.height);
    const { documentElement, body } = win.document;
    expect(documentElement.scrollHeight, `the document is no taller than ${where}`).to.be.at.most(win.innerHeight);
    expect(body.scrollHeight, `the body is no taller than ${where}`).to.be.at.most(win.innerHeight);
    expect(documentElement.scrollWidth, `the document is no wider than ${where}`).to.be.at.most(win.innerWidth);
  });
}

/** Each testid's first match lies whole in the viewport: seen, not scrolled to. */
function expectInView(viewport: Viewport, testids: readonly string[]): void {
  cy.window({ log: false }).should((win) => {
    for (const testid of testids) {
      const element = win.document.querySelector(byTestid(testid));
      expect(element, `${testid} is on the page`).not.to.eq(null);
      const box = (element as HTMLElement).getBoundingClientRect();
      expect(box.top, `${testid} starts inside ${String(viewport.height)} px`).to.be.at.least(0);
      expect(box.bottom, `${testid} ends inside ${String(viewport.height)} px`).to.be.at.most(viewport.height + EDGE_SLACK_PX);
      expect(box.right, `${testid} ends inside ${String(viewport.width)} px`).to.be.at.most(viewport.width + EDGE_SLACK_PX);
    }
  });
}

/** The hub's one scroller is its own region, never the page (R1581000). */
function expectHubRegionScrolls(): void {
  cy.get(".play-body").should(($region) => {
    expect(getComputedStyle($region[0] as HTMLElement).overflowY, "the hub's region is the one scroller").to.eq("auto");
  });
}

const ONLINE = [playTestid.modeBo1, playTestid.modeBo3, playTestid.modeRandom] as const;

/** Every other card, reached by scrolling the hub's region. */
const OTHER_CARDS = [
  playTestid.daily,
  playTestid.lobby,
  playTestid.challenge,
  playTestid.practice,
  playTestid.dungeon,
  playTestid.tutorial,
  playTestid.replays,
  playTestid.together,
] as const;

describe("#1578 #3729 the Play hub is one screen", () => {
  // The validator and the catalog are Rust WebAssembly (docs/v0.3.0/SURFACE.md §10.3); load them first.
  before(() => {
    cy.wrap(loadWasm(), { timeout: 30_000 });
  });

  for (const viewport of VIEWPORTS) {
    const size = `${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`;

    it(`R3729000 signed in at ${size}: no page scroll, four groups, the Online cards in view, and one panel from a card to Find a match`, () => {
      cy.viewport(viewport.width, viewport.height);
      serveFixtures();
      cy.mount(<PlayRoute token="token-1" />);

      cy.get(byTestid(playTestid.hub)).should("exist");
      for (const group of ["online", "daily", "friends", "solo"] as const) cy.get(byTestid(playGroupTestid(group))).should("exist");
      expectNoPageScroll(viewport);
      expectHubRegionScrolls();
      expectInView(viewport, ONLINE);
      for (const testid of OTHER_CARDS) cy.get(byTestid(testid)).scrollIntoView().should("be.visible");
      expectNoPageScroll(viewport);

      // R3729000: a card opens its panel, with the picker, the rank and Find a match in view.
      cy.get(byTestid(playTestid.modeBo1)).scrollIntoView().click();
      cy.get(byTestid(playTestid.queuePanel)).should("be.visible");
      cy.get(byTestid(playTestid.deckSelect)).should("exist");
      cy.get(byTestid(playTestid.rank)).should("exist");
      expectInView(viewport, [playTestid.queue, playTestid.deckSelect, playTestid.rank]);
      expectNoPageScroll(viewport);
    });

    it(`R3729000 signed out at ${size}: no page scroll, sign-in above the solo cards, and no Online card`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(<PlayRoute token={null} />);

      cy.get(byTestid(playTestid.signIn)).should("be.visible");
      expectNoPageScroll(viewport);
      expectHubRegionScrolls();
      expectInView(viewport, [playTestid.signIn]);
      for (const testid of [playTestid.practice, playTestid.dungeon, playTestid.tutorial, playTestid.replays, playTestid.together]) {
        cy.get(byTestid(testid)).scrollIntoView().should("be.visible");
      }
      for (const group of ["online", "daily", "friends"] as const) cy.get(byTestid(playGroupTestid(group))).should("not.exist");
    });
  }
});
