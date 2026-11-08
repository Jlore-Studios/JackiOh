// R3342000: tests for tile aura planner and pure color/mote math (issue #3342).

import { describe, expect, it } from "vitest";

import type { CardDef, PlayerView } from "@jackioh/shared";

import { artSpec } from "../../cards/art/procedural.ts";
import { FAMILY_PAINTS as STINKY_PAINTS, stinkySubject } from "../../cards/art/stinkyscape.ts";
import { compositionFor, themeFor, turnHue } from "../../cards/art/themes.ts";
import { FAMILY_PAINTS as YARD_PAINTS, yardSubject } from "../../cards/art/traininggrounds.ts";
import { ANIMATIONS } from "../../game/animations.ts";
import { BACKDROP_PARTICLE_MAX } from "../../game/config.ts";
import { testid } from "../../game/contract.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { FX_CARD_TABLE, entryAura, parseFxCardTable } from "../cards/tables.ts";
import { FX_PARTICLE_CAP_MOBILE } from "../constants.ts";
import {
  TILE_FADE_OUT_MS,
  TILE_HUE_SPREAD_DEG,
  TILE_MOTES_MYTHIC,
  TILE_PARTICLE_SHARE,
} from "./config.ts";
import {
  planTileAura,
  tierOf,
  tileHueShift,
  tileMoteBudget,
  tileMotes,
  tileResponse,
  TILE_PATTERN_OF_FAMILY,
  type TileAuraSpec,
  type TilePattern,
} from "./tile.ts";

describe("R3342000 tile aura planner", () => {
  it("R3342000 every eligible Legendary and Mythic permanent plans an aura", () => {
    let covered = 0;
    for (const [id, def] of Object.entries(CATALOG)) {
      const tier = tierOf(def);
      if (tier === null) continue;
      if (def.type !== "Unit" && def.type !== "Field Spell" && def.type !== "Field Trap" && def.type !== "Trap") continue;
      const tableEntry = FX_CARD_TABLE.get(id);
      const isUnit = def.type === "Unit";
      const view = baseView({
        you: emptySide("p1", {
          units: isUnit ? [unit("p1", { defId: id, instanceId: "u1" })] : [null],
          backrow: isUnit ? [null] : [faceUpBackrow("p1", { defId: id, instanceId: "b1", type: def.type })],
        }),
      });

      const planned = planTileAura(view, "you", isUnit ? "units" : "backrow", 1);
      // A card whose table entry wears its own aura (R3400000: a bespoke entry's too) wears that in its place.
      if (entryAura(tableEntry) !== undefined) {
        expect(planned, id).toBeNull();
        covered += 1;
        continue;
      }
      expect(planned).not.toBeNull();
      expect(planned?.tier).toBe(tier);
      if (tier === "Mythic") {
        expect(planned?.pattern).not.toBeNull();
      } else {
        expect(planned?.pattern).toBeNull();
      }
      covered += 1;
    }
    expect(covered).toBeGreaterThanOrEqual(75);
  });

  it("R3342000 no aura for Common, face-down, unrevealed, or hidden cards; token with Legendary frame gets one", () => {
    // Common unit
    const commonView = baseView({
      you: emptySide("p1", { units: [unit("p1", { defId: "core-004", instanceId: "u1" })] }),
    });
    expect(planTileAura(commonView, "you", "units", 1)).toBeNull();

    // Face-down backrow
    const faceDownView = baseView({
      you: emptySide("p1", {
        backrow: [{ faceDown: true, cost: 2 }],
      }),
    });
    expect(planTileAura(faceDownView, "you", "backrow", 1)).toBeNull();

    // Unrevealed backrow
    const unrevealedView = baseView({
      you: emptySide("p1", {
        backrow: [faceUpBackrow("p1", { defId: "core-098", unrevealed: true })],
      }),
    });
    expect(planTileAura(unrevealedView, "you", "backrow", 1)).toBeNull();

    // Hidden defId
    const hiddenView = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { defId: "hidden", instanceId: "u1" })],
      }),
    });
    expect(planTileAura(hiddenView, "you", "units", 1)).toBeNull();

    // Token with printedRarity Legendary
    const tokenDef: CardDef = {
      index: "token-legendary",
      id: "token-legendary",
      name: "Legendary Token",
      set: "Core",
      type: "Unit",
      rarity: "Common",
      printedRarity: "Legendary",
      flavor: "Physical",
      tags: [],
      token: true,
      cost: 1,
      base: { attack: 1, health: 1, keywords: [], text: "" },
      radiant: { attack: 1, health: 1, keywords: [], text: "" },
    };
    const tokenView = baseView({
      defs: { "token-legendary": tokenDef },
      you: emptySide("p1", {
        units: [unit("p1", { defId: "token-legendary", instanceId: "u1" })],
      }),
    });
    const tokenAura = planTileAura(tokenView, "you", "units", 1);
    expect(tokenAura).not.toBeNull();
    expect(tokenAura?.tier).toBe("Legendary");
  });

  it("R3342000 colours come from the card's own picture", () => {
    // Stinkyscape card: e.g. stinkyscape-004
    const stinkyId = "stinkyscape-004";
    const stinkyDef = CATALOG[stinkyId];
    expect(stinkyDef).toBeDefined();
    if (stinkyDef) {
      const shift = tileHueShift(stinkyId);
      expect(Math.abs(shift)).toBeLessThanOrEqual(TILE_HUE_SPREAD_DEG);
      const subject = stinkySubject(stinkyId, stinkyDef.name, stinkyDef.tags, stinkyDef.type);
      const paint = STINKY_PAINTS[subject.family];
      const view = baseView({
        you: emptySide("p1", {
          backrow: [faceUpBackrow("p1", { defId: stinkyId, instanceId: "b1", type: stinkyDef.type })],
        }),
      });
      const aura = planTileAura(view, "you", "backrow", 1);
      expect(aura).not.toBeNull();
      expect(aura?.colors[0]).toBe(turnHue(paint.light, shift));
      expect(aura?.colors[1]).toBe(turnHue(paint.nebula[0], shift));
      expect(aura?.colors[2]).toBe(turnHue(paint.glow, shift));
    }

    // Training Grounds card: e.g. traininggrounds-040
    const tgId = "traininggrounds-040";
    const tgDef = CATALOG[tgId];
    expect(tgDef).toBeDefined();
    if (tgDef) {
      const shift = tileHueShift(tgId);
      expect(Math.abs(shift)).toBeLessThanOrEqual(TILE_HUE_SPREAD_DEG);
      const subject = yardSubject(tgId, tgDef.name, tgDef.tags, tgDef.type);
      const paint = YARD_PAINTS[subject.family];
      const view = baseView({
        you: emptySide("p1", {
          backrow: [faceUpBackrow("p1", { defId: tgId, instanceId: "b1", type: tgDef.type })],
        }),
      });
      const aura = planTileAura(view, "you", "backrow", 1);
      expect(aura).not.toBeNull();
      expect(aura?.colors[0]).toBe(turnHue(paint.light, shift));
      expect(aura?.colors[1]).toBe(turnHue(paint.body, shift));
      expect(aura?.colors[2]).toBe(turnHue(paint.glow, shift));
    }

    // Procedural card: e.g. core-100
    const procId = "core-100";
    const procDef = CATALOG[procId];
    expect(procDef).toBeDefined();
    if (procDef) {
      const shift = tileHueShift(procId);
      expect(Math.abs(shift)).toBeLessThanOrEqual(TILE_HUE_SPREAD_DEG);
      const theme = themeFor(procDef.tags, procDef.type);
      const spec = artSpec(procId, theme, compositionFor(procDef.type), false, null);
      const view = baseView({
        you: emptySide("p1", {
          units: [unit("p1", { defId: procId, instanceId: "u1" })],
        }),
      });
      const aura = planTileAura(view, "you", "units", 1);
      expect(aura).not.toBeNull();
      expect(aura?.colors[2]).toBe(turnHue(spec.glow.color, shift));
    }

    // Hue shift within bounds for all cards
    for (const id of Object.keys(CATALOG)) {
      expect(Math.abs(tileHueShift(id))).toBeLessThanOrEqual(TILE_HUE_SPREAD_DEG);
    }
  });

  it("R3342000 two different Legendaries' auras differ and fingerprints are unique within each set", () => {
    // Two named cards differ in colors: core-052 vs core-092
    const def1 = CATALOG["core-052"];
    const def2 = CATALOG["core-092"];
    expect(def1).toBeDefined();
    expect(def2).toBeDefined();
    const view = baseView({
      you: emptySide("p1", {
        units: [
          unit("p1", { defId: "core-052", instanceId: "u1" }),
          unit("p1", { defId: "core-092", instanceId: "u2" }),
        ],
      }),
    });
    // R3402000: Silly Silas wears a signature aura in the table now, so the default is planned without one.
    const aura1 = planTileAura(view, "you", "units", 1, new Map());
    const aura2 = planTileAura(view, "you", "units", 2, new Map());
    expect(aura1).not.toBeNull();
    expect(aura1?.colors).not.toEqual(aura2?.colors);

    // Fingerprint unique within each set
    const bySet = new Map<string, Set<string>>();
    for (const [id, def] of Object.entries(CATALOG)) {
      const tier = tierOf(def);
      if (tier === null) continue;
      const set = def.set ?? "none";
      if (!bySet.has(set)) bySet.set(set, new Set());
      const setFingerprints = bySet.get(set)!;

      const isUnit = def.type === "Unit";
      const testView = baseView({
        you: emptySide("p1", {
          units: isUnit ? [unit("p1", { defId: id, instanceId: "u1" })] : [null],
          backrow: isUnit ? [null] : [faceUpBackrow("p1", { defId: id, instanceId: "b1", type: def.type })],
        }),
      });
      const planned = planTileAura(testView, "you", isUnit ? "units" : "backrow", 1, new Map());
      if (planned === null) continue;

      const fp = `${planned.colors.join(",")}|${planned.variant}|${planned.pattern}|${planned.look}|${planned.moteShape}`;
      expect(setFingerprints.has(fp), `Duplicate fingerprint ${fp} for ${id} in set ${set}`).toBe(false);
      setFingerprints.add(fp);
    }
  });

  it("R3342000 every Mythic's pattern equals TILE_PATTERN_OF_FAMILY for its set", () => {
    for (const [id, def] of Object.entries(CATALOG)) {
      if (tierOf(def) !== "Mythic") continue;
      const isUnit = def.type === "Unit";
      const view = baseView({
        you: emptySide("p1", {
          units: isUnit ? [unit("p1", { defId: id, instanceId: "u1" })] : [null],
          backrow: isUnit ? [null] : [faceUpBackrow("p1", { defId: id, instanceId: "b1", type: def.type })],
        }),
      });
      const aura = planTileAura(view, "you", isUnit ? "units" : "backrow", 1, new Map());
      expect(aura).not.toBeNull();
      const expectedPattern: TilePattern =
        def.set === "Core"
          ? TILE_PATTERN_OF_FAMILY.core
          : def.set === "Classic" || def.set === "Classic+"
            ? TILE_PATTERN_OF_FAMILY.classic
            : def.set === "Meditative" || def.set === "Meditative+"
              ? TILE_PATTERN_OF_FAMILY.meditative
              : def.set === "Stinkyscape"
                ? TILE_PATTERN_OF_FAMILY.stinkyscape
                : TILE_PATTERN_OF_FAMILY["training-grounds"];
      expect(aura?.pattern).toBe(expectedPattern);
    }
  });

  it("R3342000 a signature aura replaces the default", () => {
    // Feng Shui (meditative-040)
    const fsView = baseView({
      you: emptySide("p1", {
        backrow: [faceUpBackrow("p1", { defId: "meditative-040", instanceId: "b1", type: "Field Spell" })],
      }),
    });
    expect(planTileAura(fsView, "you", "backrow", 1)).toBeNull();

    // Silly Silas (core-052)
    const silasView = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { defId: "core-052", instanceId: "u1" })],
      }),
    });
    // With empty table: plans non-null
    expect(planTileAura(silasView, "you", "units", 1, new Map())).not.toBeNull();

    // With signature table: plans null
    const signatureTable = parseFxCardTable("test.json5", '{ "core-052": { aura: { look: "void", scope: "side" } } }');
    expect(planTileAura(silasView, "you", "units", 1, signatureTable)).toBeNull();
  });

  it("R3400000 a signature Unit's own tile aura, on a hooks or a bespoke entry, replaces the default", () => {
    for (const defId of ["meditativeplus-024", "traininggrounds-038", "traininggrounds-039-1"]) {
      const view = baseView({ you: emptySide("p1", { units: [unit("p1", { defId, instanceId: "u1" })] }) });
      expect(planTileAura(view, "you", "units", 1), defId).toBeNull();
      expect(planTileAura(view, "you", "units", 1, new Map()), defId).not.toBeNull();
    }
    const bespoke = parseFxCardTable("test.json5", '{ "meditativeplus-024": { bespoke: "bobSoldier", aura: { look: "flurry", scope: "tile" } } }');
    const bob = baseView({ you: emptySide("p1", { units: [unit("p1", { defId: "meditativeplus-024", instanceId: "u1" })] }) });
    expect(planTileAura(bob, "you", "units", 1, bespoke)).toBeNull();
  });

  it("R3342000 dim for slumber, frozen, frozenForever, and vanilla statuses", () => {
    const makeUnitView = (statuses: Partial<Parameters<typeof unit>[1]>): PlayerView =>
      baseView({
        you: emptySide("p1", {
          units: [unit("p1", { defId: "core-052", instanceId: "u1", ...statuses })],
        }),
      });

    // R3402000: Silly Silas's signature aura replaces his default in the table, so these plan without one.
    const none = new Map();
    expect(planTileAura(makeUnitView({ slumber: {} }), "you", "units", 1, none)?.dim).toBe(true);
    expect(planTileAura(makeUnitView({ frozen: true }), "you", "units", 1, none)?.dim).toBe(true);
    expect(planTileAura(makeUnitView({ frozenForever: true }), "you", "units", 1, none)?.dim).toBe(true);
    expect(planTileAura(makeUnitView({ vanilla: true }), "you", "units", 1, none)?.dim).toBe(true);
    expect(planTileAura(makeUnitView({}), "you", "units", 1, none)?.dim).toBe(false);

    // Backrow vanilla
    const backrowVanilla = baseView({
      you: emptySide("p1", {
        backrow: [faceUpBackrow("p1", { defId: "core-085", instanceId: "b1", type: "Trap", vanilla: true })],
      }),
    });
    expect(planTileAura(backrowVanilla, "you", "backrow", 1)?.dim).toBe(true);
  });

  it("R3342000 tileResponse covers hit, fading and lit; fade-out matches destroyed/exiled durations", () => {
    const spec: TileAuraSpec = {
      key: "tile-you-units-1-u1",
      defId: "core-052",
      instanceId: "u1",
      side: "you",
      row: "units",
      lane: 1,
      tier: "Legendary",
      colors: ["#ffffff", "#888888", "#000000"],
      look: "arcane",
      moteShape: "dot",
      pattern: null,
      variant: 0,
      dim: false,
      hue: 0,
    };

    const cardId = testid.card("u1");
    const zoneId = testid.zone("you", "units", 1);

    // Fading
    expect(tileResponse(spec, new Map([[cardId, "destroyed"]]))).toEqual({ fading: true, hit: false, lit: false });
    expect(tileResponse(spec, new Map([[cardId, "exiled"]]))).toEqual({ fading: true, hit: false, lit: false });

    // Hit
    expect(tileResponse(spec, new Map([[cardId, "damage"]]))).toEqual({ fading: false, hit: true, lit: false });
    expect(tileResponse(spec, new Map([[cardId, "divineShieldLost"]]))).toEqual({ fading: false, hit: true, lit: false });

    // Lit via card testid or zone testid (string or Set)
    expect(tileResponse(spec, new Map(), { legal: new Set(), selected: new Set([cardId]) })).toEqual({ fading: false, hit: false, lit: true });
    expect(tileResponse(spec, new Map(), { legal: new Set(), selected: new Set([zoneId]) })).toEqual({ fading: false, hit: false, lit: true });
    expect(tileResponse(spec, new Map(), { selected: cardId })).toEqual({ fading: false, hit: false, lit: true });
    expect(tileResponse(spec, new Map(), { selected: zoneId })).toEqual({ fading: false, hit: false, lit: true });
    expect(tileResponse(spec, new Map(), { selected: "other" })).toEqual({ fading: false, hit: false, lit: false });

    // Duration match
    expect(TILE_FADE_OUT_MS).toBe(ANIMATIONS.destroyed.durationMs);
    expect(TILE_FADE_OUT_MS).toBe(ANIMATIONS.exiled.durationMs);
  });

  it("R3342000 budgets: 20 Mythic tiles at 390 width respect cap, 0 at intensity 0, motes <= BACKDROP_PARTICLE_MAX", () => {
    const singleBudget = tileMoteBudget("Mythic", 20, 1, 390);
    const totalBudget = 20 * singleBudget;
    expect(totalBudget).toBeLessThanOrEqual(Math.floor(TILE_PARTICLE_SHARE * FX_PARTICLE_CAP_MOBILE));

    expect(tileMoteBudget("Mythic", 20, 0, 390)).toBe(0);
    expect(TILE_MOTES_MYTHIC).toBeLessThanOrEqual(BACKDROP_PARTICLE_MAX);
  });

  it("R3342000 tileMotes is deterministic across calls and differs between cards", () => {
    const spec1: TileAuraSpec = {
      key: "tile-you-units-1-u1",
      defId: "core-052",
      instanceId: "u1",
      side: "you",
      row: "units",
      lane: 1,
      tier: "Legendary",
      colors: ["#ffffff", "#888888", "#000000"],
      look: "arcane",
      moteShape: "dot",
      pattern: null,
      variant: 0,
      dim: false,
      hue: 0,
    };

    const spec2: TileAuraSpec = {
      ...spec1,
      defId: "core-100",
      instanceId: "u2",
    };

    const motes1A = tileMotes(spec1, 3);
    const motes1B = tileMotes(spec1, 3);
    const motes2 = tileMotes(spec2, 3);

    expect(motes1A).toEqual(motes1B);
    expect(motes1A).not.toEqual(motes2);
    expect(motes1A).toHaveLength(3);
  });
});
