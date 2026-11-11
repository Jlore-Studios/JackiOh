// The practice worker's driver (SPEC §9.9, R187): it feeds the practice session's events to its machine
// and carries out the machine's outputs. The machine is Rust, `crates/shared`'s `host/practice` (#3099)
// behind `practiceCall`, and decides everything: how a game is dealt, the nonces and seats, what the AI
// may take, the snapshot, the save and the kept games, the claim. What is left here is I/O: the save
// store (`saveStore.ts`), the AI's search and the lethal check under their wall-clock caps (the pure crates
// read no clock, CLAUDE.md rule 4), the public card data, and the web's own content a config names (a
// preset's cards, a lesson's decks), which it puts in the config before the machine reads it.
//
// This is the ONLY practice file that holds the game (`PracticeHost`, which has the `GameState` inside it)
// and the only one that imports `@jackioh/ai`'s `decide` and `lethalHint`; only the worker entry
// (`practice.worker.ts`) and jsdom's in-thread host (`host.ts`) load it; the page never does. That is
// rule 7 here: the state and both hands stay on this side of `handle`, and what crosses it is a
// `PracticeSnapshot`, the worker standing where §9.1 puts the server. The sure-win check (R2102000,
// R2291000) is asked of the human's own redaction, so the AI's hidden cards never steer it.
//
// Resume (R668): the worker dies with the page, so a free game in progress is saved after every answer
// (`saveStore.ts`) and, after a reload, its log is folded again by the machine under the same nonces to the
// same hash. The save never crosses to the page: its log names the AI's hidden cards.
//
// Replays (R768): the newest PRACTICE_REPLAYS_KEPT free games that ended (not a lesson, nor one Glitch
// voided, R679) are kept in the same store. `replays` folds none of them, which would stall the worker;
// `replay` answers `viewFor` of the seat the human played at each step (R677). Like the save, a kept
// game never crosses to the page. The machine opens and pages them; the folded game of the last few opened
// is kept here so that going back to one folds it no second time.
//
// Claims (R1303000) and run fights (DR14, R2706000) are the machine's outputs, relayed as they are.

import { CATALOG_VERSION } from "@jackioh/cards";
import { createRng, registeredCatalog, type GameState, type ReplayOpen, type ReplayRefusal } from "@jackioh/engine";
import { AI_BUDGET, decide, lethalHint, type SearchBudget } from "@jackioh/ai";
import type { ActionBody, PlayerId } from "@jackioh/shared";

import { PRACTICE_REPLAYS_CACHED } from "./config.ts";
import { lessonById } from "../tutorial/lessons.ts";
import { practiceCall } from "../wasm/index.ts";
import { presetById } from "./decks.ts";
import type { PracticeSaveStore } from "./saveStore.ts";
import type { AiAsk } from "../wire/generated/AiAsk.ts";
import type { PracticeConfig } from "../wire/generated/PracticeConfig.ts";
import type { PracticeEvent } from "../wire/generated/PracticeEvent.ts";
import type { PracticeHost as PracticeMachine } from "../wire/generated/PracticeHost.ts";
import type { PracticeOutput } from "../wire/generated/PracticeOutput.ts";
import type { PracticeReplay } from "../wire/generated/PracticeReplay.ts";
import type { ReplayVerdict } from "../wire/generated/ReplayVerdict.ts";
import type { RunReport } from "../wire/generated/RunReport.ts";
import type {
  PracticeClaim,
  PracticeDebug,
  PracticeRequest,
  PracticeResponse,
  PracticeSnapshot,
  PracticeStartConfig,
  PracticeTimer,
} from "./protocol.ts";
import { messageOf } from "../net/errors.ts";

export type PracticeDriverEnv = {
  /**
   * The AI's and the lethal hint's (R2102000) wall clock, in `Date.now()` terms: the clock the WebAssembly
   * side reads against the deadline; 0 is no clock. Also the time a finished game is kept at (R768).
   */
  now: () => number;
  /** answer `debug` only when true */
  dev: boolean;
  /** default AI_BUDGET */
  budget?: SearchBudget;
  /** R668, R768: where the game in progress and the finished ones kept for replay live; absent, nothing is kept. */
  saves?: PracticeSaveStore;
};

/** `handle` never throws: anything that goes wrong comes back as `"failed"`. */
export type PracticeDriver = {
  handle(request: PracticeRequest): PracticeResponse;
};

/** What a step's outputs hold for the answer. */
type Carried = {
  aiSeat?: PlayerId;
  snapshot?: PracticeSnapshot;
  decide?: AiAsk;
  claim?: PracticeClaim;
  debug?: PracticeDebug;
  report?: RunReport;
  timers: PracticeTimer[];
};

/**
 * The config a request names, with the web's content it asks for put in: a preset's cards (`decks.ts`) and
 * a lesson's decks (`tutorial/lessons.ts`). A preset or a lesson this build does not know is left without
 * them, and the machine refuses it by name.
 */
function machineConfig(config: PracticeStartConfig): PracticeConfig {
  const { aiBanned, deck, ...rest } = config;
  const lesson = config.lesson === undefined ? undefined : lessonById(config.lesson);
  return {
    ...rest,
    deck: withPresetCards(deck),
    ...(aiBanned === undefined ? {} : { aiBanned: [...aiBanned] }),
    ...(lesson === undefined
      ? {}
      : { lessonDeal: { humanDeck: [...lesson.humanDeck], aiDeck: [...lesson.aiDeck], poolHold: lesson.poolHold } }),
  };
}

/** A preset deck choice with its list, which a save or a kept game written before this build has not. */
function withPresetCards(deck: PracticeConfig["deck"]): PracticeConfig["deck"] {
  if (deck.kind !== "preset" || deck.cards !== undefined) return deck;
  const preset = presetById(deck.id);
  return preset === undefined ? deck : { ...deck, cards: [...preset.cards] };
}

/** A kept game or a save as the machine reads it: its config's preset deck with the preset's list. */
function withCards<T extends { config: PracticeConfig }>(kept: T): T {
  return { ...kept, config: { ...kept.config, deck: withPresetCards(kept.config.deck) } };
}

export function createPracticeDriver(env: PracticeDriverEnv): PracticeDriver {
  const budget = env.budget ?? AI_BUDGET;
  /** The one game, as the machine hands it back (JSON), or null before a start. */
  let machine: PracticeMachine | null = null;
  /** R768: each kept game's verdict by number: null when it folds, else part 1's refusal. */
  const verdicts = new Map<number, ReplayRefusal | null>();
  /** The last PRACTICE_REPLAYS_CACHED kept games opened, by number, the one opened longest ago first. */
  const opened = new Map<number, ReplayOpen>();

  function current(): PracticeMachine {
    if (machine === null) throw new Error("no practice game is running: send start first");
    return machine;
  }

  /** The numbers of the games kept on this device, or none when nothing is kept (R768). */
  function keptNumbers(): number[] | undefined {
    return env.saves?.readReplays().map((replay) => replay.summary.game);
  }

  /** R2102000: the lethal check, under its own wall-clock cap; one that runs past it, or throws, gives no hint. */
  function lethalFor(game: GameState, seat: PlayerId, clockMs: number): boolean {
    const now = env.now();
    try {
      return lethalHint(game, seat, now > 0 ? now + clockMs : undefined);
    } catch {
      return false;
    }
  }

  /** Carries out what the machine asked, in order, and gathers what the answer to the page holds. */
  function carryOut(outputs: readonly PracticeOutput[], game: GameState, now: number, isRun: boolean): Carried {
    const carried: Carried = { timers: [] };
    for (const output of outputs) {
      switch (output.output) {
        case "began":
          carried.aiSeat = output.aiSeat;
          break;
        case "save":
          // R668: the free game in its slot; R3476000: a run fight in its own, never the free game's.
          if (isRun) env.saves?.writeRunFight(output.save);
          else env.saves?.write(output.save);
          break;
        case "keep": {
          const saves = env.saves;
          if (saves !== undefined) saves.writeReplays([...saves.readReplays(), output.replay].slice(-output.keep));
          break;
        }
        case "timer":
          // R1563006: relayed to the page, which keeps it as a timeout; the delay is from this answer's clock.
          carried.timers.push(
            output.command.command === "set"
              ? { command: "set", key: output.command.key, timer: output.command.id, inMs: Math.max(0, output.command.at - now) }
              : { command: "cancel", key: output.command.key, timer: output.command.id },
          );
          break;
        case "snapshot":
          carried.snapshot = output.snapshot;
          break;
        case "lethal":
          if (carried.snapshot !== undefined && lethalFor(game, output.ask.seat, output.ask.clockMs)) {
            carried.snapshot = { ...carried.snapshot, lethal: true };
          }
          break;
        case "decide":
          carried.decide = output.ask;
          break;
        case "claim":
          carried.claim = output.claim;
          break;
        case "debug":
          carried.debug = output.debug;
          break;
        case "runReport":
          carried.report = output.report;
          break;
        case "failed":
          throw new Error(output.message);
        case "finished":
        case "voided":
          // The end of the game is recorded by a host that has a store of results; a practice game
          // on a device records none (SPEC §9.9).
          break;
      }
    }
    return carried;
  }

  /** One event to the machine, and what its outputs hold; the game it holds from then on. */
  function step(event: PracticeEvent): Carried {
    const now = env.now();
    const answer = practiceCall({ op: "step", host: current(), event, now });
    if (answer.reply !== "stepped") throw new Error(`the practice engine answered "${answer.reply}" unexpectedly`);
    machine = answer.host;
    return carryOut(answer.outputs, answer.host.game, now, machine.config.run !== undefined);
  }

  /** A start or a resume: the machine's refusal is the failure, and no game exists. */
  function begin(answer: ReturnType<typeof practiceCall>, now: number): Carried {
    if (answer.reply === "refused") throw new Error(answer.message);
    if (answer.reply !== "stepped") throw new Error(`the practice engine answered "${answer.reply}" unexpectedly`);
    machine = answer.host;
    return carryOut(answer.outputs, answer.host.game, now, machine.config.run !== undefined);
  }

  /** The timers an answer carries to the page, when it carries any. */
  function timersOf(carried: Carried): { timers?: PracticeTimer[] } {
    return carried.timers.length === 0 ? {} : { timers: carried.timers };
  }

  function snapshotOf(carried: Carried): PracticeSnapshot {
    if (carried.snapshot === undefined) throw new Error("the practice engine answered with no snapshot");
    return carried.snapshot;
  }

  function started(id: number, carried: Carried): PracticeResponse {
    if (carried.aiSeat === undefined) throw new Error("the practice engine began no game");
    return {
      id,
      type: "started",
      snapshot: snapshotOf(carried),
      defs: registeredCatalog(),
      aiSeat: carried.aiSeat,
      ...timersOf(carried),
    };
  }

  function actAnswer(id: number, carried: Carried): PracticeResponse {
    return { id, type: "snapshot", snapshot: snapshotOf(carried), ...timersOf(carried) };
  }

  /** One AI action, or nothing when the AI owes none. R188: a decision that gives nothing usable falls back, in the machine. */
  function aiStep(id: number): PracticeResponse {
    let carried = step({ event: "aiTurn" });
    const ask = carried.decide;
    if (ask !== undefined) {
      const rng = createRng(ask.rngSeed, ask.rngCursor);
      let body: ActionBody | null = null;
      try {
        const t0 = env.now();
        const decision = decide(current().game, ask.seat, {
          rng,
          budget,
          ...(t0 > 0 ? { deadlineMs: t0 + ask.clockMs } : {}),
        });
        body = decision?.action ?? null;
      } catch {
        // `decide` promises never to throw; if it does anyway, the machine's fallback still moves the game.
      }
      carried = step({ event: "decided", moved: { body, rngCursor: rng.cursor } });
    }
    return { id, type: "snapshot", snapshot: snapshotOf(carried), ...timersOf(carried) };
  }

  /** R768: a kept game opened once and held by its number; going back to it folds it no second time. */
  function replayAnswer(id: number, game: number, from: number, count: number): PracticeResponse {
    const replay = env.saves?.readReplays().find((kept) => kept.summary.game === game);
    if (replay === undefined) throw new Error(`no finished practice game ${String(game)} is kept on this device`);
    const held = opened.get(game);
    const answer = practiceCall({
      op: "replay",
      replay: withCards(replay),
      ...(held === undefined ? {} : { open: held }),
      from,
      count,
      build: CATALOG_VERSION,
    });
    if (answer.reply !== "replayed") throw new Error(`the practice engine answered "${answer.reply}" unexpectedly`);
    const { open, page } = answer.answer;
    // Opened again, it moves to the newest end, so it is the last to be dropped.
    opened.delete(game);
    opened.set(game, open);
    verdicts.set(game, open.kind === "ready" ? null : open.reason);
    if (opened.size > PRACTICE_REPLAYS_CACHED) {
      // A Map keeps insertion order, so its first key is the game opened longest ago.
      const oldest = opened.keys().next().value;
      if (oldest !== undefined) opened.delete(oldest);
    }
    if (page.page === "refused") throw new Error(page.message);
    return { id, type: "replay", game, from: page.from, steps: page.steps, total: page.total };
  }

  /** R768: replays wait for the game to be over: the machine says whether it is. */
  function refuseWhilePlaying(): void {
    if (machine !== null) step({ event: "replays" });
  }

  function listReplays(id: number): PracticeResponse {
    const kept: readonly PracticeReplay[] = env.saves?.readReplays() ?? [];
    const known: ReplayVerdict[] = [...verdicts].map(([game, refusal]) => ({ game, refusal }));
    const answer = practiceCall({ op: "replays", replays: kept.map(withCards), verdicts: known, build: CATALOG_VERSION });
    if (answer.reply !== "listed") throw new Error(`the practice engine answered "${answer.reply}" unexpectedly`);
    return { id, type: "replays", replays: answer.replays };
  }

  function handleUnsafe(request: PracticeRequest): PracticeResponse {
    switch (request.type) {
      case "start": {
        // A start replaces whatever was running, even when it fails: no request may land on the
        // previous game after the page asked for a new one.
        machine = null;
        const now = env.now();
        const carried = begin(
          practiceCall({
            op: "start",
            config: machineConfig(request.config),
            catalog: CATALOG_VERSION,
            ...(keptNumbers() === undefined ? {} : { kept: keptNumbers() }),
            now,
          }),
          now,
        );
        return started(request.id, carried);
      }
      case "resume": {
        // Like a start, a resume replaces whatever was running, even when it fails.
        machine = null;
        // R668: the free game from its slot; R3476000: a run fight from its own, never the free game's.
        const saved = request.config.run !== undefined ? (env.saves?.readRunFight() ?? null) : (env.saves?.read() ?? null);
        const now = env.now();
        const carried = begin(
          practiceCall({
            op: "resume",
            config: machineConfig(request.config),
            save: saved === null ? null : withCards(saved),
            catalog: CATALOG_VERSION,
            ...(keptNumbers() === undefined ? {} : { kept: keptNumbers() }),
            now,
          }),
          now,
        );
        return started(request.id, carried);
      }
      case "act":
        current();
        return actAnswer(request.id, step({ event: "act", body: request.action }));
      case "timerFired":
        // R1563006: a timer the machine set came due; the machine decides what it means.
        current();
        return actAnswer(request.id, step({ event: "timerFired", key: request.key, id: request.timer }));
      case "aiStep":
        current();
        return aiStep(request.id);
      case "catalog":
        // §5.1: the catalog is public, so the setup previews decks with it before any game exists.
        return { id: request.id, type: "catalog", defs: registeredCatalog() };
      case "replays":
        refuseWhilePlaying();
        return listReplays(request.id);
      case "replay":
        refuseWhilePlaying();
        return replayAnswer(request.id, request.game, request.from, request.count);
      case "debug": {
        if (!env.dev) return { id: request.id, type: "failed", message: "debug is available only in a development build" };
        current();
        const { debug } = step({ event: "debug" });
        if (debug === undefined) throw new Error("the practice engine answered with no debug");
        return { id: request.id, type: "debug", debug };
      }
      case "claim": {
        current();
        const { claim } = step({ event: "claim" });
        if (claim === undefined) throw new Error("the practice engine answered with no claim");
        return { id: request.id, type: "claim", claim };
      }
      case "runReport": {
        current();
        const { report } = step({ event: "runReport" });
        if (report === undefined) throw new Error("the practice engine answered with no report");
        return "action" in report
          ? { id: request.id, type: "runReport", action: report.action }
          : { id: request.id, type: "runReport", error: report.error };
      }
    }
  }

  return {
    handle(request: PracticeRequest): PracticeResponse {
      try {
        return handleUnsafe(request);
      } catch (cause) {
        return { id: request.id, type: "failed", message: messageOf(cause) };
      }
    },
  };
}
