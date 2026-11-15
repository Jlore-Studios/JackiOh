// R3732002 (#3732): the friends drawer and the Friends page, the first component spec for either. jsdom cannot
// measure layout, so this mounts them in a browser, their reads served from fixtures, at every size the menus
// run at (`MENU_SIZES`, R3728000): nothing is wider than the window, the groups are in order (the requests, in a
// game, online, offline), every friend's row has one primary button and the rest of its actions are in its ⋯ menu,
// and each shows as a screenshot for the pull request.

import { ChallengeToast } from "../../../apps/web/src/together/ChallengeToast.tsx";
import { FriendsButton } from "../../../apps/web/src/together/FriendsButton.tsx";
import { TogetherRoute } from "../../../apps/web/src/together/TogetherRoute.tsx";
import { BREAKPOINT_PX } from "../../../apps/web/src/breakpoints.ts";
import { E2E_SESSION_STORAGE_KEY } from "../../../apps/web/src/net/session.ts";
import { MENU_SIZES, type MenuSize } from "../../support/config.ts";
import { HIT_MIN_PX, emulateTouch, hitAreaProblems } from "../../support/hitAreas.ts";
import {
  FRIENDS_ALL,
  FRIENDS_BUTTON,
  FRIENDS_PANEL,
  TOGETHER_CHALLENGE,
  TOGETHER_FRIEND_ROW,
  TOGETHER_GROUP,
  TOGETHER_JOIN,
  TOGETHER_MENU,
  TOGETHER_OFFLINE_TOGGLE,
  TOGETHER_ADD_INPUT,
  TOGETHER_ROOT,
  TOGETHER_SEARCH,
  TOGETHER_WATCH,
  togetherTabId,
  ts,
} from "../../support/testids.ts";

/** Subpixel rounding. */
const EDGE_SLACK_PX = 1;

const SINCE = "2026-11-01T00:00:00.000Z";
const ME = {
  profile: { id: "p-me", status: "active" },
  needsInviteCode: false,
  emailVerified: true,
  currentMatchId: null,
  email: "me@example.test",
  username: { name: "Max#3", nextChangeAt: null, promptOwed: false },
} as const;

/** Online and free, in a game that can be watched, in a lobby with an empty seat, and offline: one of each primary button. */
const FRIENDS = [
  { profileId: "p-bo", username: "Bo#77", status: "inMatch", matchId: "m-bo", watchable: true, turn: 7, since: SINCE },
  { profileId: "p-ana", username: "Ana#1234", status: "online", matchId: null, watchable: false, since: SINCE },
  { profileId: "p-cy", username: "Cy#5", status: "online", matchId: null, watchable: false, lobbyCode: "K7Q2XM", since: SINCE },
  {
    profileId: "p-di",
    username: "Di#9",
    status: "offline",
    matchId: null,
    watchable: false,
    lastSeenAt: "2026-11-12T09:00:00.000Z",
    since: SINCE,
  },
] as const;

const BODIES: Record<string, unknown> = {
  "/api/auth/me": ME,
  "/api/social": { incoming: [], outgoing: [], unread: { "p-ana": 2 } },
  "/api/friends": {
    friends: FRIENDS,
    incoming: [{ requestId: "rq-1", from: { profileId: "p-eli", username: "Eli#42" }, to: { profileId: "p-me", username: "Max#3" }, createdAt: SINCE }],
    outgoing: [],
    blocked: [],
  },
  "/api/friends/recent": { recent: [] },
  "/api/replays/shared": { shared: [] },
  "/api/duos/live": { matchId: null },
};

/** Answers each read the screens make; anything else goes on to the network. */
function serveFixtures(): void {
  cy.window({ log: false }).then((win) => {
    win.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: "token-1" }));
    const passThrough = win.fetch.bind(win);
    cy.stub(win, "fetch").callsFake((input: RequestInfo | URL, init?: RequestInit) => {
      const url = input instanceof win.Request ? input.url : String(input);
      const body = BODIES[new URL(url, win.location.href).pathname];
      if (body === undefined) return passThrough(input, init);
      return Promise.resolve(new win.Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } }));
    });
  });
}

/** Nothing on the page is wider than the window, and no control sticks out of its right edge. */
function expectFits(size: MenuSize): void {
  cy.window({ log: false }).should((win) => {
    const where = `${size.label} (${String(size.width)}x${String(size.height)})`;
    expect(win.innerWidth, "the viewport is the one asked for").to.eq(size.width);
    expect(win.document.documentElement.scrollWidth, `the document fits ${where}`).to.be.at.most(win.innerWidth);
    expect(win.document.body.scrollWidth, `the body fits ${where}`).to.be.at.most(win.innerWidth);
    for (const control of win.document.querySelectorAll<HTMLElement>("button, input, a[href]")) {
      if (control.getClientRects().length === 0) continue;
      const box = control.getBoundingClientRect();
      const name = control.dataset.testid ?? control.textContent ?? control.tagName;
      expect(box.right, `${name} ends inside ${where}`).to.be.at.most(win.innerWidth + EDGE_SLACK_PX);
      expect(box.left, `${name} starts inside ${where}`).to.be.at.least(-EDGE_SLACK_PX);
    }
  });
}

/** The window at `size`; the sizes wider than a phone are held under a finger too, as spec 47 does (R2122000). */
function atSize(size: MenuSize): void {
  emulateTouch(size.width > BREAKPOINT_PX.phone);
  cy.viewport(size.width, size.height);
}

/** The controls #3955 names; the measure's findings about any other control are not its concern. */
const WATCHED = ["together-add-input", "together-search", "together-offline-toggle", "friends-all"];

/** The drawer, open over /decks, its rows drawn. */
function openDrawer(): void {
  serveFixtures();
  cy.window({ log: false }).then((win) => {
    win.history.replaceState(null, "", "/decks");
  });
  cy.mount(
    <>
      <ChallengeToast />
      <FriendsButton placement="rail" />
    </>,
  );
  cy.get(ts(FRIENDS_BUTTON), { timeout: 10_000 }).click();
  cy.get(ts(FRIENDS_PANEL)).should("be.visible");
  cy.get(ts(TOGETHER_FRIEND_ROW), { timeout: 10_000 }).should("exist");
}

/** The Friends page, its rows drawn. */
function openPage(): void {
  serveFixtures();
  cy.window({ log: false }).then((win) => {
    win.history.replaceState(null, "", "/together");
  });
  cy.mount(<TogetherRoute />);
  cy.get(ts(TOGETHER_FRIEND_ROW), { timeout: 10_000 }).should("exist");
}

/** What the hit-area measure finds wrong with the controls #3955 names. */
function problemsAbout(win: Window): string[] {
  return hitAreaProblems(win).filter((problem) => WATCHED.some((name) => problem.includes(name)));
}

/** The field style the add and search inputs take: a sunken ground, a 1 px line and the small radius (index.css's tokens). */
function expectThemedFields(): void {
  cy.window({ log: false }).then((win) => {
    const probe = win.document.createElement("div");
    probe.style.background = "var(--bg-sunken)";
    probe.style.borderRadius = "var(--radius-sm)";
    win.document.body.append(probe);
    const ground = win.getComputedStyle(probe).backgroundColor;
    const radius = win.getComputedStyle(probe).borderTopLeftRadius;
    probe.remove();
    for (const id of [TOGETHER_ADD_INPUT, TOGETHER_SEARCH]) {
      const field = win.document.querySelector<HTMLElement>(ts(id));
      expect(field, `${id} is drawn`).not.to.eq(null);
      if (field === null) continue;
      const style = win.getComputedStyle(field);
      expect(field.getBoundingClientRect().height, `${id} is at least ${String(HIT_MIN_PX)} px tall`).to.be.at.least(HIT_MIN_PX);
      expect(style.backgroundColor, `${id}'s ground`).to.eq(ground);
      expect(style.borderTopWidth, `${id}'s line`).to.eq("1px");
      expect(style.borderTopLeftRadius, `${id}'s corners`).to.eq(radius);
    }
  });
}

/** The Offline toggle is drawn with its heading's own face, and none of a button's padding, ground or line. */
function expectPlainHeading(): void {
  cy.get(ts(TOGETHER_OFFLINE_TOGGLE)).then(($toggle) => {
    const toggle = $toggle[0];
    const heading = toggle?.parentElement;
    expect(toggle, "the toggle").not.to.eq(undefined);
    expect(heading, "its heading").not.to.eq(null);
    if (toggle === undefined || heading === null || heading === undefined) return;
    const own = getComputedStyle(toggle);
    const head = getComputedStyle(heading);
    for (const property of ["fontFamily", "fontSize", "fontWeight", "textTransform"] as const) {
      expect(own[property], `the toggle's ${property} is its heading's`).to.eq(head[property]);
    }
    expect(own.paddingLeft, "no side padding").to.eq("0px");
    expect(own.backgroundColor, "no ground").to.eq("rgba(0, 0, 0, 0)");
    expect(own.borderTopWidth, "no line").to.eq("0px");
  });
}

/** The groups are in order, and each friend's row has one primary button and a ⋯ menu. */
function expectGroupsAndButtons(): void {
  cy.get(ts(TOGETHER_GROUP)).should("have.length", 4);
  cy.get(ts(TOGETHER_GROUP)).then((groups) => {
    expect([...groups].map((group) => group.getAttribute("data-group"))).to.deep.eq(["requests", "inGame", "online", "offline"]);
  });
  // Offline is collapsed until its heading is pressed.
  cy.get(ts(TOGETHER_OFFLINE_TOGGLE)).should("have.attr", "aria-expanded", "false").click();
  cy.get(ts(TOGETHER_FRIEND_ROW)).should("have.length", FRIENDS.length);
  cy.get(ts(TOGETHER_FRIEND_ROW)).each((row) => {
    const primaries = row.find(`${ts(TOGETHER_WATCH)}, ${ts(TOGETHER_JOIN)}, ${ts(TOGETHER_CHALLENGE)}`);
    expect(primaries.length, `one primary button on ${row.attr("data-profile-id") ?? "a row"}`).to.eq(1);
    expect(row.find(ts(TOGETHER_MENU)).length, "a ⋯ menu on the row").to.eq(1);
  });
}

describe("R3732002 the friends drawer and the Friends page at every menu size", () => {
  afterEach(() => {
    emulateTouch(false);
    cy.window({ log: false }).then((win) => {
      win.localStorage.clear();
      win.history.replaceState(null, "", "/");
    });
  });

  for (const size of MENU_SIZES) {
    it(`R3732002 the friends drawer groups the friends and gives each row one button at ${size.label}`, () => {
      atSize(size);
      serveFixtures();
      cy.window({ log: false }).then((win) => {
        win.history.replaceState(null, "", "/decks");
      });
      cy.mount(
        <>
          <ChallengeToast />
          <FriendsButton placement="rail" />
        </>,
      );
      cy.get(ts(FRIENDS_BUTTON), { timeout: 10_000 }).click();
      cy.get(ts(FRIENDS_PANEL)).should("be.visible");
      cy.get(ts(TOGETHER_FRIEND_ROW), { timeout: 10_000 }).should("exist");
      expectGroupsAndButtons();
      expectFits(size);
      cy.screenshot(`3732-friends-drawer-${size.label}`, { capture: "viewport" });
    });

    it(`R3732002 the Friends page is five tabs over the same groups at ${size.label}`, () => {
      atSize(size);
      serveFixtures();
      cy.window({ log: false }).then((win) => {
        win.history.replaceState(null, "", "/together");
      });
      cy.mount(<TogetherRoute />);
      cy.get(ts(TOGETHER_ROOT)).should("have.attr", "data-section", "friends");
      for (const section of ["friends", "requests", "recent", "blocked", "shared"]) {
        cy.get(ts(togetherTabId(section))).should("exist");
      }
      cy.get(ts(TOGETHER_FRIEND_ROW), { timeout: 10_000 }).should("exist");
      expectGroupsAndButtons();
      expectFits(size);
      cy.screenshot(`3732-friends-${size.label}`, { capture: "viewport" });
    });

    it(`R3732002 every tab of the Friends page is inside the window at ${size.label}`, () => {
      atSize(size);
      openPage();
      cy.window({ log: false }).should((win) => {
        for (const section of ["friends", "requests", "recent", "blocked", "shared"]) {
          const tab = win.document.querySelector(ts(togetherTabId(section)));
          expect(tab, `the ${section} tab is drawn`).not.to.eq(null);
          if (tab === null) continue;
          const box = tab.getBoundingClientRect();
          expect(box.left, `the ${section} tab starts inside ${size.label}`).to.be.at.least(-EDGE_SLACK_PX);
          expect(box.right, `the ${section} tab ends inside ${size.label}`).to.be.at.most(win.innerWidth + EDGE_SLACK_PX);
        }
        const row = win.document.querySelector(ts(togetherTabId("friends")))?.parentElement;
        expect(row, "the tab row").not.to.eq(null);
        if (row === null || row === undefined) return;
        expect(row.scrollWidth, "no tab hides behind a scroll").to.be.at.most(row.clientWidth + EDGE_SLACK_PX);
      });
    });

    it(`R3732002 the add and search fields wear the theme's field style and are 44 px tall on the page at ${size.label}`, () => {
      atSize(size);
      openPage();
      expectThemedFields();
    });

    it(`R3732002 the add and search fields wear the theme's field style and are 44 px tall in the drawer at ${size.label}`, () => {
      atSize(size);
      openDrawer();
      expectThemedFields();
    });

    it(`R3732002 the page's fields and Offline toggle pass the hit-area measure at ${size.label}`, () => {
      atSize(size);
      openPage();
      cy.window().should((win) => {
        expect(problemsAbout(win), "the page's watched controls").to.deep.eq([]);
      });
    });

    it(`R3732002 the drawer's fields, Offline toggle and See all friends pass the hit-area measure at ${size.label}`, () => {
      atSize(size);
      openDrawer();
      cy.window().should((win) => {
        expect(problemsAbout(win), "the drawer's watched controls").to.deep.eq([]);
      });
    });

    it(`R3732002 See all friends' tap area stays off the Offline toggle at ${size.label}`, () => {
      atSize(size);
      openDrawer();
      cy.get(ts(TOGETHER_OFFLINE_TOGGLE)).should("have.attr", "aria-expanded", "false");
      cy.get(ts(FRIENDS_ALL)).then(($all) => {
        cy.get(ts(TOGETHER_OFFLINE_TOGGLE)).then(($toggle) => {
          const all = $all[0]?.getBoundingClientRect();
          const toggle = $toggle[0]?.getBoundingClientRect();
          expect(all, "the link is drawn").not.to.eq(undefined);
          expect(toggle, "the toggle is drawn").not.to.eq(undefined);
          if (all === undefined || toggle === undefined) return;
          expect(all.height, "the link is as tall as a tap").to.be.at.least(HIT_MIN_PX);
          expect(all.top, "the link starts below the toggle").to.be.at.least(toggle.bottom - EDGE_SLACK_PX);
        });
      });
    });

    it(`R3732002 the Offline heading is drawn as a plain heading in the drawer at ${size.label}`, () => {
      atSize(size);
      openDrawer();
      expectPlainHeading();
    });

    it(`R3732002 the Offline heading is drawn as a plain heading on the page at ${size.label}`, () => {
      atSize(size);
      openPage();
      expectPlainHeading();
    });
  }
});
