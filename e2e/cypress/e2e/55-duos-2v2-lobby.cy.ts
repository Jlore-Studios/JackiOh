// #3731 55: a 2v2 private lobby from Create to the board, four people in two teams (SPEC §9.5, §13.9;
// R3731000, R3731002, R3731003). Cypress drives the host's browser on /together; the other three enter,
// pick and ready by the lobby's routes, as spec 54's guest does, and two of them open their seats over
// `/ws/duos` (`support/duos.ts`), as spec 41's guest does. The second person switches team and the host
// moves the third across, so the teams end as the host and the third against the fourth and the second.
//
// The Start sends no seed, and the seed's coin picks which lobby team plays as team A, so every assertion
// holds either way: the host's team's first seat is A1 or B1, in duel 0 either way. All Random decks, so
// nobody needs a saved one.

import { ROOM_CODE_LENGTH } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, apiUrl, bearerHeaders, timeouts, type E2EAccount } from "../../support/config.ts";
import { DuosSocket, TOGETHER_PATH, duosPath, endLiveDuos, type Frame } from "../../support/duos.ts";
import {
  DUOS_BOARD,
  DUOS_CONCEDE,
  DUOS_CONCEDE_STATUS,
  DUOS_RESULT,
  DUOS_RESULT_AGAIN,
  DUOS_TEAMMATE_STRIP,
  LOBBY_CODE,
  LOBBY_COUNTDOWN,
  LOBBY_CREATE_2V2,
  LOBBY_DECK_SELECT,
  LOBBY_MOVE,
  LOBBY_READY,
  LOBBY_SEAT,
  LOBBY_SETTINGS,
  LOBBY_START,
  LOBBY_TEAM,
  heroId,
  ts,
} from "../../support/testids.ts";

/** `GET /api/lobbies/:code`, as far as this spec reads it (apps/web/src/lobby/api.ts's `LobbyView`). */
type LobbyBody = {
  seats: { role: string; player: { profileId: string; username: string } | null; state: string }[];
  matchId: string | null;
};

/** A 2v2's seats in the server's order, the host's team (1) then the other (2). */
const ROLES = ["host", "guest", "guest2", "guest3"] as const;
type Role = (typeof ROLES)[number];

function teamOf(role: Role): "1" | "2" {
  return role === "host" || role === "guest" ? "1" : "2";
}

/** A team's first seat, in duel 0: its teammate in duel 1, and who it faces in duel 0. */
const TEAMMATE: Readonly<Record<string, string>> = { a1: "a2", b1: "b2" };
const ACROSS: Readonly<Record<string, string>> = { a1: "b1", b1: "a1" };

function lobbyPost(account: E2EAccount, code: string, verb: string, body: Record<string, unknown> = {}): void {
  cy.request({ method: "POST", url: apiUrl(`/api/lobbies/${code}/${verb}`), headers: bearerHeaders(account.token), body });
}

/**
 * R3731000: each seat holds the account named for it, as the server reads the lobby, and the host's screen
 * draws each member in that seat of that team. Each member's name as the lobby shows it goes into `names`,
 * by profile id.
 */
function expectSeated(
  code: string,
  profiles: Readonly<Record<string, string>>,
  names: Record<string, string>,
  seated: Readonly<Record<Role, E2EAccount>>,
): void {
  cy.request<LobbyBody>({ method: "GET", url: apiUrl(`/api/lobbies/${code}`), headers: bearerHeaders(seated.host.token) }).then((read) => {
    expect(
      read.body.seats.map((seat) => [seat.role, seat.player?.profileId ?? null]),
      "the seats, in order",
    ).to.deep.eq(ROLES.map((role) => [role, profiles[seated[role].email] ?? ""]));
    for (const seat of read.body.seats) {
      if (seat.player !== null) names[seat.player.profileId] = seat.player.username;
      const role = ROLES.find((each) => each === seat.role);
      if (role === undefined) continue;
      cy.get(`${ts(LOBBY_TEAM)}[data-team="${teamOf(role)}"] ${ts(LOBBY_SEAT)}[data-seat="${role}"]`, { timeout: timeouts.view }).should(
        "contain.text",
        seat.player?.username ?? "",
      );
    }
  });
}

describe("55 2v2 lobby — four people in two teams from Create to the board", () => {
  it("R3731000 R3731002 R3731003 a 2v2 lobby from Create to the board: a switch, a move, four Readys, each seat on its own board, and no Play again", () => {
    const host = accounts.p1();
    const second = accounts.p2();
    const third = accounts.p3();
    const fourth = accounts.p4();
    const profiles: Record<string, string> = {};
    const names: Record<string, string> = {};
    const teammate = new DuosSocket("teammate");
    const across = new DuosSocket("across");
    let code = "";
    let matchId = "";
    let hostSeat = "";

    for (const account of [host, second, third, fourth]) {
      cy.freeAccount(account);
      cy.request<{ profile: { id: string } }>({ method: "GET", url: apiUrl("/api/auth/me"), headers: bearerHeaders(account.token) }).then(
        (me) => {
          profiles[account.email] = me.body.profile.id;
        },
      );
    }
    // A run cut short after its Start left the host's team's game live: the host and the third end it.
    endLiveDuos(host, third);

    // R3731000: Play together's co-op panel makes a 2v2 lobby, which has nothing to set.
    cy.visitAs(host, TOGETHER_PATH);
    cy.get(ts(LOBBY_CREATE_2V2), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.location("pathname", { timeout: timeouts.view }).should("match", /^\/lobby\/[^/]+$/);
    cy.get(ts(LOBBY_CODE), { timeout: timeouts.view })
      .invoke("text")
      .then((text) => {
        code = text.trim();
        expect(code, `a lobby's code is ${String(ROOM_CODE_LENGTH)} characters`).to.have.length(ROOM_CODE_LENGTH);
      });
    cy.get(ts(LOBBY_SETTINGS)).should("contain.text", "2v2");
    cy.get(ts(LOBBY_TEAM)).should("have.length", 2);
    cy.get(`${ts(LOBBY_TEAM)}[data-team="1"] ${ts(LOBBY_SEAT)}[data-seat="host"]`).should("have.attr", "data-you", "true");

    // R3731000: an entrant takes the first empty seat in order. The second switches team, keeping their
    // pick, to the other team's empty seat, and the fourth takes the one the second left.
    cy.then(() => {
      for (const account of [second, third]) {
        lobbyPost(account, code, "enter");
        lobbyPost(account, code, "pick", { random: true });
      }
      lobbyPost(second, code, "team");
      lobbyPost(fourth, code, "enter");
      lobbyPost(fourth, code, "pick", { random: true });
    });
    cy.then(() => {
      expectSeated(code, profiles, names, { host, guest: fourth, guest2: third, guest3: second });
    });

    // R3731000: the host moves the third across; the host's team has no empty seat, so the third swaps with
    // the fourth. The host's screen reads the lobby once the move has landed, and the server's read follows.
    cy.get(`${ts(LOBBY_SEAT)}[data-seat="guest2"]`).find(ts(LOBBY_MOVE)).should("not.be.disabled").click();
    cy.then(() => {
      const moved = names[profiles[third.email] ?? ""] ?? "";
      expect(moved, "the third's name, as the lobby shows it").to.not.eq("");
      cy.get(`${ts(LOBBY_TEAM)}[data-team="1"] ${ts(LOBBY_SEAT)}[data-seat="guest"]`, { timeout: timeouts.view }).should("contain.text", moved);
    });
    cy.then(() => {
      expectSeated(code, profiles, names, { host, guest: third, guest2: fourth, guest3: second });
    });

    // A switch and a move take every Ready off, so all four ready now: three by the routes, the host here.
    cy.then(() => {
      for (const account of [second, third, fourth]) lobbyPost(account, code, "ready", { ready: true });
    });
    cy.get(ts(LOBBY_DECK_SELECT), { timeout: timeouts.view }).should("not.be.disabled").select("random");
    cy.get(ts(LOBBY_READY)).should("not.be.disabled").click();
    cy.get(ts(LOBBY_READY), { timeout: timeouts.view }).should("have.attr", "data-ready", "true");
    for (const role of ["guest", "guest2", "guest3"]) {
      cy.get(`${ts(LOBBY_SEAT)}[data-seat="${role}"]`, { timeout: timeouts.view }).should("have.attr", "data-state", "ready");
    }

    // The host's Start lights once all four are ready, and the countdown opens the Duos board.
    cy.get(ts(LOBBY_START), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(LOBBY_COUNTDOWN), { timeout: timeouts.view }).should("exist");
    cy.then(() => {
      cy.request<LobbyBody>({ method: "GET", url: apiUrl(`/api/lobbies/${code}`), headers: bearerHeaders(fourth.token) }).then((read) => {
        matchId = read.body.matchId ?? "";
        expect(matchId, "the Start began a match").to.be.a("string").and.not.eq("");
      });
    });
    cy.then(() => {
      cy.location("pathname", { timeout: timeouts.view }).should("eq", duosPath(matchId));
    });

    // R3731002: the host's board is their own duel, duel 0, from their team's first seat: as its Player 2
    // when the coin made the host's team team B.
    cy.get(ts(DUOS_BOARD), { timeout: timeouts.view })
      .should("have.attr", "data-duel", "0")
      .invoke("attr", "data-seat")
      .then((seat) => {
        hostSeat = String(seat);
        expect(hostSeat, "the host's team's first seat").to.be.oneOf(["a1", "b1"]);
      });
    cy.get(ts(heroId("you"))).should("be.visible");
    cy.then(() => {
      cy.get(ts(DUOS_TEAMMATE_STRIP)).should("have.attr", "data-seat", TEAMMATE[hostSeat] ?? "");
      cy.jackioh().should((handle) => {
        expect(handle.seat, "the host's own duel, from their side").to.eq(hostSeat === "b1" ? "p2" : "p1");
      });
    });

    // R3731002: the third, moved onto the host's team, is the host's teammate; the fourth, the other team's
    // first seat, faces the host in duel 0. Each is sent their own seat's view, their own duel from their side.
    cy.then(() => cy.wrap(Promise.all([teammate.open(third.token, matchId), across.open(fourth.token, matchId)]), { timeout: timeouts.view })).then(
      (first) => {
        const frames = first as Frame[];
        expect(frames.map((frame) => frame.type), "hello is answered with each seat's view").to.deep.eq(["view", "view"]);
        expect(teammate.view?.seat, "the third is the host's teammate").to.eq(TEAMMATE[hostSeat]);
        expect(across.view?.seat, "the fourth faces the host in duel 0").to.eq(ACROSS[hostSeat]);
        for (const seat of [teammate, across]) {
          const team = seat.view?.seat?.slice(0, 1);
          expect(seat.view?.own?.viewer, `seat ${seat.view?.seat ?? "?"} plays its own duel from its side`).to.eq(team === "b" ? "p2" : "p1");
          expect(seat.view?.team?.team, "its own team first").to.eq(team);
        }
      },
    );

    // R3731002, R3731003: the host's team concedes, the host offering in the browser and the third agreeing;
    // the other team wins, and a 2v2 offers no Play again.
    cy.get(ts(DUOS_CONCEDE), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(DUOS_CONCEDE_STATUS)).should("exist");
    cy.wrap(teammate, { timeout: timeouts.view }).should((seat) => {
      expect(seat.view?.concedeOfferedBy, "the offer stands on the teammate's view").to.eq(hostSeat);
    });
    cy.then(() => cy.wrap(teammate.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.get(ts(DUOS_RESULT), { timeout: timeouts.view }).should("have.attr", "data-outcome", "loss");
    cy.wrap(across, { timeout: timeouts.view }).should((seat) => {
      expect(seat.view?.result?.reason, "the game ends by concede").to.eq("concede");
      expect(seat.view?.result?.winner, "and the other team wins").to.eq(hostSeat.startsWith("a") ? "b" : "a");
    });
    cy.then(() => {
      cy.request<{ unavailable?: boolean }>({
        method: "GET",
        url: apiUrl(`/api/duos/matches/${matchId}/again`),
        headers: bearerHeaders(host.token),
      })
        .its("body.unavailable")
        .should("eq", true);
    });
    cy.get(ts(DUOS_RESULT_AGAIN)).should("not.exist");
    cy.then(() => {
      teammate.close();
      across.close();
    });
  });
});
