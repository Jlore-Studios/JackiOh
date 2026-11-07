// R3338000: a Terminus card's signature in the queue. The runner plays its hook's run of the event types
// its set piece names (or its first cast) as one pinned entry (`signatureRunAt`), which its set piece
// claims as the action's one hold (R2785000): capped at SET_PIECE_HOLD_CAP_MS, scaled by the effects
// speed, shortened by Big moments "short", dropped by "off" and Reduce motion. A Mythic's takeover
// yields to its own signature, so the action still holds once; a Trap's firing is its signature's entry.

import { CATALOG } from "@jackioh/cards";
import type { CardType, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { FX_CARD_TABLE } from "../fx/cards/tables.ts";
import { doomFixture, galleryCardEvents } from "../fx/motifs/gallery.ts";
import { DOOM_MISS_HOLD_MS, SET_PIECES, hasSetPiece, signatureClaim, signatureRunOf, wipeClaim, type SetPieceKey } from "../fx/setPieces.ts";
import type { FxBigMoments } from "../fx/settings.ts";
import { BIG_CRUNCH, DICE_OF_DOOM, LAST_WORD, OLIVIAN_EXPANSE } from "../fx/terminus/ids.ts";
import { TERMINUS_PLANNERS } from "../fx/terminus/index.ts";
import {
  CAST_ENTRY_MS,
  SET_PIECE_HOLD_CAP_MS,
  SET_PIECE_SHORT_HOLD_MS,
  SIGNATURE_RUN_MS,
  TAKEOVER_HOLD_MS,
  TAKEOVER_SET_PIECE,
  createAnimationQueue,
  planEntries,
  setPieceHoldMs,
  type AnimationEntry,
  type EntranceResolver,
} from "./animations.ts";
import { createPlayTracker } from "./runs.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";

/** As Game resolves an entrance: a token by the rarity it prints (R503, `createEntranceResolver`). */
const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type as CardType, rarity: def.printedRarity ?? def.rarity };
};

/** The Terminus signatures (the board clears among the Terminus cards are wipes, animations.wipe.test.ts). */
const SIGNATURE_KEYS = Object.keys(TERMINUS_PLANNERS) as SetPieceKey[];

function cardOf(key: SetPieceKey): string {
  const found = [...FX_CARD_TABLE].find(([, entry]) => entry.kind === "setPiece" && entry.key === key);
  if (found === undefined) throw new Error(`no card names ${key}`);
  return found[0];
}

const signed = (entries: readonly AnimationEntry[]): AnimationEntry[] => entries.filter((entry) => entry.signature !== undefined);

/** A queue whose timers run when the test says, naming the set pieces as Game does. */
function queueOf(settings: { speed?: number; bigMoments?: FxBigMoments; reduced?: boolean } = {}, burstBudgetMs?: number) {
  const timers: { ms: number; run: () => void }[] = [];
  const started: AnimationEntry[] = [];
  const stills: AnimationEntry[] = [];
  const queue = createAnimationQueue({
    reducedMotion: settings.reduced ?? false,
    schedule: (run, ms) => {
      timers.push({ ms, run });
    },
    settings: () => ({ speed: settings.speed ?? 1, motion: "system", intensity: "normal", bigMoments: settings.bigMoments ?? "full" }),
    entranceOf: resolve,
    wipeOf: hasSetPiece,
    signatureOf: signatureRunOf,
    setPieceOf: (entry) => wipeClaim(entry) ?? signatureClaim(entry),
    ...(burstBudgetMs === undefined ? {} : { burstBudgetMs }),
  });
  queue.subscribeSignals((signal) => {
    if (signal.kind === "start") started.push(signal.entry);
    if (signal.kind === "still") stills.push(signal.entry);
  });
  const playAll = (): AnimationEntry[] => {
    for (let guard = 0; guard < 400 && timers.length > 0; guard += 1) timers.shift()?.run();
    return started;
  };
  return { queue, playAll, stills };
}

/** The gallery's play of `key`'s card by the viewer: its play, its run, its resolution. */
function galleryPlay(key: SetPieceKey): GameEvent[] {
  return galleryCardEvents(cardOf(key), { kind: "setPiece", key }, fullBoardView(), 1);
}

describe("R3338000 a Terminus card's signature is one entry", () => {
  it("R3338000 a signature run is one entry per card, pinned, its rows grouped", () => {
    const view = fullBoardView();
    for (const key of SIGNATURE_KEYS) {
      const defId = cardOf(key);
      const events = galleryPlay(key);
      const entries = planEntries(events, withEvents(view, events), false, createPlayTracker(), hasSetPiece, signatureRunOf);
      const [entry, ...more] = signed(entries);
      expect(more, key).toEqual([]);
      expect(entry?.signature?.by, key).toBe(defId);
      expect(entry?.signature?.player, key).toBe(view.viewer);
      const moment = SET_PIECES[key].moment;
      if (moment === "cast") {
        // The Big Crunch signs its first cast alone.
        expect(entry?.cast?.by, key).toBe(BIG_CRUNCH);
        expect(entry?.durationMs, key).toBe(CAST_ENTRY_MS);
      } else {
        expect(entry?.durationMs, key).toBe(SIGNATURE_RUN_MS);
        if (moment === "wipe") throw new Error(`${key} is a wipe`);
        // Every event of the run is in the one entry, of the types its set piece names.
        const run = events.filter((event) => moment.includes(event.type));
        expect(entry?.events, key).toEqual(run);
      }
      // A caller that names no signatures plans as it always did.
      expect(signed(planEntries(events, view, false, createPlayTracker(), hasSetPiece)), key).toEqual([]);
      // Pinned: a burst budget far smaller than it never squeezes it.
      const { queue, playAll } = queueOf({ bigMoments: "off" }, 1);
      queue.enqueue(events, withEvents(view, []));
      expect(playAll().find((started) => started.signature !== undefined)?.durationMs, key).toBe(entry?.durationMs);
    }
  });

  it("R3338000 a hidden hook signs nothing, and a hook signs once", () => {
    const view = fullBoardView();
    const hidden = galleryPlay("eventHorizon").map((event) => ("defId" in event && event.defId === cardOf("eventHorizon") ? { ...event, defId: "hidden" } : event)) as GameEvent[];
    expect(signed(planEntries(hidden, view, false, createPlayTracker(), hasSetPiece, signatureRunOf))).toEqual([]);
    // Two runs under one marker: the first is its signature, the second plays as ever.
    const events = galleryPlay("lastExam");
    const damage = events.find((event) => event.type === "damage");
    if (damage === undefined) throw new Error("no damage");
    const twice = [...events.slice(0, -1), { type: "drawn", player: "p1", instanceId: "x", defId: "core-001" } as GameEvent, damage, ...events.slice(-1)];
    expect(signed(planEntries(twice, view, false, createPlayTracker(), hasSetPiece, signatureRunOf))).toHaveLength(1);
  });

  it("R3338000 the takeover yields: one hold per action, a Trap's firing its own signature's entry", () => {
    const view = fullBoardView();
    // Dice of Doom is a Mythic: its takeover yields to its roll.
    const { queue, playAll } = queueOf();
    queue.enqueue(galleryPlay("diceOfDoom"), withEvents(view, []));
    const started = playAll();
    const entrance = started.find((entry) => entry.entrance !== undefined);
    expect(entrance?.entrance).toMatchObject({ tier: "mythic", defId: DICE_OF_DOOM, yielded: true });
    expect(entrance?.setPiece).toBeUndefined();
    expect(started.filter((entry) => entry.setPiece !== undefined).map((entry) => entry.setPiece?.key)).toEqual(["diceOfDoom"]);

    // Last Word reveals itself: the reveal's entrance and its signature are one entry, held once, as its set piece.
    const fired: GameEvent[] = [
      { type: "trapFired", instanceId: "lw", defId: LAST_WORD, controller: "p1", row: "backrow", lane: 1 },
      { type: "enteredGraveyard", instanceId: "lw", defId: LAST_WORD, owner: "p1" },
      { type: "modifierChanged", player: "p2", modifierId: "m26", added: true },
      { type: "modifierChanged", player: "p1", modifierId: "m27", added: true },
    ];
    const trap = queueOf();
    trap.queue.enqueue(fired, withEvents(view, []));
    const [firing, ...rest] = trap.playAll();
    expect(rest).toEqual([]);
    expect(firing?.entrance).toMatchObject({ tier: "mythic", reveal: true, yielded: true });
    expect(firing?.setPiece).toEqual({ key: "lastWord", holdMs: SET_PIECES.lastWord.holdMs });
    expect(firing?.signature).toEqual({ by: LAST_WORD, sourceId: "lw", player: "p1" });

    // Olivian Expanse's Discover waits on its pick: its takeover holds the play's action, its rift the pick's.
    const discover = queueOf();
    discover.queue.enqueue(
      [
        { type: "cardPlayed", player: "p1", instanceId: "olv", defId: OLIVIAN_EXPANSE, costPaid: 4 },
        { type: "hookResolving", sourceId: "olv", defId: OLIVIAN_EXPANSE, controller: "p1", hook: "cry" },
      ],
      withEvents(view, []),
    );
    discover.queue.enqueue(
      [
        { type: "addedToHand", player: "p1", instanceId: "pick", defId: "stinkyscape-004-3" },
        { type: "cardResolved", player: "p1", instanceId: "olv", defId: OLIVIAN_EXPANSE, permanent: false, costPaid: 4 },
      ],
      withEvents(view, []),
    );
    const held = discover.playAll().filter((entry) => entry.setPiece !== undefined);
    expect(held.map((entry) => entry.setPiece)).toEqual([
      { key: TAKEOVER_SET_PIECE, holdMs: TAKEOVER_HOLD_MS },
      { key: "terminusRift", holdMs: SET_PIECES.terminusRift.holdMs },
    ]);
  });

  it("R3338000 full, short, off and speed hold within SET_PIECE_HOLD_CAP_MS", () => {
    const view = fullBoardView();
    const heldBy = (key: SetPieceKey, settings: { speed?: number; bigMoments?: FxBigMoments }): AnimationEntry | undefined => {
      const { queue, playAll } = queueOf(settings);
      queue.enqueue(galleryPlay(key), withEvents(view, []));
      return playAll().find((entry) => entry.signature !== undefined);
    };
    for (const key of SIGNATURE_KEYS) {
      const asked = SET_PIECES[key].holdMs;
      expect(asked, key).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
      const full = heldBy(key, {});
      expect(full?.setPiece, key).toEqual({ key, holdMs: asked });
      expect(heldBy(key, { bigMoments: "short" })?.setPiece?.holdMs, key).toBe(SET_PIECE_SHORT_HOLD_MS);
      const off = heldBy(key, { bigMoments: "off" });
      expect(off?.setPiece, key).toBeUndefined();
      for (const speed of [0.25, 0.5, 2]) {
        const hold = heldBy(key, { speed })?.setPiece?.holdMs ?? 0;
        expect(hold, `${key} ×${String(speed)}`).toBe(setPieceHoldMs(asked, "full", speed));
        expect(hold, `${key} ×${String(speed)}`).toBeLessThanOrEqual(SET_PIECE_HOLD_CAP_MS);
      }
    }
  });

  it("R3338000 reduced motion holds nothing, and each signature plays a still", () => {
    const view = fullBoardView();
    for (const key of SIGNATURE_KEYS) {
      const { queue, playAll, stills } = queueOf({ reduced: true });
      queue.enqueue(galleryPlay(key), withEvents(view, []));
      expect(playAll().filter((entry) => entry.setPiece !== undefined), key).toEqual([]);
      const still = stills.find((entry) => entry.signature !== undefined);
      expect(still?.durationMs, key).toBe(0);
      expect(still?.setPiece, key).toBeUndefined();
    }
  });

  it("R3338000 a Dice miss asks DOOM_MISS_HOLD_MS, a 67 its full beat", () => {
    const view = fullBoardView();
    const doom = (roll: "jackpot" | "miss" | "embiggened", player: PlayerId = "p1"): GameEvent[] => [
      { type: "cardPlayed", player, instanceId: "dd", defId: DICE_OF_DOOM, costPaid: 0 },
      ...doomFixture(view, "dd", player, roll),
      { type: "cardResolved", player, instanceId: "dd", defId: DICE_OF_DOOM, permanent: false, costPaid: 0 },
    ];
    const holdOf = (events: readonly GameEvent[]): number | undefined => {
      const { queue, playAll } = queueOf();
      queue.enqueue(events, withEvents(view, []));
      return playAll().find((entry) => entry.signature !== undefined)?.setPiece?.holdMs;
    };
    expect(holdOf(doom("jackpot"))).toBe(SET_PIECES.diceOfDoom.holdMs);
    expect(holdOf(doom("miss"))).toBe(DOOM_MISS_HOLD_MS);
    expect(holdOf(doom("embiggened"))).toBe(DOOM_MISS_HOLD_MS);
    // The jackpot is what the run did, never the face: a 67 that added nothing (a full hand burns them) still rains.
    const burned = doom("jackpot").map((event) => (event.type === "addedToHand" ? ({ type: "burned", instanceId: event.instanceId, defId: event.defId, owner: event.player } as GameEvent) : event));
    expect(holdOf(burned)).toBe(SET_PIECES.diceOfDoom.holdMs);
    expect(DOOM_MISS_HOLD_MS).toBeLessThan(SET_PIECES.diceOfDoom.holdMs);
  });

  it("R3338000 Dice of Doom's end-of-turn return plays its own rows, into a hand or burned, on both seats", () => {
    const view = fullBoardView();
    const back = (player: PlayerId, full: boolean): GameEvent[] => [
      { type: "hookResolving", sourceId: "dd", defId: DICE_OF_DOOM, controller: player, hook: "endOfTurn" },
      { type: "bounced", instanceId: "dd", defId: DICE_OF_DOOM, owner: player },
      ...(full
        ? [
            { type: "burned", instanceId: "dd", defId: DICE_OF_DOOM, owner: player } as const,
            { type: "enteredGraveyard", instanceId: "dd", defId: DICE_OF_DOOM, owner: player } as const,
          ]
        : [{ type: "addedToHand", player, instanceId: "dd", defId: DICE_OF_DOOM } as const]),
      { type: "turnEnded", player, turn: 3, unspentMana: 0 },
    ];
    for (const player of ["p1", "p2"] as const) {
      for (const full of [false, true]) {
        const events = back(player, full);
        const label = `${player} ${full ? "burned" : "in hand"}`;
        expect(signed(planEntries(events, withEvents(view, events), false, createPlayTracker(), hasSetPiece, signatureRunOf)), label).toEqual([]);
        const { queue, playAll } = queueOf();
        queue.enqueue(events, withEvents(view, []));
        expect(playAll().filter((entry) => entry.setPiece !== undefined), label).toEqual([]);
      }
    }
  });

  it("R3338000 a Dice jackpot into a full hand is one entry, its burns and their graveyards among it", () => {
    const view = fullBoardView();
    const jackpot = doomFixture(view, "dd", "p1", "jackpot");
    // The hand holds five: the last two Terminus cards burn, each into the graveyard.
    let added = 0;
    const full = jackpot.flatMap((event): GameEvent[] => {
      if (event.type !== "addedToHand") return [event];
      added += 1;
      if (added <= 5) return [event];
      return [
        { type: "burned", instanceId: event.instanceId, defId: event.defId, owner: event.player },
        { type: "enteredGraveyard", instanceId: event.instanceId, defId: event.defId, owner: event.player },
      ];
    });
    const events: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "dd", defId: DICE_OF_DOOM, costPaid: 0 },
      ...full,
      { type: "cardResolved", player: "p1", instanceId: "dd", defId: DICE_OF_DOOM, permanent: false, costPaid: 0 },
    ];
    const [entry, ...more] = signed(planEntries(events, withEvents(view, events), false, createPlayTracker(), hasSetPiece, signatureRunOf));
    expect(more).toEqual([]);
    expect(entry?.events).toEqual(full.slice(1));
  });
});
