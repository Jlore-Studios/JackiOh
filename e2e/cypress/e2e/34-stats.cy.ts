// Spec 34 — the public Statistics page (issue #131; SPEC §9.11, R641), on the landing page and `/stats`,
// against `build:e2e` with stubbed `/api/stats/*` endpoints.
//
// BUILD M8's key assertions for this row: "the footer's "Stats" link opens `/stats`; provisional banner
// shows live game count; summary tiles render; cards table searches and filters; card drill-down
// opens CardFace modal; players tab renders public stats without Elo; Back returns to the landing page".
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
  STATS_SUMMARY_LIVE_GAMES,
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
      name: "Footman",
      cost: 1,
      rarity: "common",
      set: "core",
      games: 120,
      wins: 78,
      winRate: 0.65,
      drawnGames: 90,
      drawnWins: 60,
      drawnWinRate: 0.667,
      playRate: 0.85,
    },
    {
      id: "core-002",
      name: "Recruit",
      cost: 2,
      rarity: "rare",
      set: "core",
      games: 80,
      wins: 28,
      winRate: 0.35,
      drawnGames: 70,
      drawnWins: 24,
      drawnWinRate: 0.343,
      playRate: 0.55,
    },
  ],
  summary: {
    totalGames: 1412,
    liveGames: 412,
    activePatch: "v0.2.0",
    source: "provisional",
    bestCard: { id: "core-001", name: "Footman", winRate: 0.65 },
    worstCard: { id: "core-002", name: "Recruit", winRate: 0.35 },
  },
};

const CARD_DRILL_DOWN_FIXTURE = {
  card: {
    id: "core-001",
    name: "Footman",
    cost: 1,
    rarity: "common",
  },
  patches: [
    { patch: "v0.1.1", games: 50, wins: 30, winRate: 0.6 },
  ],
  byTurn: [
    { turn: 1, games: 40, wins: 28, winRate: 0.7 },
    { turn: 2, games: 30, wins: 18, winRate: 0.6 },
  ],
  coPlayed: [
    { id: "core-002", name: "Recruit", games: 45, winRate: 0.6 },
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

describe("Spec 34 — the public Statistics page (R641)", () => {
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

  it("R641 signed out: the footer's Stats link, provisional banner, search, drill-down, players tab without Elo, and Back", () => {
    expect(seed, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");

    cy.visit("/");
    cy.get(ts(landingTestid.root)).should("be.visible");

    // Landing CTA link to stats is present
    cy.get(ts(landingTestid.statsLink))
      .should("have.attr", "href", "/stats")
      .and("contain.text", "Stats");

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

    // Provisional banner shows live game count progress below the 1000 gate
    cy.get(ts(STATS_PROVISIONAL_BANNER))
      .should("be.visible")
      .and("contain.text", "412 / 1000 live games on this patch");

    // Summary tiles show total games and live games
    cy.get(ts(STATS_SUMMARY_TILES)).should("be.visible");
    cy.get(ts(STATS_SUMMARY_TOTAL_GAMES)).should("contain.text", "1,412");
    cy.get(ts(STATS_SUMMARY_LIVE_GAMES)).should("contain.text", "412");

    // Cards table renders rows
    cy.get(ts(STATS_CARDS_TABLE)).should("be.visible");
    cy.get(ts(STATS_CARDS_TABLE)).should("contain.text", "Footman").and("contain.text", "Recruit");

    // Search filters cards
    cy.get(ts(STATS_SEARCH_INPUT)).type("Footman");
    cy.get(ts(STATS_CARDS_TABLE)).should("contain.text", "Footman").and("not.contain.text", "Recruit");
    cy.get(ts(STATS_SEARCH_INPUT)).clear();
    cy.get(ts(STATS_CARDS_TABLE)).should("contain.text", "Recruit");

    // Card drill-down modal opens and closes
    cy.get(ts(STATS_CARDS_TABLE)).contains("Footman").click();
    cy.get(ts(STATS_DRILL_DOWN_MODAL)).should("be.visible").and("contain.text", "Footman");
    cy.get(ts(STATS_DRILL_DOWN_CLOSE)).click();
    cy.get(ts(STATS_DRILL_DOWN_MODAL)).should("not.exist");

    // Players tab renders public player data without exposing Elo
    cy.get(ts(STATS_TAB_PLAYERS)).click();
    cy.get(ts(STATS_PLAYERS_TABLE)).should("be.visible");
    cy.get(ts(STATS_PLAYERS_TABLE)).should("contain.text", "AcePlayer");
    cy.get(ts(STATS_PLAYERS_TABLE)).should("contain.text", "150");
    cy.get(ts(STATS_PLAYERS_TABLE)).should("contain.text", "Footman");
    // Assert Elo and ratings are kept separate and not exposed
    cy.get(ts(STATS_PLAYERS_TABLE)).should("not.contain.text", "Elo").and("not.contain.text", "Rating");

    // Back returns to landing page
    cy.get(ts(NAV_BACK)).click();
    cy.location("pathname").should("eq", "/");
    cy.get(ts(landingTestid.root)).should("be.visible");
  });
});
