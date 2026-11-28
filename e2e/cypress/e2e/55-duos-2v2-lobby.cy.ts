// #3731 55: a 2v2 private lobby from Create to the board, four people in two teams (SPEC §9.5, §13.9;
// R3731000, R3731002, R3731003). Cypress drives the host's browser from /play; the other three enter,
// pick and ready by the lobby's routes, as spec 54's guest does, and two of them open their seats over
// `/ws/duos` (`support/duos.ts`), as spec 41's guest does. The second person switches team and the host
// moves the third across, so the teams end as the host and the third against the fourth and the second.
//
// The Start sends no seed, and the seed's coin picks which lobby team plays as team A, so every assertion
// holds either way: the host's team's first seat is A1 or B1, in duel 0 either way. All Random decks, so
// nobody needs a saved one.

import { ROOM_CODE_LENGTH } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, apiUrl, bearerHeaders, routes, timeouts, type E2EAccount } from "../../support/config.ts";
import { DuosSocket, duosPath, endLiveDuos, type Frame } from "../../support/duos.ts";
import { PLAY_LOBBY, PLAY_LOBBY_DIALOG, heroId, ts } from "../../support/testids.ts";
import { duosTestid } from "../../../apps/web/src/duos/testids.ts";
import { lobbyTestid } from "../../../apps/web/src/lobby/testids.ts";

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
      cy.get(`${ts(lobbyTestid.team)}[data-team="${teamOf(role)}"] ${ts(lobbyTestid.seat)}[data-seat="${role}"]`, { timeout: timeouts.view }).should(
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

    // R3731000: the co-op panel, on /play under the duel's (R3732002), makes a 2v2 lobby, which has nothing to set.
    // R3729000: the panels open from Play's Private lobby card.
    cy.visitAs(host, routes.play());
    cy.get(ts(PLAY_LOBBY), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(PLAY_LOBBY_DIALOG), { timeout: timeouts.view }).should("be.visible");
    cy.get(`${ts(lobbyTestid.panel)}[data-format="coop"]`, { timeout: timeouts.view })
      .find(ts(lobbyTestid.create2v2))
      .should("not.be.disabled")
      .click();
    cy.location("pathname", { timeout: timeouts.view }).should("match", /^\/lobby\/[^/]+$/);
    cy.get(ts(lobbyTestid.code), { timeout: timeouts.view })
      .invoke("text")
      .then((text) => {
        code = text.trim();
        expect(code, `a lobby's code is ${String(ROOM_CODE_LENGTH)} characters`).to.have.length(ROOM_CODE_LENGTH);
      });
    cy.get(ts(lobbyTestid.settings)).should("contain.text", "2v2");
    cy.get(ts(lobbyTestid.team)).should("have.length", 2);
    cy.get(`${ts(lobbyTestid.team)}[data-team="1"] ${ts(lobbyTestid.seat)}[data-seat="host"]`).should("have.attr", "data-you", "true");

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
    cy.get(`${ts(lobbyTestid.seat)}[data-seat="guest2"]`).find(ts(lobbyTestid.move)).should("not.be.disabled").click();
    cy.then(() => {
      const moved = names[profiles[third.email] ?? ""] ?? "";
      expect(moved, "the third's name, as the lobby shows it").to.not.eq("");
      cy.get(`${ts(lobbyTestid.team)}[data-team="1"] ${ts(lobbyTestid.seat)}[data-seat="guest"]`, { timeout: timeouts.view }).should("contain.text", moved);
    });
    cy.then(() => {
      expectSeated(code, profiles, names, { host, guest: third, guest2: fourth, guest3: second });
    });

    // A switch and a move take every Ready off, so all four ready now: three by the routes, the host here.
    cy.then(() => {
      for (const account of [second, third, fourth]) lobbyPost(account, code, "ready", { ready: true });
    });
    cy.get(ts(lobbyTestid.deckSelect), { timeout: timeouts.view }).should("not.be.disabled").select("random");
    cy.get(ts(lobbyTestid.ready)).should("not.be.disabled").click();
    cy.get(ts(lobbyTestid.ready), { timeout: timeouts.view }).should("have.attr", "data-ready", "true");
    for (const role of ["guest", "guest2", "guest3"]) {
      cy.get(`${ts(lobbyTestid.seat)}[data-seat="${role}"]`, { timeout: timeouts.view }).should("have.attr", "data-state", "ready");
    }

    // The host's Start lights once all four are ready, and the countdown opens the Duos board.
    cy.get(ts(lobbyTestid.start), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(lobbyTestid.countdown), { timeout: timeouts.view }).should("exist");
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
    cy.get(ts(duosTestid.board), { timeout: timeouts.view })
      .should("have.attr", "data-duel", "0")
      .invoke("attr", "data-seat")
      .then((seat) => {
        hostSeat = String(seat);
        expect(hostSeat, "the host's team's first seat").to.be.oneOf(["a1", "b1"]);
      });
    cy.get(ts(heroId("you"))).should("be.visible");
    cy.then(() => {
      cy.get(ts(duosTestid.teammateStrip)).should("have.attr", "data-seat", TEAMMATE[hostSeat] ?? "");
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
    cy.get(ts(duosTestid.concede), { timeout: timeouts.view }).should("not.be.disabled").click();
    cy.get(ts(duosTestid.concedeStatus)).should("exist");
    cy.wrap(teammate, { timeout: timeouts.view }).should((seat) => {
      expect(seat.view?.concedeOfferedBy, "the offer stands on the teammate's view").to.eq(hostSeat);
    });
    cy.then(() => cy.wrap(teammate.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.get(ts(duosTestid.result), { timeout: timeouts.view }).should("have.attr", "data-outcome", "loss");
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
    cy.get(ts(duosTestid.resultAgain)).should("not.exist");
    cy.then(() => {
      teammate.close();
      across.close();
    });
  });
});
