// R2784000: what the /dev/fx gallery (routes/dev/fx.tsx) plays: a fixture event for every motif on a
// view's own ids, the entry it fills, and a card's table entry as a play that resolves. Pure, so the
// motif tests plan from the same fixtures the gallery shows. R2794000: a card's entry is a gallery card:
// its play's motif, its first drawn death, trigger or activate motif (played through the gallery's own
// director, as no play opens for a hook), its Flourish on that Flourish's fixture, or its marquee entrance.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../cards/flavor.ts";
import { animationRow, scaleForSpeed, SET_PIECE_HOLD_MS, type AnimationEntry, type SetPieceClaimer } from "../../game/animations.ts";
import { CATALOG } from "../../wire/cards.ts";
import { entranceKeyOf } from "../cardFx.ts";
import { isEntranceKey, type CardRecipeKey } from "../cards/bespoke.ts";
import { pageMendFixture } from "../cards/classic/bookOfHeal.ts";
import type { FlourishFixture } from "../cards/classic/flourish.ts";
import { crownSwapFixture } from "../cards/classic/kingWagtoggle.ts";
import { recycleLoopFixture } from "../cards/classic/recycle.ts";
import type { EntranceKey } from "../cards/recipe.ts";
import type { FxCardEntry } from "../cards/tables.ts";
import { paletteLook, type FxPalette } from "../flavors.ts";
import { createFxMemory } from "../memory.ts";
import type { FxCue } from "../types.ts";
import { motifScale } from "./build.ts";
import { planMotif } from "./index.ts";
import { isDrawnMotif, type DrawnMotif } from "./names.ts";

/** The instance id the gallery's card plays under; the n-th play takes `-n` after it, so each is new to the board. */
export const GALLERY_CARD_ID = "fx-gallery-card";

/** The instance id of the gallery's n-th card play. */
export function galleryPlayId(n: number): string {
  return n === 0 ? GALLERY_CARD_ID : `${GALLERY_CARD_ID}-${String(n)}`;
}

/** R2785000: the key of the gallery's fixture set piece. */
export const GALLERY_SET_PIECE = "fixture";

/**
 * R2785000: the gallery's fixture set piece: the entry holding the play of `instanceId` claims the set
 * piece's default beat, as a card or a system of issue #2782 will claim its own.
 */
export function gallerySetPiece(instanceId: string): SetPieceClaimer {
  return (entry) =>
    entry.events.some((event) => event.type === "cardPlayed" && event.instanceId === instanceId)
      ? { key: GALLERY_SET_PIECE, holdMs: SET_PIECE_HOLD_MS }
      : null;
}

type UnitOf = NonNullable<PlayerView["you"]["units"][number]>;

function unitsOf(side: PlayerView["you"]): UnitOf[] {
  return side.units.filter((unit): unit is UnitOf => unit !== null);
}

/**
 * One motif's fixture events, as `player` would deal them from `sourceId` on `view`: bolt and scorch hit
 * the first enemy unit, volley the first three, barrage rains six hits over four units and the hero,
 * chain hops four units in order, nova blasts every enemy unit, lance hits lane 3, wipe destroys every
 * unit, vanish exiles the second enemy unit, negate counters a card, drain streams from the enemy hero
 * into its first unit, bless buffs and curse degrades the viewer's first unit, ward shields the first
 * enemy unit, conjure and rise summon a token, offering sinks the viewer's first unit as a cost, shift
 * swaps the board, bind locks an enemy zone, tether takes the first enemy unit, mirror and transmute
 * turn the viewer's third unit into a token, flight draws a card, glimpse judges a prediction, mana
 * refills the viewer's crystals, tithe discounts the viewer's first hand card, tally ticks a counter,
 * snap fires a hidden trap and recall returns a card to the hand.
 */
export function motifFixture(motif: DrawnMotif, view: PlayerView, sourceId: string | null, player: PlayerId): GameEvent[] {
  const enemy = player === view.viewer ? view.opponent : view.you;
  const foes = unitsOf(enemy);
  const damage = (targetId: string, amount: number): GameEvent => ({ type: "damage", sourceId, targetId, amount, combat: false });
  const ids = foes.map((unit) => unit.instanceId);
  switch (motif) {
    case "bolt":
    case "scorch":
      return ids.slice(0, 1).map((id) => damage(id, 3));
    case "volley":
      return ids.slice(0, 3).map((id) => damage(id, 2));
    case "barrage": {
      const targets = [...ids.slice(0, 4), `hero-${enemy.player}`];
      return Array.from({ length: 6 }, (_, k) => damage(targets[k % targets.length] ?? "", 1));
    }
    case "chain":
      return ids.slice(0, 4).map((id) => damage(id, 2));
    case "nova":
      return ids.map((id) => damage(id, 2));
    case "lance": {
      const lane3 = enemy.units[2];
      return lane3 === null || lane3 === undefined ? [] : [damage(lane3.instanceId, 4)];
    }
    case "wipe":
      return [...unitsOf(view.you), ...foes].map((unit) => ({
        type: "destroyed",
        instanceId: unit.instanceId,
        defId: unit.defId,
        owner: unit.owner,
        controller: unit.controller,
        attack: unit.attack,
        maxHealth: unit.maxHealth,
        killerId: null,
      }));
    case "vanish":
      return foes.slice(1, 2).map((unit) => ({ type: "exiled", instanceId: unit.instanceId, defId: unit.defId, owner: unit.owner }));
    case "negate":
      return [{ type: "countered", player: enemy.player, instanceId: "fx-gallery-countered", defId: "core-016", byInstanceId: null, to: "graveyard" }];
    case "drain": {
      const foe = foes[0];
      if (foe === undefined) return [];
      return [
        { type: "healthLost", player: enemy.player, amount: 3 },
        { type: "healed", targetId: foe.instanceId, amount: 2 },
      ];
    }
    case "bless": {
      const mine = unitsOf(view.you)[0];
      return mine === undefined ? [] : [{ type: "buffed", instanceId: mine.instanceId, attack: 1, health: 1 }];
    }
    case "curse": {
      const mine = unitsOf(view.you)[0];
      return mine === undefined ? [] : [{ type: "degraded", instanceId: mine.instanceId, defId: mine.defId, change: { kind: "stats", attack: -1, health: -1 } }];
    }
    case "ward": {
      const foe = foes[0];
      return foe === undefined ? [] : [{ type: "damageAbsorbed", sourceId: null, targetId: foe.instanceId, absorbed: 5, combat: false }];
    }
    case "conjure":
    case "rise":
      return [{ type: "summoned", player, instanceId: "fx-conjured", defId: "token-sheep", row: "units", lane: 1 }];
    case "offering": {
      const mine = unitsOf(view.you)[0];
      return mine === undefined
        ? []
        : [
            {
              type: "destroyed",
              instanceId: mine.instanceId,
              defId: mine.defId,
              owner: mine.owner,
              controller: mine.controller,
              attack: mine.attack,
              maxHealth: mine.maxHealth,
              killerId: null,
            },
          ];
    }
    case "shift":
      return [{ type: "swapped", what: "board" }];
    case "bind":
      return [{ type: "locked", player: enemy.player, row: "units", lane: 2 }];
    case "tether": {
      const foe = foes[0];
      return foe === undefined ? [] : [{ type: "controlChanged", instanceId: foe.instanceId, controller: view.viewer, row: "units", lane: 4 }];
    }
    case "mirror":
    case "transmute": {
      const mine = unitsOf(view.you)[2] ?? unitsOf(view.you)[0];
      return mine === undefined
        ? []
        : [{ type: "transformed", instanceId: mine.instanceId, fromDefId: mine.defId, toDefId: "token-sheep", newInstanceId: "fx-double" }];
    }
    case "flight":
      return [{ type: "drawn", player, instanceId: "fx-drawn", defId: "core-055" }];
    case "glimpse":
      return [{ type: "predicted", player, secretId: "fx-secret", guess: "attack", outcome: "won" }];
    case "mana":
      return [{ type: "manaChanged", player, current: 4, max: 4 }];
    case "tithe": {
      const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
      return hand === undefined ? [] : [{ type: "costChanged", instanceId: hand.instanceId, cost: 0 }];
    }
    case "tally": {
      const mine = unitsOf(view.you)[2] ?? unitsOf(view.you)[0];
      return mine === undefined ? [] : [{ type: "counterChanged", instanceId: mine.instanceId, counter: "grade", value: 1 }];
    }
    case "snap":
      return [{ type: "trapFired", instanceId: "hidden", defId: "hidden", controller: enemy.player, row: "backrow", lane: 4 }];
    case "recall":
      return [{ type: "addedToHand", player, instanceId: "fx-recalled", defId: "core-003" }];
  }
}

/** The entry the runner would give `events`: its first event's row, its duration over the effects speed (R201). */
export function galleryEntry(events: readonly GameEvent[], view: PlayerView, speed = 1): AnimationEntry {
  const type = events[0]?.type ?? "damage";
  return { events, type, durationMs: scaleForSpeed(animationRow(type)?.durationMs ?? 0, speed), frames: new Map(), view };
}

/** The source the gallery's shots fly from: the viewer's first unit. */
export function gallerySource(view: PlayerView): string | null {
  return unitsOf(view.you)[0]?.instanceId ?? null;
}

/** One motif in one palette at one scale, on its fixture, as a card of that scale would play it. */
export function galleryCues(motif: DrawnMotif, palette: FxPalette, scale: number, view: PlayerView, intensity: number, speed = 1): FxCue[] {
  const entry = galleryEntry(motifFixture(motif, view, gallerySource(view), view.viewer), view, speed);
  const look = paletteLook(palette);
  if (look === undefined) return [];
  const spec = { motif, look, scale: scale * intensity, row: "replace" } as const;
  return planMotif(spec, entry, view, { intensity, card: () => undefined, memory: createFxMemory() });
}

/** A card's table entry played out: the viewer plays it, its motif's fixture resolves under it, and it resolves. */
export function cardFixtureEvents(defId: string, motif: DrawnMotif, view: PlayerView, n = 0): GameEvent[] {
  const instanceId = galleryPlayId(n);
  const player = view.viewer;
  return [
    { type: "cardPlayed", player, instanceId, defId, costPaid: 1 },
    ...motifFixture(motif, view, instanceId, player),
    { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 },
  ];
}

/** R2794000: the lane of the viewer's unit row a gallery entrance lands in. */
export const GALLERY_ENTRANCE_LANE = 3;

/** R2794000: each Flourish's fixture, the events its play resolves with in the gallery. */
export const FLOURISH_FIXTURES = {
  pageMend: pageMendFixture,
  recycleLoop: recycleLoopFixture,
  crownSwap: crownSwapFixture,
} as const satisfies Partial<Readonly<Record<CardRecipeKey, FlourishFixture>>>;

export type FlourishKey = keyof typeof FLOURISH_FIXTURES;

export function isFlourishKey(key: string): key is FlourishKey {
  return Object.prototype.hasOwnProperty.call(FLOURISH_FIXTURES, key);
}

/** The hooks a gallery card's hook motif is read from, in order; `cast` and `cry` are its play's. */
const GALLERY_HOOKS = ["death", "trigger", "activate"] as const;

/**
 * R2794000: what the gallery plays for one card's entry: its play (`cast` or `cry`) with that motif's
 * fixture, a hook's motif in the hook's palette at the card's scale, a Flourish on its own fixture, or a
 * marquee Unit's entrance as it lands.
 */
export type GalleryCard =
  | { kind: "play"; defId: string; motif: DrawnMotif }
  | { kind: "hook"; defId: string; hook: (typeof GALLERY_HOOKS)[number]; motif: DrawnMotif; palette: FxPalette }
  | { kind: "flourish"; defId: string; key: FlourishKey }
  | { kind: "entrance"; defId: string; key: EntranceKey };

/** R2794000: the gallery card a table entry plays as, or null: a `none`, a set piece, an aura alone, a recipe with no fixture. */
export function galleryCardOf(defId: string, entry: FxCardEntry): GalleryCard | null {
  if (entry.kind === "bespoke") {
    if (isFlourishKey(entry.key)) return { kind: "flourish", defId, key: entry.key };
    return isEntranceKey(entry.key) ? { kind: "entrance", defId, key: entry.key } : null;
  }
  if (entry.kind !== "hooks") return null;
  const play = (entry.hooks.cast ?? entry.hooks.cry)?.motif;
  if (play !== undefined && isDrawnMotif(play)) return { kind: "play", defId, motif: play };
  for (const hook of GALLERY_HOOKS) {
    const spec = entry.hooks[hook];
    if (spec === undefined || !isDrawnMotif(spec.motif)) continue;
    return { kind: "hook", defId, hook, motif: spec.motif, palette: spec.palette ?? CATALOG[defId]?.flavor ?? FLAVOR_FALLBACK };
  }
  return null;
}

/** R2794000: a gallery card's label: its motif, its hook and motif, or its recipe's key. */
export function galleryLabel(card: GalleryCard): string {
  switch (card.kind) {
    case "play":
      return card.motif;
    case "hook":
      return `${card.hook} ${card.motif}`;
    case "flourish":
    case "entrance":
      return card.key;
  }
}

/**
 * R2794000: a gallery card's play as the board would see it, under the n-th play's id: its motif's
 * fixture, its Flourish's fixture, or its entrance's landing (a marquee Unit with a play motif lands
 * first, then its Cry resolves). A hook card opens no play: none.
 */
export function galleryPlayEvents(card: GalleryCard, view: PlayerView, n = 0): GameEvent[] {
  const instanceId = galleryPlayId(n);
  const player = view.viewer;
  const { defId } = card;
  const landing = (): GameEvent[] =>
    card.kind === "entrance" || entranceKeyOf(defId) !== undefined ? [{ type: "summoned", player, instanceId, defId, row: "units", lane: GALLERY_ENTRANCE_LANE }] : [];
  const played: GameEvent = { type: "cardPlayed", player, instanceId, defId, costPaid: 1 };
  const resolved: GameEvent = { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 };
  switch (card.kind) {
    case "play": {
      const [first, ...rest] = cardFixtureEvents(defId, card.motif, view, n);
      return first === undefined ? [] : [first, ...landing(), ...rest];
    }
    case "flourish":
      return [played, ...landing(), ...FLOURISH_FIXTURES[card.key](view, instanceId, player), resolved];
    case "entrance":
      return [played, ...landing(), resolved];
    case "hook":
      return [];
  }
}

/** R2794000: a hook card's motif in its palette at its card's scale, on the motif's fixture. */
export function galleryHookCues(card: Extract<GalleryCard, { kind: "hook" }>, view: PlayerView, intensity: number, speed = 1): FxCue[] {
  return galleryCues(card.motif, card.palette, motifScale(card.defId, view, 1), view, intensity, speed);
}
