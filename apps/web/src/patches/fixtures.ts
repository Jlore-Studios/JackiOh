// Test support only (nothing outside a test imports it): a small patch history in the real files'
// shape (crates/cards/patches/), fed through `PatchSourceProvider`.
//
// Three patches whose version strings sort the wrong way on purpose ("v9-first" < "v1-second" is
// false), so a screen that ordered patches by comparing versions instead of reading patches.json's
// order (R105 as B4.2 rewrites it) lists them wrongly and fails.
//
//   v9-first   adds Test Knight, Test Bolt, Old Relic, Quiet Golem (Core)
//   v1-second  changes Test Knight (cost, stats, both texts), removes Old Relic
//   v5-third   changes Test Bolt (text via a {key} param, cost to embiggen, a Radiant keyword),
//              Quiet Golem (its lines of code only), adds Fresh Face (Classic), Pancake Pal and
//              its token Syrup Token (Classic+)

import type { CardDef } from "@jackioh/shared";

import { sourceFromData, type HistoryIndex, type Patch, type PatchSource, type Snapshot } from "./source.ts";

export const V1 = "v9-first";
export const V2 = "v1-second";
export const V3 = "v5-third";

function card(fields: Partial<CardDef> & Pick<CardDef, "id" | "index" | "name">): CardDef {
  return {
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 2,
    base: { attack: 2, health: 3, keywords: [], text: "" },
    radiant: { attack: 4, health: 6, keywords: [], text: "" },
    ...fields,
  };
}

const knightV1 = card({
  id: "core-001",
  index: "1",
  name: "Test Knight",
  base: { attack: 2, health: 3, keywords: [], text: "Cry: Deal 1 damage to a target." },
  radiant: { attack: 4, health: 6, keywords: [], text: "Cry: Deal 2 damage to a target." },
});
const knightV2 = card({
  ...knightV1,
  cost: 3,
  base: { attack: 3, health: 3, keywords: [], text: "Cry: Deal 1 damage to a target. Draw 1." },
  radiant: { attack: 6, health: 6, keywords: [], text: "Cry: Deal 2 damage to a target. Draw 2." },
});

const boltV1 = card({
  id: "core-002",
  index: "2",
  name: "Test Bolt",
  type: "Spell",
  cost: 1,
  base: { keywords: [], text: "Deal 3 damage. Spells cost (1) less this turn." },
  radiant: { keywords: [], text: "Deal 6 damage. Spells cost (1) less this turn." },
});
const boltV3 = card({
  ...boltV1,
  cost: { base: 2, embiggen: 4 },
  params: [{ key: "damage", base: 4, radiant: 6, better: "up", step: 1, min: 1 }],
  base: { keywords: [], text: "Deal {damage} damage. Your Spells cost (1) less." },
  radiant: { keywords: [{ kind: "Pierce" }], text: "Deal {damage} damage. Your Spells cost (1) less." },
});

const relic = card({ id: "core-003", index: "3", name: "Old Relic", type: "Field Spell", cost: 0, base: { keywords: [], text: "Aura: Nothing." }, radiant: { keywords: [], text: "Aura: Nothing at all." } });

const golemV1 = card({ id: "core-004", index: "4", name: "Quiet Golem", base: { attack: 5, health: 5, keywords: [{ kind: "Taunt" }], text: "" }, radiant: { attack: 10, health: 10, keywords: [{ kind: "Taunt" }], text: "" } });
const golemV3 = card({ ...golemV1, loc: 12 });

const fresh = card({ id: "classic-001", index: "1", name: "Fresh Face", set: "Classic", rarity: "Rare", base: { attack: 1, health: 1, keywords: [], text: "Rush" }, radiant: { attack: 2, health: 2, keywords: [{ kind: "Rush" }], text: "Rush" } });
const pal = card({ id: "classicplus-012", index: "12", name: "Pancake Pal", set: "Classic+", tags: ["Pancake"], refs: ["classicplus-012-1"], base: { attack: 2, health: 2, keywords: [], text: "Cry: Summon a Syrup Token." }, radiant: { attack: 4, health: 4, keywords: [], text: "Cry: Summon two Syrup Tokens." } });
const syrup = card({ id: "classicplus-012-1", index: "12.1", name: "Syrup Token", set: "Classic+", token: true, rarity: "Token", printedRarity: "Legendary", tags: ["Pancake", "Token"], cost: 1, base: { attack: 1, health: 1, keywords: [], text: "" }, radiant: { attack: 2, health: 2, keywords: [], text: "" } });

function snapshot(...defs: CardDef[]): Snapshot {
  return Object.fromEntries(defs.map((def) => [def.id, def]));
}

export const FIXTURE_SNAPSHOTS: Readonly<Record<string, Snapshot>> = {
  [V1]: snapshot(knightV1, boltV1, relic, golemV1),
  [V2]: snapshot(knightV2, boltV1, golemV1),
  [V3]: snapshot(knightV2, boltV3, golemV3, fresh, pal, syrup),
};

export const FIXTURE_PATCHES: readonly Patch[] = [
  {
    version: V1,
    date: "2026-01-02",
    title: "The first cards",
    source: "Fixture commit 1",
    notes: "Four cards to start with.",
    changes: [
      { id: "core-001", name: "Test Knight", kind: "added" },
      { id: "core-002", name: "Test Bolt", kind: "added" },
      { id: "core-003", name: "Old Relic", kind: "added" },
      { id: "core-004", name: "Quiet Golem", kind: "added" },
    ],
  },
  {
    version: V2,
    date: "2026-02-03",
    title: "The knight grows",
    source: "Fixture commit 2",
    notes: "Test Knight costs more and draws; Old Relic leaves.",
    changes: [
      { id: "core-001", name: "Test Knight", kind: "changed", fields: ["cost", "base.attack", "radiant.attack", "base.text", "radiant.text"] },
      { id: "core-003", name: "Old Relic", kind: "removed" },
    ],
  },
  {
    version: V3,
    date: "2026-03-04",
    title: "Two new sets",
    source: "Fixture commit 3",
    notes: "Classic and Classic+ arrive.",
    changes: [
      { id: "core-002", name: "Test Bolt", kind: "changed", fields: ["cost", "params", "base.text", "radiant.keywords", "radiant.text"] },
      { id: "core-004", name: "Quiet Golem", kind: "changed", fields: ["loc"] },
      { id: "classic-001", name: "Fresh Face", kind: "added" },
      { id: "classicplus-012", name: "Pancake Pal", kind: "added" },
      { id: "classicplus-012-1", name: "Syrup Token", kind: "added" },
    ],
  },
];

export const FIXTURE_INDEX: HistoryIndex = {
  "core-001": [V1, V2],
  "core-002": [V1, V3],
  "core-003": [V1, V2],
  "core-004": [V1, V3],
  "classic-001": [V3],
  "classicplus-012": [V3],
  "classicplus-012-1": [V3],
};

/** The fixture history as a source; `onSnapshot` sees every snapshot asked for, in order. */
export function fixtureSource(onSnapshot?: (version: string) => void): PatchSource {
  const source = sourceFromData({ patches: FIXTURE_PATCHES, index: FIXTURE_INDEX, snapshots: FIXTURE_SNAPSHOTS });
  return {
    ...source,
    snapshot: (version) => {
      onSnapshot?.(version);
      return source.snapshot(version);
    },
  };
}

/** A fixture card as a patch left it. */
export function fixtureDef(version: string, id: string): CardDef {
  const def = FIXTURE_SNAPSHOTS[version]?.[id];
  if (def === undefined) throw new Error(`no ${id} in ${version}`);
  return def;
}
