// #3727: a daily game's board. Each action is one POST sent after the last action read, under a fresh
// nonce, and the board draws the answer; a stale board reads the game again; Next game appears once the
// move has passed on; the result says Lost on time or Won on time; an older card version says so; a
// finished game's Rematch challenges its opponent again at its mode and time per move.

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DailyGame } from "../net/daily.ts";
import { clearReadCache } from "../net/readCache.ts";
import { resetSettings, writeSettings } from "../settings/store.ts";
import { baseView } from "../test/fixtures.ts";
import { at, refusal, signIn, stubServer, type Call, type Reply } from "../test/stubServer.ts";
import { dailyConstants } from "../wasm/index.ts";
import DailyBoard from "./DailyBoard.tsx";
import { dailyTestid } from "./testids.ts";

/** The board's first read and a move's answer can take longer than the 1 s default under a full suite. */
const SLOW = { timeout: 5_000 } as const;

const END_TURN: ActionBody = { type: "endTurn" };
const CONCEDE: ActionBody = { type: "concede" };

function dailyGame(over: Partial<DailyGame> = {}): DailyGame {
  const { MS_PER_DAY, MS_PER_HOUR } = dailyConstants();
  const now = Date.now();
  return {
    matchId: "g-1",
    seat: "p1",
    view: baseView(),
    legal: [END_TURN, CONCEDE],
    seq: 7,
    holds: [{ player: "p1", key: "turn:active", since: now, deadline: now + MS_PER_DAY + 4 * MS_PER_HOUR + MS_PER_HOUR / 2 }],
    hasMove: true,
    perMoveDays: 3,
    mode: "random",
    catalogVersion: "v1",
    olderCatalog: false,
    opponent: { profileId: "o-1", username: "Opp#7" },
    over: false,
    lostOnTime: false,
    nextGame: null,
    ...over,
  };
}

/** The game's reads and actions, each answered by `answer`, else the read. */
function serve(read: () => DailyGame, answer?: (call: Call) => Reply | undefined) {
  return stubServer((call) => {
    const special = answer?.(call);
    if (special !== undefined) return special;
    if (call.method === "GET" && call.path === "/api/daily/games/g-1") return { status: 200, body: read() };
    return undefined;
  });
}

beforeEach(() => {
  window.localStorage.clear();
  clearReadCache();
  signIn("tok-daily-board");
  at("/daily/g-1");
});

afterEach(() => {
  cleanup();
  resetSettings();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("#3727 a daily game's board", () => {
  it("#3727 End turn reads End turn and send, and an action is sent after the last seq under a fresh nonce", async () => {
    const after = dailyGame({ seq: 8, hasMove: false, legal: [CONCEDE], view: baseView({ active: "p2" }), holds: [] });
    const server = serve(dailyGame, (call) => (call.method === "POST" ? { status: 200, body: after } : undefined));
    render(<DailyBoard matchId="g-1" />);

    const endTurn = await screen.findByTestId("end-turn", undefined, SLOW);
    expect(endTurn).toHaveTextContent("End turn and send");
    expect(screen.getByTestId(dailyTestid.clock)).toHaveTextContent("Your move · 1d 4h left");
    expect(screen.getByTestId(dailyTestid.clock)).toHaveAttribute("data-holder", "you");
    fireEvent.click(endTurn);

    await waitFor(() => {
      expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(1);
    }, SLOW);
    const body = server.to("POST", "/api/daily/games/g-1/actions")[0]?.body as { after: number; nonce: string; action: ActionBody };
    expect(body.after).toBe(7);
    expect(body.action).toEqual(END_TURN);
    expect(typeof body.nonce).toBe("string");
    expect(body.nonce.length).toBeGreaterThan(0);
    await waitFor(() => {
      expect(screen.getByTestId(dailyTestid.board)).toHaveAttribute("data-has-move", "false");
    }, SLOW);
  });

  it("#3727 each action goes under its own nonce", async () => {
    let seq = 7;
    const server = serve(
      () => dailyGame({ seq }),
      (call) => {
        if (call.method !== "POST") return undefined;
        seq += 1;
        return { status: 200, body: dailyGame({ seq, view: baseView({ turn: seq }) }) };
      },
    );
    render(<DailyBoard matchId="g-1" />);
    fireEvent.click(await screen.findByTestId("end-turn", undefined, SLOW));
    await waitFor(() => {
      expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(1);
    }, SLOW);
    await waitFor(() => {
      expect(screen.getByTestId("end-turn")).toBeEnabled();
    }, SLOW);
    fireEvent.click(screen.getByTestId("end-turn"));
    await waitFor(() => {
      expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(2);
    }, SLOW);
    const [first, second] = server.to("POST", "/api/daily/games/g-1/actions").map((call) => call.body as { after: number; nonce: string });
    expect(first?.after).toBe(7);
    expect(second?.after).toBe(8);
    expect(first?.nonce).not.toBe(second?.nonce);
  });

  it("#3727 End turn automatically waits for the player's move: none is sent while the opponent holds it", async () => {
    // The setting is off and the engine's is on, so the board's Game wants to send it at once.
    writeSettings({ autoEndTurn: false });
    let mine = false;
    const server = serve(
      () => (mine ? dailyGame({ seq: 9 }) : dailyGame({ hasMove: false, legal: [CONCEDE], view: baseView({ active: "p2" }), holds: [] })),
      // The concede below meets a game that moved on: the move is the player's now.
      (call) => {
        if (call.method !== "POST" || (call.body as { action: ActionBody }).action.type !== "concede") return undefined;
        mine = true;
        return refusal(409, "stale", "The game moved on.");
      },
    );
    render(<DailyBoard matchId="g-1" />);
    fireEvent.click(await screen.findByTestId("concede", undefined, SLOW));
    expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(0);
    fireEvent.click(await screen.findByTestId("concede-confirm", undefined, SLOW));

    await waitFor(() => {
      expect(screen.getByTestId(dailyTestid.board)).toHaveAttribute("data-has-move", "true");
    }, SLOW);
    await waitFor(() => {
      expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(2);
    }, SLOW);
    const actions = server.to("POST", "/api/daily/games/g-1/actions").map((call) => call.body as { after: number; action: ActionBody });
    // Nothing but the concede went out while the opponent held the move; the setting went once it was the player's.
    expect(actions[0]?.action).toEqual(CONCEDE);
    expect(actions[1]).toMatchObject({ after: 9, action: { type: "setAutoEndTurn", enabled: false } });
  });

  it("#3727 Next game appears once the move has passed on and another game waits, and opens it", async () => {
    const after = dailyGame({ seq: 8, hasMove: false, legal: [CONCEDE], view: baseView({ active: "p2" }), nextGame: "g-2" });
    serve(dailyGame, (call) => (call.method === "POST" ? { status: 200, body: after } : undefined));
    render(<DailyBoard matchId="g-1" />);

    fireEvent.click(await screen.findByTestId("end-turn", undefined, SLOW));
    expect(screen.queryByTestId(dailyTestid.nextGame)).toBeNull();
    const next = await screen.findByTestId(dailyTestid.nextGame, undefined, SLOW);
    fireEvent.click(next);
    expect(window.location.pathname).toBe("/daily/g-2");
  });

  it("#3727 no Next game while the move is the player's, even with another game waiting", async () => {
    serve(() => dailyGame({ nextGame: "g-2" }));
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("end-turn", undefined, SLOW);
    expect(screen.queryByTestId(dailyTestid.nextGame)).toBeNull();
  });

  it("#3727 a stale board reads the game again and says nothing; another refusal says the server's words", async () => {
    let refuse: Reply = refusal(409, "stale", "The game moved on.");
    const server = serve(dailyGame, (call) => (call.method === "POST" ? refuse : undefined));
    render(<DailyBoard matchId="g-1" />);

    fireEvent.click(await screen.findByTestId("end-turn", undefined, SLOW));
    await waitFor(() => {
      expect(server.to("GET", "/api/daily/games/g-1")).toHaveLength(2);
    }, SLOW);
    expect(screen.queryByTestId("action-error")).toBeNull();

    refuse = refusal(409, "time_up", "This move's time ran out.");
    await waitFor(() => {
      expect(screen.getByTestId("end-turn")).toBeEnabled();
    }, SLOW);
    fireEvent.click(screen.getByTestId("end-turn"));
    expect(await screen.findByTestId("action-error", undefined, SLOW)).toHaveTextContent("This move's time ran out.");
    await waitFor(() => {
      expect(server.to("GET", "/api/daily/games/g-1")).toHaveLength(3);
    }, SLOW);
  });

  it("#3727 the clock says Their move while the opponent holds it", async () => {
    const { MS_PER_HOUR } = dailyConstants();
    const now = Date.now();
    serve(() =>
      dailyGame({
        hasMove: false,
        legal: [CONCEDE],
        view: baseView({ active: "p2" }),
        holds: [{ player: "p2", key: "turn:active", since: now, deadline: now + 2 * MS_PER_HOUR + MS_PER_HOUR / 2 }],
      }),
    );
    render(<DailyBoard matchId="g-1" />);
    const clock = await screen.findByTestId(dailyTestid.clock, undefined, SLOW);
    expect(clock).toHaveTextContent("Their move · 2h left");
    expect(clock).toHaveAttribute("data-holder", "opponent");
    expect(clock).toHaveAttribute("data-urgent", "true");
  });

  it.each([
    ["p2", "Lost on time"],
    ["p1", "Won on time"],
  ] as const)("#3727 a game whose move ran out (won by %s) says %s", async (winner, words) => {
    const over: PlayerView = baseView({ result: { winner, reason: "disconnect" } });
    serve(() => dailyGame({ view: over, over: true, lostOnTime: true, hasMove: false, legal: [], holds: [] }));
    render(<DailyBoard matchId="g-1" />);
    const result = await screen.findByTestId("result-overlay", undefined, SLOW);
    expect(result).toHaveAttribute("data-reason", "disconnect");
    expect(result).toHaveTextContent(words);
    expect(result).not.toHaveTextContent("left the match");
    expect(screen.queryByTestId(dailyTestid.clock)).toBeNull();
  });

  it("#3727 a finished game's Rematch sends its opponent a daily challenge at the same mode and time", async () => {
    const over: PlayerView = baseView({ result: { winner: "p1", reason: "concede" } });
    const server = serve(
      () => dailyGame({ view: over, over: true, hasMove: false, legal: [], holds: [], mode: "random", perMoveDays: 7 }),
      (call) => {
        if (call.method === "GET" && call.path === "/api/decks") return { status: 200, body: { decks: [] } };
        if (call.method === "POST" && call.path === "/api/daily/challenges") return { status: 201, body: { id: "ch-re" } };
        return undefined;
      },
    );
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("result-overlay", undefined, SLOW);
    fireEvent.click(screen.getByTestId(dailyTestid.rematch));
    const dialog = await screen.findByTestId(dailyTestid.dialog, undefined, SLOW);
    expect(dialog).toHaveTextContent("A new daily challenge to Opp#7, at the same mode and time per move.");
    // The game's mode and time are the rematch's: neither is picked again, and no friend list is read.
    expect(within(dialog).queryByTestId(dailyTestid.mode)).toBeNull();
    expect(within(dialog).queryByTestId(dailyTestid.perMove)).toBeNull();
    expect(within(dialog).queryByTestId(dailyTestid.friend)).toBeNull();
    await waitFor(() => {
      expect(within(dialog).getByTestId(dailyTestid.send)).toBeEnabled();
    }, SLOW);
    fireEvent.click(within(dialog).getByTestId(dailyTestid.send));
    await waitFor(() => {
      expect(server.to("POST", "/api/daily/challenges")).toHaveLength(1);
    }, SLOW);
    expect(server.to("POST", "/api/daily/challenges")[0]?.body).toEqual({ friendId: "o-1", mode: "random", perMoveDays: 7 });
    expect(server.to("GET", "/api/friends")).toHaveLength(0);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/daily");
    }, SLOW);
  });

  it("#3727 a game on an older card version says so", async () => {
    serve(() => dailyGame({ olderCatalog: true }));
    render(<DailyBoard matchId="g-1" />);
    expect(await screen.findByTestId(dailyTestid.olderCatalog, undefined, SLOW)).toHaveTextContent(
      "This game plays on the card version it started on, an older one than today's.",
    );
  });

  it("#3727 a failed read offers Try again, which reads the game", async () => {
    let fail = true;
    const server = serve(dailyGame, (call) => (call.method === "GET" && fail ? refusal(404, "not_found", "No such game.") : undefined));
    render(<DailyBoard matchId="g-1" />);
    expect(await screen.findByTestId(dailyTestid.boardError, undefined, SLOW)).toHaveTextContent("No such game.");
    fail = false;
    fireEvent.click(screen.getByTestId(dailyTestid.boardRetry));
    await screen.findByTestId("end-turn", undefined, SLOW);
    expect(server.to("GET", "/api/daily/games/g-1")).toHaveLength(2);
  });
});
