// R2722001: a Legendary's or a Mythic's entrance and a Mythic's takeover of the board (takeover.ts),
// planned from the entry's `entrance` (game/animations.ts `entranceEntries`), held as its set piece
// (`setPieceEntries`, R2785000) and bounded by R200.
// R2722002: a hidden card's at its reveal, centred on the zone that flipped. R3332000: a Mythic's own
// look is its effects table entry's `takeover`. R3400000: a signature Spell's cast of its own draws in place
// of the shared rays, burst and rings, its takeover's tail FX_SIGNATURE_TAKEOVER_TAIL_MS; three own looks join.
// R3389000: In Too Deep's is "abyss", and Classic's Spell's and Trap's signature casts draw no shared rays.
// R3402000: two more own looks, "pawn" indigo; R3402001: Core's signature Spells and Traps cast on their own.
// R3395001: Stinkyscape's five Mythics' own looks, each with art of its own.
// R3407000: Stinkyscape part two's six Mythics take looks of their own, each drawing its card's own picture.
// R3405000: each Meditative Mythic wears a look of its own with art. R3399002: Classic+'s four Mythics wear looks
// of their own, each drawn as a picture of its own (takeoverArt.ts).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

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
import { testid } from "../game/contract.ts";
import { baseView, emptySide } from "../test/fixtures.ts";
import { CATALOG } from "../wire/cards.ts";
import { FX_CARD_TABLE, parseFxCardTable } from "./cards/tables.ts";
import { FX_CENTER, FX_INTENSITY_SCALE, FX_LEGENDARY_TRAUMA, FX_MAX_TAIL_MS, FX_SIGNATURE_TAKEOVER_TAIL_MS, FX_TAKEOVER_TAIL_MS, FX_TAKEOVER_TRAUMA } from "./constants.ts";
import { planFx } from "./cues.ts";
import { mountDomEffect } from "./dom.ts";
import { createFxMemory } from "./memory.ts";
import { entranceCues, entranceLandMs, ownCast, TAKEOVER_ART_LOOKS, TAKEOVER_TINTS, takeoverArtOf, takeoverLookOf } from "./takeover.ts";
import { TAKEOVER_ART_DRAWINGS, takeoverArtParts } from "./takeoverArt.ts";
import type { FxCue, FxPlanEnv, FxTakeoverCue, TakeoverLook } from "./types.ts";

const CARDS: Record<string, { type: CardType; rarity: string }> = {
  "core-100": { type: "Unit", rarity: "Mythic" },
  "core-096": { type: "Trap", rarity: "Mythic" },
  "core-097": { type: "Spell", rarity: "Mythic" },
  "core-098": { type: "Field Spell", rarity: "Mythic" },
  "core-099": { type: "Spell", rarity: "Mythic" },
  "meditative-049": { type: "Unit", rarity: "Mythic" },
  "meditative-008": { type: "Spell", rarity: "Mythic" },
  "classicplus-029": { type: "Spell", rarity: "Mythic" },
  "core-052": { type: "Unit", rarity: "Legendary" },
  "core-083": { type: "Spell", rarity: "Legendary" },
  "core-085": { type: "Trap", rarity: "Legendary" },
  "classic-009": { type: "Trap", rarity: "Legendary" },
  "classic-044": { type: "Spell", rarity: "Legendary" },
  "classicplus-035": { type: "Spell", rarity: "Legendary" },
  "classic-090": { type: "Field Spell", rarity: "Mythic" },
  "traininggrounds-040": { type: "Spell", rarity: "Mythic" },
  "traininggrounds-037": { type: "Spell", rarity: "Legendary" },
  "traininggrounds-038": { type: "Unit", rarity: "Legendary" },
  "stinkyscape-044": { type: "Spell", rarity: "Mythic" },
  "stinkyscape-036": { type: "Spell", rarity: "Mythic" },
  "stinkyscape-137-1": { type: "Spell", rarity: "Legendary" },
  "stinkyscape-109": { type: "Unit", rarity: "Mythic" },
  "stinkyscape-110": { type: "Unit", rarity: "Mythic" },
  "stinkyscape-111": { type: "Unit", rarity: "Mythic" },
  "stinkyscape-148": { type: "Field Spell", rarity: "Mythic" },
  "stinkyscape-148-1": { type: "Spell", rarity: "Mythic" },
  "stinkyscape-t-red-narcor": { type: "Spell", rarity: "Mythic" },
  "classicplus-027": { type: "Spell", rarity: "Mythic" },
  "classicplus-065-5": { type: "Spell", rarity: "Mythic" },
  "classicplus-074": { type: "Field Trap", rarity: "Mythic" },
};
const resolve: EntranceResolver = (defId) => CARDS[defId] ?? null;
/** The effects layer's stylesheet, for the takeover looks' art. */
const fxCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "fx.css"), "utf8");
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
/** Income Tax as it was before its signature (R3389001): no Legendary Trap in the catalog is still without one. */
const PLAIN_TRAP = parseFxCardTable("test.json5", `{ "classic-009": { cast: { motif: "tether", palette: "Hustle/cash" } } }`);

describe("R2722001 a Mythic takes over the board", () => {
  it("R2722001 a Mythic entrance plans the takeover, every cue ending within FX_MAX_TAIL_MS of its entry", () => {
    for (const events of [[play("core-100"), summon("core-100")], [play("meditative-008")], [play("core-097")], [fired("core-096")]]) {
      const e = entry(events);
      const D = e.durationMs;
      const cues = planFx(e, e.view, env());
      const takeover = takeoverOf(cues);
      expect(takeover, JSON.stringify(events)).toBeDefined();
      // It lands after the hold, inside the entry, and fades out within the tail.
      expect(takeover?.landMs).toBe(entranceLandMs(e, D));
      expect(takeover?.landMs ?? 0).toBeGreaterThan(e.setPiece?.holdMs ?? 0);
      expect(takeover?.landMs ?? Infinity).toBeLessThan(D);
      // R3400000: a signature Spell's or Trap's own cast (Zephrys, My Pawn) keeps its takeover alone, the sooner gone.
      const first = events[0];
      const defId = first?.type === "cardPlayed" || first?.type === "trapFired" ? first.defId : "";
      const own = ownCast(defId, CARDS[defId]?.type);
      expect(takeover?.durationMs).toBe(D + (own ? FX_SIGNATURE_TAKEOVER_TAIL_MS : FX_TAKEOVER_TAIL_MS));
      if (own) expect(shakes(cues)).not.toContain(FX_TAKEOVER_TRAUMA);
      else expect(shakes(cues)).toContain(FX_TAKEOVER_TRAUMA);
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
    expect(takeoverOf(planFx(entry([play("core-099")]), p1, env()))?.look).toBe("crystalForge");
    expect(takeoverOf(planFx(entry([play("meditative-049"), summon("meditative-049")]), p1, env()))?.look).toBe("ai");
    // Stinkyscape #36 Drone Core X, a Mythic with no signature yet, wears the shared look.
    expect(takeoverOf(planFx(entry([play("stinkyscape-036")]), p1, env()))?.look).toBe("prismatic");
    // R3332000: the four marquee Mythics name a look of their own in the effects table; R3400000: and three
    // signatures; R3389000: and Classic's In Too Deep, its signature's; R3402000: and Core's two more;
    // R3405000: and Meditative's Mythics; R3399002: and Classic+'s four, in table order.
    const own = [...FX_CARD_TABLE].flatMap(([defId, card]) => (card.kind !== "none" && card.takeover !== undefined ? [defId] : []));
    const meditative = ["008", "017", "020", "039-4", "039-5", "049", "049-1", "049-2", "049-3", "071", "071-1", "097", "097-9"].map((n) => `meditative-${n}`);
    // R3395001: and Stinkyscape's five Mythics; R3407000: and Stinkyscape part two's six.
    expect(own).toEqual([
    "core-096",
    "core-097",
    "core-098",
    "core-099",
    "core-100",
    "classic-090",
    "classicplus-027",
    "classicplus-029",
    "classicplus-065-5",
    "classicplus-074",
    ...meditative,
    "meditativeplus-001",
    "meditativeplus-026",
    "stinkyscape-006",
    "stinkyscape-016",
    "stinkyscape-044",
    "stinkyscape-074",
    "stinkyscape-086",
    "stinkyscape-109",
    "stinkyscape-110",
    "stinkyscape-111",
    "stinkyscape-148",
    "stinkyscape-148-1",
    "stinkyscape-t-red-narcor",
    "traininggrounds-040",
  ]);
    expect(own.map((defId) => takeoverLookOf(defId))).toEqual([
      "pawn",
      "genie",
      "rotunda",
      "crystalForge",
      "void",
      "abyss",
      "zealot",
      "portal",
      "vineyard",
      "stair",
      "ascent",
      "workshop",
      "brush",
      "mine",
      "quarry",
      "ai",
      "pavilion",
      "virus",
      "dojo",
      "studyHall",
      "caneRack",
      "warRoom",
      "fortress",
      "summit",
      "canvas",
      "vault",
      "rod",
      "wire",
      "whiteDwarf",
      "forge",
      "freshJack",
      "ikwzki",
      "shockwave",
      "alteran",
      "edge",
      "redNarcor",
      "napkin",
    ]);
    for (const defId of own) expect(CATALOG[defId]?.printedRarity ?? CATALOG[defId]?.rarity, defId).toBe("Mythic");
    for (const events of [[play("core-052"), summon("core-052")], [play("core-083")], [fired("core-085")], [play("classic-044")], [fired("classic-009")]]) {
      const e = entry(events);
      expect(e.entrance?.tier).toBe("legendary");
      expect(takeoverOf(planFx(e, p1, env())), JSON.stringify(events)).toBeUndefined();
    }
    // A Legendary Spell's cast, which no summon draws, takes its rays, gold burst, ring with its
    // echo (R2803000: the two-wave landing, one volume down) and its shake.
    // Stinkyscape #137.1 Land Doctrine, a token Spell printed Legendary with no signature yet.
    const legend = entranceCues(entry([play("stinkyscape-137-1")]), env(), 400);
    expect(legend.map((c) => c.kind).sort()).toEqual(["burst", "rays", "ring", "ring", "shake"]);
    const rings = legend.filter((c) => c.kind === "ring");
    expect(rings.length).toBe(2);
    expect(rings[1]?.delayMs ?? 0).toBeGreaterThan(rings[0]?.delayMs ?? 0);
  });

  it("R3332000 takeoverLookOf reads the table, prismatic by default", () => {
    expect(takeoverLookOf("core-100")).toBe("void");
    expect(takeoverLookOf("stinkyscape-036")).toBe("prismatic");
    expect(takeoverLookOf("core-052")).toBe("prismatic");
    expect(takeoverLookOf("not-a-card")).toBe("prismatic");
    // A table that gives a Mythic another look: its takeover wears it.
    const table = parseFxCardTable("test.json5", `{ "stinkyscape-036": { cast: { motif: "vanish" }, takeover: "genie" }, "core-100": "none: pending" }`);
    expect(takeoverLookOf("stinkyscape-036", table)).toBe("genie");
    expect(takeoverLookOf("core-100", table)).toBe("prismatic");
    const e = entry([play("stinkyscape-036")]);
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
      expect(TAKEOVER_ART_DRAWINGS[look], look).toBeDefined();
    }
  });

  it("R3399002 R3402000 every Classic+ own look has its drawing, which names no card, and no other look but Core's four scenes has one", () => {
    // R3402000: Core's four Mythics' looks draw a scene of shapes in the same layer (takeoverArt.test.ts holds them).
    expect(Object.keys(TAKEOVER_ART_DRAWINGS).sort()).toEqual(["crystalForge", "genie", "pawn", "portal", "rotunda", "stair", "vineyard", "zealot"]);
    // A drawn look draws no parts beside it (R3400000's), so its art layer holds the drawing alone.
    for (const look of Object.keys(TAKEOVER_ART_DRAWINGS) as TakeoverLook[]) expect(takeoverArtParts(look), look).toEqual([]);
    /** What each concept moves: the zealot's cyclone, the portal's vortex and its ruins, the stair, the grape and its vines. */
    const moving: Record<string, string[]> = { zealot: ["spin"], portal: ["spin", "drift"], stair: ["spin"], vineyard: ["spin", "drift"] };
    for (const look of ["zealot", "portal", "stair", "vineyard"] as const) {
      const art = TAKEOVER_ART_DRAWINGS[look];
      if (art === undefined || !("paths" in art)) throw new Error(`${look} is drawn as paths`);
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
    expect(JSON.stringify(TAKEOVER_ART_DRAWINGS)).not.toContain("classicplus-");
  });

  it("R3400000 a signature Spell draws no shared rays, burst or ring, its takeover ending FX_SIGNATURE_TAKEOVER_TAIL_MS after its entry", () => {
    expect(ownCast("traininggrounds-040", "Spell")).toBe(true);
    expect(ownCast("traininggrounds-037", "Spell")).toBe(true);
    // A Unit's signature is its entrance, and a card with no signature has no cast of its own.
    expect(ownCast("traininggrounds-038", "Unit")).toBe(false);
    expect(ownCast("stinkyscape-036", "Spell")).toBe(false);
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

  it("R3402000 rotunda and crystalForge have their own tints, and pawn turns indigo", () => {
    const looks = ["pawn", "rotunda", "crystalForge"] as const;
    const tints = looks.map((look) => JSON.stringify(TAKEOVER_TINTS[look]));
    expect(new Set([...tints, JSON.stringify(TAKEOVER_TINTS.prismatic)]).size).toBe(looks.length + 1);
    expect(TAKEOVER_TINTS.pawn.glow, "My Pawn's indigo").toBe("#6366f1");
    expect(TAKEOVER_TINTS.rotunda.rim, "Heroic Power's white marble").toBe("#f8fafc");
    expect(TAKEOVER_TINTS.crystalForge.rim, "Craft a Card's blue crystal").toBe("#60a5fa");
    expect([takeoverLookOf("core-096"), takeoverLookOf("core-098"), takeoverLookOf("core-099")]).toEqual([...looks]);
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

  it("R3395001 vault, rod, wire, whiteDwarf and forge have their own tints and art", () => {
    const looks = { "stinkyscape-006": "vault", "stinkyscape-016": "rod", "stinkyscape-044": "wire", "stinkyscape-074": "whiteDwarf", "stinkyscape-086": "forge" } as const;
    const tints = Object.values(looks).map((look) => JSON.stringify(TAKEOVER_TINTS[look]));
    const others = (["prismatic", "void", "pawn", "genie", "ai", "summit", "canvas", "napkin", "abyss"] as const).map((look) => JSON.stringify(TAKEOVER_TINTS[look]));
    expect(new Set([...tints, ...others]).size).toBe(tints.length + others.length);
    expect(TAKEOVER_TINTS.vault.rim, "Tylannt's steel").toBe("#94a3b8");
    expect(TAKEOVER_TINTS.rod.rim, "Rod of Ages' silver").toBe("#cbd5e1");
    expect(TAKEOVER_TINTS.wire.rim, "the paperclip's wire").toBe("#d1d5db");
    expect(TAKEOVER_TINTS.whiteDwarf.rim, "Paragon-X's white dwarf").toBe("#f8fafc");
    expect(TAKEOVER_TINTS.forge.rim, "Prototype-X2's molten forge").toBe("#f97316");
    for (const [defId, look] of Object.entries(looks)) {
      expect(takeoverLookOf(defId), defId).toBe(look);
      // Each draws art of its own over the dark, parts no other look shares.
      expect(takeoverArtParts(look).length, look).toBeGreaterThan(0);
    }
    const parts = Object.values(looks).flatMap((look) => takeoverArtParts(look));
    expect(new Set(parts).size).toBe(parts.length);
    // The Mythic Spell's own cast keeps its takeover inside its ceiling: its own look alone, the shorter tail.
    const paperclip = entry([play("stinkyscape-044")]);
    const cues = entranceCues(paperclip, env(), paperclip.durationMs);
    expect(cues.map((cue) => cue.kind)).toEqual(["takeover"]);
    expect(takeoverOf(cues)).toMatchObject({ look: "wire", durationMs: paperclip.durationMs + FX_SIGNATURE_TAKEOVER_TAIL_MS });
  });

  it("R3407000 Stinkyscape part two's six Mythics take their own looks and tints, each drawing its card's own picture; no other look draws one", () => {
    const looks = {
      "stinkyscape-109": "freshJack",
      "stinkyscape-110": "ikwzki",
      "stinkyscape-111": "shockwave",
      "stinkyscape-148": "alteran",
      "stinkyscape-148-1": "edge",
      "stinkyscape-t-red-narcor": "redNarcor",
    } as const;
    expect([...TAKEOVER_ART_LOOKS].sort()).toEqual(Object.values(looks).sort());
    const tints = new Set(Object.values(TAKEOVER_TINTS).map((tint) => JSON.stringify(tint)));
    expect(tints.size, "every look its own tint").toBe(Object.keys(TAKEOVER_TINTS).length);
    for (const [defId, look] of Object.entries(looks)) {
      expect(takeoverLookOf(defId), defId).toBe(look);
      const art = takeoverArtOf(defId);
      expect(art, defId).toMatch(/^data:image\/svg\+xml/);
      const events = CARDS[defId]?.type === "Unit" ? [play(defId), summon(defId)] : [play(defId)];
      const e = entry(events);
      expect(takeoverOf(entranceCues(e, env(), e.durationMs)), defId).toMatchObject({ look, tint: TAKEOVER_TINTS[look], art });
    }
    // The Mythics' pictures are each their own.
    expect(new Set(Object.keys(looks).map((defId) => takeoverArtOf(defId))).size).toBe(Object.keys(looks).length);
    // Every older look draws no picture, the hidden sentinel none (R202), and a table without the look none.
    for (const defId of ["core-100", "core-097", "meditative-049", "meditative-008", "traininggrounds-040", "hidden", "not-a-card"]) {
      expect(takeoverArtOf(defId), defId).toBeUndefined();
    }
    const e = entry([play("core-097")]);
    expect(takeoverOf(entranceCues(e, env(), e.durationMs))).not.toHaveProperty("art");
    const plain = parseFxCardTable("test.json5", `{ "stinkyscape-109": { bespoke: "freshJackComet", entrance: "freshJackVault" } }`);
    expect(takeoverArtOf("stinkyscape-109", plain)).toBeUndefined();
  });

  it("R3407000 a Mythic Field Spell landing with its own entrance sweeps its takeover into the zone it lands in, not the hand card it left", () => {
    const reach = "stinkyscape-148";
    const landed: GameEvent = { type: "summoned", player: "p1", instanceId: `i-${reach}`, defId: reach, row: "backrow", lane: 3 };
    const e = entry([play(reach), landed]);
    expect(takeoverOf(entranceCues(e, env(), e.durationMs))).toMatchObject({ look: "alteran", at: { kind: "testid", testid: testid.zone("you", "backrow", 3) } });
  });

  it("R3405000 every Meditative Mythic wears its own look with art and tints of its own", () => {
    const mythics = [...FX_CARD_TABLE].flatMap(([defId, card]) =>
      defId.startsWith("meditative-") && (CATALOG[defId]?.printedRarity ?? CATALOG[defId]?.rarity) === "Mythic" ? [[defId, card] as const] : [],
    );
    expect(mythics).toHaveLength(13);
    const looks = mythics.map(([defId, card]) => {
      const look = card.kind === "none" ? undefined : card.takeover;
      if (look === undefined) throw new Error(`${defId} has no look of its own`);
      expect(takeoverLookOf(defId), defId).toBe(look);
      expect(takeoverArtParts(look).length, `${defId}'s ${look} art`).toBeGreaterThanOrEqual(3);
      return look;
    });
    // One look a card, each with tints no other look wears.
    expect(new Set(looks).size).toBe(looks.length);
    const tints = new Set(Object.values(TAKEOVER_TINTS).map((tint) => JSON.stringify(tint)));
    expect(tints.size).toBe(Object.keys(TAKEOVER_TINTS).length);
  });

  it("R2722001 intensity 0 plans none", () => {
  const e = entry([play("core-097")]);
  expect(entranceCues(e, env(FX_INTENSITY_SCALE.off), e.durationMs)).toEqual([]);
  expect(planFx(e, p1, env(FX_INTENSITY_SCALE.off))).toEqual([]);
});
});

describe("R3402001 Core's signature casts", () => {
  it("R3402001 Core's signature Spells and Traps draw their own first beat, a Mythic's own takeover alone", () => {
    const shared = (cues: readonly FxCue[]): string[] => cues.flatMap((c) => (c.kind === "rays" || c.kind === "burst" || c.kind === "ring" ? [c.kind] : []));
    // Unlicensed Experimentation's firing and Transmogulate's cast draw their own first beat (fx/cards/core/), no shared landing.
    for (const events of [[fired("core-085")], [play("core-083")]]) {
      const e = entry(events);
      expect(e.entrance?.tier, JSON.stringify(events)).toBe("legendary");
      expect(entranceCues(e, env(), e.durationMs), JSON.stringify(events)).toEqual([]);
    }
    // A Legendary Trap that is no signature (Income Tax before its own, PLAIN_TRAP) still lands in the shared rays, burst and rings.
    const income = entry([fired("classic-009")]);
    expect(shared(entranceCues(income, env(), income.durationMs, PLAIN_TRAP)).sort()).toEqual(["burst", "rays", "ring", "ring"]);
    expect(shakes(entranceCues(income, env(), income.durationMs, PLAIN_TRAP))).toContain(FX_LEGENDARY_TRAUMA);
    // Zephrys's and Craft a Card's casts and My Pawn's firing keep only their own look's takeover (R3400000), centred, the sooner gone.
    for (const [events, look] of [[[play("core-097")], "genie"], [[play("core-099")], "crystalForge"], [[fired("core-096")], "pawn"]] as const) {
      const e = entry([...events]);
      const cues = entranceCues(e, env(), e.durationMs);
      expect(cues.map((c) => c.kind), look).toEqual(["takeover"]);
      expect(takeoverOf(cues), look).toMatchObject({ look, durationMs: e.durationMs + FX_SIGNATURE_TAKEOVER_TAIL_MS });
      expect(takeoverOf(cues)?.at, look).toEqual({ kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } });
    }
  });
});

describe("R2722002 a hidden card's entrance centres on the zone that flipped", () => {
  it("R2722002 a Legendary or Mythic reveal draws its rays at the zone that flipped; a firing again none", () => {
    // A Legendary Trap with no cast of its own (PLAIN_TRAP: every Legendary Trap in the catalog has one now).
    const plain = entry([fired("classic-009")]);
    const rays = entranceCues(plain, env(), plain.durationMs, PLAIN_TRAP).find((c) => c.kind === "rays");
    expect(rays).toMatchObject({ tone: "legendary", at: { kind: "testid", testid: "zone-you-backrow-3" } });
    // The other seat reads a Cloaked card's reveal at its firing too (R2734000), centred where it flips.
    // A Mythic Spell with no cast of its own (Drone Core X: Core's and Classic+'s Mythic Spells have their signatures now).
    const reveal: GameEvent = { type: "activated", player: "p1", instanceId: "c61", defId: "stinkyscape-036", ability: "reveal" };
    const entered = entranceEntries(planEntries([reveal, fired("stinkyscape-036")], p2, false), resolve, { seen: new Set() });
    const theirs = setPieceEntries(entered, takeoverClaim, { reduced: false, bigMoments: "full", speed: 1 });
    const at = theirs.find((e) => e.entrance !== undefined);
    if (at === undefined) throw new Error("no entry");
    expect(at.events.some((e) => e.type === "trapFired")).toBe(true);
    expect(takeoverOf(planFx(at, p2, env()))?.at).toEqual({ kind: "testid", testid: "zone-opponent-backrow-3" });
    // The sentinel's firing, and a Field Trap firing again, make no entrance.
    expect(planFx(entry([fired("hidden")], p2), p2, env()).some((c) => c.kind === "rays" || c.kind === "takeover")).toBe(false);
    expect(planFx(entry([fired("core-096", true)]), p1, env()).some((c) => c.kind === "rays" || c.kind === "takeover")).toBe(false);
  });

  it("R3389000 In Too Deep's takeover wears abyss; a Spell's own cast drops the shared rays", () => {
    // In Too Deep lands face up in the backrow under a takeover of its own, its tint its look's.
    const deep = entry([play("classic-090"), { type: "summoned", player: "p1", instanceId: "i-classic-090", defId: "classic-090", row: "backrow", lane: 2 }]);
    const cues = planFx(deep, p1, env());
    const takeover = takeoverOf(cues);
    expect(takeover?.look).toBe("abyss");
    // It centres on the backrow zone it lands in, as a Unit's on its unit zone.
    expect(deep.entrance?.anchor).toBe("zone-you-backrow-2");
    expect(takeover?.at).toEqual({ kind: "testid", testid: "zone-you-backrow-2" });
    expect(takeover?.tint).toEqual(TAKEOVER_TINTS.abyss);
    expect(takeoverLookOf("classic-090")).toBe("abyss");
    // Its own entrance lands with the takeover, after the hold, at the same moment: its shake at the takeover's landing.
    const landMs = entranceLandMs(deep, deep.durationMs);
    expect(takeover?.landMs).toBe(landMs);
    expect(landMs).toBeGreaterThan(deep.setPiece?.holdMs ?? 0);
    expect(cues.some((cue) => cue.kind === "shake" && cue.delayMs === landMs && cue.trauma === 0.5)).toBe(true);
    // Its art: a spiral shaft, rushing water and shrine lights, each its own rule.
    for (const part of ["dark::after", "sweep", "rays"]) expect(fxCss, part).toContain(`.fx-takeover[data-look="abyss"] .fx-takeover-${part}`);
    expect(fxCss).toContain("@keyframes fx-takeover-plunge");

    // A Spell's and a Trap's own cast: no shared rays, gold burst or ring; a Legendary with none keeps them.
    expect(ownCast("classic-044", "Spell")).toBe(true);
    expect(ownCast("classic-009", "Trap")).toBe(true);
    expect(ownCast("classic-090", "Field Spell")).toBe(false);
    expect(ownCast("stinkyscape-137-1", "Spell")).toBe(false);
    expect(entranceCues(entry([play("classic-044")]), env(), 400)).toEqual([]);
    expect(entranceCues(entry([fired("classic-009")]), env(), 400)).toEqual([]);
    expect(entranceCues(entry([play("stinkyscape-137-1")]), env(), 400).some((cue) => cue.kind === "rays")).toBe(true);
    expect(entranceCues(entry([fired("classic-009")]), env(), 400, PLAIN_TRAP).some((cue) => cue.kind === "rays")).toBe(true);
    // A table that gives the Spell no signature: it takes the shared entrance again.
    const table = parseFxCardTable("test.json5", `{ "classic-044": { bespoke: "graveRise" } }`);
    expect(ownCast("classic-044", "Spell", table)).toBe(false);
    expect(entranceCues(entry([play("classic-044")]), env(), 400, table).some((cue) => cue.kind === "rays")).toBe(true);
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
      // R202: the cue is plain data holding the card's picture and name, never its definition, so a second plan equals it.
      expect(takeover?.stage?.planes.length, mythic.id).toBe(ART_PLANES.length);
      expect(takeover?.stage?.name, mythic.id).toBe(mythic.name);
      expect(JSON.stringify(takeover?.stage).includes(mythic.id), mythic.id).toBe(false);
      expect(takeoverOf(planFx(found, p1, env())), mythic.id).toEqual(takeover);

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
