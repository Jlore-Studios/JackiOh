// Spec 34 — the public Statistics page (issues #131 and #255; SPEC §9.11, R654, R661), on the landing
// page and `/stats`, against `build:e2e` with stubbed `/api/stats/*` endpoints.
//
// BUILD M8's key assertions for this row: "the landing's calls to action carry no Stats link and the
// footer's "Stats" link opens `/stats`; the provisional banner names no data source and no count
// towards the gate; summary tiles render the patch's games once, with no data source; cards table
// searches and filters; card drill-down opens CardFace modal; players tab renders public stats
// without Elo; Back returns to the landing page".
//
// Run it:
//   pnpm build:e2e
//   pnpm --dir apps/web exec vite preview --port 5173 --strictPort
//   pnpm --dir e2e exec cypress run --spec cypress/e2e/34-stats.cy.ts

import { landingTestid } from "../../../apps/web/src/auth/testids.ts";
import { seedFor } from "../../support/config.ts";
import {
  NAV_BACK,
  SITE_FOOTER,
  SITE_FOOTER_ALMANAC,
  SITE_FOOTER_STATS,
  STATS_CARDS_TABLE,
  STATS_DRILL_DOWN_CLOSE,
  STATS_DRILL_DOWN_MODAL,
  STATS_PLAYERS_TABLE,
  STATS_PROVISIONAL_BANNER,
  STATS_SCREEN,
  STATS_SEARCH_INPUT,
  STATS_SUMMARY_TILES,
  STATS_SUMMARY_TOTAL_GAMES,
  STATS_TAB_PLAYERS,
  ts,
} from "../../support/testids.ts";

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
      drawnGames: 90,
      drawnWins: 60,
      drawnWinRate: 0.667,
      playRate: 0.85,
      hasEnoughGames: true,
    },
    {
      id: "core-002",
      name: "Bigot",
      cost: 2,
      rarity: "Common",
      set: "core",
      games: 80,
      wins: 28,
      winRate: 0.35,
      drawnGames: 70,
      drawnWins: 24,
      drawnWinRate: 0.343,
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
    worstCard: { id: "core-002", name: "Bigot", winRate: 0.35 },
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
    { id: "core-002", name: "Bigot", games: 45, winRate: 0.6 },
  ],
};

const PLAYERS_FIXTURE = {
  players: [
    {
      profileId: "profile-1",
      displayName: "AcePlayer",
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

describe("Spec 34 — the public Statistics page (R654)", () => {
  const seed = seedFor("34-stats");

  beforeEach(() => {
    // Intercept card drill-down first
    cy.intercept("GET", "**/api/stats/cards/core-001*", (req) => {
      req.reply(CARD_DRILL_DOWN_FIXTURE);
    }).as("getCardDrillDown");

    // Intercept public card aggregates
    cy.intercept("GET", "**/api/stats/cards*", (req) => {
      if (!req.url.includes("/api/stats/cards/")) {
        req.reply(PROVISIONAL_CARDS_FIXTURE);
      }
    }).as("getCards");

    // Intercept public player summaries
    cy.intercept("GET", "**/api/stats/players*", (req) => {
      req.reply(PLAYERS_FIXTURE);
    }).as("getPlayers");

    // All unauthenticated auth checks return 401
    cy.intercept({ url: /\/api\/auth\// }, (request) => {
      request.reply({ statusCode: 401, body: { code: "unauthenticated", message: "signed out" } });
    });
  });

  it("R654 R661 signed out: no Stats call to action, the footer's Stats link, a provisional banner with no source or gate, search, drill-down, players tab without Elo, and Back", () => {
    expect(seed, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");

    cy.visit("/");
    cy.get(ts(landingTestid.root)).should("be.visible");

    // R661: the landing's calls to action carry no Stats link; the site footer is the way in
    cy.get(ts(landingTestid.root)).find(".landing-ctas").should("not.contain.text", "Stats");
    cy.get(ts(landingTestid.root)).find(".landing-ctas a[href='/stats']").should("not.exist");

    // The footer's Stats link sits right after Card almanac
    cy.get(ts(SITE_FOOTER))
      .find(ts(SITE_FOOTER_STATS))
      .should("have.attr", "href", "/stats")
      .and("contain.text", "Stats")
      .prev()
      .should("have.attr", "data-testid", SITE_FOOTER_ALMANAC);

    // Click the footer link to navigate to /stats
    cy.get(ts(SITE_FOOTER_STATS)).click();
    cy.location("pathname").should("eq", "/stats");
    cy.title().should("eq", "Statistics · JackiOh");
    cy.get(ts(STATS_SCREEN)).should("be.visible");

    // The provisional banner says the figures are provisional, and names no source and no gate (R661)
    cy.get(ts(STATS_PROVISIONAL_BANNER))
      .should("be.visible")
      .and("contain.text", "Provisional statistics")
      .and("not.contain.text", "1000")
      .and("not.contain.text", "AI development games");
    cy.get(ts(STATS_SCREEN))
      .should("not.contain.text", "Data source")
      .and("not.contain.text", "AI games + live games (provisional)");

    // The summary counts the patch's games in one tile — never the gate's live count or the word
    // "live" (R661)
    cy.get(ts(STATS_SUMMARY_TILES)).should("be.visible").and("not.contain.text", "live");
    cy.get(ts(STATS_SUMMARY_TOTAL_GAMES))
      .should("contain.text", "Games on patch v0.2.0")
      .and("contain.text", "1,412");

    // Cards table renders rows
    cy.get(ts(STATS_CARDS_TABLE)).should("be.visible");
    cy.get(ts(STATS_CARDS_TABLE)).should("contain.text", "Big D-fender").and("contain.text", "Bigot");

    // Search filters cards
    cy.get(ts(STATS_SEARCH_INPUT)).type("D-fender");
    cy.get(ts(STATS_CARDS_TABLE)).should("contain.text", "Big D-fender").and("not.contain.text", "Bigot");
    cy.get(ts(STATS_SEARCH_INPUT)).clear();
    cy.get(ts(STATS_CARDS_TABLE)).should("contain.text", "Bigot");

    // Card drill-down modal opens and closes (the title reads the card's catalog name)
    cy.get(ts(STATS_CARDS_TABLE)).contains("Big D-fender").click();
    cy.get(ts(STATS_DRILL_DOWN_MODAL)).should("be.visible").and("contain.text", "Big D-fender");
    cy.get(ts(STATS_DRILL_DOWN_CLOSE)).click();
    cy.get(ts(STATS_DRILL_DOWN_MODAL)).should("not.exist");

    // Players tab renders public player data without exposing Elo
    cy.get(ts(STATS_TAB_PLAYERS)).click();
    cy.get(ts(STATS_PLAYERS_TABLE)).should("be.visible");
    cy.get(ts(STATS_PLAYERS_TABLE)).should("contain.text", "AcePlayer");
    cy.get(ts(STATS_PLAYERS_TABLE)).should("contain.text", "150");
    cy.get(ts(STATS_PLAYERS_TABLE)).should("contain.text", "Big D-fender");
    // Assert Elo and ratings are kept separate and not exposed
    cy.get(ts(STATS_PLAYERS_TABLE)).should("not.contain.text", "Elo").and("not.contain.text", "Rating");

    // Back returns to landing page
    cy.get(ts(NAV_BACK)).click();
    cy.location("pathname").should("eq", "/");
    cy.get(ts(landingTestid.root)).should("be.visible");
  });
});
