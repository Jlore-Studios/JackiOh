// R2722001: a Legendary's or a Mythic's entrance and a Mythic's takeover of the board (takeover.ts),
// planned from the entry's `entrance` (game/animations.ts `entranceEntries`), held as its set piece
// (`setPieceEntries`, R2785000) and bounded by R200.
// R2722002: a hidden card's at its reveal, centred on the zone that flipped. R3332000: a Mythic's own
// look is its effects table entry's `takeover`. R3400000: a signature Spell's cast of its own draws in place
// of the shared rays, burst and rings, its takeover's tail FX_SIGNATURE_TAKEOVER_TAIL_MS; three own looks join.

import type { CardType, GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import {
  HIDDEN_ID,
  TAKEOVER_HOLD_MS,
  entranceEntries,
  planEntries,
  setPieceEntries,
  takeoverClaim,
  type AnimationEntry,
  type EntranceResolver,
} from "../game/animations.ts";
import { ART_PLANES } from "../cards/art/index.ts";
import { signatureFaceOf } from "../cards/signatureFaces.ts";
import { baseView, emptySide } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { FX_CARD_TABLE, parseFxCardTable } from "./cards/tables.ts";
import { FX_CENTER, FX_INTENSITY_SCALE, FX_MAX_TAIL_MS, FX_SIGNATURE_TAKEOVER_TAIL_MS, FX_TAKEOVER_TAIL_MS, FX_TAKEOVER_TRAUMA } from "./constants.ts";
import { planFx } from "./cues.ts";
import { mountDomEffect } from "./dom.ts";
import { createFxMemory } from "./memory.ts";
import { entranceCues, entranceLandMs, ownCast, TAKEOVER_TINTS, takeoverLookOf } from "./takeover.ts";
import type { FxCue, FxPlanEnv, FxTakeoverCue } from "./types.ts";

const CARDS: Record<string, { type: CardType; rarity: string }> = {
  "core-100": { type: "Unit", rarity: "Mythic" },
  "core-096": { type: "Trap", rarity: "Mythic" },
  "core-097": { type: "Spell", rarity: "Mythic" },
  "meditative-049": { type: "Unit", rarity: "Mythic" },
  "meditative-008": { type: "Spell", rarity: "Mythic" },
  "core-052": { type: "Unit", rarity: "Legendary" },
  "core-083": { type: "Spell", rarity: "Legendary" },
  "core-085": { type: "Trap", rarity: "Legendary" },
  "traininggrounds-040": { type: "Spell", rarity: "Mythic" },
  "traininggrounds-037": { type: "Spell", rarity: "Legendary" },
  "traininggrounds-038": { type: "Unit", rarity: "Legendary" },
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
    // R3332000: the four marquee Mythics name a look of their own in the effects table; R3400000: and three signatures.
    const own = [...FX_CARD_TABLE].flatMap(([defId, card]) => (card.kind !== "none" && card.takeover !== undefined ? [defId] : []));
    expect(own).toEqual(["core-096", "core-097", "core-100", "meditative-049", "meditativeplus-001", "meditativeplus-026", "traininggrounds-040"]);
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

  it("R3400000 a signature Spell draws no shared rays, burst or ring, its takeover ending FX_SIGNATURE_TAKEOVER_TAIL_MS after its entry", () => {
    expect(ownCast("traininggrounds-040", "Spell")).toBe(true);
    expect(ownCast("traininggrounds-037", "Spell")).toBe(true);
    // A Unit's signature is its entrance, and a card with no signature has no cast of its own.
    expect(ownCast("traininggrounds-038", "Unit")).toBe(false);
    expect(ownCast("core-097", "Spell")).toBe(false);
    expect(ownCast("traininggrounds-040", undefined)).toBe(false);
    // The Mythic: its own look, its tail shorter, no shared rings, burst, rays or shake.
    const mythic = entry([play("traininggrounds-040")]);
    const D = mythic.durationMs;
    expect(mythic.setPiece?.holdMs).toBeGreaterThan(0);
    const cues = entranceCues(mythic, env(), D);
    expect(cues.map((c) => c.kind)).toEqual(["takeover"]);
    expect(takeoverOf(cues)).toMatchObject({ look: "napkin", tint: TAKEOVER_TINTS.napkin, landMs: entranceLandMs(mythic, D), delayMs: 0, durationMs: D + FX_SIGNATURE_TAKEOVER_TAIL_MS });
    // Centred on the screen, as its recipe draws at the board's centre, whatever card element its play left.
    expect(takeoverOf(cues)?.at).toEqual({ kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } });
    expect(FX_SIGNATURE_TAKEOVER_TAIL_MS).toBeLessThan(FX_TAKEOVER_TAIL_MS);
    // The Legendary: nothing shared at all; its recipe draws its cast.
    const legend = entry([play("traininggrounds-037")]);
    expect(legend.entrance?.tier).toBe("legendary");
    expect(entranceCues(legend, env(), legend.durationMs)).toEqual([]);
    // Off the signature tier (a table that names the same card's recipe alone), the shared entrance plays.
    const plain = parseFxCardTable("test.json5", `{ "traininggrounds-040": { bespoke: "gameDesign", takeover: "napkin" }, "traininggrounds-037": { bespoke: "seance" } }`);
    const shared = entranceCues(mythic, env(), D, plain);
    expect(shared.map((c) => c.kind).sort()).toEqual(["burst", "rays", "ring", "ring", "ring", "shake", "shake", "takeover"]);
    expect(takeoverOf(shared)?.durationMs).toBe(D + FX_TAKEOVER_TAIL_MS);
    expect(entranceCues(legend, env(), legend.durationMs, plain).map((c) => c.kind).sort()).toEqual(["burst", "rays", "ring", "ring", "shake"]);
    // Within R200 either way.
    for (const cue of [...cues, ...shared]) if ("durationMs" in cue) expect(cue.delayMs + cue.durationMs).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
  });

  it("R3400000 summit, canvas and napkin have their own tints", () => {
    const looks = ["summit", "canvas", "napkin"] as const;
    const tints = looks.map((look) => JSON.stringify(TAKEOVER_TINTS[look]));
    expect(new Set([...tints, JSON.stringify(TAKEOVER_TINTS.prismatic)]).size).toBe(looks.length + 1);
    expect(TAKEOVER_TINTS.summit.rim, "Bob's red brushstroke").toBe("#dc2626");
    expect(TAKEOVER_TINTS.canvas.rim, "Scratch Pad's cyan cursor").toBe("#22d3ee");
    expect(TAKEOVER_TINTS.napkin.rim, "the napkin's magenta pixels").toBe("#e879f9");
    expect([takeoverLookOf("meditativeplus-001"), takeoverLookOf("meditativeplus-026"), takeoverLookOf("traininggrounds-040")]).toEqual([...looks]);
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

describe("R3345000 Mythic takeover in depth planes", () => {
  it("R3345000 every Mythic in the catalog plans a takeover with its defId and mounts depth planes", () => {
    const mythics = Object.values(CATALOG).filter((def) => def.rarity === "Mythic" || def.printedRarity === "Mythic");
    expect(mythics.length).toBeGreaterThan(0);
    const catalogResolver: EntranceResolver = (defId) => {
      const def = CATALOG[defId];
      return def ? { type: def.type, rarity: def.rarity, printedRarity: def.printedRarity } : null;
    };

    for (const mythic of mythics) {
      const isTrap = mythic.type === "Trap" || mythic.type === "Field Trap";
      const events: GameEvent[] = [
        isTrap ? fired(mythic.id) : play(mythic.id),
        ...(mythic.type === "Unit" ? [summon(mythic.id)] : []),
      ];
      const entered = entranceEntries(planEntries(events, p1, false), catalogResolver, { seen: new Set() });
      const entries = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 });
      const found = entries.find((e) => e.entrance !== undefined);
      expect(found, mythic.id).toBeDefined();
      if (!found) continue;
      const cues = planFx(found, p1, env());
      const takeover = takeoverOf(cues);
      expect(takeover, mythic.id).toBeDefined();
      expect(takeover?.card().defId).toBe(mythic.id);
      // R202: the cue's own data names no card; its card is behind the function.
      expect(Object.keys(JSON.parse(JSON.stringify(takeover)) as object), mythic.id).not.toContain("card");

      // Mount into DOM and verify depth planes & signature
      const root = document.createElement("div");
      const mounted = mountDomEffect(root, takeover!, { at: { x: 0, y: 0, width: 100, height: 100 } });
      expect(mounted).not.toBeNull();
      const stage = root.querySelector(".fx-takeover-stage");
      expect(stage, mythic.id).not.toBeNull();
      const planes = root.querySelector(".fx-takeover-planes");
      expect(planes, mythic.id).not.toBeNull();
      for (const plane of ART_PLANES) {
        const el = planes?.querySelector(`.fx-takeover-plane[data-plane="${plane}"]`);
        expect(el, `${mythic.id} plane ${plane}`).not.toBeNull();
      }
      // R2715002: the name signs in the card's set's hand, which cosmetics.css styles under `.cf-signature`.
      const hand = root.querySelector(".fx-takeover-signature .cf-signature");
      expect(hand, mythic.id).not.toBeNull();
      expect(hand?.getAttribute("data-signature-face"), mythic.id).toBe(signatureFaceOf(mythic.id));
      expect(hand?.querySelector(".cf-signature-text")?.textContent, mythic.id).toBe(mythic.name);
      // B27: the plane's picture is set through the cards' helper, one per plane.
      for (const plane of ART_PLANES) {
        const el = planes?.querySelector<HTMLElement>(`.fx-takeover-plane[data-plane="${plane}"]`);
        expect(el?.style.backgroundImage, `${mythic.id} plane ${plane} picture`).not.toBe("");
      }
    }
  });

  it("R3345000 bigMoments short gives 500ms hold and off gives 0ms hold", () => {
    const events = [play("core-100"), summon("core-100")];
    const entered = entranceEntries(planEntries(events, p1, false), resolve, { seen: new Set() });

    // full: 1000ms
    const full = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 });
    expect(full[0]?.setPiece?.holdMs).toBe(TAKEOVER_HOLD_MS);

    // short: 500ms
    const short = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "short", speed: 1 });
    expect(short[0]?.setPiece?.holdMs).toBe(TAKEOVER_HOLD_MS / 2);

    // off: no setPiece (0ms)
    const off = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "off", speed: 1 });
    expect(off[0]?.setPiece).toBeUndefined();
  });

  it("R3345000 a Mythic entry that holds no set piece mounts no full-window stage", () => {
    const events = [play("core-100"), summon("core-100")];
    const entered = entranceEntries(planEntries(events, p1, false), resolve, { seen: new Set() });
    for (const bigMoments of ["off", "full"] as const) {
      const [held] = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments, speed: 1 });
      const takeover = takeoverOf(planFx(held!, p1, env()));
      expect(takeover, bigMoments).toBeDefined();
      const root = document.createElement("div");
      mountDomEffect(root, takeover!, { at: { x: 0, y: 0, width: 100, height: 100 } });
      expect(root.querySelector(".fx-takeover-stage") !== null, bigMoments).toBe(bigMoments === "full");
      expect(root.querySelector(".fx-takeover-dark"), bigMoments).not.toBeNull();
      expect(root.querySelector(".fx-takeover-sweep"), bigMoments).not.toBeNull();
    }
  });

  it("R3345000 a hidden play shows no takeover set piece (R202)", () => {
    const hiddenPlay: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "h1", defId: HIDDEN_ID, costPaid: 3 };
    const entered = entranceEntries(planEntries([hiddenPlay], p1, false), resolve, { seen: new Set() });
    const entries = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 });
    expect(entries[0]?.setPiece).toBeUndefined();
  });

  it("R3345000 an opponent's public Mythic play triggers takeover for both seats", () => {
    const events = [play("core-100"), summon("core-100")];
    // p1 view
    const e1 = entry(events, p1);
    expect(e1.setPiece?.key).toBe("takeover");
    expect(takeoverOf(planFx(e1, p1, env()))).toBeDefined();

    // p2 view (opponent played publicly)
    const entered2 = entranceEntries(planEntries(events, p2, false), resolve, { seen: new Set() });
    const e2 = setPieceEntries(entered2, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 })[0];
    expect(e2?.setPiece?.key).toBe("takeover");
    expect(takeoverOf(planFx(e2!, p2, env()))).toBeDefined();
  });
});
