// R3399000: Classic+'s signatures (#3399), every Legendary and Mythic of classicplus.json5, tokens by the
// rarity they print. Each card's gallery play, planned as the board's effects layer plans it (remember,
// then plan, with the queue's holds), draws its own first beat (a Unit's or a Field Spell's entrance on its
// `summoned`, a Spell's or a Trap's cast) and its text's beat on the events its recipe claims; its recipe
// answers nothing outside the hook it is drawn for; no cue names a card (R202); and every own cue (its
// entrance, its recipe and its takeover) ends within its ceiling of the entry it plays in (the per-beat
// measure of R3399005), shaking within its type's. R3399003: played from the other seat, with the
// events that seat reads (a card added to a hand, shuffled into a deck or stolen, a hand card's cost, Radiance
// or fusion hidden, a prompt naming only its player), no beat shows a hidden card and no owner-only beat plays.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  entranceEntries,
  planEntries,
  setPieceEntries,
  takeoverClaim,
  type AnimationEntry,
  type EntranceResolver,
} from "../../../game/animations.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { planCardFx } from "../../cardFx.ts";
import {
  FX_INTENSITY_SCALE,
  FX_MAX_PARTICLE_LIFE_MS,
  FX_MAX_TAIL_MS,
  SIGNATURE_LEGENDARY_MAX_MS,
  SIGNATURE_MYTHIC_MAX_MS,
  SIGNATURE_PERMANENT_MAX_TRAUMA,
  SIGNATURE_SPELL_MAX_TRAUMA,
  SIGNATURE_SPELL_SHARE,
} from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { createFxMemory } from "../../memory.ts";
import { PHONE_HEAVIEST_SIGNATURES, RECIPE_FIXTURES, galleryBoardOf, galleryCardEvents, galleryFieldView, galleryPlayId, galleryPlayOf } from "../../motifs/gallery.ts";
import type { FxCardHook } from "../../motifs/names.ts";
import { hasSetPiece, wipeClaim } from "../../setPieces.ts";
import { entranceCues } from "../../takeover.ts";
import type { FxCue, FxPlanEnv, FxPlay } from "../../types.ts";
import { CARD_RECIPES } from "../bespoke.ts";
import { FX_CARD_TABLE, isSignature, type FxCardEntry } from "../tables.ts";

/** Every Classic+ signature, in table order. */
const SIGNATURES: readonly (readonly [string, FxCardEntry])[] = [...FX_CARD_TABLE].filter(([defId, entry]) => defId.startsWith("classicplus-") && isSignature(entry));

/** The hook each recipe draws its text's beat on (undefined: its play, a Spell's cast or a Unit's Cry), and the event types it claims there. */
const CLAIMS: Readonly<Record<string, { hook: FxCardHook | undefined; events: readonly GameEvent["type"][] }>> = {
  "classicplus-012": { hook: "trigger", events: ["addedToHand"] },
  "classicplus-012-1": { hook: undefined, events: ["cardPlayed", "destroyed", "damage"] },
  "classicplus-012-2": { hook: undefined, events: ["cardPlayed", "damage"] },
  "classicplus-012-3": { hook: undefined, events: ["cardPlayed", "stolen"] },
  "classicplus-012-4": { hook: undefined, events: ["cardPlayed", "damage"] },
  "classicplus-012-5": { hook: undefined, events: ["keywordGranted"] },
  "classicplus-012-6": { hook: undefined, events: ["cardPlayed", "destroyed", "exiled"] },
  "classicplus-012-7": { hook: undefined, events: ["cardPlayed", "exiled", "summoned"] },
  "classicplus-012-8": { hook: "death", events: ["summoned"] },
  "classicplus-013": { hook: "death", events: ["addedToHand"] },
  "classicplus-019": { hook: undefined, events: ["cardPlayed", "rolled", "buffed"] },
  "classicplus-019-3": { hook: undefined, events: ["rolled", "buffed"] },
  "classicplus-019-4": { hook: "trigger", events: ["healed"] },
  "classicplus-019-5": { hook: "trigger", events: ["buffed"] },
  "classicplus-027": { hook: undefined, events: ["enteredGraveyard", "addedToHand"] },
  "classicplus-029": { hook: undefined, events: ["addedToHand"] },
  "classicplus-035": { hook: undefined, events: ["cardPlayed", "rolledBack"] },
  "classicplus-037": { hook: "trigger", events: ["cardAnnounced"] },
  "classicplus-042": { hook: undefined, events: ["cardPlayed", "addedToHand"] },
  "classicplus-042-1": { hook: "activate", events: ["discarded", "addedToHand"] },
  "classicplus-043": { hook: undefined, events: ["cardPlayed", "fused", "addedToHand"] },
  "classicplus-046": { hook: "death", events: ["shuffledIn"] },
  "classicplus-047": { hook: undefined, events: ["cardPlayed", "cardAnnounced"] },
  "classicplus-048": { hook: "death", events: ["addedToHand"] },
  "classicplus-065-4": { hook: undefined, events: ["cardPlayed", "radiantSet"] },
  "classicplus-065-5": { hook: undefined, events: ["enteredGraveyard", "addedToHand"] },
  "classicplus-073": { hook: undefined, events: ["cardPlayed", "chaosRolled"] },
  "classicplus-074": { hook: "cast", events: ["fused"] },
  "classicplus-075": { hook: undefined, events: ["shuffledIn"] },
  "classicplus-075-1": { hook: undefined, events: ["cardPlayed", "addedToHand"] },
  "classicplus-078": { hook: "activate", events: ["addedToHand"] },
};

/** As the board resolves an entrance: the card's type and rarity, a token's the one it prints (B2.5). */
const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

/** A cue's end, as tiers.test.ts counts it: bursts burn, shots fly then burst, shakes ride, the rest run out. */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shot":
    case "projectile":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

/** One entry of a gallery play: its own cues (entrance and takeover, recipe), each event's recipe answer, and the play resolving. */
type Beat = { entry: AnimationEntry; entrance: FxCue[]; recipe: { event: GameEvent; cues: FxCue[] }[]; play: FxPlay | undefined; all: FxCue[] };

/** `defId`'s gallery play over its own board, entered and held as the queue holds it, at speed 1 with Big moments full. */
function galleryBeats(defId: string, entry: FxCardEntry): Beat[] {
  const play = galleryPlayOf(defId, entry);
  if (play === null) throw new Error(`${defId}'s signature has no gallery play`);
  const board = galleryBoardOf(defId, fullBoardView());
  const events = galleryCardEvents(defId, play, board, 1);
  const fielded = withEvents(galleryFieldView(defId, board), events);
  const entered = entranceEntries(planEntries(events, fielded, false, undefined, hasSetPiece), resolve, { seen: new Set() });
  const planned = setPieceEntries(entered, (e) => wipeClaim(e) ?? takeoverClaim(e), { reduced: false, bigMoments: "full", speed: 1 });
  const env = envOf();
  return planned.map((planEntry) => {
    env.memory.remember(planEntry.events);
    const recipe = planEntry.events.flatMap((event) => {
      const result = planCardFx(event, { entry: planEntry, view: planEntry.view, env, D: planEntry.durationMs });
      return result === null ? [] : [{ event, cues: result.cues }];
    });
    const entrance = entranceCues(planEntry, env, planEntry.durationMs);
    return { entry: planEntry, entrance, recipe, play: env.memory.resolving(), all: [...entrance, ...planFx(planEntry, planEntry.view, env)] };
  });
}

describe("R3399000 every Classic+ Legendary and Mythic plays its own signature", () => {
  it("R3399000 the Classic+ signatures are the set's 35 Legendaries and Mythics, tokens by printed rarity, each claiming its listed events", () => {
    const owed = Object.keys(CATALOG).filter((id) => {
      const def = CATALOG[id];
      const rarity = def?.printedRarity ?? def?.rarity;
      return id.startsWith("classicplus-") && (rarity === "Legendary" || rarity === "Mythic");
    });
    expect(owed).toHaveLength(35);
    expect(SIGNATURES.map(([defId]) => defId).sort()).toEqual([...owed].sort());
    // Each one with a recipe has its claims listed; the four with an entrance alone have none.
    for (const [defId, entry] of SIGNATURES) expect(entry.kind === "bespoke", defId).toBe(CLAIMS[defId] !== undefined);
  });

  for (const [defId, entry] of SIGNATURES) {
    const name = CATALOG[defId]?.name ?? defId;
    it(`R3399000 ${name}'s signature draws its own beats, names no card and keeps its per-beat ceiling`, () => {
      const def = CATALOG[defId];
      const rarity = def?.printedRarity ?? def?.rarity;
      const spell = def?.type === "Spell";
      const ceiling = (rarity === "Mythic" ? SIGNATURE_MYTHIC_MAX_MS : SIGNATURE_LEGENDARY_MAX_MS) * (spell ? SIGNATURE_SPELL_SHARE : 1);
      const maxTrauma = spell ? SIGNATURE_SPELL_MAX_TRAUMA : SIGNATURE_PERMANENT_MAX_TRAUMA;
      const beats = galleryBeats(defId, entry);
      const own = beats.flatMap((beat) => [...beat.entrance, ...beat.recipe.flatMap((r) => r.cues)]);
      expect(own.length, `${defId} draws`).toBeGreaterThan(0);
      // R202: no cue names a card.
      expect(JSON.stringify(own), `${defId} names no card`).not.toContain("classicplus-");
      // Its first beat: a permanent's entrance lands on its `summoned`; a Spell's or a Trap's cast is its recipe's.
      if (entry.kind !== "none" && entry.entrance !== undefined) {
        const landed = beats.some((beat) => beat.recipe.some((r) => r.event.type === "summoned" && r.event.defId === defId && r.cues.length > 0));
        expect(landed, `${defId} lands with its entrance`).toBe(true);
      }
      // Its text's beat: the recipe claims each listed event on its hook.
      const claims = CLAIMS[defId];
      if (claims !== undefined && entry.kind === "bespoke") {
        const recipe = CARD_RECIPES[entry.key];
        for (const type of claims.events) {
          const claimed = beats.some((beat) => beat.play?.defId === defId && beat.play.hook === claims.hook && beat.entry.events.some((event) => event.type === type && recipe(event, { entry: beat.entry, view: beat.entry.view, env: envOf(), D: beat.entry.durationMs }, beat.play as FxPlay) !== null));
          expect(claimed, `${defId} claims ${type}`).toBe(true);
        }
        // Outside its hook it answers nothing.
        const other: FxCardHook | undefined = claims.hook === undefined ? "trigger" : undefined;
        for (const beat of beats) {
          if (beat.play === undefined) continue;
          for (const event of beat.entry.events) {
            const answer = recipe(event, { entry: beat.entry, view: beat.entry.view, env: envOf(), D: beat.entry.durationMs }, { ...beat.play, hook: other });
            expect(answer, `${defId} answers ${event.type} outside its hook`).toBeNull();
          }
        }
      }
      // Each own cue ends within its ceiling of its entry's start, and R200's tail: a Mythic's takeover beat (its
      // entrance's cues, the takeover's hold included) within the Mythic's, with no Spell share; the rest within its own.
      for (const beat of beats) {
        const D = beat.entry.durationMs;
        const takeover = rarity === "Mythic" ? beat.entrance : [];
        for (const cue of [...beat.entrance, ...beat.recipe.flatMap((r) => r.cues)]) {
          const where = `${defId} ${cue.kind} at ${String(cue.delayMs)} of ${String(D)}`;
          if (cue.kind === "pull" || cue.kind === "conceal") continue;
          expect(cue.delayMs, where).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, where).toBeLessThanOrEqual(D);
          expect(endOf(cue), where).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          expect(endOf(cue), where).toBeLessThanOrEqual(takeover.includes(cue) ? SIGNATURE_MYTHIC_MAX_MS : ceiling);
          if (cue.kind === "shake") expect(cue.trauma, where).toBeLessThanOrEqual(maxTrauma);
        }
      }
      // A Mythic's takeover is its own look.
      if (rarity === "Mythic") {
        const takeover = own.find((cue) => cue.kind === "takeover");
        expect(takeover?.kind === "takeover" ? takeover.look : undefined, `${defId} takeover`).toBe(entry.kind === "none" ? undefined : entry.takeover);
      }
    });
  }
});

/** The sentinel a redacted event names in place of a card the seat may not read (R202). */
const HIDDEN = "hidden";
/** The cost a redacted `costChanged` carries (the engine's HIDDEN_COST). */
const HIDDEN_COST = -1;

/** The ids and definitions of the cards `events` put in or take from a hand or a deck, or change there: the owner's secrets. */
function secretsOf(events: readonly GameEvent[]): { ids: Set<string>; defIds: Set<string> } {
  const ids = new Set<string>();
  const defIds = new Set<string>();
  for (const event of events) {
    if (event.type === "addedToHand" || event.type === "shuffledIn" || event.type === "stolen" || (event.type === "radiantSet" && event.zone.z !== "field")) {
      ids.add(event.instanceId);
      defIds.add(event.defId);
    }
  }
  for (const event of events) {
    if (event.type === "fused" && ids.has(event.resultInstanceId)) defIds.add(event.defId);
  }
  return { ids, defIds };
}

/** `events` as the other seat reads them: R202's redaction of every secret, and a prompt naming only its player. */
function redacted(events: readonly GameEvent[], ids: ReadonlySet<string>): GameEvent[] {
  return events.map((event): GameEvent => {
    switch (event.type) {
      case "addedToHand":
      case "shuffledIn":
      case "stolen":
      case "radiantSet":
        return ids.has(event.instanceId) ? { ...event, instanceId: HIDDEN, defId: HIDDEN } : event;
      case "costChanged":
        return ids.has(event.instanceId) ? { type: "costChanged", instanceId: HIDDEN, cost: HIDDEN_COST } : event;
      case "fused":
        return ids.has(event.resultInstanceId) ? { ...event, instanceIds: event.instanceIds.map(() => HIDDEN), resultInstanceId: HIDDEN, defId: HIDDEN } : event;
      case "promptOpened":
      case "promptAnswered":
        return { type: event.type, player: event.player };
      default:
        return event;
    }
  });
}

describe("R3399003 from the other seat no Classic+ signature shows a hidden card", () => {
  it("R3399003 each signature played by the opponent draws its public beats, and none names a card in a hand or deck or plays an owner-only beat", () => {
    let secretsSeen = 0;
    for (const [defId, entry] of SIGNATURES) {
      if (entry.kind !== "bespoke") continue;
      const board = galleryBoardOf(defId, fullBoardView());
      const player = board.opponent.player;
      const instanceId = galleryPlayId(1);
      const field = CATALOG[defId]?.type === "Field Spell";
      const landing: GameEvent[] =
        entry.entrance === undefined ? [] : [{ type: "summoned", player, instanceId, defId, row: field ? "backrow" : "units", lane: field ? 1 : 3 }];
      const owned: GameEvent[] = [
        { type: "cardPlayed", player, instanceId, defId, costPaid: 1 },
        ...landing,
        ...RECIPE_FIXTURES[entry.key](board, instanceId, player),
        { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 },
      ];
      const secrets = secretsOf(owned);
      if (secrets.ids.size > 0) secretsSeen += 1;
      const events = redacted(owned, secrets.ids);
      const env = envOf();
      const cues: FxCue[] = [];
      for (const planned of planEntries(events, withEvents(board, events), false, undefined, hasSetPiece)) {
        env.memory.remember(planned.events);
        for (const event of planned.events) {
          const result = planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs });
          if (result === null) continue;
          cues.push(...result.cues);
          // An owner-only beat (a Discover's options, a test's problems) plays on the owner's seat alone.
          if (event.type === "promptOpened") expect(result.cues, `${defId} prompt on the other seat`).toEqual([]);
        }
      }
      expect(cues.length, `${defId} draws from the other seat`).toBeGreaterThan(0);
      const drawn = JSON.stringify(cues);
      for (const id of secrets.ids) expect(drawn, `${defId} shows ${id}`).not.toContain(`"${id}"`);
      for (const id of secrets.ids) expect(drawn, `${defId} shows ${id}`).not.toContain(`-${id}"`);
      for (const secret of secrets.defIds) expect(drawn, `${defId} shows ${secret}`).not.toContain(secret);
      expect(drawn, `${defId} names no card`).not.toContain("classicplus-");
    }
    // The cards that add to a hand, Discover, steal or shuffle in, among them.
    expect(secretsSeen).toBeGreaterThanOrEqual(14);
  });

  it("R3399003 Golden Grape on a card in the opponent's hand shimmers over the hand pile and names nothing", () => {
    const view = fullBoardView();
    const player = view.opponent.player;
    const instanceId = galleryPlayId(1);
    const secret = { instanceId: "fx-grape-target", defId: "core-097" } as const;
    const owned: GameEvent[] = [
      { type: "cardPlayed", player, instanceId, defId: "classicplus-065-4", costPaid: 1 },
      { type: "radiantSet", instanceId: secret.instanceId, defId: secret.defId, zone: { z: "hand", player } },
      { type: "cardResolved", player, instanceId, defId: "classicplus-065-4", permanent: false, costPaid: 1 },
    ];
    const events = redacted(owned, secretsOf(owned).ids);
    const env = envOf();
    const cues: FxCue[] = [];
    for (const planned of planEntries(events, withEvents(view, events), false)) {
      env.memory.remember(planned.events);
      for (const event of planned.events) {
        const result = planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs });
        if (result !== null && event.type === "radiantSet") cues.push(...result.cues);
      }
    }
    expect(cues.length).toBeGreaterThan(0);
    expect(cues.every((cue) => !("at" in cue) || cue.at.kind !== "testid" || cue.at.testid === "hand-opponent")).toBe(true);
    expect(JSON.stringify(cues)).not.toContain(secret.instanceId);
    expect(JSON.stringify(cues)).not.toContain(secret.defId);
  });
});

describe("R3399000 the phone frame spec plays Classic+'s two heaviest signatures", () => {
  it("R3399000 PHONE_HEAVIEST_SIGNATURES are the two Classic+ signatures whose gallery plays plan the most particles", () => {
    const particles = SIGNATURES.map(([defId, entry]) => ({
      defId,
      count: galleryBeats(defId, entry)
        .flatMap((beat) => beat.all)
        .reduce((sum, cue) => sum + (cue.kind === "burst" ? cue.count : 0), 0),
    })).sort((a, b) => b.count - a.count);
    expect([...PHONE_HEAVIEST_SIGNATURES], JSON.stringify(particles.slice(0, 4))).toEqual(particles.slice(0, 2).map((card) => card.defId));
  });
});
