// Spec 34 — the public Statistics page (issues #131 and #255; SPEC §9.11, R654, R661), on the landing
// page and `/stats`, against `build:e2e` with stubbed `/api/stats/*` endpoints. BUILD M8's key assertions
// for this row: the landing's calls to action carry no Stats link and the footer's "Stats" link opens
// `/stats`; no provisional banner and no reading notes (R2325000); no data source is named anywhere.
// R4680001: the Experimental tab's Versions view opens an old play version's drill-down, drawn from the
// catalog files the built client carries.
// R4676001: at a phone's size the drill-down's Extended stats panel opens from a shared link inside the screen.

import { landingTestid } from "../../../apps/web/src/auth/testids.ts";
import { patchTestid } from "../../../apps/web/src/patches/testids.ts";
import { statsTestid } from "../../../apps/web/src/stats/testids.ts";
import { CARD_NAMES, cardId } from "../../support/cards.ts";
import { seedFor, viewports } from "../../support/config.ts";
import { NAV_BACK, SITE_FOOTER, SITE_FOOTER_ALMANAC, SITE_FOOTER_STATS, ts } from "../../support/testids.ts";

const PROVISIONAL_CARDS_FIXTURE = {
  patch: "v0.2.0",
  previousPatch: null,
  gate: {
    cleared: false,
    liveGames: 412,
    minLiveGames: 1000,
  },
  source: "provisional",
  sourceLabel: "AI games + live games (provisional)",
  minSample: 20,
  totalGames: 1412,
  cards: [
    {
      id: "core-001",
      name: "Big D-fender",
      cost: 2,
      rarity: "Common",
      set: "core",
      games: 120,
      wins: 78,
      winRate: 0.65,
      drawnWins: 60,
      playRate: 0.85,
      hasEnoughGames: true,
    },
    {
      id: "core-002",
      name: "Speciesist",
      cost: 2,
      rarity: "Common",
      set: "core",
      games: 80,
      wins: 28,
      winRate: 0.35,
      drawnWins: 24,
      playRate: 0.55,
      hasEnoughGames: true,
    },
  ],
  summary: {
    totalGames: 1412,
    liveGames: 412,
    activePatch: "v0.2.0",
    source: "provisional",
    bestCard: { id: "core-001", name: "Big D-fender", winRate: 0.65 },
    worstCard: { id: "core-002", name: "Speciesist", winRate: 0.35 },
  },
};

const CARD_DRILL_DOWN_FIXTURE = {
  card: {
    id: "core-001",
    name: "Big D-fender",
    cost: 2,
    rarity: "Common",
  },
  patches: [
    { patch: "v0.1.1", games: 50, wins: 30, winRate: 0.6 },
  ],
  byTurn: [
    { turn: 1, games: 40, wins: 28, winRate: 0.7 },
    { turn: 2, games: 30, wins: 18, winRate: 0.6 },
  ],
  coPlayed: [
    { id: "core-002", name: "Speciesist", games: 45, winRate: 0.6 },
  ],
};

const PLAYERS_FIXTURE = {
  players: [
    {
      profileId: "profile-1",
      username: "AcePlayer",
      games: 150,
      wins: 95,
      losses: 55,
      draws: 0,
      winRate: 0.633,
      favouriteCards: [{ id: "core-001", count: 85 }],
      funStats: {
        nemesisCardId: "core-002",
        totalDestroyed: 88,
        totalDefeated: 42,
      },
      updatedAt: 1234567890,
    },
  ],
  page: 1,
  limit: 50,
};

/** R4680001: Rapid Replenish, changed twice: three play versions, the two old ones' catalogs in the built client. */
const REPLENISH = cardId(10);

function playVersion(first: string, last: string, live: boolean, games: number) {
  return {
    id: `${REPLENISH}~${first}`,
    card: REPLENISH,
    name: CARD_NAMES[10],
    first,
    last,
    live,
    patches: 2,
    games,
    adjustedWinRate: games > 0 ? 0.52 : null,
    adjustedLow: games > 0 ? 0.5 : null,
    adjustedHigh: games > 0 ? 0.54 : null,
    tier: games > 0 ? "B" : null,
    hasEnoughGames: games > 0,
  };
}

const VERSIONS_FIXTURE = {
  pilot: "merged",
  mode: "all",
  field: { center: 0, spread: 0.05, cards: 3, baseline: 0.5, drawnFloor: 20 },
  bandLow: 0.47,
  bandHigh: 0.53,
  minSample: 20,
  versions: [
    playVersion("v0.1.0", "v0.2.5f", false, 0),
    playVersion("v0.2.6", "v0.6.3c", false, 120),
    playVersion("v0.6.4", "v0.6.13c", true, 80),
  ],
};

const VERSION_CARDS_FIXTURE = {
  ...PROVISIONAL_CARDS_FIXTURE,
  cards: [{ ...PROVISIONAL_CARDS_FIXTURE.cards[0], id: REPLENISH, name: CARD_NAMES[10], cost: 0 }],
};

describe("Spec 34 — the public Statistics page (R654)", () => {
  const seed = seedFor("34-stats");

  beforeEach(() => {
    // Intercept card drill-down first
    cy.intercept("GET", "**/api/stats/cards/core-001*", (req) => {
      req.reply(CARD_DRILL_DOWN_FIXTURE);
    }).as("getCardDrillDown");

    cy.intercept("GET", "**/api/stats/cards*", (req) => {
      if (!req.url.includes("/api/stats/cards/")) {
        req.reply(PROVISIONAL_CARDS_FIXTURE);
      }
    }).as("getCards");

    cy.intercept("GET", "**/api/stats/players*", (req) => {
      req.reply(PLAYERS_FIXTURE);
    }).as("getPlayers");

    cy.intercept({ url: /\/api\/auth\// }, (request) => {
      request.reply({ statusCode: 401, body: { code: "unauthenticated", message: "signed out" } });
    });
  });

  it("R2325000 R654 R661 signed out: no Stats call to action, the footer's Stats link, no banner and no reading notes, search, drill-down, players tab without Elo, and Back", () => {
    expect(seed, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");

    cy.visit("/");
    cy.get(ts(landingTestid.root)).should("be.visible");

    // R661: the landing's calls to action carry no Stats link; the site footer is the way in
    cy.get(ts(landingTestid.root)).find(".landing-ctas").should("not.contain.text", "Statistics");
    cy.get(ts(landingTestid.root)).find(".landing-ctas a[href='/stats']").should("not.exist");

    // The footer's Stats link sits right after Card almanac
    cy.get(ts(SITE_FOOTER))
      .find(ts(SITE_FOOTER_STATS))
      .should("have.attr", "href", "/stats")
      .and("contain.text", "Statistics")
      .prev()
      .should("have.attr", "data-testid", SITE_FOOTER_ALMANAC);

    cy.get(ts(SITE_FOOTER_STATS)).click();
    cy.location("pathname").should("eq", "/stats");
    cy.title().should("eq", "Statistics · JackiOh");
    cy.get(ts(statsTestid.screen)).should("be.visible");

    // R2325000: no provisional banner and no reading notes, even with a provisional source (R661)
    cy.get('[data-testid="stats-provisional-banner"]').should("not.exist");
    cy.get(ts(statsTestid.screen))
      .should("not.contain.text", "Provisional statistics")
      .and("not.contain.text", "How to read these statistics")
      .and("not.contain.text", "Data source")
      .and("not.contain.text", "AI games + live games (provisional)");

    // The summary counts the patch's games in one tile — never the gate's live count or the word
    // "live" (R661)
    cy.get(ts(statsTestid.summaryTiles)).should("be.visible").and("not.contain.text", "live");
    cy.get(ts(statsTestid.summaryTotalGames))
      .should("contain.text", "Games on patch v0.2.0")
      .and("contain.text", "1,412");

    cy.get(ts(statsTestid.cardsTable)).should("be.visible");
    cy.get(ts(statsTestid.cardsTable)).should("contain.text", "Big D-fender").and("contain.text", "Speciesist");

    cy.get(ts(statsTestid.searchInput)).type("D-fender");
    cy.get(ts(statsTestid.cardsTable)).should("contain.text", "Big D-fender").and("not.contain.text", "Speciesist");
    cy.get(ts(statsTestid.searchInput)).clear();
    cy.get(ts(statsTestid.cardsTable)).should("contain.text", "Speciesist");

    // Card drill-down modal opens and closes (the title reads the card's catalog name)
    cy.get(ts(statsTestid.cardsTable)).contains("Big D-fender").click();
    cy.get(ts(statsTestid.drillDownModal)).should("be.visible").and("contain.text", "Big D-fender");
    cy.get(ts(statsTestid.drillDownClose)).click();
    cy.get(ts(statsTestid.drillDownModal)).should("not.exist");

    cy.get(ts(statsTestid.tabPlayers)).click();
    cy.get(ts(statsTestid.playersTable)).should("be.visible");
    cy.get(ts(statsTestid.playersTable)).should("contain.text", "AcePlayer");
    cy.get(ts(statsTestid.playersTable)).should("contain.text", "150");
    cy.get(ts(statsTestid.playersTable)).should("contain.text", "Big D-fender");
    cy.get(ts(statsTestid.playersTable)).should("not.contain.text", "Elo").and("not.contain.text", "Rating");

    cy.get(ts(NAV_BACK)).click();
    cy.location("pathname").should("eq", "/");
    cy.get(ts(landingTestid.root)).should("be.visible");
  });
  it("R4680001 the Experimental tab's Versions view opens an old version's drill-down: its faces from its own catalog and the versions before and after it", () => {
    cy.intercept("GET", "**/api/stats/cards*", (req) => {
      if (!req.url.includes("/api/stats/cards/")) req.reply(VERSION_CARDS_FIXTURE);
    }).as("getVersionCards");
    cy.intercept("GET", "**/api/stats/versions*", VERSIONS_FIXTURE).as("getVersions");

    cy.visit("/stats?tab=experimental&experimental=versions");
    cy.get(ts(statsTestid.versionsTable)).should("be.visible").find("tbody tr").should("have.length", 3);
    cy.get(ts(statsTestid.versionsPastOnly)).click();
    cy.get(ts(statsTestid.versionsPastOnly)).should("have.attr", "aria-pressed", "true");
    cy.get(ts(statsTestid.versionsTable)).find("tbody tr").should("have.length", 2);

    cy.get(ts(statsTestid.versionsTable)).find(`tr[data-version-id="${REPLENISH}~v0.2.6"]`).click();
    cy.get(ts(statsTestid.drillDownModal)).should("be.visible").and("have.attr", "data-version-id", `${REPLENISH}~v0.2.6`);
    cy.location("search").should("include", `card=${REPLENISH}`).and("include", "version=v0.2.6");
    cy.get(ts(statsTestid.versionFaces)).find(".patch-face").should("have.length", 2);
    cy.get(ts(statsTestid.versionFigures)).should("contain.text", "52.0%").and("contain.text", "v0.2.6 – v0.6.3c");
    cy.get(ts(statsTestid.versionBefore)).find(ts(statsTestid.versionOpen)).should("have.text", "v0.1.0 – v0.2.5f");
    cy.get(ts(statsTestid.versionBefore)).find(".patch-face").should("have.length", 1);
    cy.get(ts(statsTestid.versionBefore)).find(ts(patchTestid.change)).should("have.length.at.least", 1);
    cy.get(ts(statsTestid.versionAfter)).find(ts(statsTestid.versionOpen)).should("have.text", "v0.6.4 – live");

    // The version before opens its own drill-down, the card's first.
    cy.get(ts(statsTestid.versionBefore)).find(ts(statsTestid.versionOpen)).click();
    cy.location("search").should("include", "version=v0.1.0");
    cy.get(ts(statsTestid.versionBefore)).should("contain.text", "None: this is its first version.");
    cy.get(ts(statsTestid.drillDownClose)).click();
    cy.get(ts(statsTestid.drillDownModal)).should("not.exist");
    cy.location("search").should("not.include", "version=");
  });

  it("R4676001 at a phone's size the Extended stats panel opens from a shared link inside the dialog and the screen", () => {
    const figures = (rate: number, tier: string) => ({
      games: 40,
      adjustedWinRate: rate,
      adjustedLow: rate - 0.03,
      adjustedHigh: rate + 0.03,
      tier,
      hasEnoughGames: true,
    });
    cy.intercept("GET", "**/api/stats/cards/core-001*", (req) => {
      req.reply({
        ...CARD_DRILL_DOWN_FIXTURE,
        face: "all",
        faces: { normal: figures(0.51, "B"), radiant: figures(0.57, "A"), gap: { gap: 0.06, low: 0.01, high: 0.11 } },
        variants: [
          {
            kind: "mode",
            rows: [
              { key: "bounce all enemy units", label: "bounce all enemy units", share: 0.6, ...figures(0.55, "A") },
              { key: "destroy all enemy units", label: "destroy all enemy units", share: 0.4, ...figures(0.47, "C") },
            ],
          },
        ],
      });
    }).as("getExtended");
    cy.viewport(viewports.phone.width, viewports.phone.height);
    cy.visit("/stats?card=core-001&extended=choices");
    cy.get(ts(statsTestid.extendedPanel), { timeout: 20_000 }).should("exist");
    cy.get(ts(statsTestid.extendedToggle)).should("have.attr", "aria-pressed", "true");
    cy.get(ts(statsTestid.extendedRow)).should("have.length", 2);
    // The panel scrolls inside the dialog: brought into view, it lies wholly inside the screen's width.
    cy.get(ts(statsTestid.extendedPanel)).scrollIntoView();
    cy.get(ts(statsTestid.extendedPanel)).then(($panel) => {
      const box = $panel.get(0)?.getBoundingClientRect();
      if (!box) throw new Error("the panel has no box");
      expect(box.left, "left edge").to.be.at.least(0);
      expect(box.right, "right edge").to.be.at.most(viewports.phone.width);
    });
    cy.get(ts(statsTestid.extendedFacesTab)).click();
    cy.get(ts(statsTestid.drillDownFaceGap)).should("be.visible");
    cy.get(ts(statsTestid.extendedChoices)).should("not.exist");
  });
});
