// R2784000: what the /dev/fx gallery (routes/dev/fx.tsx) plays: a fixture event for every motif on a
// view's own ids, the entry it fills, and a card's table entry as a play that resolves. Pure, so the
// motif tests plan from the same fixtures the gallery shows.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { flavorOf } from "../../cards/flavor.ts";
import { animationRow, scaleForSpeed, SET_PIECE_HOLD_MS, type AnimationEntry, type SetPieceClaimer } from "../../game/animations.ts";
import { CATALOG } from "../../wire/cards.ts";
import { isAuraFieldType } from "../aura/looks.ts";
import { isCardRecipeKey, type CardRecipeKey } from "../cards/bespoke.ts";
import type { FxCardEntry } from "../cards/tables.ts";
import { FX_NEXT_REFRESH_MODIFIER_ID } from "../constants.ts";
import { paletteLook, type FxPalette } from "../flavors.ts";
import { createFxMemory } from "../memory.ts";
import type { FxCue } from "../types.ts";
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
 * snap fires a hidden trap, send puts a card in the hand, radiance makes the viewer's first hand card
 * Radiant and mend heals the viewer's first unit.
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
    case "send":
      return [{ type: "addedToHand", player, instanceId: "fx-added", defId: "core-055" }];
    case "radiance": {
      const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
      return hand === undefined ? [] : [{ type: "radiantSet", instanceId: hand.instanceId, defId: hand.defId, zone: { z: "hand", player: view.viewer } }];
    }
    case "mend": {
      const mine = unitsOf(view.you)[0];
      return mine === undefined ? [] : [{ type: "healed", targetId: mine.instanceId, amount: 3 }];
    }
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
 * #2793: how the gallery plays a card's table entry. `board` is a motif on its play (`cast`, `cry`),
 * played through the board's effects layer as the card resolves; `director` a motif on a `death`,
 * `trigger` or `activate` hook, which the board does not draw yet, played through the gallery's own
 * director in the hook's palette at the card's scale; `recipe` a bespoke card recipe on its fixture;
 * `setPiece` a board clear's set piece on the wipe fixture; `aura` an entry with only an aura, which
 * stands while the card does.
 */
export type GalleryCardPlay =
  | { kind: "board"; motif: DrawnMotif }
  | { kind: "director"; motif: DrawnMotif; palette: FxPalette }
  | { kind: "recipe"; key: CardRecipeKey }
  | { kind: "setPiece" }
  | { kind: "aura" };

/** The hooks the board does not draw yet, in the order the gallery picks one. */
const DIRECTOR_HOOKS = ["death", "trigger", "activate"] as const;

/**
 * #2793: how the gallery plays `entry`, or null for a `none` entry and a bespoke entrance (whose card's
 * summon the hotseat plays). A motif on the play wins over the other hooks, any hook over an aura.
 */
export function galleryPlayOf(defId: string, entry: FxCardEntry): GalleryCardPlay | null {
  switch (entry.kind) {
    case "none":
      return null;
    case "bespoke":
      return isCardRecipeKey(entry.key) ? { kind: "recipe", key: entry.key } : null;
    case "setPiece":
      return { kind: "setPiece" };
    case "hooks": {
      const play = entry.hooks.cast ?? entry.hooks.cry;
      if (play !== undefined && isDrawnMotif(play.motif)) return { kind: "board", motif: play.motif };
      for (const hook of DIRECTOR_HOOKS) {
        const spec = entry.hooks[hook];
        if (spec === undefined || !isDrawnMotif(spec.motif)) continue;
        return { kind: "director", motif: spec.motif, palette: spec.palette ?? flavorOf(CATALOG[defId]) };
      }
      return entry.aura === undefined ? null : { kind: "aura" };
    }
  }
}

/**
 * #2793: each card recipe's fixture events, as `player` would see its play of `instanceId` resolve on
 * `view`: Hinder adds the next-refresh rider to the other player, Blood Ridden Glowy Jelly Bean makes the
 * viewer's first hand card Radiant at a life price, Crushing Walls takes wipe's fixture, Transmogulate
 * turns the viewer's first hand card into a Legendary, Zephrys adds its card to the hand and Craft a
 * Card adds its crafted card, then fuses it; #2799: Celestial Caller summons a Sun in lane 2 and a Moon
 * in lane 4, Dino-size sets the viewer's first unit to 10/10 and Goetzmann Game Design adds its card.
 */
export const RECIPE_FIXTURES: { readonly [K in CardRecipeKey]: (view: PlayerView, instanceId: string, player: PlayerId) => GameEvent[] } = {
  manaCrack: (_view, _instanceId, player) => [{ type: "modifierChanged", player: player === "p1" ? "p2" : "p1", modifierId: FX_NEXT_REFRESH_MODIFIER_ID, added: true }],
  bloodDrain: (view, instanceId, player) => [...motifFixture("radiance", view, instanceId, player), { type: "healthLost", player, amount: 3 }],
  crushingWalls: (view, instanceId, player) => motifFixture("wipe", view, instanceId, player),
  legendaryTide: (view) => {
    const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    return hand === undefined
      ? []
      : [{ type: "transformed", instanceId: hand.instanceId, fromDefId: hand.defId, toDefId: "core-052", newInstanceId: hand.instanceId }];
  },
  perfectWish: (_view, _instanceId, player) => [{ type: "addedToHand", player, instanceId: "fx-wished", defId: "core-055" }],
  fusionForge: (_view, _instanceId, player) => [
    { type: "addedToHand", player, instanceId: "fx-fused", defId: "core-055" },
    { type: "fused", instanceIds: ["fx-half-a", "fx-half-b"], resultInstanceId: "fx-fused", defId: "core-055" },
  ],
  celestialCall: (_view, _instanceId, player) => [
    { type: "summoned", player, instanceId: "fx-sun", defId: "traininggrounds-006", row: "units", lane: 2 },
    { type: "summoned", player, instanceId: "fx-moon", defId: "traininggrounds-007", row: "units", lane: 4 },
  ],
  dinoStomp: (view) => {
    const mine = unitsOf(view.you)[0];
    return mine === undefined
      ? []
      : [
          { type: "numberChanged", instanceId: mine.instanceId, defId: mine.defId, key: "attack", value: 10 },
          { type: "numberChanged", instanceId: mine.instanceId, defId: mine.defId, key: "health", value: 10 },
        ];
  },
  gameDesign: (_view, _instanceId, player) => [{ type: "addedToHand", player, instanceId: "fx-designed", defId: "core-055" }],
};

/**
 * #2793: a card's table entry played out: the viewer plays the n-th play's id, its play's fixture
 * resolves under it (a set piece's on its hook's marker), and it resolves. `director` and `aura` play
 * nothing under it: the director draws the one, the card standing on the board the other.
 */
export function galleryCardEvents(defId: string, play: GalleryCardPlay, view: PlayerView, n = 0): GameEvent[] {
  const instanceId = galleryPlayId(n);
  const player = view.viewer;
  const under = ((): GameEvent[] => {
    switch (play.kind) {
      case "board":
        return motifFixture(play.motif, view, instanceId, player);
      case "recipe":
        return RECIPE_FIXTURES[play.key](view, instanceId, player);
      case "setPiece":
        return [{ type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: "cry" }, ...motifFixture("wipe", view, instanceId, player)];
      case "director":
      case "aura":
        return [];
    }
  })();
  return [
    { type: "cardPlayed", player, instanceId, defId, costPaid: 1 },
    ...under,
    { type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 },
  ];
}

/**
 * #2793: the view a card plays over: a Field Spell's or a Field Trap's own, face up in the viewer's
 * first backrow zone, so its aura stands there (fx/aura/plan.ts); any other card's the view as it is.
 */
export function galleryFieldView(defId: string, view: PlayerView): PlayerView {
  const type = CATALOG[defId]?.type;
  const first = view.you.backrow[0];
  if (type === undefined || !isAuraFieldType(type) || first === null || first === undefined || first.faceDown) return view;
  return { ...view, you: { ...view.you, backrow: [{ ...first, defId, type }, ...view.you.backrow.slice(1)] } };
}
