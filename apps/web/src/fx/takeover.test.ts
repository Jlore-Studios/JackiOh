// R2722001: a Legendary's or a Mythic's entrance and a Mythic's takeover of the board (takeover.ts),
// planned from the entry's `entrance` (game/animations.ts `entranceEntries`), held as its set piece
// (`setPieceEntries`, R2785000) and bounded by R200.
// R2722002: a hidden card's at its reveal, centred on the zone that flipped. R3332000: a Mythic's own
// look is its effects table entry's `takeover`. R3399002: Classic+'s four Mythics wear looks of their own, each
// with a picture of its own (takeoverArt.ts).

import type { CardType, GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  entranceEntries,
  planEntries,
  setPieceEntries,
  takeoverClaim,
  type AnimationEntry,
  type EntranceResolver,
} from "../game/animations.ts";
import { baseView, emptySide } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { FX_CARD_TABLE, parseFxCardTable } from "./cards/tables.ts";
import { FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_TAKEOVER_TAIL_MS, FX_TAKEOVER_TRAUMA } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import { entranceCues, entranceLandMs, TAKEOVER_TINTS, takeoverLookOf } from "./takeover.ts";
import { TAKEOVER_ART } from "./takeoverArt.ts";
import type { FxCue, FxPlanEnv, FxTakeoverCue, TakeoverLook } from "./types.ts";

const CARDS: Record<string, { type: CardType; rarity: string }> = {
  "core-100": { type: "Unit", rarity: "Mythic" },
  "core-096": { type: "Trap", rarity: "Mythic" },
  "core-097": { type: "Spell", rarity: "Mythic" },
  "meditative-049": { type: "Unit", rarity: "Mythic" },
  "meditative-008": { type: "Spell", rarity: "Mythic" },
  "core-052": { type: "Unit", rarity: "Legendary" },
  "core-083": { type: "Spell", rarity: "Legendary" },
  "core-085": { type: "Trap", rarity: "Legendary" },
  "classicplus-027": { type: "Spell", rarity: "Mythic" },
  "classicplus-029": { type: "Spell", rarity: "Mythic" },
  "classicplus-065-5": { type: "Spell", rarity: "Mythic" },
  "classicplus-074": { type: "Field Trap", rarity: "Mythic" },
};
const resolve: EntranceResolver = (defId) => CARDS[defId] ?? null;
const p1 = baseView();
const p2 = baseView({ viewer: "p2", you: emptySide("p2"), opponent: emptySide("p1") });

const env = (intensity: number = FX_INTENSITY_SCALE.normal): FxPlanEnv => ({ intensity, card: () => undefined, memory: createFxMemory() });
const play = (defId: string): GameEvent => ({ type: "cardPlayed", player: "p1", instanceId: `i-${defId}`, defId, costPaid: 3 });
const summon = (defId: string): GameEvent => ({ type: "summoned", player: "p1", instanceId: `i-${defId}`, defId, row: "units", lane: 2 });
const fired = (defId: string, again = false): GameEvent => ({
  type: "trapFired",
  instanceId: defId === "hidden" ? "hidden" : "b1",
  defId,
  controller: "p1",
  row: "backrow",
  lane: 3,
  ...(again ? { again: true } : {}),
});

function entry(events: GameEvent[], view: PlayerView = p1): AnimationEntry {
  const entered = entranceEntries(planEntries(events, view, false), resolve, { seen: new Set() });
  const entries = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 });
  const found = entries.find((e) => e.entrance !== undefined) ?? entries[0];
  if (found === undefined) throw new Error("no entry");
  return found;
}
const takeoverOf = (cues: readonly FxCue[]): FxTakeoverCue | undefined => cues.find((c): c is FxTakeoverCue => c.kind === "takeover");
const shakes = (cues: readonly FxCue[]): number[] => cues.flatMap((c) => (c.kind === "shake" ? [c.trauma] : []));

describe("R2722001 a Mythic takes over the board", () => {
  it("R2722001 a Mythic entrance plans the takeover, every cue ending within FX_MAX_TAIL_MS of its entry", () => {
    for (const events of [[play("core-100"), summon("core-100")], [play("core-097")], [fired("core-096")]]) {
      const e = entry(events);
      const D = e.durationMs;
      const cues = planFx(e, e.view, env());
      const takeover = takeoverOf(cues);
      expect(takeover, JSON.stringify(events)).toBeDefined();
      // It lands after the hold, inside the entry, and fades out within the tail.
      expect(takeover?.landMs).toBe(entranceLandMs(e, D));
      expect(takeover?.landMs ?? 0).toBeGreaterThan(e.setPiece?.holdMs ?? 0);
      expect(takeover?.landMs ?? Infinity).toBeLessThan(D);
      expect(takeover?.durationMs).toBe(D + FX_TAKEOVER_TAIL_MS);
      expect(shakes(cues)).toContain(FX_TAKEOVER_TRAUMA);
      for (const cue of cues) {
        expect(cue.delayMs, JSON.stringify(cue)).toBeGreaterThanOrEqual(0);
        expect(cue.delayMs, JSON.stringify(cue)).toBeLessThanOrEqual(D);
        if ("durationMs" in cue) expect(cue.delayMs + cue.durationMs, JSON.stringify(cue)).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
      }
    }
  });

  it("R2722001 R670 R2803000 the marquee Mythics take their own look, the rest the shared one, a Legendary none", () => {
    expect(takeoverOf(planFx(entry([play("core-100"), summon("core-100")]), p1, env()))?.look).toBe("void");
    expect(takeoverOf(planFx(entry([fired("core-096")]), p1, env()))?.look).toBe("pawn");
    expect(takeoverOf(planFx(entry([play("core-097")]), p1, env()))?.look).toBe("genie");
    expect(takeoverOf(planFx(entry([play("meditative-049"), summon("meditative-049")]), p1, env()))?.look).toBe("ai");
    expect(takeoverOf(planFx(entry([play("meditative-008")]), p1, env()))?.look).toBe("prismatic");
    // R3332000: exactly the four marquee Mythics name a look of their own in the effects table, and
    // R3399002: Classic+'s four, in table order.
    const own = [...FX_CARD_TABLE].flatMap(([defId, card]) => (card.kind !== "none" && card.takeover !== undefined ? [defId] : []));
    expect(own).toEqual(["core-096", "core-097", "core-100", "classicplus-027", "classicplus-029", "classicplus-065-5", "classicplus-074", "meditative-049"]);
    for (const defId of own) expect(CATALOG[defId]?.printedRarity ?? CATALOG[defId]?.rarity, defId).toBe("Mythic");
    for (const events of [[play("core-052"), summon("core-052")], [play("core-083")], [fired("core-085")]]) {
      const e = entry(events);
      expect(e.entrance?.tier).toBe("legendary");
      expect(takeoverOf(planFx(e, p1, env())), JSON.stringify(events)).toBeUndefined();
    }
    // A Legendary Spell's cast, which no summon draws, takes its rays, gold burst, ring with its
    // echo (R2803000: the two-wave landing, one volume down) and its shake.
    const legend = entranceCues(entry([play("core-083")]), env(), 400);
    expect(legend.map((c) => c.kind).sort()).toEqual(["burst", "rays", "ring", "ring", "shake"]);
    const rings = legend.filter((c) => c.kind === "ring");
    expect(rings.length).toBe(2);
    expect(rings[1]?.delayMs ?? 0).toBeGreaterThan(rings[0]?.delayMs ?? 0);
  });

  it("R3332000 takeoverLookOf reads the table, prismatic by default", () => {
    expect(takeoverLookOf("core-100")).toBe("void");
    expect(takeoverLookOf("meditative-008")).toBe("prismatic");
    expect(takeoverLookOf("core-052")).toBe("prismatic");
    expect(takeoverLookOf("not-a-card")).toBe("prismatic");
    // A table that gives a Mythic another look: its takeover wears it.
    const table = parseFxCardTable("test.json5", `{ "meditative-008": { cast: { motif: "vanish" }, takeover: "genie" }, "core-100": "none: pending" }`);
    expect(takeoverLookOf("meditative-008", table)).toBe("genie");
    expect(takeoverLookOf("core-100", table)).toBe("prismatic");
    const e = entry([play("meditative-008")]);
    expect(takeoverOf(entranceCues(e, env(), e.durationMs, table))?.look).toBe("genie");
    expect(takeoverOf(entranceCues(e, env(), e.durationMs))?.look).toBe("prismatic");
  });

  it("R3399002 each Classic+ Mythic's takeover wears its own look", () => {
    const looks: [GameEvent[], TakeoverLook][] = [
      [[play("classicplus-027")], "zealot"],
      [[play("classicplus-029")], "portal"],
      [[play("classicplus-065-5")], "vineyard"],
      [[fired("classicplus-074")], "stair"],
    ];
    for (const [events, look] of looks) {
      const takeover = takeoverOf(planFx(entry(events), p1, env()));
      expect(takeover?.look, JSON.stringify(events)).toBe(look);
      expect(takeover?.tint).toEqual(TAKEOVER_TINTS[look]);
      expect(TAKEOVER_ART[look], look).toBeDefined();
    }
  });

  it("R3399002 every Classic+ own look has its picture, which names no card, and no other look has one", () => {
    expect(Object.keys(TAKEOVER_ART).sort()).toEqual(["portal", "stair", "vineyard", "zealot"]);
    /** What each concept moves: the zealot's cyclone, the portal's vortex and its ruins, the stair, the grape and its vines. */
    const moving: Record<string, string[]> = { zealot: ["spin"], portal: ["spin", "drift"], stair: ["spin"], vineyard: ["spin", "drift"] };
    for (const [look, art] of Object.entries(TAKEOVER_ART)) {
      expect(art.viewBox, look).toBe("0 0 160 90");
      expect(art.paths.length, look).toBeGreaterThanOrEqual(6);
      expect(art.paths.length, look).toBeLessThanOrEqual(14);
      const parts = new Set<string>(art.paths.map((path) => path.part));
      for (const part of moving[look] ?? []) expect(parts.has(part), `${look} ${part}`).toBe(true);
      for (const path of art.paths) {
        expect(path.fill, look).toMatch(/^#[0-9a-f]{6}$/);
        // Path data alone: commands and numbers, so no word, let alone a card's name or id, can hide in it.
        expect(path.d, look).toMatch(/^M[MLHVCSQTAZmlhvcsqtaz0-9 .-]+$/);
      }
    }
    expect(JSON.stringify(TAKEOVER_ART)).not.toContain("classicplus-");
  });

  it("R2722001 intensity 0 plans none", () => {
    const e = entry([play("core-097")]);
    expect(entranceCues(e, env(FX_INTENSITY_SCALE.off), e.durationMs)).toEqual([]);
    expect(planFx(e, p1, env(FX_INTENSITY_SCALE.off))).toEqual([]);
  });
});

describe("R2722002 a hidden card's entrance centres on the zone that flipped", () => {
  it("R2722002 a Legendary or Mythic reveal draws its rays at the zone that flipped; a firing again none", () => {
    const legend = planFx(entry([fired("core-085")]), p1, env());
    const rays = legend.find((c) => c.kind === "rays");
    expect(rays).toMatchObject({ tone: "legendary", at: { kind: "testid", testid: "zone-you-backrow-3" } });
    // The other seat reads a Cloaked card's reveal at its firing too (R2734000), centred where it flips.
    const reveal: GameEvent = { type: "activated", player: "p1", instanceId: "c61", defId: "meditative-008", ability: "reveal" };
    const entered = entranceEntries(planEntries([reveal, fired("meditative-008")], p2, false), resolve, { seen: new Set() });
    const theirs = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 });
    const at = theirs.find((e) => e.entrance !== undefined);
    if (at === undefined) throw new Error("no entry");
    expect(at.events.some((e) => e.type === "trapFired")).toBe(true);
    expect(takeoverOf(planFx(at, p2, env()))?.at).toEqual({ kind: "testid", testid: "zone-opponent-backrow-3" });
    // The sentinel's firing, and a Field Trap firing again, make no entrance.
    expect(planFx(entry([fired("hidden")], p2), p2, env()).some((c) => c.kind === "rays" || c.kind === "takeover")).toBe(false);
    expect(planFx(entry([fired("core-096", true)]), p1, env()).some((c) => c.kind === "rays" || c.kind === "takeover")).toBe(false);
  });
});
