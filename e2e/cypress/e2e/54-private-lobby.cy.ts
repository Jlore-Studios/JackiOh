// #3730 54: a 1v1 private lobby from Create to both players on the board (SPEC §9.5; R3730000 to R3730002).
// Cypress drives the host's browser; the guest enters, picks and readies by the lobby's routes and opens
// its socket onto the match the host's Start began, as spec 06's second seat does (`wsPlayer`).
// All Random, so neither seat needs a saved deck; the seats fall by the seed's coin (R1989000).

import { ROOM_CODE_LENGTH } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, apiUrl, bearerHeaders, routes, server, timeouts } from "../../support/config.ts";
import {
  LOBBY_CODE,
  LOBBY_COUNTDOWN,
  LOBBY_CREATE,
  LOBBY_PANEL,
  LOBBY_READY,
  LOBBY_SEAT,
  LOBBY_SETTINGS,
  LOBBY_START,
  heroId,
  playModeId,
  ts,
} from "../../support/testids.ts";

const GUEST = "lobby-guest";
const MATCH_PATH = /^\/match\/([^/]+)$/;

describe("54 private lobby — a 1v1 lobby from Create to the board", () => {
  it("R3730002 a 1v1 lobby from Create to both players on the board", () => {
    const host = accounts.p1();
    const guest = accounts.p2();
    let code = "";
    let matchId = "";

    cy.freeAccount(host);
    cy.freeAccount(guest);

    // R3730000: Play's "Play a friend" makes a lobby in the mode Play has chosen.
    cy.visitAs(host, routes.play());
    cy.get(ts(playModeId("random")), { timeout: timeouts.view }).check();
    // R3732002: /play holds two lobby panels, the duel's and the co-op one's; this is the duel's.
    cy.get(`${ts(LOBBY_PANEL)}[data-format="duel"]`).find(ts(LOBBY_CREATE)).should("not.be.disabled").click();
    cy.location("pathname", { timeout: timeouts.view }).should("match", /^\/lobby\/[^/]+$/);
    cy.get(ts(LOBBY_CODE), { timeout: timeouts.view })
      .invoke("text")
      .then((text) => {
        code = text.trim();
        expect(code, `a lobby's code is ${String(ROOM_CODE_LENGTH)} characters`).to.have.length(ROOM_CODE_LENGTH);
      });
    cy.get(ts(LOBBY_SETTINGS)).should("have.value", "random");
    cy.get(ts(LOBBY_START)).should("be.disabled");

    // R3730001: the host takes the dealt deck and is ready; the guest enters, picks and is ready by the routes.
    cy.get(ts(LOBBY_READY)).should("not.be.disabled").click();
    cy.get(ts(LOBBY_READY), { timeout: timeouts.view }).should("have.attr", "data-ready", "true");
    cy.then(() => {
      for (const [verb, body] of [
        ["enter", {}],
        ["pick", { mode: "random" }],
        ["ready", { ready: true }],
      ] as const) {
        cy.request({ method: "POST", url: apiUrl(`/api/lobbies/${code}/${verb}`), headers: bearerHeaders(guest.token), body });
      }
    });
    cy.get(`${ts(LOBBY_SEAT)}[data-seat="guest"]`, { timeout: timeouts.view }).should("have.attr", "data-state", "ready");

    // R3730002: the Start lights, counts down, and opens the board.
    cy.get(ts(LOBBY_START), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(LOBBY_COUNTDOWN), { timeout: timeouts.view }).should("exist");
    cy.location("pathname", { timeout: timeouts.view })
      .should("match", MATCH_PATH)
      .then((pathname) => {
        matchId = MATCH_PATH.exec(pathname)?.[1] ?? "";
        expect(matchId, "the match the Start began").to.not.eq("");
      });
    cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");

    // The guest's view of the lobby names the same match, and its socket opens onto it.
    cy.then(() => {
      cy.request<{ matchId: string | null; startedAt: number | null }>({
        method: "GET",
        url: apiUrl(`/api/lobbies/${code}`),
        headers: bearerHeaders(guest.token),
      }).then((read) => {
        expect(read.body.matchId, "the guest reads the match the Start began").to.eq(matchId);
        expect(read.body.startedAt).to.be.a("number");
      });
    });
    cy.then(() => {
      cy.wsPlayer({ action: "connect", name: GUEST, url: server.ws(), token: guest.token, matchId }).should((result) => {
        expect(result.view, "the guest's socket hears its own view").to.be.an("object");
      });
    });

    // Free both accounts for the next spec.
    cy.then(() => {
      cy.wsPlayer({ action: "disconnect", name: GUEST });
      cy.concedeAs(host, matchId).its("ok").should("eq", true);
    });
  });
});
