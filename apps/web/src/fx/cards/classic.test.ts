// R2794000: Classic's card animations (fx/cards/classic.json5): the set's coverage floor, a `cast` entry on
// every non-Unit and an aura on every Field Spell, its three marquee entrances (R670), and its three
// Flourishes (fx/cards/classic/), each drawn on its play's events inside its entry (R200) and never for a
// play the seat cannot read (R202), planned the way FxLayer plans them: remember, then plan. As a set's
// pass asks (docs/polish/9-card-animations.md, "A set's pass"), every hook names a drawn motif, every play's
// motif draws on its gallery fixture as that card's play (none for a hidden one), and classic.md, the pass's
// per-card table, holds one row per Classic card that matches the catalog and the table. R3389001: its
// eleven Legendaries and Mythics are signatures (classic/signatures.test.ts plays them), nine of them with
// an entrance of their own, King Wagtoggle's crownSwap among its Flourishes still.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries, type AnimationEntry } from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { entranceKeyOf, planCardMotif } from "../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_SET_PASS_FLOOR } from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryEntry, motifFixture, RECIPE_FIXTURES } from "../motifs/gallery.ts";
import { FX_CARD_HOOKS, isDrawnMotif, type DrawnMotif } from "../motifs/names.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";
import { CARD_RECIPES, type CardRecipeKey } from "./bespoke.ts";
import rawTable from "./classic.md?raw";
import { entryAura, FX_CARD_FILES, FX_CARD_TABLE, parseFxCardTable, type FxCardEntry } from "./tables.ts";

/** Every Classic catalog card, its token included, in catalog order. */
const CLASSIC = Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === "Classic");

/** Classic's cards, its one token apart. */
const CLASSIC_CARDS = 90;

/** The Classic table's entries of its cards, the token left out. */
function classicCards(): [string, FxCardEntry][] {
  const file = FX_CARD_FILES.find((candidate) => candidate.file === "classic.json5");
  if (file === undefined) throw new Error("fx/cards/classic.json5 is missing");
  return [...parseFxCardTable(file.file, file.source)].filter(([defId]) => CATALOG[defId]?.token !== true);
}

function entryOf(id: string): FxCardEntry {
  const entry = FX_CARD_TABLE.get(id);
  if (entry === undefined) throw new Error(`${id} has no entry`);
  return entry;
}

/**
 * An entry as classic.md's Entry column writes it: core.md's, a marquee entrance first. R3389001: a
 * signature's recipe as `signature; [entrance <e>; ]bespoke <k>[; aura <look> <scope>[ <palette>]][; takeover <look>]`.
 */
function describeEntry(entry: FxCardEntry): string {
  switch (entry.kind) {
    case "none":
      return `none: ${entry.reason}`;
    case "bespoke": {
      if (entry.signature !== true) return `bespoke ${entry.key}`;
      const parts = ["signature"];
      if (entry.entrance !== undefined) parts.push(`entrance ${entry.entrance}`);
      parts.push(`bespoke ${entry.key}`);
      if (entry.aura !== undefined) parts.push(["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((part) => part !== undefined).join(" "));
      if (entry.takeover !== undefined) parts.push(`takeover ${entry.takeover}`);
      return parts.join("; ");
    }
    case "setPiece":
      return `setPiece ${entry.key}`;
    case "hooks": {
      const parts: string[] = [];
      if (entry.entrance !== undefined) parts.push(`entrance ${entry.entrance}`);
      for (const hook of FX_CARD_HOOKS) {
        const spec = entry.hooks[hook];
        if (spec === undefined) continue;
        parts.push([hook, spec.motif, spec.palette, spec.row === "keep" ? "keep" : undefined].filter((part) => part !== undefined).join(" "));
      }
      if (entry.aura !== undefined) parts.push(["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((part) => part !== undefined).join(" "));
      return parts.join("; ");
    }
  }
}

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planAll(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): { entry: AnimationEntry; cues: FxCue[] }[] {
  const planned = planEntries(events, withEvents(view, [...events]), false);
  return planned.map((entry) => {
    env.memory.remember(entry.events);
    return { entry, cues: planFx(entry, entry.view, env) };
  });
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 1,
});

/** Where a cue ends, in ms after its entry starts (R200's bound is D + FX_MAX_TAIL_MS). */
function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "projectile":
    case "shot":
      return cue.delayMs + cue.flightMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

/** The cues of every entry but the play's own `cardPlayed` and `cardResolved`. */
function fixtureCues(planned: readonly { entry: AnimationEntry; cues: FxCue[] }[]): FxCue[] {
  return planned.filter(({ entry }) => !entry.events.some((event) => event.type === "cardPlayed" || event.type === "cardResolved")).flatMap(({ cues }) => cues);
}

/** The play's motif on its gallery fixture, as the board plans it once the play's `cardPlayed` is remembered. */
function playMotif(defId: string, instanceId: string, motif: DrawnMotif) {
  const view = fullBoardView();
  const env = envOf();
  env.memory.remember([played("p1", instanceId, defId)]);
  const entry = { ...galleryEntry(motifFixture(motif, view, instanceId, "p1"), view), durationMs: 400 };
  return planCardMotif(entry, view, env);
}

/** Each Classic card's play motif: its `cry` (a Unit) or `cast`, when a drawn motif. */
function playMotifs(): { id: string; motif: DrawnMotif }[] {
  return CLASSIC.flatMap((id) => {
    const entry = entryOf(id);
    if (entry.kind !== "hooks") return [];
    const hook = entry.hooks[CATALOG[id]?.type === "Unit" ? "cry" : "cast"];
    return hook !== undefined && isDrawnMotif(hook.motif) ? [{ id, motif: hook.motif }] : [];
  });
}

/** Each Flourish and the Classic card whose entry names it. */
const FLOURISHES: readonly (readonly [CardRecipeKey, string])[] = [
  ["pageMend", "classic-003"],
  ["recycleLoop", "classic-030"],
  ["crownSwap", "classic-085"],
];

/** A Flourish's play by `player`: its `cardPlayed`, its fixture and its `cardResolved`. */
function flourishStream(key: CardRecipeKey, defId: string, view: PlayerView, player: PlayerId, instanceId: string): GameEvent[] {
  return [played(player, instanceId, defId), ...RECIPE_FIXTURES[key](view, instanceId, player), resolved(player, instanceId, defId)];
}

describe("R2794000 Classic's card animations", () => {
  it("R2794000 at least 55% of Classic's 90 cards have an entry other than none, and none is pending", () => {
    const cards = classicCards();
    expect(cards).toHaveLength(CLASSIC_CARDS);
    const drawn = cards.filter(([, entry]) => entry.kind !== "none");
    expect(drawn.length / CLASSIC_CARDS).toBeGreaterThanOrEqual(FX_SET_PASS_FLOOR);
    for (const [defId, entry] of cards) expect(entry.kind === "none" ? entry.reason : "drawn", defId).not.toBe("pending");
  });

  it("R2794000 every Spell, Field Spell, Trap and Field Trap has a cast entry: a cast motif, a Flourish or a set piece", () => {
    const spells = classicCards().filter(([defId]) => CATALOG[defId]?.type !== "Unit");
    expect(spells.length).toBeGreaterThan(0);
    for (const [defId, entry] of spells) {
      const cast = (entry.kind === "hooks" && entry.hooks.cast !== undefined) || (entry.kind === "bespoke" && entry.key in CARD_RECIPES) || entry.kind === "setPiece";
      expect(cast, `${defId} ${JSON.stringify(entry)}`).toBe(true);
    }
  });

  it("R2794000 every Field Spell wears an aura", () => {
    const fields = classicCards().filter(([defId]) => CATALOG[defId]?.type === "Field Spell");
    expect(fields).toHaveLength(16);
    // R3389000: a signature's `{ bespoke }` wears its own beside its recipe.
    for (const [defId, entry] of fields) expect(entryAura(entry), defId).toBeDefined();
  });

  it("R3389001 Classic's nine entrances: the three marquees' keep their recipes, grown, beside six of the signatures' own", () => {
    const marquee = Object.fromEntries(classicCards().flatMap(([defId]) => (entranceKeyOf(defId) === undefined ? [] : [[defId, entranceKeyOf(defId)]])));
    expect(marquee).toEqual({
      "classic-004": "seeingOrb",
      "classic-007": "sceptrePlant",
      "classic-028": "galeBurst",
      "classic-033": "silkDrop",
      "classic-045": "titanBloom",
      "classic-056": "tyrantSigil",
      "classic-080": "bigBoom",
      "classic-085": "royalCloak",
      "classic-090": "abyssSurface",
    });
    // R670's three keep their entrances; Nature Titan's Cry is its own recipe now, its moss its own aura.
    expect(FX_CARD_TABLE.get("classic-045")).toEqual({ kind: "bespoke", key: "titanLeaf", entrance: "titanBloom", aura: { look: "mossCreep", scope: "tile" }, signature: true });
    expect(entryAura(FX_CARD_TABLE.get("classic-045"))).toEqual({ look: "mossCreep", scope: "tile" });
  });

  it("R2794000 every Classic hook names a drawn motif", () => {
    for (const id of CLASSIC) {
      const entry = entryOf(id);
      if (entry.kind !== "hooks") continue;
      for (const [hook, spec] of Object.entries(entry.hooks)) expect(isDrawnMotif(spec.motif), `${id} ${hook} ${spec.motif}`).toBe(true);
    }
  });

  it("R2794000 every Classic cast and cry motif draws on its gallery fixture as its card's play", () => {
    const motifs = playMotifs();
    expect(motifs.length).toBeGreaterThan(0);
    for (const { id, motif } of motifs) {
      const result = playMotif(id, "ex", motif);
      expect(result, `${id} ${motif}`).not.toBeNull();
      expect(result?.cues.length ?? 0, `${id} ${motif}`).toBeGreaterThan(0);
      // R202: no cue names a definition.
      expect(JSON.stringify(result?.cues), id).not.toMatch(/classic-\d{3}/);
    }
  });

  it("R2794000 R202 a hidden play keys no Classic entry", () => {
    for (const { id, motif } of playMotifs()) expect(playMotif("hidden", "hidden", motif), `${id} ${motif}`).toBeNull();
  });

  it("R2794000 classic.md has one row per Classic card, in order, with its name, type, entry, scale, census and why", () => {
    const rows = rawTable
      .split("\n")
      .filter((line) => line.startsWith("| `classic-"))
      .map((line) => line.slice(1, -1).split(" | ").map((cell) => cell.trim()));
    expect(rows.map((row) => row[0])).toEqual(CLASSIC.map((id) => `\`${id}\``));
    const view = fullBoardView();
    for (const [cell, name, type, entry, scale, census, why] of rows) {
      const id = (cell ?? "").replace(/`/g, "");
      const def = CATALOG[id];
      const own = entryOf(id);
      expect(name, id).toBe(def?.name.replace(/\s+/g, " ").trim());
      expect(type, id).toBe(def?.type);
      expect(entry, id).toBe(describeEntry(own));
      expect(scale, id).toBe(own.kind === "none" ? "—" : motifScale(id, view, 1).toFixed(2));
      expect(census, id).toMatch(/\S/);
      expect(why, id).toMatch(/\S/);
    }
  });
});

describe("R2794000 Classic's Flourishes", () => {
  for (const [key, defId] of FLOURISHES) {
    it(`R2794000 ${key} draws on its play's events`, () => {
      expect(FX_CARD_TABLE.get(defId)).toMatchObject({ kind: "bespoke", key });
      const view = fullBoardView();
      const fixture = RECIPE_FIXTURES[key](view, "ex", "p1");
      expect(fixture.length).toBeGreaterThan(0);
      const drawn = fixtureCues(planAll(flourishStream(key, defId, view, "p1", "ex"), view));
      const bare = planAll(fixture, view).flatMap(({ cues }) => cues);
      expect(drawn.length).toBeGreaterThan(bare.length);
    });

    it(`R200 ${key}'s every cue starts inside its entry and ends within FX_MAX_TAIL_MS of its end`, () => {
      for (const D of [200, 400, 1200]) {
        const view = fullBoardView();
        const env = envOf();
        const cues: FxCue[] = [];
        for (const entry of planEntries(flourishStream(key, defId, view, "p1", "ex"), withEvents(view, flourishStream(key, defId, view, "p1", "ex")), false)) {
          env.memory.remember(entry.events);
          const play = env.memory.resolving();
          if (play === undefined) continue;
          for (const event of entry.events) cues.push(...(CARD_RECIPES[key](event, { entry: { ...entry, durationMs: D }, view: entry.view, env, D }, play)?.cues ?? []));
        }
        expect(cues.length, `D=${String(D)}`).toBeGreaterThan(0);
        for (const cue of cues) {
          const label = `${key} ${cue.kind} at D=${String(D)}`;
          expect(cue.delayMs, label).toBeGreaterThanOrEqual(0);
          expect(cue.delayMs, label).toBeLessThanOrEqual(D);
          if (cue.kind === "projectile" || cue.kind === "shot") expect(cue.delayMs + cue.flightMs, label).toBeLessThanOrEqual(D);
          expect(endOf(cue), label).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
          if (cue.kind === "shake") expect(cue.trauma, label).toBeLessThanOrEqual(1);
        }
      }
    });

    it(`R2794000 R202 ${key}: a hidden play plans what no play does, and no cue names the card`, () => {
      const view = fullBoardView();
      const fixture = RECIPE_FIXTURES[key](view, "hidden", "p2");
      const hidden = fixtureCues(planAll([played("p2", "hidden", "hidden"), ...fixture, resolved("p2", "hidden", "hidden")], view));
      expect(hidden).toEqual(planAll(fixture, view).flatMap(({ cues }) => cues));
      const readable = planAll(flourishStream(key, defId, view, "p2", "ex"), view).flatMap(({ cues }) => cues);
      const json = JSON.stringify(readable);
      expect(json).not.toContain(defId);
      expect(json).not.toContain(CATALOG[defId]?.name ?? defId);
      // The effects off plans nothing.
      expect(planAll(flourishStream(key, defId, view, "p1", "ex"), view, envOf({ intensity: 0 })).flatMap(({ cues }) => cues)).toEqual([]);
    });
  }
});
