// #3727: a daily game's board. Each action is one POST sent after the last action read, under a fresh
// nonce, and the board draws the answer; a stale board reads the game again; Next game appears once the
// move has passed on; the result says Lost on time or Won on time; an older card version says so; a
// finished game's Rematch challenges its opponent again at its mode and time per move. R3855000: it shows what
// every table shows: the opponent's name on the hero, the modifiers in play, an aimed attack's outcomes from
// the read's forecast, and Watch the replay on a finished game's result. R3840000: a game that ended with the
// count of games waiting on the player unchanged shows as ended on the slow read and when the tab shows again.
// R3934000: a finished game whose moves the retention purge took opens as its result, not as a board.

import type { ActionBody, AttackForecast, CombatForecast, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { setAudioEngineForTests } from "../audio/engine.ts";
import { resetAudioSettingsForTests } from "../audio/settings.ts";
import { fakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { AudioEngine } from "../audio/types.ts";
import { modifierListTestid } from "../game/ModifierList.tsx";
import { formatDateTime } from "../i18n/i18n.ts";
import type { DailyExpiredGame, DailyGame } from "../net/daily.ts";
import { paths } from "../net/navigate.ts";
import { clearReadCache } from "../net/readCache.ts";
import { replayTestid } from "../replay/ReplayPlayer.tsx";
import { resetSettings, writeSettings } from "../settings/store.ts";
import { baseView, card, emptySide, opponentModifiers, pendingFor, unit, yourModifiers } from "../test/fixtures.ts";
import { pressBack } from "../test/history.ts";
import { foldedReplaySteps } from "../test/replay.ts";
import { resetVisibility, setVisibility } from "../test/setup.ts";
import { at, refusal, signIn, stubServer, type Call, type Reply } from "../test/stubServer.ts";
import { dailyConstants } from "../wasm/index.ts";
import { DAILY_REREAD_MS } from "./config.ts";
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

/** R3934000: what the server reads of a finished game whose moves are no longer kept. */
function expiredGame(over: Partial<DailyExpiredGame> = {}): DailyExpiredGame {
  return {
    expired: true,
    matchId: "g-1",
    mode: "random",
    perMoveDays: 3,
    seat: "p1",
    opponent: { profileId: "o-1", username: "Opp#7" },
    portraits: { me: "vanilla", them: "vanilla" },
    turn: 12,
    life: { me: 7, them: 0 },
    hasMove: false,
    theirMove: false,
    deadline: null,
    over: true,
    result: { outcome: "win", reason: "concede" },
    endedAt: Date.UTC(2026, 1, 20, 12),
    createdAt: Date.UTC(2026, 1, 1),
    ...over,
  };
}

/** The game's reads and actions, each answered by `answer`, else the read. */
function serve(read: () => DailyGame | DailyExpiredGame, answer?: (call: Call) => Reply | undefined) {
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
  vi.useRealTimers();
  resetVisibility();
  resetSettings();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

async function flush(): Promise<void> {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

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

  it("#3841 End turn automatically waits for the mulligan to be over: none is sent during it, and it is sent once it is", async () => {
    writeSettings({ autoEndTurn: false });
    let over = false;
    const server = serve(
      () => (over ? dailyGame({ seq: 9 }) : dailyGame({ view: baseView({ phase: "mulligan" }) })),
      // The concede below meets a game that moved on: the mulligan is over.
      (call) => {
        if (call.method !== "POST" || (call.body as { action: ActionBody }).action.type !== "concede") return undefined;
        over = true;
        return refusal(409, "stale", "The game moved on.");
      },
    );
    render(<DailyBoard matchId="g-1" />);
    fireEvent.click(await screen.findByTestId("concede", undefined, SLOW));
    // The player holds the move and the setting differs, but both seats hold the move in the mulligan.
    expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(0);
    fireEvent.click(await screen.findByTestId("concede-confirm", undefined, SLOW));

    await waitFor(() => {
      expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(2);
    }, SLOW);
    const actions = server.to("POST", "/api/daily/games/g-1/actions").map((call) => call.body as { after: number; action: ActionBody });
    expect(actions[0]?.action).toEqual(CONCEDE);
    expect(actions[1]).toMatchObject({ after: 9, action: { type: "setAutoEndTurn", enabled: false } });
  });

  it("#3841 End turn automatically goes out just before the player's own mulligan answer, so it is in the game before the first turn", async () => {
    writeSettings({ autoEndTurn: false });
    const hand = [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" })];
    const mulliganView = baseView({
      turn: 0,
      phase: "mulligan",
      you: emptySide("p1", { hand }),
      opponent: emptySide("p2", { hand: { count: 4 } }),
      pending: pendingFor(
        "mulligan",
        hand.map((c) => ({ key: c.instanceId, label: c.defId, instanceId: c.instanceId })),
        { min: 0, max: hand.length, prompt: "Choose the cards to keep; the rest are returned and redrawn" },
      ),
      mulligan: { youReady: false, opponentReady: false },
    });
    let seq = 7;
    const server = serve(
      () => dailyGame({ seq, view: mulliganView }),
      (call) => {
        if (call.method !== "POST") return undefined;
        seq += 1;
        return { status: 200, body: dailyGame({ seq, view: mulliganView }) };
      },
    );
    render(<DailyBoard matchId="g-1" />);
    fireEvent.click(await screen.findByTestId("prompt-submit", undefined, SLOW));

    await waitFor(() => {
      expect(server.to("POST", "/api/daily/games/g-1/actions")).toHaveLength(2);
    }, SLOW);
    const actions = server.to("POST", "/api/daily/games/g-1/actions").map((call) => call.body as { after: number; action: ActionBody });
    // The setting first, the answer after it: each follows the seq the one before it made.
    expect(actions[0]).toMatchObject({ after: 7, action: { type: "setAutoEndTurn", enabled: false } });
    expect(actions[1]).toMatchObject({ after: 8, action: { type: "mulligan", keep: ["h1", "h2"] } });
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

  it("R3727003 a KY's Competition question shows no 60-second countdown, only the move's daily clock", async () => {
    const options = [
      { key: "mode:A", label: "285" },
      { key: "mode:B", label: "295" },
      { key: "none", label: "Stop" },
    ];
    serve(() =>
      dailyGame({ view: baseView({ pending: pendingFor("answer", options, { prompt: "482 − 197 = ?", streak: 0 }) }), legal: [] }),
    );
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("prompt-streak", undefined, SLOW);
    expect(screen.queryByTestId("prompt-sequence-clock")).toBeNull();
    expect(screen.getByTestId(dailyTestid.clock)).toBeInTheDocument();
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

/** The forecast entry of one attack, the numbers named and the rest at rest. */
function forecastEntry(attackerId: string, targetId: string, fields: Partial<AttackForecast>): AttackForecast {
  return {
    attackerId,
    targetId,
    dealt: 0,
    taken: 0,
    kills: false,
    survives: true,
    pops: false,
    attackerSize: 0,
    targetSize: 0,
    ...fields,
  };
}

describe("R3855000 a daily board shows what every table shows", () => {
  const steps = foldedReplaySteps();

  it("R3855000 the opponent's hero reads the opponent's username", async () => {
    serve(dailyGame);
    render(<DailyBoard matchId="g-1" />);
    const opponent = await screen.findByTestId("hero-opponent", undefined, SLOW);
    expect(opponent.querySelector(".hero-seat")).toHaveTextContent("Opp#7");
    expect(screen.getByTestId("hero-you").querySelector(".hero-seat")).toHaveTextContent("You");
  });

  it("R3855000 the board rail lists the modifiers the view holds", async () => {
    const view = baseView();
    serve(() =>
      dailyGame({ view: { ...view, you: { ...view.you, modifiers: yourModifiers }, opponent: { ...view.opponent, modifiers: opponentModifiers } } }),
    );
    render(<DailyBoard matchId="g-1" />);
    const chip = await screen.findByTestId(modifierListTestid.chip, undefined, SLOW);
    expect(chip).toHaveAttribute("data-count", String(yourModifiers.length + opponentModifiers.length));
    fireEvent.click(chip);
    const panel = screen.getByTestId(modifierListTestid.panel);
    for (const modifier of [...yourModifiers, ...opponentModifiers]) expect(panel).toHaveTextContent(modifier.label);
    expect(Array.from(panel.querySelectorAll(".modifier-list__who"), (who) => who.textContent)).toEqual(["You", "Opponent"]);
  });

  describe("an aimed attack", () => {
    const POINTER_SPOT = { clientX: 10, clientY: 10 };
    let under: Element[] = [];

    /** Puts `element` under the pointer (null: bare board); the hovered-target reader runs on the move. */
    function hover(element: Element | null): void {
      under = element === null ? [] : [element];
      fireEvent.pointerMove(window, POINTER_SPOT);
    }

    beforeEach(() => {
      under = [];
      document.elementsFromPoint = (() => under) as Document["elementsFromPoint"];
      resetAudioSettingsForTests();
      setAudioEngineForTests(fakeAudioEngine({ contextsCreated: vi.fn<AudioEngine["contextsCreated"]>(() => 1) }));
    });

    afterEach(() => {
      setAudioEngineForTests(null);
      delete (document as Partial<Document>).elementsFromPoint;
      document.documentElement.removeAttribute("data-dragging");
      resetAudioSettingsForTests();
    });

    it("R3855000 selecting an attacker and hovering its target shows the forecast's damage", async () => {
      const view = baseView({
        you: emptySide("p1", { units: [unit("p1", { instanceId: "u1", defId: "core-066" }), null, null, null, null] }),
        opponent: emptySide("p2", { hand: { count: 3 }, units: [unit("p2", { instanceId: "e1", defId: "core-055" }), null, null, null, null] }),
      });
      const forecast: CombatForecast = {
        attacks: [
          forecastEntry("u1", "e1", { dealt: 2, taken: 3, kills: true, survives: false, pops: true }),
          forecastEntry("u1", "hero-p2", { dealt: 2 }),
        ],
        lethal: false,
      };
      serve(() =>
        dailyGame({
          view,
          legal: [{ type: "attack", attackerId: "u1", targetId: "e1" }, { type: "attack", attackerId: "u1", targetId: "hero-p2" }, END_TURN, CONCEDE],
          forecast,
        }),
      );
      render(<DailyBoard matchId="g-1" />);
      fireEvent.click(await screen.findByTestId("card-u1", undefined, SLOW));
      expect(screen.queryAllByTestId(/^outcome-/)).toEqual([]);

      hover(screen.getByTestId("card-e1"));
      const target = screen.getByTestId("outcome-card-e1");
      expect(target).toHaveAttribute("data-dealt", "2");
      expect(target.querySelector(".fx-splat")).toHaveAttribute("data-amount", "−2");
      expect(screen.getByTestId("outcome-taken-card-u1")).toHaveAttribute("data-taken", "3");

      hover(null);
      expect(screen.queryAllByTestId(/^outcome-/)).toEqual([]);
    });
  });

  describe("a finished game's result", () => {
    const finished = (): DailyGame =>
      dailyGame({ view: baseView({ result: { winner: "p1", reason: "concede" } }), over: true, hasMove: false, legal: [], holds: [] });

    it("R3855000 holds Watch the replay, which opens the game's replay and returns to the board", async () => {
      const server = serve(finished, (call) => {
        if (call.method !== "GET" || call.path !== "/api/replays/g-1") return undefined;
        const from = Number(call.query.get("from") ?? "0");
        const count = Number(call.query.get("count") ?? "1");
        return { status: 200, body: { matchId: "g-1", from, steps: steps.slice(from, from + count), total: steps.length } };
      });
      render(<DailyBoard matchId="g-1" />);
      await screen.findByTestId("result-overlay", undefined, SLOW);

      fireEvent.click(screen.getByTestId(dailyTestid.watchReplay));
      expect(await screen.findByTestId(replayTestid.player, undefined, SLOW)).toHaveAttribute("data-step", "0");
      const first = server.to("GET", "/api/replays/g-1")[0];
      expect(first?.query.get("from")).toBe("0");
      expect(first?.headers.authorization).toBe("Bearer tok-daily-board");
      expect(screen.getByTestId("hero-opponent").querySelector(".hero-seat")).toHaveTextContent("Opp#7");

      fireEvent.click(screen.getByTestId(replayTestid.back));
      expect(await screen.findByTestId(dailyTestid.watchReplay, undefined, SLOW)).toBeInTheDocument();
      expect(screen.queryByTestId(replayTestid.player)).toBeNull();
    });

    it("R4619000 one Back on a finished daily game's replay returns to its result on /daily/:id", async () => {
      at(paths.daily);
      window.history.pushState(null, "", paths.dailyGame("g-1"));
      serve(finished, (call) => {
        if (call.method !== "GET" || call.path !== "/api/replays/g-1") return undefined;
        const from = Number(call.query.get("from") ?? "0");
        const count = Number(call.query.get("count") ?? "1");
        return { status: 200, body: { matchId: "g-1", from, steps: steps.slice(from, from + count), total: steps.length } };
      });
      render(<DailyBoard matchId="g-1" />);
      await screen.findByTestId("result-overlay", undefined, SLOW);
      fireEvent.click(screen.getByTestId(dailyTestid.watchReplay));
      expect(await screen.findByTestId(replayTestid.player, undefined, SLOW)).toHaveAttribute("data-step", "0");

      await pressBack();
      expect(await screen.findByTestId(dailyTestid.watchReplay, undefined, SLOW)).toBeInTheDocument();
      expect(screen.queryByTestId(replayTestid.player)).toBeNull();
      expect(window.location.pathname).toBe(paths.dailyGame("g-1"));
    });

    it("R3855000 a voided game's result offers no replay", async () => {
      serve(() => dailyGame({ ...finished(), view: baseView({ result: { winner: "draw", reason: "voided" } }) }));
      render(<DailyBoard matchId="g-1" />);
      await screen.findByTestId("result-overlay", undefined, SLOW);
      expect(screen.getByTestId(dailyTestid.rematch)).toBeInTheDocument();
      expect(screen.queryByTestId(dailyTestid.watchReplay)).toBeNull();
    });
  });
});

describe("R3840000 the open daily board reads a game that ended", () => {
  const GAME = "/api/daily/games/g-1";

  it("R3840000 a game the opponent ends on their own move shows as ended on the slow read, and is read no more", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    let ended = false;
    const running = dailyGame({ hasMove: false, legal: [CONCEDE], view: baseView({ active: "p2" }), holds: [] });
    const over = dailyGame({
      seq: 8,
      over: true,
      hasMove: false,
      legal: [],
      holds: [],
      view: baseView({ active: "p2", result: { winner: "p1", reason: "concede" } }),
    });
    const server = serve(() => (ended ? over : running));
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("concede", undefined, SLOW);
    expect(screen.queryByTestId("result-overlay")).toBeNull();
    const reads = server.to("GET", GAME).length;

    ended = true;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(DAILY_REREAD_MS);
    });
    await screen.findByTestId("result-overlay", undefined, SLOW);
    expect(server.to("GET", GAME)).toHaveLength(reads + 1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(DAILY_REREAD_MS);
    });
    await flush();
    expect(server.to("GET", GAME)).toHaveLength(reads + 1);
  });

  it("R3840000 a game the opponent concedes while the move is the player's shows as ended when the tab shows again, and a hidden tab reads nothing", async () => {
    let ended = false;
    const over = dailyGame({
      seq: 8,
      over: true,
      hasMove: false,
      legal: [],
      holds: [],
      view: baseView({ result: { winner: "p1", reason: "concede" } }),
    });
    const server = serve(() => (ended ? over : dailyGame()));
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("end-turn", undefined, SLOW);
    const reads = server.to("GET", GAME).length;

    ended = true;
    act(() => {
      setVisibility("hidden");
    });
    await flush();
    expect(server.to("GET", GAME)).toHaveLength(reads);

    act(() => {
      setVisibility("visible");
    });
    await screen.findByTestId("result-overlay", undefined, SLOW);
    expect(server.to("GET", GAME)).toHaveLength(reads + 1);
  });
});

describe("R3934000 a finished game whose moves are no longer kept", () => {
  const GAME = "/api/daily/games/g-1";

  it("R3934000 opens as its result with both heroes' life, when it ended and the moves-gone line, and no board", async () => {
    const game = expiredGame();
    const server = serve(() => game);
    render(<DailyBoard matchId="g-1" />);

    const panel = await screen.findByTestId(dailyTestid.expired, undefined, SLOW);
    const result = within(panel).getByTestId(dailyTestid.expiredResult);
    expect(result).toHaveTextContent("Win");
    expect(result).toHaveAttribute("data-outcome", "win");
    expect(panel).toHaveTextContent("You 7 – 0 Opponent");
    expect(panel).toHaveTextContent("vs Opp#7");
    expect(panel).toHaveTextContent("Turn 12");
    expect(panel).toHaveTextContent(`Ended ${formatDateTime(game.endedAt ?? 0)}`);
    expect(panel).toHaveTextContent("This game's moves are no longer kept, so it can't be shown or replayed.");
    for (const absent of ["result-overlay", "end-turn", dailyTestid.board, dailyTestid.clock, dailyTestid.watchReplay, dailyTestid.rematch]) {
      expect(screen.queryByTestId(absent)).toBeNull();
    }
    expect(server.to("GET", GAME)).toHaveLength(1);
    expect(server.to("POST", `${GAME}/actions`)).toHaveLength(0);
  });

  it.each([
    [{ outcome: "loss", reason: "disconnect" }, "Lost on time"],
    [{ outcome: "win", reason: "disconnect" }, "Won on time"],
  ] as const)("R3934000 a game %j says %s", async (result, words) => {
    serve(() => expiredGame({ result }));
    render(<DailyBoard matchId="g-1" />);

    const shown = await screen.findByTestId(dailyTestid.expiredResult, undefined, SLOW);
    expect(shown).toHaveTextContent(words);
    expect(shown).toHaveAttribute("data-outcome", result.outcome);
  });

  it("R3934000 Back to daily games returns to the list", async () => {
    serve(() => expiredGame());
    render(<DailyBoard matchId="g-1" />);

    fireEvent.click(await screen.findByTestId(dailyTestid.backToList, undefined, SLOW));

    await waitFor(() => {
      expect(window.location.pathname).toBe("/daily");
    });
  });

  it("R3934000 a game that reads as expired after it showed a board is read no more", async () => {
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
    let purged = false;
    const running = dailyGame({ hasMove: false, legal: [CONCEDE], view: baseView({ active: "p2" }), holds: [] });
    const server = serve(() => (purged ? expiredGame() : running));
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("concede", undefined, SLOW);
    const reads = server.to("GET", GAME).length;

    purged = true;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(DAILY_REREAD_MS);
    });
    await screen.findByTestId(dailyTestid.expired, undefined, SLOW);
    expect(screen.queryByTestId("concede")).toBeNull();
    expect(server.to("GET", GAME)).toHaveLength(reads + 1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2 * DAILY_REREAD_MS);
    });
    await flush();
    expect(server.to("GET", GAME)).toHaveLength(reads + 1);
  });
});

describe("R4621000 a finished game whose opponent deleted their account", () => {
  const steps = foldedReplaySteps();
  const GONE = { profileId: "", username: "" };
  const finished = (): DailyGame =>
    dailyGame({ view: baseView({ result: { winner: "p1", reason: "concede" } }), over: true, hasMove: false, legal: [], holds: [], opponent: GONE });

  it("R4621000 names them a deleted account on the board and offers no Rematch", async () => {
    serve(finished);
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("result-overlay", undefined, SLOW);

    expect(screen.getByTestId("hero-opponent").querySelector(".hero-seat")).toHaveTextContent("a deleted account");
    expect(screen.queryByTestId(dailyTestid.rematch)).toBeNull();
    expect(screen.getByTestId(dailyTestid.watchReplay)).toBeInTheDocument();
    expect(screen.getByTestId(dailyTestid.backToList)).toBeInTheDocument();
  });

  it("R4621000 its replay names them a deleted account", async () => {
    serve(finished, (call) => {
      if (call.method !== "GET" || call.path !== "/api/replays/g-1") return undefined;
      const from = Number(call.query.get("from") ?? "0");
      const count = Number(call.query.get("count") ?? "1");
      return { status: 200, body: { matchId: "g-1", from, steps: steps.slice(from, from + count), total: steps.length } };
    });
    render(<DailyBoard matchId="g-1" />);
    await screen.findByTestId("result-overlay", undefined, SLOW);

    fireEvent.click(screen.getByTestId(dailyTestid.watchReplay));
    expect(await screen.findByTestId(replayTestid.player, undefined, SLOW)).toHaveAttribute("data-step", "0");
    expect(screen.getByTestId("hero-opponent").querySelector(".hero-seat")).toHaveTextContent("a deleted account");
  });

  it("R4621000 a game whose moves are no longer kept reads vs a deleted account", async () => {
    serve(() => expiredGame({ opponent: GONE }));
    render(<DailyBoard matchId="g-1" />);

    const panel = await screen.findByTestId(dailyTestid.expired, undefined, SLOW);
    expect(panel).toHaveTextContent("vs a deleted account");
  });
});
