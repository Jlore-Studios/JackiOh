// #3730, R3730000: the private lobby screen at every menu size (#3728's `MENU_SIZES`), the first component spec
// for anything on `/together`. jsdom cannot measure layout, so this mounts the screen in a browser with its
// reads served from fixtures: a 1v1 lobby as its host and as its guest, a co-op lobby and a 2v2 as its host
// (#3731: its two teams, a Move on every seat but the host's), each with no sideways scroll and every control at
// least `--hit-min` (44 px), and a screenshot of each for the pull request.

import { CATALOG, CATALOG_VERSION } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import { DECK_NAME_MAX_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "@jackioh/server-config";

import type { LobbyView } from "../../../apps/web/src/lobby/api.ts";
import { LobbyScreen } from "../../../apps/web/src/lobby/LobbyScreen.tsx";
import { lobbyTestid } from "../../../apps/web/src/lobby/testids.ts";
import { E2E_SESSION_STORAGE_KEY } from "../../../apps/web/src/net/session.ts";
import { setShips } from "../../../apps/web/src/wire/catalog.ts";
import { loadWasm } from "../../../apps/web/src/wasm/index.ts";
import { MENU_SIZES, type MenuSize } from "../../support/config.ts";

/** `--hit-min`: every control a finger presses is at least this big (docs/polish/10-menus.md, convention 8). */
const HIT_MIN_PX = 44;

/** Subpixel rounding. */
const EDGE_SLACK_PX = 1;

/** Validator L3's deckable cards, as play-screen.cy.tsx's: the fixture deck is legal, so the shots show a pick as it is. */
const DECKABLE = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token") && setShips(def.set));

const CODE = "K7Q2XM";
const ANA = { profileId: "p-ana", username: "Ana#1234", status: "online" as const };
const BO = { profileId: "p-bo", username: "Bo#77", status: "online" as const };
const ED = { profileId: "p-ed", username: "Ed#42", status: "online" as const };

function view(over: Partial<LobbyView>): LobbyView {
  return {
    kind: "lobby",
    code: CODE,
    settings: { format: "duel", mode: "bo1" },
    expiresAt: Date.now() + 900_000,
    you: "host",
    yourPick: null,
    seats: [
      { role: "host", player: ANA, state: "ready", deck: "Pyre Control", random: false },
      { role: "guest", player: null, state: "choosing", deck: null, random: false },
    ],
    startedAt: null,
    matchId: null,
    seriesId: null,
    ...over,
  };
}

const KINDS: readonly { label: string; lobby: LobbyView }[] = [
  { label: "host", lobby: view({}) },
  {
    label: "guest",
    lobby: view({
      you: "guest",
      seats: [
        { role: "host", player: ANA, state: "ready", deck: null, random: false },
        { role: "guest", player: BO, state: "chosen", deck: "Tide Tempo", random: false },
      ],
    }),
  },
  {
    label: "coop",
    lobby: view({
      settings: { format: "coop", difficulty: "medium" },
      seats: [
        { role: "host", player: ANA, state: "chosen", deck: null, random: true },
        { role: "guest", player: BO, state: "ready", deck: "Tide Tempo", random: false },
      ],
    }),
  },
  {
    label: "2v2",
    lobby: view({
      settings: { format: "2v2" },
      seats: [
        { role: "host", player: ANA, state: "ready", deck: "Pyre Control", random: false },
        { role: "guest", player: BO, state: "chosen", deck: "Tide Tempo", random: false },
        { role: "guest2", player: ED, state: "chosen", deck: null, random: false },
        { role: "guest3", player: null, state: "choosing", deck: null, random: false },
      ],
    }),
  },
];

/** Answers each read the screen makes; anything else goes on to the network. */
function serveFixtures(lobby: LobbyView): void {
  const bodies: Record<string, unknown> = {
    [`/api/lobbies/${CODE}`]: lobby,
    "/api/decks": {
      catalogVersion: CATALOG_VERSION,
      decks: [
        {
          id: "d-1",
          name: "Pyre Control",
          cards: DECKABLE.slice(0, DECK_SIZE).map((def) => def.id),
          portrait: null,
          catalogVersion: CATALOG_VERSION,
          createdAt: 0,
          updatedAt: 0,
        },
      ],
      trios: [],
      limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
    },
    "/api/collection": { catalogVersion: CATALOG_VERSION, entries: DECKABLE.map((def) => ({ cardId: def.id, quantity: 1 })) },
    "/api/social": { incoming: [], outgoing: [], unread: {} },
    "/api/friends": {
      friends: [
        { ...BO, matchId: null, watchable: false, since: "2026-11-01T00:00:00.000Z" },
        { profileId: "p-cy", username: "Cy#5", status: "inMatch", matchId: null, watchable: false, since: "2026-11-01T00:00:00.000Z" },
        { profileId: "p-di", username: "Di#9", status: "offline", matchId: null, watchable: false, since: "2026-11-01T00:00:00.000Z" },
      ],
      incoming: [],
      outgoing: [],
      blocked: [],
    },
  };
  cy.window({ log: false }).then((win) => {
    win.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "token-1" }));
    const passThrough = win.fetch.bind(win);
    cy.stub(win, "fetch").callsFake((input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof win.Request ? input.url : String(input);
      const body = bodies[new URL(url, win.location.href).pathname];
      if (body === undefined) return passThrough(input, init);
      return Promise.resolve(new win.Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } }));
    });
  });
}

/**
 * Nothing on the page is wider than the window, and every control drawn is at least `HIT_MIN_PX` each way. A
 * control the layout does not draw at this size (`display: none`, as the top bar's language button on a phone,
 * routes/top-bar.css) has no box and is no target, as spec 47 reads it.
 */
function expectFitAndTargets(size: MenuSize): void {
  cy.window({ log: false }).should((win) => {
    const where = `${String(size.width)}x${String(size.height)}`;
    expect(win.document.documentElement.scrollWidth, `the page is no wider than ${where}`).to.be.at.most(win.innerWidth);
    const controls = win.document.querySelectorAll<HTMLElement>(
      `[data-testid="${lobbyTestid.screen}"] button, [data-testid="${lobbyTestid.screen}"] select`,
    );
    expect(controls.length, "the screen has controls").to.be.greaterThan(0);
    for (const control of controls) {
      if (control.getClientRects().length === 0) continue;
      const box = control.getBoundingClientRect();
      const name = control.dataset.testid ?? control.textContent ?? control.tagName;
      expect(box.right, `${name} ends inside ${where}`).to.be.at.most(win.innerWidth + EDGE_SLACK_PX);
      expect(box.height, `${name} is at least ${String(HIT_MIN_PX)} px tall`).to.be.at.least(HIT_MIN_PX - EDGE_SLACK_PX);
      expect(box.width, `${name} is at least ${String(HIT_MIN_PX)} px wide`).to.be.at.least(HIT_MIN_PX - EDGE_SLACK_PX);
    }
  });
}

describe("#3730 the private lobby at every menu size", () => {
  before(() => {
    cy.wrap(loadWasm(), { timeout: 30_000 });
  });

  for (const kind of KINDS) {
    for (const size of MENU_SIZES) {
      it(`R3730000 the ${kind.label} lobby at ${size.label}: no sideways scroll, every target at least 44 px`, () => {
        cy.viewport(size.width, size.height);
        serveFixtures(kind.lobby);
        cy.mount(<LobbyScreen token="token-1" code={CODE} />);
        cy.get(`[data-testid="${lobbyTestid.code}"]`).should("have.text", CODE);
        cy.get(`[data-testid="${lobbyTestid.seat}"]`).should("have.length.at.least", kind.lobby.settings.format === "coop" ? 4 : kind.lobby.settings.format === "2v2" ? 3 : 1);
        expectFitAndTargets(size);
        cy.screenshot(`lobby-${kind.label}-${String(size.width)}x${String(size.height)}`, { capture: "viewport" });
      });
    }
  }
});
