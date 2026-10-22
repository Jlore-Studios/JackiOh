// #1578, R1578000: the Play hub is one screen. jsdom cannot measure layout, so this mounts the route in
// a browser at a desktop and a phone size, serves the reads it makes from fixtures, and checks that the
// page does not scroll, that online play comes first, and that every way to play and the room are in
// view on a desktop, and on a phone, where the online panel comes first and whole, reachable by
// scrolling the hub's own region. The signed-out variant is held to the same.

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import { DECK_NAME_MAX_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "@jackioh/server-config";

import type { DecksResponse } from "../../../apps/web/src/net/api.ts";
import PlayRoute, { playTestid } from "../../../apps/web/src/routes/play.tsx";
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

/** Deckable cards follow validator L3: no token by flag or tag, and none of a set not shipped yet (R1420). */
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

/** What each read the lobby makes answers; any other request goes on to the network. */
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
      profileId: "5d0c7a1e-0b9f-4c55-9a3e-6f2d8b1c4e70",
      username: "Max#3",
      rank: { tier: "normal", division: 3, pips: 1, pipsPerDivision: 3, floor: "rotten" },
      streak: 2,
      record: { games: 10, wins: 7, losses: 2, draws: 1 },
      badges: [],
    },
    "/api/rooms": { rooms: [] },
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

/** The page does not scroll, either way. */
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

/** Every testid's first match lies whole inside the viewport: seen, not scrolled to. */
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

/** Each testid is reached by scrolling the hub's own region, never the page. */
function expectReachable(viewport: Viewport, testids: readonly string[]): void {
  for (const testid of testids) {
    cy.get(byTestid(testid)).scrollIntoView().should("be.visible");
    expectInView(viewport, [testid]);
  }
  cy.window({ log: false }).should((win) => {
    expect(win.scrollY, "the page itself never scrolled").to.eq(0);
  });
  expectNoPageScroll(viewport);
}

/** The online panel starts at or before the ways beside it (left of them) or under it (above them). */
function expectOnlineFirst(): void {
  cy.window({ log: false }).should((win) => {
    const online = win.document.querySelector(byTestid(playTestid.online));
    const ways = win.document.querySelector(byTestid(playTestid.ways));
    expect(online, "the online panel is on the page").not.to.eq(null);
    expect(ways, "the ways to play are on the page").not.to.eq(null);
    const first = (online as HTMLElement).getBoundingClientRect();
    const second = (ways as HTMLElement).getBoundingClientRect();
    expect(first.top, "online play starts no lower").to.be.at.most(second.top + EDGE_SLACK_PX);
    expect(first.left, "online play starts no further right").to.be.at.most(second.left + EDGE_SLACK_PX);
    expect(first.width * first.height, "online play is the bigger panel").to.be.greaterThan(second.width * second.height);
  });
}

const WAYS = [playTestid.practice, playTestid.creator, playTestid.together, playTestid.replays] as const;

describe("#1578 the Play hub is one screen", () => {
  // The validator and the catalog are Rust WebAssembly (docs/v0.3.0/SURFACE.md §10.3); load them first.
  before(() => {
    cy.wrap(loadWasm(), { timeout: 30_000 });
  });

  for (const viewport of VIEWPORTS) {
    const size = `${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`;

    it(`R1578000 signed in at ${size}: no page scroll, online first, every way to play reachable`, () => {
      cy.viewport(viewport.width, viewport.height);
      serveFixtures();
      cy.mount(<PlayRoute token="token-1" />);

      cy.get(byTestid(playTestid.deckSelect)).should("exist");
      cy.get(byTestid(playTestid.rank)).should("exist");
      expectNoPageScroll(viewport);
      expectOnlineFirst();
      expectInView(viewport, [playTestid.queue, playTestid.deckSelect, playTestid.rank]);
      if (viewport.label === "desktop") {
        // A desktop shows every way to play and the room at once.
        expectInView(viewport, [...WAYS, playTestid.createRoom, playTestid.joinInput, playTestid.joinSubmit]);
      } else {
        // A phone stacks them under the online panel, in the hub's own scroll region.
        expectReachable(viewport, [...WAYS, playTestid.createRoom, playTestid.joinInput, playTestid.joinSubmit]);
      }
    });

    it(`R1578000 signed out at ${size}: no page scroll, sign-in in the online panel, every way in view`, () => {
      cy.viewport(viewport.width, viewport.height);
      cy.mount(<PlayRoute token={null} />);

      cy.get(byTestid(playTestid.signIn)).should("be.visible");
      expectNoPageScroll(viewport);
      expectOnlineFirst();
      expectInView(viewport, [playTestid.signIn, ...WAYS]);
    });
  }
});
