// R2784000: what the /dev/fx gallery (routes/dev/fx.tsx) plays: a fixture event for every motif on a
// view's own ids, the entry it fills, and every card's table entry (#2796): a cast, a cry, a card recipe
// or an aura as a play that resolves on the board, a death, trigger or activate motif through the
// gallery's own director, and a Field card's aura standing in the first backrow zone. Pure, so the
// motif tests plan from the same fixtures the gallery shows.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../cards/flavor.ts";
import { animationRow, scaleForSpeed, SET_PIECE_HOLD_MS, type AnimationEntry, type SetPieceClaimer } from "../../game/animations.ts";
import { CATALOG } from "../../wire/cards.ts";
import { isAuraFieldType, type AuraLook } from "../aura/looks.ts";
import { isCardRecipeKey, type CardRecipeKey } from "../cards/bespoke.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../cards/tables.ts";
import { FX_NEXT_REFRESH_MODIFIER_ID } from "../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../flavors.ts";
import { flavorLookOf } from "../flavorSource.ts";
import { createFxMemory } from "../memory.ts";
import type { FxCue } from "../types.ts";
import { motifScale } from "./build.ts";
import { planMotif } from "./index.ts";
import { FX_CARD_HOOKS, isDrawnMotif, type DrawnMotif, type FxCardHook } from "./names.ts";

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
 * refills the viewer's crystals, tithe discounts the viewer's first hand card, tally ticks a counter
 * and snap fires a hidden trap. Deal adds a card to the player's hand and shuffles a hidden one into
 * their deck, and seal lays a modifier on the enemy seat.
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
    case "deal":
      return [
        { type: "addedToHand", player, instanceId: "fx-dealt", defId: "core-055" },
        { type: "shuffledIn", player, instanceId: "hidden", defId: "hidden", position: 0 },
      ];
    case "seal":
      return [{ type: "modifierChanged", player: enemy.player, modifierId: "fx-sealed", added: true }];
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

/**
 * #2796: one card the gallery plays: the hook its entry draws first and that hook's motif and palette,
 * the card recipe its bespoke entry names, or, for an entry that is only an aura, its aura's look.
 */
export type GalleryCard =
  | { defId: string; hook: FxCardHook; fixture: DrawnMotif; palette?: FxPalette }
  | { defId: string; hook: "bespoke"; fixture: CardRecipeKey }
  | { defId: string; hook: "aura"; fixture: AuraLook };

/**
 * #2796: what the gallery plays of a card's entry: a card recipe, else the first hook (in
 * FX_CARD_HOOKS order) whose motif is drawn, else its aura; none for a `none`, a set piece (its play is
 * "Play as a set piece"), an entrance and a hooks entry that draws nothing.
 */
export function galleryCardOf(defId: string, entry: FxCardEntry): GalleryCard | null {
  if (entry.kind === "bespoke") return isCardRecipeKey(entry.key) ? { defId, hook: "bespoke", fixture: entry.key } : null;
  if (entry.kind !== "hooks") return null;
  for (const hook of FX_CARD_HOOKS) {
    const spec = entry.hooks[hook];
    if (spec === undefined || !isDrawnMotif(spec.motif)) continue;
    return spec.palette === undefined ? { defId, hook, fixture: spec.motif } : { defId, hook, fixture: spec.motif, palette: spec.palette };
  }
  return entry.aura === undefined ? null : { defId, hook: "aura", fixture: entry.aura.look };
}

/**
 * #2796: each card recipe's fixture events, as `player` would meet them playing it from `sourceId`:
 * Hinder lays the next-refresh modifier on the enemy, Blood Ridden makes the first hand card Radiant and
 * costs its caster 3, Crushing Walls destroys every unit, Auspicious Rock costs 2 and rolls a Jade into
 * the hand, Spiritually 中国 turns every viewer's hand card into one CN card and summons a CN Unit, and
 * Harmony Spirit sets every unit to 3/3.
 */
export const RECIPE_FIXTURES: { readonly [K in CardRecipeKey]: (view: PlayerView, sourceId: string, player: PlayerId) => GameEvent[] } = {
  manaCrack: (view, _sourceId, player) => {
    const enemy = player === view.viewer ? view.opponent : view.you;
    return [{ type: "modifierChanged", player: enemy.player, modifierId: FX_NEXT_REFRESH_MODIFIER_ID, added: true }];
  },
  bloodDrain: (view, _sourceId, player) => {
    const card = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    if (card === undefined) return [];
    return [
      { type: "radiantSet", instanceId: card.instanceId, defId: card.defId, zone: { z: "hand", player } },
      { type: "healthLost", player, amount: 3 },
    ];
  },
  crushingWalls: (view, sourceId, player) => motifFixture("wipe", view, sourceId, player),
  jadeCrack: (_view, _sourceId, player) => [
    { type: "healthLost", player, amount: 2 },
    { type: "addedToHand", player, instanceId: "fx-jade", defId: "meditative-039-2" },
  ],
  cnSweep: (view, _sourceId, player) => {
    const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
    return [
      ...hand.map((card, k): GameEvent => ({
        type: "transformed",
        instanceId: card.instanceId,
        fromDefId: card.defId,
        toDefId: "meditative-036",
        newInstanceId: `fx-cn-${String(k)}`,
      })),
      { type: "summoned", player, instanceId: "fx-cn-unit", defId: "meditative-041", row: "units", lane: 1 },
    ];
  },
  harmonyWave: (view) =>
    [...unitsOf(view.you), ...unitsOf(view.opponent)].flatMap((unit): GameEvent[] => [
      { type: "numberChanged", instanceId: unit.instanceId, defId: unit.defId, key: "attack", value: 3 },
      { type: "numberChanged", instanceId: unit.instanceId, defId: unit.defId, key: "health", value: 3 },
    ]),
};

/**
 * #2796: whether the card plays as a play resolving on the board (a cast, a cry, a card recipe or an
 * aura), not through the gallery's own director.
 */
export function galleryPlaysOnBoard(card: GalleryCard): boolean {
  return card.hook === "cast" || card.hook === "cry" || card.hook === "bespoke" || card.hook === "aura";
}

/** The fixture events that resolve under the card's play, as `player` would meet them from `sourceId`: its recipe's or its motif's, none for an aura alone. */
export function galleryFixture(card: GalleryCard, view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  if (card.hook === "bespoke") return RECIPE_FIXTURES[card.fixture](view, instanceId, player);
  if (card.hook === "aura") return [];
  return motifFixture(card.fixture, view, instanceId, player);
}

/** A card's table entry played out: the viewer plays it, its recipe's or its motif's fixture resolves under it, and it resolves. */
export function galleryCardEvents(card: GalleryCard, view: PlayerView, n = 0): GameEvent[] {
  const instanceId = galleryPlayId(n);
  const player = view.viewer;
  const { defId } = card;
  return [
    { type: "cardPlayed", player, instanceId, defId, costPaid: 1 },
    ...galleryFixture(card, view, instanceId, player),
    { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 },
  ];
}

/**
 * #2796: a hook's motif on its fixture, in the card's palette (its entry's, else its flavor's) at the
 * card's scale: how the board will play a death, trigger or activate motif once #2783's
 * `hookResolving` reaches the motifs. None for a card recipe or an aura, which play on the board.
 */
export function galleryHookCues(card: GalleryCard, view: PlayerView, intensity: number, speed = 1): FxCue[] {
  if (card.hook === "bespoke" || card.hook === "aura") return [];
  const look = (card.palette !== undefined ? paletteLook(card.palette) : undefined) ?? flavorLookOf(card.defId, GALLERY_CARD_ID, view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const spec = { motif: card.fixture, look, scale: motifScale(card.defId, view, intensity), row: "replace" } as const;
  const entry = galleryEntry(motifFixture(card.fixture, view, gallerySource(view), view.viewer), view, speed);
  return planMotif(spec, entry, view, { intensity, card: () => undefined, memory: createFxMemory() });
}

/**
 * #2796: the view the board shows for the n-th play of `card`: its events, and, for a Field card whose
 * entry wears an aura, the card itself standing face-up in the viewer's first backrow zone (when that
 * zone shows a face-up card), so the aura layer draws it.
 */
export function galleryShownView(view: PlayerView, card: GalleryCard, n = 0): PlayerView {
  const shown: PlayerView = { ...view, events: galleryCardEvents(card, view, n) };
  const entry = FX_CARD_TABLE.get(card.defId);
  const type = CATALOG[card.defId]?.type;
  const slot = view.you.backrow[0];
  if (entry?.kind !== "hooks" || entry.aura === undefined || type === undefined || !isAuraFieldType(type)) return shown;
  if (slot === null || slot === undefined || slot.faceDown) return shown;
  const backrow = [{ ...slot, defId: card.defId, instanceId: galleryPlayId(n), type }, ...view.you.backrow.slice(1)];
  return { ...shown, you: { ...view.you, backrow } };
}
