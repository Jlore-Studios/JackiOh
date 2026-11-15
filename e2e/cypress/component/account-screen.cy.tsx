// R3733000 (#3733): the account screen is a profile header over a settings list. jsdom cannot measure layout,
// so this mounts the route in a browser, its reads served from fixtures, at every size the menus run at
// (`MENU_SIZES`, R3728000): nothing is wider than the viewport, the page does not scroll, and the list and the
// open section sit side by side over 600 px and one at a time at 600 px and under.

import AccountRoute, { accountTestid } from "../../../apps/web/src/routes/account.tsx";
import { MENU_SIZES } from "../../support/config.ts";

type Size = (typeof MENU_SIZES)[number];

/** The phone's width and under: one of list and section at a time (`breakpoints.ts`, `phone`). */
const PHONE_MAX_WIDTH_PX = 600;

const ME = {
  profile: { id: "p1", status: "active" },
  needsInviteCode: false,
  emailVerified: true,
  currentMatchId: null,
  email: "player1@example.com",
  username: { name: "Max#3", nextChangeAt: null, promptOwed: false },
} as const;

const RANK = {
  season: "v0.2",
  ladder: "constructed",
  profileId: "p1",
  username: "Max#3",
  rank: { tier: "normal", division: 3, pips: 1, pipsPerDivision: 3, floor: "rotten" },
  streak: 4,
  record: { games: 10, wins: 7, losses: 2, draws: 1 },
  badges: [{ seasonId: "v0.1", tier: "normal", division: 2 }],
};

const BODIES: Record<string, unknown> = {
  "/api/profile": {
    id: "p1",
    email: "player1@example.com",
    status: "active",
    record: { wins: 7, losses: 2, draws: 1 },
    winRate: 0.7,
    createdAt: Date.UTC(2026, 9, 5),
  },
  "/api/ranked": RANK,
  "/api/replays": {
    replays: [
      {
        matchId: "m1",
        mode: "bo1",
        endedAt: Date.UTC(2026, 10, 1, 18),
        seat: "p1",
        result: "win",
        turns: 7,
        steps: 40,
        portraits: { p1: "gary", p2: "timmy" },
        opponent: { profileId: "p2", username: "Grape#12" },
      },
    ],
    nextOffset: null,
  },
  "/api/daily/settings": { emailOn: true, hasEmail: true, vapidPublicKey: null, pushDevices: 0 },
};

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** Answers each read the screen makes; other requests go on to the network. */
function serveFixtures(): void {
  cy.window({ log: false }).then((win) => {
    const passThrough = win.fetch.bind(win);
    cy.stub(win, "fetch").callsFake((input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof win.Request ? input.url : String(input);
      const body = BODIES[new URL(url, win.location.href).pathname];
      if (body === undefined) return passThrough(input, init);
      return Promise.resolve(
        new win.Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } }),
      );
    });
  });
}

/** Shows the screen at `search`, with the viewport of `size`. */
function mountAt(size: Size, search: string): void {
  cy.viewport(size.width, size.height);
  serveFixtures();
  cy.window({ log: false }).then((win) => {
    win.history.replaceState(null, "", `/account${search}`);
  });
  cy.mount(<AccountRoute token="token-1" me={ME} />);
  cy.get(byTestid(accountTestid.profile)).should("exist");
}

function expectFits(size: Size): void {
  cy.window({ log: false }).should((win) => {
    const where = `${size.label} (${String(size.width)}x${String(size.height)})`;
    expect(win.innerWidth, "the viewport is the one asked for").to.eq(size.width);
    expect(win.document.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(win.innerWidth);
    expect(win.document.body.scrollWidth, `the body fits ${where}`).to.be.at.most(win.innerWidth);
    expect(win.document.documentElement.scrollHeight, `the page does not scroll at ${where}`).to.be.at.most(win.innerHeight);
  });
}

function layout(): Cypress.Chainable<JQuery<HTMLElement>> {
  return cy.get(byTestid(accountTestid.screen)).find(".account-layout");
}

describe("R3733000 the account screen at every menu size", () => {
  afterEach(() => {
    cy.window({ log: false }).then((win) => {
      win.history.replaceState(null, "", "/");
    });
  });

  for (const size of MENU_SIZES) {
    const phone = size.width <= PHONE_MAX_WIDTH_PX;

    it(`R3733000 /account fits one screen and ${phone ? "shows the list alone" : "shows the list beside Overview"} at ${size.label}`, () => {
      mountAt(size, "");

      cy.get(byTestid(accountTestid.list)).should("be.visible");
      cy.get(byTestid(accountTestid.rank)).should("be.visible");
      if (phone) {
        cy.get(byTestid(accountTestid.section)).should("not.be.visible");
        layout().should("have.attr", "data-open", "list");
      } else {
        cy.get(byTestid(accountTestid.section)).should("be.visible");
        cy.get(byTestid(accountTestid.email)).should("be.visible");
      }
      expectFits(size);
    });

    it(`R3733000 ?section=history fits one screen and ${phone ? "shows the section alone" : "shows both"} at ${size.label}`, () => {
      mountAt(size, "?section=history");

      cy.get(byTestid(accountTestid.historyRow("m1"))).should("be.visible");
      cy.get(byTestid(accountTestid.section)).should("be.visible");
      if (phone) {
        cy.get(byTestid(accountTestid.list)).should("not.be.visible");
        cy.get(byTestid(accountTestid.sectionBack)).should("be.visible");
      } else {
        cy.get(byTestid(accountTestid.list)).should("be.visible");
        cy.get(byTestid(accountTestid.sectionBack)).should("not.be.visible");
      }
      expectFits(size);
    });
  }
});
