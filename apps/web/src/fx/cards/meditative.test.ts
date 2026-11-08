// R2784000 #2796: Meditative's and Meditative+'s pass over every card (docs/polish/9-card-animations.md):
// at least FX_SET_PASS_FLOOR of each set wears an entry, every Spell, Field Spell, Trap and Field Trap
// plays on its `cast`, every Field Spell wears an aura of its own (R2787000), a hidden play keys none of
// it (R202), and the per-card table (meditative.md) holds every card of both tables as they are.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries } from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import { planCardFx, planCardMotif } from "../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_SET_PASS_FLOOR } from "../constants.ts";
import { planFx } from "../cues.ts";
import { createFxMemory } from "../memory.ts";
import { motifScale } from "../motifs/build.ts";
import { galleryPlayOf, motifFixture, RECIPE_FIXTURES, type GalleryCardPlay } from "../motifs/gallery.ts";
import { FX_CARD_HOOKS } from "../motifs/names.ts";
import type { FxPlanEnv } from "../types.ts";
import rawTable from "./meditative.md?raw";
import { FX_CARD_FILES, parseFxCardTable, type FxCardEntry } from "./tables.ts";

const FILES = ["meditative.json5", "meditativeplus.json5"] as const;

function tableOf(name: (typeof FILES)[number]): ReadonlyMap<string, FxCardEntry> {
  const file = FX_CARD_FILES.find((candidate) => candidate.file === name);
  if (file === undefined) throw new Error(`fx/cards/${name} is missing`);
  return parseFxCardTable(file.file, file.source);
}

const TABLES = FILES.map((file) => [file, tableOf(file)] as const);
const ENTRIES: readonly (readonly [string, FxCardEntry])[] = TABLES.flatMap(([, table]) => [...table]);

const isToken = (id: string): boolean => CATALOG[id]?.token === true;
const typeOf = (id: string): string => CATALOG[id]?.type ?? "";

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

/** Plans each entry of `events` in order, as FxLayer does on every `start`: remember, then plan. */
function planEach(events: readonly GameEvent[], view: PlayerView, plan: (entry: ReturnType<typeof planEntries>[number], env: FxPlanEnv) => void): void {
  const env = envOf();
  for (const entry of planEntries(events, withEvents(view, [...events]), false)) {
    env.memory.remember(entry.events);
    plan(entry, env);
  }
}

type BoardPlay = Extract<GalleryCardPlay, { kind: "board" | "recipe" }>;

/** The board cards of both sets: every entry the gallery plays as its card's play over a fixture (a motif on its play, or a recipe). */
function boardCards(): { defId: string; play: BoardPlay }[] {
  return ENTRIES.flatMap(([defId, entry]) => {
    const play = galleryPlayOf(defId, entry);
    return play !== null && (play.kind === "board" || play.kind === "recipe") ? [{ defId, play }] : [];
  });
}

/** The card's fixture events, as `player` would meet them from `sourceId`: its motif's or its recipe's. */
function fixtureOf(play: BoardPlay, view: PlayerView, sourceId: string, player: PlayerId): GameEvent[] {
  return play.kind === "board" ? motifFixture(play.motif, view, sourceId, player) : RECIPE_FIXTURES[play.key](view, sourceId, player);
}

/** An entry as meditative.md writes it: `none: <reason>`, `bespoke: <key>`, or each hook then the aura, joined by `; `. */
function entryText(entry: FxCardEntry): string {
  if (entry.kind === "none") return `none: ${entry.reason}`;
  if (entry.kind === "bespoke") return `bespoke: ${entry.key}`;
  if (entry.kind === "setPiece") return `setPiece: ${entry.key}`;
  const hooks = FX_CARD_HOOKS.flatMap((hook) => {
    const spec = entry.hooks[hook];
    return spec === undefined ? [] : [[hook, spec.motif, spec.palette].filter((part) => part !== undefined).join(" ")];
  });
  const aura = entry.aura === undefined ? [] : [["aura", entry.aura.look, entry.aura.scope, entry.aura.palette].filter((part) => part !== undefined).join(" ")];
  return [...hooks, ...aura].join("; ");
}

/** meditative.md's table rows: Card, Name, Entry, Scale, Census, Why. */
function rowsOf(markdown: string): string[][] {
  return markdown
    .split("\n")
    .filter((line) => line.startsWith("| "))
    .map((line) =>
      line
        .slice(1, -1)
        .split(" | ")
        .map((cell) => cell.trim()),
    )
    .filter((cells) => cells[0] !== "Card" && !/^-+$/.test(cells[0] ?? ""));
}

describe("R2784000 #2796 Meditative's and Meditative+'s card animations", () => {
  it("R2784000 #2796 at least FX_SET_PASS_FLOOR of each set's cards have an entry other than none", () => {
    for (const [file, table] of TABLES) {
      const cards = [...table].filter(([id]) => !isToken(id));
      const drawn = cards.filter(([, entry]) => entry.kind !== "none");
      expect(cards.length, file).toBeGreaterThan(0);
      expect(drawn.length / cards.length, `${file}: ${String(drawn.length)} of ${String(cards.length)}`).toBeGreaterThanOrEqual(FX_SET_PASS_FLOOR);
    }
    // The issue's counts: 57 of Meditative's 102 cards, 15 of Meditative+'s 26.
    const [meditative, plus] = TABLES.map(([, table]) => [...table].filter(([id, entry]) => !isToken(id) && entry.kind !== "none").length);
    expect(meditative).toBeGreaterThanOrEqual(57);
    expect(plus).toBeGreaterThanOrEqual(15);
  });

  it("R2784000 #2796 every Spell, Field Spell, Trap and Field Trap has a cast entry or a bespoke key", () => {
    const spells = ENTRIES.filter(([id]) => !isToken(id) && typeOf(id) !== "Unit");
    expect(spells.length).toBe(58 + 17);
    for (const [id, entry] of spells) {
      const cast = entry.kind === "bespoke" || (entry.kind === "hooks" && entry.hooks.cast !== undefined);
      expect(cast, `${id} ${typeOf(id)}`).toBe(true);
    }
  });

  it("R2787000 #2796 every Field Spell wears an aura, no two alike", () => {
    const fields = ENTRIES.filter(([id]) => typeOf(id) === "Field Spell");
    expect(fields.filter(([id]) => !isToken(id))).toHaveLength(20 + 2);
    const looks = new Map<string, string>();
    for (const [id, entry] of fields) {
      // R3400000: a bespoke entry's aura too (R3405001: Meditative's signature Field Spells).
      const aura = entry.kind === "hooks" || entry.kind === "bespoke" ? entry.aura : undefined;
      expect(aura, id).toBeDefined();
      if (aura === undefined) continue;
      const key = `${aura.look} ${aura.scope} ${aura.palette ?? CATALOG[id]?.flavor ?? ""}`;
      expect(looks.get(key), `${id} wears ${key}`).toBeUndefined();
      looks.set(key, id);
    }
  });

  it("R202 #2796 a hidden play keys no entry", () => {
    const view = fullBoardView();
    const cards = boardCards();
    expect(cards.length).toBeGreaterThan(0);
    for (const card of cards) {
      const fixture = fixtureOf(card.play, view, "hidden", "p2");
      const stream: GameEvent[] = [
        { type: "cardPlayed", player: "p2", instanceId: "hidden", defId: "hidden", costPaid: 1 },
        ...fixture,
        { type: "cardResolved", player: "p2", instanceId: "hidden", defId: "hidden", permanent: false, costPaid: 1 },
      ];
      planEach(stream, view, (entry, env) => {
        expect(planCardMotif(entry, entry.view, env), card.defId).toBeNull();
        for (const event of entry.events) expect(planCardFx(event, { entry, view: entry.view, env, D: entry.durationMs }), `${card.defId} ${event.type}`).toBeNull();
      });
      // A readable play draws its entry, and no cue names a card.
      const readable: GameEvent[] = [
        { type: "cardPlayed", player: "p2", instanceId: "ex", defId: card.defId, costPaid: 1 },
        ...fixtureOf(card.play, view, "ex", "p2"),
        { type: "cardResolved", player: "p2", instanceId: "ex", defId: card.defId, permanent: false, costPaid: 1 },
      ];
      planEach(readable, view, (entry, env) => {
        expect(JSON.stringify(planFx(entry, entry.view, env)), card.defId).not.toMatch(/meditative/);
      });
    }
  });

  it("R2784000 #2796 meditative.md holds one row per card of both tables, in order, with its name, entry and scale", () => {
    const view = fullBoardView();
    const rows = rowsOf(rawTable);
    expect(rows.map((row) => row[0])).toEqual(ENTRIES.map(([id]) => id));
    for (const [index, [id, entry]] of ENTRIES.entries()) {
      const row = rows[index] ?? [];
      expect(row, id).toHaveLength(6);
      expect(row[1], id).toBe((CATALOG[id]?.name ?? "").replace(/\s+/g, " ").trim());
      expect(row[2], id).toBe(entryText(entry));
      expect(row[3], id).toBe(entry.kind === "none" ? "—" : `×${motifScale(id, view, 1).toFixed(2)}`);
      expect(row[5], `${id} gives a reason`).not.toBe("");
    }
  });
});
