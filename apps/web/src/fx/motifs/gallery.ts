// R2784000: what the /dev/fx gallery (routes/dev/fx.tsx) plays: a fixture event for every motif on a
// view's own ids, the entry it fills, and a card's table entry as a play that resolves. Pure, so the
// motif tests plan from the same fixtures the gallery shows. R2795000: every entry but `none`, an aura
// alone and a bespoke entrance plays: a card recipe on a fixture of its own, a set piece under its hook's
// marker, a death, trigger or activate hook as its motif, and a Field card standing with its aura.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../cards/flavor.ts";
import { animationRow, scaleForSpeed, SET_PIECE_HOLD_MS, type AnimationEntry, type SetPieceClaimer } from "../../game/animations.ts";
import { CATALOG } from "../../wire/cards.ts";
import { isCardRecipeKey, type CardRecipeKey } from "../cards/bespoke.ts";
import type { FxCardEntry, FxHookSpec } from "../cards/tables.ts";
import { FX_NEXT_REFRESH_MODIFIER_ID } from "../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../flavors.ts";
import { flavorLookOf } from "../flavorSource.ts";
import { createFxMemory } from "../memory.ts";
import type { FxCue } from "../types.ts";
import { motifScale, type MotifSpec } from "./build.ts";
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
 * refills the viewer's crystals, tithe discounts the viewer's first hand card, tally ticks a counter,
 * snap fires a hidden trap, and deal adds a card to the player's hand and shuffles a hidden one into
 * their library (R2795000).
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
 * R2795000: one card the gallery offers: the hook it plays (the first whose motif is drawn, its bespoke
 * recipe or its set piece), the fixture played under it and the hook's spec.
 */
export type GalleryCard = { defId: string; label: string; hook: FxCardHook | "bespoke" | "setPiece"; fixture: DrawnMotif | CardRecipeKey; spec: FxHookSpec | null };

/** R2795000: what the gallery plays for a card's entry; none for `none`, an aura alone, a motif not drawn and a bespoke entrance. */
export function galleryCardOf(defId: string, entry: FxCardEntry): GalleryCard | null {
  switch (entry.kind) {
    case "none":
      return null;
    case "setPiece":
      return { defId, label: `set piece: ${entry.key}`, hook: "setPiece", fixture: "wipe", spec: null };
    case "bespoke":
      return isCardRecipeKey(entry.key) ? { defId, label: `bespoke: ${entry.key}`, hook: "bespoke", fixture: entry.key, spec: null } : null;
    case "hooks": {
      for (const hook of FX_CARD_HOOKS) {
        const spec = entry.hooks[hook];
        if (spec !== undefined && isDrawnMotif(spec.motif)) return { defId, label: `${hook}: ${spec.motif}`, hook, fixture: spec.motif, spec };
      }
      return null;
    }
  }
}

/** R2795000: a card recipe's fixture events, as `player` would see its play resolve from `sourceId` on `view`. */
export function recipeFixture(key: CardRecipeKey, view: PlayerView, sourceId: string | null, player: PlayerId): GameEvent[] {
  const enemy = player === view.viewer ? view.opponent : view.you;
  switch (key) {
    case "manaCrack":
      return [{ type: "modifierChanged", player: enemy.player, modifierId: FX_NEXT_REFRESH_MODIFIER_ID, added: true }];
    case "bloodDrain":
      return [
        { type: "radiantSet", instanceId: "hidden", defId: "hidden", zone: { z: "hand", player } },
        { type: "healthLost", player, amount: 3 },
      ];
    case "crushingWalls":
      return motifFixture("wipe", view, sourceId, player);
    case "rollbackSweep":
      return [{ type: "rolledBack", player, turnsAgo: 1, sides: [view.you.player, view.opponent.player] }];
    case "constantStamp": {
      const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
      return hand === undefined ? [] : [{ type: "numberChanged", instanceId: hand.instanceId, defId: hand.defId, key: "cost", value: 3 }];
    }
    case "inkTear":
      return [
        { type: "discarded", instanceId: "fx-discarded-1", defId: "core-055", owner: enemy.player },
        { type: "discarded", instanceId: "fx-discarded-2", defId: "core-002", owner: enemy.player },
      ];
  }
}

/**
 * R2795000: a card's gallery play: the viewer plays it, a set piece's hook is marked (R2783000), the
 * fixture resolves under it and it resolves.
 */
export function galleryCardEvents(card: GalleryCard, view: PlayerView, n = 0): GameEvent[] {
  const instanceId = galleryPlayId(n);
  const player = view.viewer;
  const { defId, fixture } = card;
  const marker: GameEvent[] = card.hook === "setPiece" ? [{ type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: "cry" }] : [];
  return [
    { type: "cardPlayed", player, instanceId, defId, costPaid: 1 },
    ...marker,
    ...(isCardRecipeKey(fixture) ? recipeFixture(fixture, view, instanceId, player) : motifFixture(fixture, view, instanceId, player)),
    { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 },
  ];
}

/**
 * R2795000: a death, trigger or activate hook's motif on its fixture, in the card's look at its scale as
 * `planCardMotif` would draw it, for the gallery's own director: the board plans those hooks nothing yet.
 */
export function galleryHookCues(card: GalleryCard, view: PlayerView, intensity: number, speed = 1): FxCue[] {
  const { spec, fixture } = card;
  if (spec === null || isCardRecipeKey(fixture)) return [];
  const look = (spec.palette !== undefined ? paletteLook(spec.palette) : undefined) ?? flavorLookOf(card.defId, GALLERY_CARD_ID, view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const motif: MotifSpec = { motif: fixture, look, scale: motifScale(card.defId, view, intensity), row: spec.row ?? "replace" };
  const entry = galleryEntry(motifFixture(fixture, view, gallerySource(view), view.viewer), view, speed);
  return planMotif(motif, entry, view, { intensity, card: () => undefined, memory: createFxMemory() });
}

/**
 * R2795000: the view the gallery shows for a card's play: its events, and a Field Spell or Field Trap
 * standing face-up in the viewer's first backrow zone, so its aura shows.
 */
export function galleryShownView(view: PlayerView, card: GalleryCard, n = 0): PlayerView {
  const events = galleryCardEvents(card, view, n);
  const type = CATALOG[card.defId]?.type;
  const slot = view.you.backrow[0];
  if ((type !== "Field Spell" && type !== "Field Trap") || slot === null || slot === undefined || slot.faceDown) return { ...view, events };
  const backrow = [{ ...slot, defId: card.defId, instanceId: galleryPlayId(n), type }, ...view.you.backrow.slice(1)];
  return { ...view, you: { ...view.you, backrow }, events };
}
