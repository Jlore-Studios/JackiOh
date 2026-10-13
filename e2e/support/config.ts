// A6: routes, endpoints, and fixtures are overridden through Cypress `expose`; BUILD M8 fixes hotseat only.

function env(key: string, fallback: string): string {
  const value: unknown = Cypress.expose(key);
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

/** BUILD §4: override a spec seed for nightly `01` runs. */
export function seedFor(fallback: string): string {
  return env("seed", fallback);
}

/** BUILD M5-T3/M8 hotseat route. */
export function hotseatUrl(seed: string, a: string, b: string): string {
  const params = new URLSearchParams({ seed, a, b });
  return `/dev/hotseat?${params.toString()}`;
}

/** A6: routes for the specs M8 points at features rather than URLs. */
export const routes = {
  login: () => env("loginRoute", "/login"),
  invite: () => env("inviteRoute", "/invite"),
  deckbuilder: () => env("deckbuilderRoute", "/decks"),
  play: () => env("playRoute", "/play"),
  match: (matchId: string) => `${env("matchRoute", "/match")}/${matchId}`,
  /** R259: a Best-of-3 series between games. */
  series: (seriesId: string) => `${env("seriesRoute", "/series")}/${seriesId}`,
};

/** A6: `WS_PATH` is `/ws/match`; support type-checks without `apps/*`. */
export const server = {
  http: () => env("apiUrl", "http://localhost:8787"),
  ws: () => env("wsUrl", "ws://localhost:8787/ws/match"),
};

/** A10: specs seed the session key before load; it must survive spec 05's mid-match reload. */
export const SESSION_STORAGE_KEY = "jackioh.e2e.session";

/** A17/R256: mirrors unsynced drafts, including the L3/L6 cases spec 09 restores. */
export function deckMirrorKey(profileId: string): string {
  return `jackioh.decks.v1.${profileId}`;
}

/** SPEC §9.10, R294: completed tutorial lessons, seeded and read by specs 22 and 23. */
export const TUTORIAL_PROGRESS_KEY = "jackioh.tutorial.v1";
export const TUTORIAL_PROGRESS_VERSION = 1;

/** R290: spec 23 compares the tutorial opponent handicap to the engine constant. */
export const TUTORIAL_HANDICAP = {
  deckSize: 12,
  manaBonus: 0,
  manaCap: 3,
  extraOpeningCards: 0,
  extraDrawsPerTurn: 0,
  heroHealth: 20,
} as const;

export type E2EAccount = { email: string; password: string; token: string };

/** A6: `pending` waits at the spec-10 invite gate; `p1`/`p2` own every card. */
export const accounts: Record<"p1" | "p2" | "pending", () => E2EAccount> = {
  p1: () => ({ email: env("p1Email", "e2e-p1@jackioh.test"), password: env("p1Password", "e2e-p1-password"), token: env("p1Token", "e2e-token-p1") }),
  p2: () => ({ email: env("p2Email", "e2e-p2@jackioh.test"), password: env("p2Password", "e2e-p2-password"), token: env("p2Token", "e2e-token-p2") }),
  pending: () => ({ email: env("pendingEmail", "e2e-pending@jackioh.test"), password: env("pendingPassword", "e2e-pending-password"), token: env("pendingToken", "e2e-token-pending") }),
};

/** A6: seeded invite codes cover each §9.4 failure mode and success. */
export const inviteCodes = {
  good: () => env("goodCode", "ABCD-EFGH-JKMN-PQRS"),
  missing: () => env("missingCode", "ZZZZ-ZZZZ-ZZZZ-ZZZZ"),
  expired: () => env("expiredCode", "XPRD-XPRD-XPRD-XPRD"),
  exhausted: () => env("exhaustedCode", "XHST-XHST-XHST-XHST"),
};

/** BUILD M8: assertion timeouts; specs never sleep with `cy.wait(ms)`. */
export const timeouts = {
  /** BUILD M5-T4: R82 can make the longest burst; 4 s left too little CI slack, while 8 s still fails stalled assertions. */
  animation: 8_000,
  /** M6-T4: a server view push or seat handover. */
  view: 10_000,
  game: 180_000,
  task: 20_000,
};

/** SPEC §2 / BUILD §2 constants so specs do not hard-code them. */
export const constants = {
  DECK_SIZE: 20,
  MAX_COPIES: 1,
  MAX_MANA: 4,
  HERO_HEALTH: 30,
  TURN_CAP_PLAYER_TURNS: 60,
  HAND_CAP: 10,
  UNIT_ZONES: 5,
  BACKROW_ZONES: 5,
  OPENING_DRAW: [3, 4] as const,
  ROOM_CODE_LENGTH: 6,
  DECKS_PER_LOADOUT: 3,
  /** R80: a library holds at most this many cards (`LIBRARY_CAP`), so no handicap deck is larger. */
  LIBRARY_CAP: 60,
};

/** R201: spec 25 seeds effects settings so its screenshot catches a notice. */
export const FX_SETTINGS_KEY = "jackioh.fx.v1";
