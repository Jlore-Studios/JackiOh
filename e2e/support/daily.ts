// #3727: what specs 52 and 53 do over HTTP for daily games: the two fixture accounts made friends with
// nothing left between them, a seat's read and its actions (each sent after the read's `seq` under a
// fresh nonce, as the board sends them), and the E2E server's daily clock and sweep. The shapes are the
// server's (apps/web/src/net/daily.ts), restated as far as the specs read them, since e2e type-checks alone.

import { apiUrl, bearerHeaders, type E2EAccount } from "./config.ts";
import type { ActionBody } from "./types.ts";

type PlayerRef = { profileId: string; username: string };

export type DailyInviteBody = { id: string; kind: "challenge" | "seek"; from: PlayerRef; to: PlayerRef | null; mode: string };

export type DailyGameRowBody = { matchId: string; hasMove: boolean; over: boolean; opponent: PlayerRef };

export type DailyListBody = {
  games: DailyGameRowBody[];
  challenges: { incoming: DailyInviteBody[]; outgoing: DailyInviteBody[] };
  seeks: DailyInviteBody[];
  waiting: number;
};

export type DailyHoldBody = { player: string; key: string; since: number; deadline: number };

export type DailyGameBody = {
  matchId: string;
  seat: string;
  seq: number;
  hasMove: boolean;
  over: boolean;
  lostOnTime: boolean;
  holds: DailyHoldBody[];
  legal: ActionBody[];
  view: { you?: { hand?: { instanceId: string }[] } };
  nextGame: string | null;
};

type FriendsBody = {
  friends: PlayerRef[];
  incoming: { requestId: string; from: PlayerRef }[];
  outgoing: { requestId: string; to: PlayerRef }[];
};

/** `POST /api/e2e/daily-clock`: moves the E2E server's daily clock by `offsetMs` (0 puts it back). */
export const DAILY_CLOCK_PATH = "/api/e2e/daily-clock";

/** The settle the Worker's cron asks for (outside `/api`, no token): it ends the games whose move ran out. */
export const DAILY_SWEEP_PATH = "/daily-sweep";

let nonces = 0;

/** A fresh nonce for each action: the server applies one action per nonce. */
function nonce(): string {
  nonces += 1;
  return `e2e-daily-${String(Date.now())}-${String(nonces)}`;
}

export function profileOf(account: E2EAccount): Cypress.Chainable<{ id: string; name: string }> {
  return cy
    .request<{ profile: { id: string }; username?: { name: string } }>({
      method: "GET",
      url: apiUrl("/api/auth/me"),
      headers: bearerHeaders(account.token),
    })
    .its("body")
    .then((body) => ({ id: body.profile.id, name: body.username?.name ?? "" }));
}

export function dailyOf(account: E2EAccount): Cypress.Chainable<DailyListBody> {
  return cy.request<DailyListBody>({ method: "GET", url: apiUrl("/api/daily"), headers: bearerHeaders(account.token) }).its("body");
}

export function dailyGameOf(account: E2EAccount, matchId: string): Cypress.Chainable<DailyGameBody> {
  return cy
    .request<DailyGameBody>({ method: "GET", url: apiUrl(`/api/daily/games/${matchId}`), headers: bearerHeaders(account.token) })
    .its("body");
}

/** One action of the seat's, picked from its read, sent after the read's seq; yields the answer. */
export function dailyActAs(
  account: E2EAccount,
  matchId: string,
  pick: (game: DailyGameBody) => ActionBody,
): Cypress.Chainable<DailyGameBody> {
  return dailyGameOf(account, matchId).then((game) =>
    cy
      .request<DailyGameBody>({
        method: "POST",
        url: apiUrl(`/api/daily/games/${matchId}/actions`),
        headers: bearerHeaders(account.token),
        body: { after: game.seq, nonce: nonce(), action: pick(game) },
      })
      .its("body"),
  );
}

/** The seat keeps its whole opening hand (R9), as spec 42's seat two does, if its mulligan is open. */
export function dailyKeepHandAs(account: E2EAccount, matchId: string): void {
  dailyGameOf(account, matchId).then((game) => {
    if (!game.legal.some((action) => action.type === "mulligan")) return;
    const keep = (game.view.you?.hand ?? []).map((card) => card.instanceId);
    dailyActAs(account, matchId, () => ({ type: "mulligan", keep }));
  });
}

export function dailyEndTurnAs(account: E2EAccount, matchId: string): Cypress.Chainable<DailyGameBody> {
  return dailyActAs(account, matchId, () => ({ type: "endTurn" }));
}

/** Ends the game by a concede, for the next run: a running daily game counts against the account's cap. */
export function dailyConcedeAs(account: E2EAccount, matchId: string): void {
  dailyGameOf(account, matchId).then((game) => {
    if (game.over) return;
    cy.request({
      method: "POST",
      url: apiUrl(`/api/daily/games/${matchId}/actions`),
      headers: bearerHeaders(account.token),
      body: { after: game.seq, nonce: nonce(), action: { type: "concede" } },
      failOnStatusCode: false,
    });
  });
}

/** Leave the account no running daily game, open challenge or seek that an earlier run left. */
export function clearDaily(account: E2EAccount): void {
  dailyOf(account).then((body) => {
    for (const game of body.games.filter((entry) => !entry.over)) dailyConcedeAs(account, game.matchId);
    for (const challenge of body.challenges.outgoing) {
      cy.request({
        method: "DELETE",
        url: apiUrl(`/api/daily/challenges/${challenge.id}`),
        headers: bearerHeaders(account.token),
        failOnStatusCode: false,
      });
    }
    for (const challenge of body.challenges.incoming) {
      cy.request({
        method: "POST",
        url: apiUrl(`/api/daily/challenges/${challenge.id}/decline`),
        headers: bearerHeaders(account.token),
        failOnStatusCode: false,
      });
    }
    for (const seek of body.seeks) {
      cy.request({ method: "DELETE", url: apiUrl(`/api/daily/seeks/${seek.id}`), headers: bearerHeaders(account.token), failOnStatusCode: false });
    }
  });
}

function friendsOf(account: E2EAccount): Cypress.Chainable<FriendsBody> {
  return cy.request<FriendsBody>({ method: "GET", url: apiUrl("/api/friends"), headers: bearerHeaders(account.token) }).its("body");
}

/**
 * `asked` and `asker` as friends, whatever an earlier run left: any block lifted, a friendship kept, and
 * otherwise a request by name and its accept (B43, B45).
 */
export function makeFriends(asker: E2EAccount, asked: E2EAccount, askedName: string, askerId: string, askedId: string): void {
  cy.request({ method: "DELETE", url: apiUrl(`/api/blocks/${askedId}`), headers: bearerHeaders(asker.token), failOnStatusCode: false });
  cy.request({ method: "DELETE", url: apiUrl(`/api/blocks/${askerId}`), headers: bearerHeaders(asked.token), failOnStatusCode: false });
  friendsOf(asker).then((body) => {
    if (body.friends.some((friend) => friend.profileId === askedId)) return;
    if (!body.outgoing.some((request) => request.to.profileId === askedId)) {
      cy.request({ method: "POST", url: apiUrl("/api/friends/requests"), headers: bearerHeaders(asker.token), body: { username: askedName } });
    }
    friendsOf(asked).then((theirs) => {
      const request = theirs.incoming.find((got) => got.from.profileId === askerId);
      expect(request, "B43: the request reached the friend").to.not.eq(undefined);
      cy.request({
        method: "POST",
        url: apiUrl(`/api/friends/requests/${request?.requestId ?? ""}/accept`),
        headers: bearerHeaders(asked.token),
      })
        .its("status")
        .should("eq", 200);
    });
  });
}

/** Moves the E2E server's daily clock; 0 puts it back. */
export function setDailyClock(offsetMs: number): void {
  cy.request({ method: "POST", url: apiUrl(DAILY_CLOCK_PATH), body: { offsetMs } }).its("status").should("eq", 200);
}

/** Asks the server to settle the games whose move ran out, as the Worker's cron does. */
export function sweepDaily(): void {
  cy.request({ method: "POST", url: apiUrl(DAILY_SWEEP_PATH), failOnStatusCode: false });
}
