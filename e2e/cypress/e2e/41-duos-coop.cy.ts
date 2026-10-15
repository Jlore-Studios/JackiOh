// #936 41: Duos co-op against the Easy AI, two people on team A (B60–B64, and B39's client half).
// Cypress drives one browser, so the host (A1) plays in it and the guest (A2) is a second client
// this spec drives over `/ws/duos` itself, as the networked specs drive their second seat over
// `/ws/match` (spec 06's `wsPlayer`, whose frames are the 1v1 protocol's, not S6's).
//
// What it asserts, by behaviour:
// - B60 "The co-op panel creates a room (difficulty, deck or random), shows its code, … joins by code,
//   and navigates both players to `/duos/:matchId` once the guest joins."
// - B61 "The Duos board renders `Game.tsx` for the own duel; a life strip shows own HP (`duos-own-hp`)
//   and the team pool (`duos-pool`) for both teams."
// - B62 "The pass control (`duos-pass`) on an inspected hand card … sends `Pass` only when `legal`
//   holds it, showing cost and passes left."
// - B63 "The portal (`duos-portal`) flips the board to the teammate's duel read-only …; the teammate
//   strip (`duos-teammate-strip`) always shows their HP, mana, hand count and whether they have
//   ended; 'Waiting for your teammate' shows after you end."
// - B64 "The ping wheel (`duos-ping-wheel`) … offers the five pings; a received ping … marks the card
//   (`duos-ping-marker`) … and flashes the portal when it is on the other view."
// - B39 (client half): a ping goes to the teammate only, and the teammate's arrives.
// - §13.9 (#965): the result panel's "Watch the replay" plays the host's own seat's steps with the replay controls, offering nothing live.
//
// S6's room takes no seed, so the deal is not fixed: every assertion holds for any deal. Where B9
// ends a seat that has nothing but End turn to do, the guest ends too and the spec waits for the
// team's next round, until the host has a round with moves in it.

import { ROOM_CODE_LENGTH } from "../../../apps/web/src/wire/serverConfig.ts";
import { accounts, server, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  COOP_CODE,
  COOP_CREATE,
  COOP_DIFFICULTY,
  DUOS_BOARD,
  DUOS_CARD_ACTIONS,
  DUOS_CONCEDE,
  DUOS_CONCEDE_STATUS,
  DUOS_OWN_HP,
  DUOS_PASS,
  DUOS_PING_MARKER,
  DUOS_PING_WHEEL,
  DUOS_POOL,
  DUOS_PORTAL,
  DUOS_REPLAY,
  DUOS_RESULT,
  DUOS_RESULT_REPLAY,
  DUOS_TEAMMATE_STRIP,
  DUOS_WAITING,
  END_TURN,
  REPLAY_BACK,
  REPLAY_BACK_STEP,
  REPLAY_END,
  REPLAY_FORWARD_STEP,
  REPLAY_READOUT,
  REPLAY_SCRUBBER,
  duosPingId,
  handCardId,
  heroId,
  ts,
} from "../../support/testids.ts";

/** S8's paths; support/config.ts's `routes` predate them. */
const TOGETHER_PATH = "/together";
function duosPath(matchId: string): string {
  return `/duos/${matchId}`;
}

/** How many of team A's rounds the spec plays through looking for one where the host has a move. */
const ROUNDS_BUDGET = 6;

/** The host's slot and the guest's in a team's per-seat arrays: their duels (A1 is duel 0, A2 duel 1). */
const HOST_SLOT = 0;
const GUEST_SLOT = 1;

/** `handCardId("")` is the prefix, so no spec spells a testid format. */
const HAND_CARD_PREFIX = handCardId("");

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(token: string): Record<string, string> {
  return { authorization: `Bearer ${token}` };
}

/** `/ws/duos` beside the configured `/ws/match` (S6), as the client's `duosSocketUrl` finds it. */
function duosSocketUrl(): string {
  return server.ws().replace(/\/ws\/match\/?$/u, "/ws/duos");
}

// The views as far as this spec reads them (S2's `DuosView`, camelCase).

type Frame = { type: string; [key: string]: unknown };
type CardLike = { instanceId: string };
type PlayerViewLike = {
  active?: string;
  you?: { player?: string; hand?: CardLike[] | { count: number } };
  pending?: { forYou?: boolean; kind?: string } | null;
};
type TeamLike = { team?: string; pool?: number; passesLeft?: number[]; ended?: boolean[] };
type DuosViewLike = {
  seat?: string;
  own?: PlayerViewLike;
  teammate?: PlayerViewLike;
  team?: TeamLike;
  enemy?: TeamLike;
  round?: number;
  concedeOfferedBy?: string | null;
  result?: { outcome?: string; winner?: string | null; reason?: string } | null;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function ownHandIds(view: PlayerViewLike | undefined): string[] {
  const hand = view?.you?.hand;
  return Array.isArray(hand) ? hand.map((card) => card.instanceId) : [];
}

/** A round of team A's is open: past the mulligans, nobody has won, and the duels' P1 is active. */
function teamARoundOpen(view: DuosViewLike | null): boolean {
  return view !== null && (view.round ?? 0) >= 1 && (view.result ?? null) === null && view.own?.active === "p1";
}

type Waiter = { match: (frame: Frame) => boolean; resolve: (frame: Frame) => void; timer: ReturnType<typeof setTimeout> };

/** A seat on `/ws/duos` the spec drives by hand: S6's hello, actions under nonces, pings. */
class DuosSocket {
  readonly frames: Frame[] = [];
  view: DuosViewLike | null = null;
  private socket: WebSocket | null = null;
  private waiters: Waiter[] = [];
  private sent = 0;

  constructor(private readonly name: string) {}

  private deliver(frame: Frame): void {
    this.frames.push(frame);
    if (frame.type === "view" && isRecord(frame.view)) this.view = frame.view as DuosViewLike;
    for (const waiter of [...this.waiters]) {
      if (!waiter.match(frame)) continue;
      clearTimeout(waiter.timer);
      this.waiters = this.waiters.filter((other) => other !== waiter);
      waiter.resolve(frame);
    }
  }

  private waitFor(match: (frame: Frame) => boolean, what: string): Promise<Frame> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.waiters = this.waiters.filter((waiter) => waiter.timer !== timer);
        reject(new Error(`${this.name}: no ${what} on /ws/duos`));
      }, timeouts.view);
      this.waiters.push({ match, resolve, timer });
    });
  }

  /** Opens the socket (auth as `/ws/match`: the token and the match in the query) and says hello. */
  open(token: string, matchId: string): Promise<Frame> {
    const url = new URL(duosSocketUrl());
    url.searchParams.set("token", token);
    url.searchParams.set("matchId", matchId);
    const socket = new WebSocket(url.toString());
    this.socket = socket;
    socket.addEventListener("message", (event: MessageEvent) => {
      if (typeof event.data !== "string") return;
      let parsed: unknown;
      try {
        parsed = JSON.parse(event.data);
      } catch {
        return;
      }
      if (isRecord(parsed) && typeof parsed.type === "string") this.deliver(parsed as Frame);
    });
    const first = this.waitFor((frame) => frame.type === "view" || frame.type === "error", "first view");
    socket.addEventListener("open", () => {
      socket.send(JSON.stringify({ type: "hello", matchId }));
    });
    return first;
  }

  /** One action; resolves with its `ack`, or with the `error` that refused it. */
  act(body: Record<string, unknown>): Promise<Frame> {
    this.sent += 1;
    const nonce = `${this.name}-${String(this.sent)}`;
    const reply = this.waitFor(
      (frame) => (frame.type === "ack" || frame.type === "error") && (frame.nonce === nonce || frame.nonce === undefined),
      `answer to ${nonce}`,
    );
    this.socket?.send(JSON.stringify({ type: "action", nonce, body }));
    return reply;
  }

  ping(ping: string, target: Record<string, unknown>): void {
    this.socket?.send(JSON.stringify({ type: "ping", ping, target }));
  }

  close(): void {
    this.socket?.close();
    this.socket = null;
  }
}

/**
 * A co-op match an earlier run left live for these two accounts ends here: both teammates concede
 * (B11), each from a socket of its own. Nothing to do when neither is in one.
 */
function endLiveDuos(first: E2EAccount, second: E2EAccount): void {
  const live: (string | null)[] = [];
  for (const account of [first, second]) {
    cy.request<{ matchId: string | null }>({ method: "GET", url: api("/api/duos/live"), headers: bearer(account.token) }).then(
      (answer) => {
        live.push(answer.body.matchId);
      },
    );
  }
  cy.then(() => {
    const [one, two] = live;
    if (one === null || one === undefined || one !== two) return;
    const firstSeat = new DuosSocket("cleanup-a");
    const secondSeat = new DuosSocket("cleanup-b");
    cy.wrap(Promise.all([firstSeat.open(first.token, one), secondSeat.open(second.token, one)]), { timeout: timeouts.view });
    cy.then(() => cy.wrap(firstSeat.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.then(() => cy.wrap(secondSeat.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.then(() => {
      firstSeat.close();
      secondSeat.close();
    });
  });
}

/** B2: the guest's own mulligan is open; it keeps its whole hand. */
function guestKeepsMulligan(guest: DuosSocket): void {
  cy.wrap(guest, { timeout: timeouts.view }).should((seat) => {
    const pending = seat.view?.own?.pending;
    expect(pending?.forYou === true && pending.kind === "mulligan", "B2: the guest's own mulligan is open").to.eq(true);
  });
  cy.then(() =>
    cy.wrap(guest.act({ kind: "duel", body: { type: "mulligan", keep: ownHandIds(guest.view?.own) } }), {
      timeout: timeouts.view,
    }),
  ).then((reply) => {
    expect((reply as Frame).type, "the guest's mulligan is accepted").to.eq("ack");
  });
}

/**
 * Waits for a round of team A's in which the host has moves (B4). A round where B9 already ended the
 * host (nothing but End turn to do) is closed by the guest ending too, and the next one awaited.
 */
function untilHostMayAct(guest: DuosSocket, rounds: number): void {
  cy.wrap(guest, { timeout: timeouts.game }).should((seat) => {
    expect(teamARoundOpen(seat.view), "a round of team A's is open").to.eq(true);
  });
  cy.then(() => {
    const view = guest.view;
    if (view?.team?.ended?.[HOST_SLOT] === false) return;
    expect(rounds, "the host gets a round with a move in it").to.be.greaterThan(0);
    const round = view?.round ?? 0;
    if (view?.team?.ended?.[GUEST_SLOT] === false) {
      cy.wrap(guest.act({ kind: "duel", body: { type: "endTurn" } }), { timeout: timeouts.view });
    }
    // B5: the round closes, the AI team plays the next, and team A's comes round again.
    cy.wrap(guest, { timeout: timeouts.game }).should((seat) => {
      expect(seat.view?.round ?? 0, "team A's next round").to.be.at.least(round + 2);
    });
    untilHostMayAct(guest, rounds - 1);
  });
}

describe("40 duos co-op — two players against the Easy AI", () => {
  it("B60–B64: make and join a room, pass, ping both ways, look through the portal, end the round together and concede as a team", () => {
    const host = accounts.p1();
    const guestAccount = accounts.p2();
    const guest = new DuosSocket("guest");
    let code = "";
    let matchId = "";
    let guestHand = 0;
    let passesBefore = 0;
    let roundOfPass = 0;
    let guestCards: string[] = [];
    let guestStillToEnd = false;

    cy.freeAccount(host);
    cy.freeAccount(guestAccount);
    endLiveDuos(host, guestAccount);

    // B60: the host's co-op panel makes an Easy room with a random deck and shows its code.
    cy.visitAs(host, TOGETHER_PATH);
    cy.get(ts(COOP_DIFFICULTY), { timeout: timeouts.view }).select("easy");
    cy.get(ts(COOP_CREATE)).should("not.be.disabled").click();
    cy.get(ts(COOP_CODE), { timeout: timeouts.view })
      .invoke("text")
      .then((text) => {
        code = text.trim();
        expect(code, `S6: a room code is ${String(ROOM_CODE_LENGTH)} characters`).to.have.length(ROOM_CODE_LENGTH);
      });

    // B33: the guest joins by the code, and the host's panel follows the room to the board (B60).
    cy.then(() => {
      cy.request<{ matchId: string }>({
        method: "POST",
        url: api(`/api/duos/rooms/${code}/join`),
        headers: bearer(guestAccount.token),
        body: { random: true },
      }).then((joined) => {
        matchId = joined.body.matchId;
        expect(matchId, "the join answers the match it started").to.be.a("string").and.not.eq("");
      });
    });
    cy.then(() => {
      cy.location("pathname", { timeout: timeouts.view }).should("eq", duosPath(matchId));
    });
    cy.then(() => cy.wrap(guest.open(guestAccount.token, matchId), { timeout: timeouts.view })).then((first) => {
      const frame = first as Frame;
      expect(frame.type, "B35: hello is answered with the seat's view").to.eq("view");
      expect((frame.view as DuosViewLike | undefined)?.seat, "the guest is A2 (S6)").to.eq("a2");
    });

    // B61: the own duel's board, and both teams' life.
    cy.get(ts(DUOS_BOARD), { timeout: timeouts.view }).should("have.attr", "data-seat", "a1").and("have.attr", "data-showing", "own");
    cy.get(ts(heroId("you"))).should("be.visible");
    cy.get(ts(DUOS_OWN_HP)).first().should("have.attr", "data-seat", "a1").and("have.attr", "data-spent", "false");
    cy.get(ts(DUOS_OWN_HP)).should("have.length", 4);
    cy.get(`${ts(DUOS_POOL)}[data-team="b"]`).should("exist");
    cy.wrap(guest).should((seat) => {
      expect(seat.view?.team?.pool, "the team's pool, as the guest's view has it").to.be.a("number");
    });
    cy.then(() => {
      cy.get(`${ts(DUOS_POOL)}[data-team="a"]`).should("have.attr", "data-value", String(guest.view?.team?.pool));
    });
    cy.get(ts(DUOS_TEAMMATE_STRIP)).should("have.attr", "data-seat", "a2");

    // B2: all four mulligans; round 1 opens once they are in.
    cy.keepMulligans();
    guestKeepsMulligan(guest);
    untilHostMayAct(guest, ROUNDS_BUDGET);
    cy.get(ts(END_TURN), { timeout: timeouts.view }).should("not.be.disabled");
    cy.settled();

    // B62: the pass control on a hand card, with its cost and the passes left; B20: the card lands
    // at the end of the teammate's hand, and B22: one pass fewer.
    cy.wrap(guest).should((seat) => {
      guestHand = ownHandIds(seat.view?.own).length;
      passesBefore = seat.view?.team?.passesLeft?.[HOST_SLOT] ?? 0;
      roundOfPass = seat.view?.round ?? 0;
      expect(passesBefore, "the host has passes left").to.be.greaterThan(0);
    });
    cy.get(`[data-testid^="${HAND_CARD_PREFIX}"]`).first().trigger("pointerover", { pointerType: "mouse", force: true });
    cy.get(ts(DUOS_CARD_ACTIONS)).should("exist");
    cy.then(() => {
      cy.get(ts(DUOS_PASS)).should("not.be.disabled").and("contain.text", String(passesBefore)).click();
    });
    cy.wrap(guest, { timeout: timeouts.view }).should((seat) => {
      expect(ownHandIds(seat.view?.own).length, "B20: the passed card joins the guest's hand").to.eq(guestHand + 1);
      expect(seat.view?.team?.passesLeft?.[HOST_SLOT], "B22: the host has one pass fewer").to.eq(passesBefore - 1);
    });

    // B64 / B39: the host pings the enemy hero; the marker is the host's own, and the ping reaches the guest.
    cy.get(ts(heroId("opponent"))).trigger("pointerover", { pointerType: "mouse" });
    cy.get(ts(DUOS_PING_WHEEL)).should("be.visible");
    cy.get(ts(duosPingId("killThis"))).click();
    cy.get(`${ts(DUOS_PING_MARKER)}[data-from="a1"][data-ping="killThis"]`).should("exist");
    cy.wrap(guest, { timeout: timeouts.view }).should((seat) => {
      const ping = seat.frames.find((frame) => frame.type === "ping");
      expect(ping, "B39: the host's ping is relayed to the teammate").to.not.eq(undefined);
      expect(ping?.from).to.eq("a1");
      expect(ping?.ping).to.eq("killThis");
      expect(ping?.target).to.deep.eq({ duel: 0, at: "hero", side: "p2" });
    });

    // B64: the guest's ping on the host's own duel marks it there; one on the guest's duel flashes the portal.
    cy.then(() => {
      guest.ping("careful", { duel: 0, at: "hero", side: "p1" });
    });
    cy.get(`${ts(DUOS_PING_MARKER)}[data-from="a2"][data-ping="careful"]`, { timeout: timeouts.view }).should("exist");
    cy.then(() => {
      guest.ping("portal", { duel: 1, at: "hero", side: "p1" });
    });
    cy.get(ts(DUOS_PORTAL), { timeout: timeouts.view }).should("have.attr", "data-flash", "ping");

    // B63: the portal shows the teammate's own board — their hand among it — and offers nothing.
    cy.wrap(guest).should((seat) => {
      guestCards = ownHandIds(seat.view?.own);
      expect(guestCards.length, "the guest holds cards").to.be.greaterThan(0);
    });
    cy.get(ts(DUOS_PORTAL)).click();
    cy.get(ts(DUOS_BOARD)).should("have.attr", "data-showing", "teammate");
    cy.then(() => {
      cy.get(ts(handCardId(guestCards[0] ?? ""))).should("exist");
    });
    cy.get(ts(END_TURN)).should("be.disabled");
    cy.then(() => {
      cy.get(ts(DUOS_TEAMMATE_STRIP)).should("have.attr", "data-hand", String(guestCards.length));
    });
    cy.get(ts(DUOS_PORTAL)).click();
    cy.get(ts(DUOS_BOARD)).should("have.attr", "data-showing", "own");

    // B5, B63: the host ends; while the guest has not, "Waiting for your teammate"; then the round closes.
    cy.then(() => {
      // Read before the host's end, which closes the round at once when B9 already ended the guest.
      guestStillToEnd = guest.view?.team?.ended?.[GUEST_SLOT] === false;
      if (guest.view?.team?.ended?.[HOST_SLOT] === false) cy.endTurn({ handOver: false });
    });
    cy.then(() => {
      if (!guestStillToEnd) return;
      cy.get(ts(DUOS_WAITING), { timeout: timeouts.view }).should("be.visible");
      cy.wrap(guest, { timeout: timeouts.view }).should((seat) => {
        expect(seat.view?.team?.ended?.[HOST_SLOT], "B5: the host's end is held until the guest's").to.eq(true);
      });
      cy.then(() => cy.wrap(guest.act({ kind: "duel", body: { type: "endTurn" } }), { timeout: timeouts.view })).then(
        (reply) => {
          expect((reply as Frame).type, "the guest's end turn is accepted").to.eq("ack");
        },
      );
    });
    cy.wrap(guest, { timeout: timeouts.game }).should((seat) => {
      expect(seat.view?.round ?? 0, "B5: both ended, so the round closed").to.be.greaterThan(roundOfPass);
    });
    cy.get(ts(DUOS_WAITING)).should("not.exist");

    // B11: the host offers to concede, the guest agrees, and the AI team wins.
    cy.get(ts(DUOS_CONCEDE), { timeout: timeouts.game }).should("not.be.disabled").click();
    cy.get(ts(DUOS_CONCEDE_STATUS)).should("exist");
    cy.wrap(guest, { timeout: timeouts.view }).should((seat) => {
      expect(seat.view?.concedeOfferedBy, "B11: the offer stands on the teammate's view").to.eq("a1");
    });
    cy.then(() => cy.wrap(guest.act({ kind: "concede" }), { timeout: timeouts.view }));
    cy.get(ts(DUOS_RESULT), { timeout: timeouts.view }).should("have.attr", "data-outcome", "loss");
    cy.wrap(guest, { timeout: timeouts.view }).should((seat) => {
      expect(seat.view?.result?.reason, "B11: the game ends by concede").to.eq("concede");
      expect(seat.view?.result?.winner, "and the other team wins").to.eq("b");
    });

    // §13.9: the result panel's replay, the host's own seat's steps, with nothing live on it.
    cy.get(ts(DUOS_RESULT_REPLAY)).click();
    cy.get(ts(DUOS_REPLAY), { timeout: timeouts.view }).should("have.attr", "data-step", "0");
    cy.get(ts(DUOS_RESULT)).should("not.exist");
    cy.get(ts(DUOS_CONCEDE)).should("not.exist");
    cy.get(ts(DUOS_WAITING)).should("not.exist");
    cy.get(ts(heroId("opponent"))).trigger("pointerover", { pointerType: "mouse" });
    cy.get(ts(DUOS_PING_WHEEL)).should("not.exist");
    cy.get(ts(REPLAY_FORWARD_STEP)).click();
    cy.get(ts(DUOS_REPLAY)).should("have.attr", "data-step", "1");
    cy.get(ts(REPLAY_BACK_STEP)).click();
    cy.get(ts(DUOS_REPLAY)).should("have.attr", "data-step", "0");
    cy.get(ts(REPLAY_SCRUBBER))
      .invoke("attr", "max")
      .then((max) => {
        const last = Number(max);
        expect(last, "the replay holds the game's steps").to.be.greaterThan(1);
        cy.get(ts(REPLAY_END)).click();
        cy.get(ts(DUOS_REPLAY)).should("have.attr", "data-step", String(last));
        cy.get(ts(REPLAY_SCRUBBER)).should("have.value", String(last));
      });
    cy.get(ts(REPLAY_READOUT)).should("contain.text", "Round");
    cy.get(ts(REPLAY_BACK)).click();
    cy.get(ts(DUOS_RESULT)).should("have.attr", "data-outcome", "loss");
    cy.then(() => {
      guest.close();
    });
  });
});
