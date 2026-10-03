// The turn stages patch v0.2.0 adds to R62's order (docs/classic-sets.md B3.1, B3.3; SPEC §2.2): the
// Brittle tick right after the mana refresh, the "Animated on your turn" cards stepping into their
// unit zones after it and before the delayed effects, and those cards going home as cleanup's last
// step. The bodies are `brittle.ts`'s and `animated.ts`'s; what is proved here is the wiring — where
// each stage runs, that each settles before the next, and that a stage which asks something parks the
// rest of the turn on `state.work` (R113, R117) so each later stage runs exactly once, through a JSON
// round trip and a replay.
//
// So the two modules are replaced by test doubles for this file only (`vi.mock`): a double that
// records when it ran, and in some tests asks its player something or emits an event, exactly as a
// Brittle crumble's Death hook or an arrival's trap would. Their real behaviour is their own tests'.

import type { Action, ActionInput, GameEvent, PlayerId } from "@jackioh/shared";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { animateAtTurnStart, returnAtCleanup } from "../src/animated";
import { brittleTick } from "../src/brittle";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { DECK_SIZE } from "../src/config";
import { scheduleDelayed } from "../src/modifiers";
import { openPrompt } from "../src/prompts";
import { beginGame, reduce } from "../src/reduce";
import { fold, hashState } from "../src/replay";
import type { EngineSink } from "../src/resolve";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { createGame, type CardInstance, type GameState } from "../src/state";
import { END_OF_TURN_WORK, START_OF_TURN_WORK } from "../src/turn";
import { vanillaDeck } from "./fixtures/catalog";
import { inHand, newGame, put, setLibrary, setupCatalog, slot } from "./fixtures/harness";
import {
  LOG_LANE,
  TURN_SCRIPTS,
  castSpell,
  clock,
  crumbleWatcher,
  logCard,
  note,
  notes,
  reminder,
  turnCatalog,
  write,
} from "./fixtures/turn";

// Only the two stage bodies are doubled: every other export of the modules (the readers other
// modules import, such as `animated.faceTypeOf` and the Brittle count helpers) stays the real one.
vi.mock("../src/brittle", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../src/brittle")>()),
  brittleTick: vi.fn(),
}));
vi.mock("../src/animated", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../src/animated")>()),
  animateAtTurnStart: vi.fn(),
  returnAtCleanup: vi.fn(),
}));

/** The log card also answers the doubles' prompts, and holds a delayed step for them. */
const LOG_SCRIPTS: CardScripts = {
  base: {
    resume: {
      brittled: () => [note("brittle:answered")],
      animated: () => [note("animate:answered")],
      returned: () => [note("return:answered")],
    },
    delayed: () => [note("late-delayed")],
  },
  radiant: {},
};

function register(): void {
  registerCatalog(turnCatalog(registeredCatalog()));
  registerScripts({ ...registeredScripts(), ...TURN_SCRIPTS, [logCard.id]: LOG_SCRIPTS });
}

/**
 * A test double's prompt for `player`, whose answer re-enters the log card's `step`. The turn hands
 * every stage its whole sink; the Animated stages declare only the narrower `FieldSink` they use.
 */
function ask(sink: Pick<EngineSink, "state" | "events">, player: PlayerId, step: string): void {
  openPrompt(sink as EngineSink, {
    player,
    kind: "target",
    prompt: "a stage asks",
    options: [{ key: "none", label: "nothing", selection: { pick: "none" } }],
    resume: { defId: logCard.id, hook: "resume", step, radiant: false, data: {} },
  });
}

/** What each stage does unless a test says otherwise: note that it ran, with what it can see. */
function recordingDoubles(): void {
  vi.mocked(brittleTick).mockImplementation((sink, player) => {
    write(sink.state, `brittle:${player}:${sink.state.players[player].mana.current}`);
  });
  vi.mocked(animateAtTurnStart).mockImplementation((sink, player) => {
    write(sink.state, `animate:${player}`);
  });
  vi.mocked(returnAtCleanup).mockImplementation((sink, player) => {
    const side = sink.state.players[player];
    write(sink.state, `return:${player}:${side.turnLog.unspentAtEnd === undefined ? "open" : "closed"}`);
  });
}

beforeEach(() => {
  recordingDoubles();
});

let nonce = 0;

function act(state: GameState, body: ActionInput): { state: GameState; events: GameEvent[] } {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `tw${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return { state: result.state, events: result.events };
}

/** Past both mulligans, in p1's main phase on turn 1, with the note log in p2's backrow. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(`turn-wiring-${seed}`)).state;
  register();
  for (const player of ["p1", "p2"] as const) {
    state = act(state, { type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player }).state;
  }
  put(state, logCard.id, slot("p2", "backrow", LOG_LANE));
  state.players.p1.autoEndTurn = false;
  state.players.p2.autoEndTurn = false;
  return state;
}

/**
 * p1 has a clock (start- and end-of-turn hooks), a reminder scheduled for the start of their next
 * turn, and a cast-on-draw card on top of their library: every stage of p1's next start leaves a note.
 */
function staged(seed: string): GameState {
  let state = playing(seed);
  put(state, clock.id, slot("p1", "backrow", 1));
  const card = inHand(state, reminder.id, "p1")[0] as CardInstance;
  state = act(state, { type: "play", instanceId: card.id, playerId: "p1" }).state;
  setLibrary(state, "p1", [castSpell.id, "fx-9"]);
  return state;
}

/** p1 ends turn 1 and p2 turn 2: p1's turn 3 is starting. */
function toTurnThree(state: GameState): { state: GameState; events: GameEvent[] } {
  const theirs = act(state, { type: "endTurn", playerId: "p1" }).state;
  return act(theirs, { type: "endTurn", playerId: "p2" });
}

function answer(state: GameState): { state: GameState; events: GameEvent[] } {
  const pending = state.pending;
  if (pending === null) throw new Error("expected a prompt");
  return act(state, { type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: pending.playerId });
}

const roundTrip = (state: GameState): GameState => JSON.parse(JSON.stringify(state)) as GameState;
/** The notes from `from` on, so a test reads one turn's stages. */
const since = (state: GameState, from: number): string[] => notes(state).slice(from);

describe("R62's start of a turn with the Brittle and Animated stages (B3.1, B3.3)", () => {
  it("R62 refresh, then the Brittle tick, then the Animated stage, then delayed effects, triggers and the draw", () => {
    const state = staged("order");
    const before = notes(state).length;
    const { state: after } = toTurnThree(state);
    const p1 = since(after, before).filter((entry) => !entry.endsWith(":p2") && !entry.includes(":p2:"));
    // The tick sees the refreshed mana (turn 3: two crystals).
    expect(p1).toEqual(["end-of-turn:p1", "return:p1:closed", "brittle:p1:2", "animate:p1", "delayed", "start-of-turn:p1", "cast-spell"]);
    // And the other player's turn ran the same stages for p2, on p2's turn only.
    expect(since(after, before).filter((entry) => entry.includes("p2"))).toEqual(["brittle:p2:1", "animate:p2", "return:p2:closed"]);
    expect(vi.mocked(brittleTick).mock.calls.map(([, player]) => player)).toContain("p1");
  });

  it("R62 a prompt inside the Brittle stage parks the rest of the start; the answer runs each later stage once", () => {
    const state = staged("brittle-asks");
    vi.mocked(brittleTick).mockImplementation((sink, player) => {
      write(sink.state, `brittle:${player}`);
      if (player === "p1" && sink.state.turn === 3) ask(sink, player, "brittled");
    });
    const before = notes(state).length;
    const paused = toTurnThree(state).state;

    expect(paused.pending?.playerId).toBe("p1");
    expect(paused.phase).toBe("start");
    expect(paused.work.map((item) => [item.resume.hook, item.resume.step])).toEqual([[START_OF_TURN_WORK, "brittle"]]);
    expect(since(paused, before).filter((entry) => !entry.includes("p2"))).toEqual([
      "end-of-turn:p1",
      "return:p1:closed",
      "brittle:p1",
    ]);

    const copy = roundTrip(paused);
    expect(copy).toEqual(paused);
    const live = answer(paused).state;
    expect(since(live, before).filter((entry) => !entry.includes("p2"))).toEqual([
      "end-of-turn:p1",
      "return:p1:closed",
      "brittle:p1",
      "brittle:answered",
      "animate:p1",
      "delayed",
      "start-of-turn:p1",
      "cast-spell",
    ]);
    expect(live.phase).toBe("main");
    expect(live.work).toEqual([]);
    expect(hashState(answer(copy).state)).toBe(hashState(live));
  });

  it("R62 a trigger the Brittle stage's own events wake may ask too: the stage's settle pauses and the rest waits", () => {
    const state = staged("brittle-event");
    put(state, crumbleWatcher.id, slot("p1", "backrow", 2));
    vi.mocked(brittleTick).mockImplementation((sink, player) => {
      write(sink.state, `brittle:${player}`);
      const card = sink.state.players[player].hand[0];
      if (player !== "p1" || card === undefined) return;
      sink.events.push({ type: "crumbled", instanceId: card.id, defId: card.defId, owner: player, zone: "field" });
    });
    const before = notes(state).length;
    const paused = toTurnThree(state).state;
    expect(paused.pending?.playerId).toBe("p1");
    expect(since(paused, before).filter((entry) => !entry.includes("p2"))).toEqual([
      "end-of-turn:p1",
      "return:p1:closed",
      "brittle:p1",
      "crumble-seen",
    ]);
    expect(paused.work.at(-1)?.resume).toMatchObject({ hook: START_OF_TURN_WORK, step: "brittle" });

    const live = answer(paused).state;
    expect(since(live, before).filter((entry) => !entry.includes("p2"))).toEqual([
      "end-of-turn:p1",
      "return:p1:closed",
      "brittle:p1",
      "crumble-seen",
      "crumble-answered",
      "animate:p1",
      "delayed",
      "start-of-turn:p1",
      "cast-spell",
    ]);
  });

  it("R62 a prompt inside the Animated stage parks the delayed effects, the triggers and the draw", () => {
    const state = staged("animate-asks");
    vi.mocked(animateAtTurnStart).mockImplementation((sink, player) => {
      write(sink.state, `animate:${player}`);
      if (player === "p1" && sink.state.turn === 3) ask(sink, player, "animated");
    });
    const before = notes(state).length;
    const paused = toTurnThree(state).state;
    expect(paused.work.map((item) => [item.resume.hook, item.resume.step])).toEqual([[START_OF_TURN_WORK, "animate"]]);
    const copy = roundTrip(paused);
    const live = answer(paused).state;
    expect(since(live, before).filter((entry) => !entry.includes("p2"))).toEqual([
      "end-of-turn:p1",
      "return:p1:closed",
      "brittle:p1:2",
      "animate:p1",
      "animate:answered",
      "delayed",
      "start-of-turn:p1",
      "cast-spell",
    ]);
    expect(hashState(answer(copy).state)).toBe(hashState(live));
  });

  it("R62 the delayed effects due at a start are the ones that existed as the turn began, not one a stage made", () => {
    const state = playing("late-delayed");
    vi.mocked(brittleTick).mockImplementation((sink, player) => {
      write(sink.state, `brittle:${player}`);
      if (player !== "p1" || sink.state.turn !== 3) return;
      scheduleDelayed(sink, "p1", { phase: "start", player: "p1" }, {
        defId: logCard.id,
        hook: "delayed",
        step: "late",
        radiant: false,
        data: {},
      });
    });
    const three = toTurnThree(state).state;
    expect(notes(three)).not.toContain("late-delayed");
    expect(three.delayed).toHaveLength(1);
    const five = toTurnThree(three).state;
    expect(notes(five).filter((entry) => entry === "late-delayed")).toHaveLength(1);
    expect(five.delayed).toEqual([]);
  });
});

describe("R62's cleanup with the Animated return as its last step (B3.1)", () => {
  it("R62 the return runs after every end-of-turn step and cleanup's own steps, before the turn cap and the next turn", () => {
    const state = playing("return-order");
    put(state, clock.id, slot("p1", "backrow", 1));
    const { state: after, events } = act(state, { type: "endTurn", playerId: "p1" });
    expect(notes(after).filter((entry) => entry.includes("p1"))).toEqual(["end-of-turn:p1", "return:p1:closed"]);
    const types = events.map((event) => event.type);
    expect(types.indexOf("turnEnded")).toBeLessThan(types.lastIndexOf("turnStarted"));
    expect(vi.mocked(returnAtCleanup).mock.calls.at(-1)?.[1]).toBe("p1");
  });

  it("R62 a prompt inside the return parks the turn cap and the next turn; the answer starts the next turn once", () => {
    const state = playing("return-asks");
    vi.mocked(returnAtCleanup).mockImplementation((sink, player) => {
      write(sink.state, `return:${player}`);
      if (player === "p1") ask(sink, player, "returned");
    });
    const paused = act(state, { type: "endTurn", playerId: "p1" }).state;
    expect(paused.active).toBe("p1");
    expect(paused.phase).toBe("end");
    expect(paused.work.map((item) => [item.resume.hook, item.resume.step])).toEqual([[END_OF_TURN_WORK, "next"]]);
    const copy = roundTrip(paused);
    const { state: live, events } = answer(paused);
    expect(live.active).toBe("p2");
    expect(live.phase).toBe("main");
    expect(events.filter((event) => event.type === "turnStarted")).toHaveLength(1);
    expect(notes(live)).toEqual(["return:p1", "return:answered", "brittle:p2:1", "animate:p2"]);
    expect(hashState(answer(copy).state)).toBe(hashState(live));
  });

  it("R62 a game paused in these stages replays from its log to the same state (§9.3)", () => {
    vi.mocked(brittleTick).mockImplementation((sink, player) => {
      if (sink.state.turn === 3) ask(sink, player, "brittled");
    });
    vi.mocked(returnAtCleanup).mockImplementation((sink, player) => {
      if (sink.state.turn === 2) ask(sink, player, "returned");
    });
    const seed = "turn-wiring-replay";
    const decks: [string[], string[]] = [vanillaDeck(DECK_SIZE, 1), vanillaDeck(DECK_SIZE, 21)];
    setupCatalog();
    register();
    let state = beginGame(createGame({ seed, decks })).state;
    const log: Action[] = [];
    const step = (body: ActionInput): void => {
      const action = { ...body, nonce: `rp${log.length}` } as Action;
      const result = reduce(state, action);
      if (result.error !== undefined) throw new Error(result.error);
      log.push(action);
      state = result.state;
    };
    const answerOpen = (): void => {
      const pending = state.pending;
      if (pending === null) throw new Error("expected a prompt");
      step({ type: "answer", choiceId: pending.id, selection: [{ pick: "none" }], playerId: pending.playerId });
    };
    for (const player of ["p1", "p2"] as const) {
      step({ type: "mulligan", keep: state.players[player].hand.map((card) => card.id), playerId: player });
    }
    step({ type: "setAutoEndTurn", enabled: false, playerId: "p1" });
    step({ type: "setAutoEndTurn", enabled: false, playerId: "p2" });
    step({ type: "endTurn", playerId: "p1" });
    step({ type: "endTurn", playerId: "p2" });
    expect(state.work.map((item) => item.resume.hook)).toEqual([END_OF_TURN_WORK]);
    answerOpen();
    expect(state.work.map((item) => item.resume.hook)).toEqual([START_OF_TURN_WORK]);
    answerOpen();
    expect(state.phase).toBe("main");
    expect(state.turn).toBe(3);

    const replayed = fold({ seed, decks, log });
    expect(replayed.errors).toEqual([]);
    expect(hashState(replayed.state)).toBe(hashState(state));
  });
});
