// R2784000: what the /dev/fx gallery (routes/dev/fx.tsx) plays: a fixture event for every motif on a
// view's own ids, the entry it fills, and a card's table entry as a play that resolves (#2793, #2796:
// a motif, a card recipe, a set piece or a Field card's aura on the board; R2797000 a death, trigger
// or activate motif too, from its hook's marker). Pure, so the motif tests plan from the same fixtures
// the gallery shows. R2794000: it also plays a marquee Unit's entrance as the Unit lands, and Classic's
// Flourishes on their own fixtures. R3332000: an entry's `entrance` plays as its card lands, whatever the
// entry's kind, a Field Spell's face up in the backrow; R3332001 over the board its entrance reads
// (`galleryBoardOf`: Felinor Fiender's over three Felinors).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { DUO_MARK, DUO_MARK_COLOR } from "../../cards/marks.ts";
import { animationRow, scaleForSpeed, SET_PIECE_HOLD_MS, type AnimationEntry, type SetPieceClaimer } from "../../game/animations.ts";
import { FROZEN_MARK, FROZEN_MARK_COLOR } from "../../stats/config.ts";
import { CATALOG } from "../../wire/cards.ts";
import { isAuraFieldType } from "../aura/looks.ts";
import { entranceKeyOf } from "../cardFx.ts";
import type { CardRecipeKey } from "../cards/bespoke.ts";
import { pageMendFixture } from "../cards/classic/bookOfHeal.ts";
import { crownSwapFixture } from "../cards/classic/kingWagtoggle.ts";
import { recycleLoopFixture } from "../cards/classic/recycle.ts";
import { felinorBoard } from "../cards/core/felinorFiender.ts";
import type { EntranceKey } from "../cards/recipe.ts";
import type { FxCardEntry } from "../cards/tables.ts";
import { FX_NEXT_REFRESH_MODIFIER_ID } from "../constants.ts";
import { paletteLook, type FxPalette } from "../flavors.ts";
import { createFxMemory } from "../memory.ts";
import { SYSTEM_MOMENTS, type SystemMoment } from "../systemMoments.ts";
import type { FxCue } from "../types.ts";
import { planMotif } from "./index.ts";
import { isDrawnMotif, type DrawnMotif, type FxCardHook } from "./names.ts";

/** R2797000: the lane Imitated Instrumentality's first Field Spell boots in (the next two take the next lanes), and Drone Core X's Battleship's. */
const GALLERY_MAGI_LANE = 3;
const GALLERY_WARP_LANE = 3;

/** R2797000: the backrow lane a Trap the gallery plays fires from, the first, where a Field Trap stands (`galleryFieldView`). */
const GALLERY_TRAP_LANE = 1;

/** Whether a card of `type` fires as a Trap does (R2797000: its `cast` plays from its `trapFired`). */
function isTrapType(type: string | undefined): boolean {
  return type === "Trap" || type === "Field Trap";
}

export { SYSTEM_MOMENTS, type SystemMoment };

/** The instance id the gallery's card plays under; the n-th play takes `-n` after it, so each is new to the board. */
export const GALLERY_CARD_ID = "fx-gallery-card";

/** The instance id of the gallery's n-th card play. */
export function galleryPlayId(n: number): string {
  return n === 0 ? GALLERY_CARD_ID : `${GALLERY_CARD_ID}-${String(n)}`;
}

/** R2785000: the key of the gallery's fixture set piece. */
export const GALLERY_SET_PIECE = "fixture";

/** #2798: Stinkyscape #68 Stratos, the Unit Stratos Swarm fills a board with. */
const STRATOS = "stinkyscape-068";

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
 * nova blasts every enemy unit, lance hits lane 3, wipe destroys every
 * unit, vanish exiles the second enemy unit, negate counters a card, drain streams from the enemy hero
 * into its first unit, bless buffs and curse degrades the viewer's first unit,
 * conjure and rise summon a token, offering sinks the viewer's first unit as a cost, shift
 * swaps the board, bind locks an enemy zone, tether takes the first enemy unit, transmute
 * turns the viewer's third unit into a token, flight draws a card, mana
 * refills the viewer's crystals, tithe discounts the viewer's first hand card, tally ticks a counter,
 * snap fires a hidden trap, send puts a card in the hand, radiance makes the viewer's first hand card
 * Radiant and mend heals the viewer's first unit. Deal (#2796) adds a card to the player's hand and
 * shuffles a hidden one into their deck, seal lays a modifier on the enemy seat, recall (R2794000)
 * returns a card to the hand, and hex (R2797000) freezes the first enemy unit.
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
    case "transmute": {
      const mine = unitsOf(view.you)[2] ?? unitsOf(view.you)[0];
      return mine === undefined
        ? []
        : [{ type: "transformed", instanceId: mine.instanceId, fromDefId: mine.defId, toDefId: "token-sheep", newInstanceId: "fx-double" }];
    }
    case "flight":
      return [{ type: "drawn", player, instanceId: "fx-drawn", defId: "core-055" }];
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
    case "deal":
      return [
        { type: "addedToHand", player, instanceId: "fx-dealt", defId: "core-055" },
        { type: "shuffledIn", player, instanceId: "hidden", defId: "hidden", position: 0 },
      ];
    case "seal":
      return [{ type: "modifierChanged", player: enemy.player, modifierId: "fx-sealed", added: true }];
    case "recall":
      return [{ type: "addedToHand", player, instanceId: "fx-recalled", defId: "core-003" }];
    case "hex": {
      const foe = foes[0];
      return foe === undefined ? [] : [{ type: "marked", instanceId: foe.instanceId, mark: "frozen", color: "cyan", added: true }];
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
 * One system moment's fixture events off `fullBoardView` ids (own units, hand, heroes), mirroring
 * `motifFixture`: a quest ticks and completes on the viewer's third unit, jade and credits tick by
 * the hero, Market lots move between the hero, the Marketplace and the hand, ballots drop from the
 * viewer's first unit onto its hero, and the transform family transmutes the viewer's third unit.
 */
export function systemMomentFixture(moment: SystemMoment, view: PlayerView): GameEvent[] {
  const mine = unitsOf(view.you);
  const third = mine[2] ?? mine[0];
  const first = mine[0];
  const foe = unitsOf(view.opponent)[0];
  const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  switch (moment) {
    case "questProgressed":
      return third === undefined ? [] : [{ type: "questProgressed", player: view.viewer, instanceId: third.instanceId, quest: "fx-quest", progress: 1, goal: 3 }];
    case "questCompleted":
      return third === undefined ? [] : [{ type: "questCompleted", player: view.viewer, instanceId: third.instanceId, quest: "fx-quest" }];
    case "jadeChanged":
      return [{ type: "jadeChanged", player: view.viewer, value: 3 }];
    case "marketCreditsChanged":
      return [{ type: "marketCreditsChanged", player: view.viewer, value: 5 }];
    case "marketBought":
      return [{ type: "marketBought", player: view.viewer, instanceId: "fx-bought", defId: "core-002", credits: 2 }];
    case "marketSold":
      return [{ type: "marketSold", player: view.viewer, instanceId: hand?.instanceId ?? "fx-sold", defId: hand?.defId ?? "core-002", credits: 1 }];
    case "voteCast":
      return first === undefined
        ? []
        : [{ type: "voteCast", player: view.viewer, instanceId: first.instanceId, defId: first.defId, votes: 1, needed: 3 }];
    case "votedOut":
      return first === undefined ? [] : [{ type: "votedOut", player: view.opponent.player, ballots: [{ instanceId: first.instanceId, defId: first.defId }] }];
    case "starscapeTravelled":
      return [{ type: "starscapeTravelled", player: view.viewer, node: 1, reward: "draw" }];
    case "fled":
      return [{ type: "fled", player: view.opponent.player, opponent: "draw" }];
    case "glitched":
      return [{ type: "glitched", player: view.viewer, outcome: "swap" }];
    case "awakened":
      return third === undefined ? [] : [{ type: "awakened", player: view.viewer, instanceId: third.instanceId, defId: third.defId }];
    case "boardReplaced":
      return [{ type: "boardReplaced", player: view.viewer }];
    case "rolledBack":
      return [{ type: "rolledBack", player: view.viewer, turnsAgo: 1, sides: [view.viewer] }];
    case "fengShui":
      return first === undefined || foe === undefined
        ? []
        : [{ type: "fengShui", instanceId: first.instanceId, sourceId: foe.instanceId, player: view.viewer, outcome: "positive" }];
    case "transformed":
      return third === undefined
        ? []
        : [{ type: "transformed", instanceId: third.instanceId, fromDefId: third.defId, toDefId: "token-sheep", newInstanceId: "fx-double" }];
    case "fused": {
      const second = mine[1];
      return first === undefined || second === undefined
        ? []
        : [{ type: "fused", instanceIds: [first.instanceId, second.instanceId], resultInstanceId: "fx-fused", defId: "core-088" }];
    }
    case "upgraded":
      return third === undefined
        ? []
        : [{ type: "upgraded", instanceId: third.instanceId, defId: third.defId, change: { kind: "stats", attack: 1, health: 1 } }];
    case "degraded":
      return third === undefined
        ? []
        : [{ type: "degraded", instanceId: third.instanceId, defId: third.defId, change: { kind: "stats", attack: -1, health: -1 } }];
  }
}

/**
 * #2793: how the gallery plays a card's table entry. `board` is a motif on its play (`cast`, `cry`),
 * played through the board's effects layer as the card resolves; R2797000 `hook` a motif on a `death`,
 * `trigger` or `activate` hook, played the same way from the hook's own marker under the play; `recipe`
 * a bespoke card recipe on its fixture; `entrance` (R2794000, R3332000) an entry of an entrance alone,
 * played as its card lands; `setPiece` a board clear's set piece on the wipe fixture; `aura` an entry
 * with only an aura, which stands while the card does.
 */
export type GalleryCardPlay =
  | { kind: "board"; motif: DrawnMotif }
  | { kind: "hook"; hook: GalleryHook; motif: DrawnMotif }
  | { kind: "recipe"; key: CardRecipeKey }
  | { kind: "entrance"; key: EntranceKey }
  | { kind: "setPiece" }
  | { kind: "aura" };

/** The hooks besides the play's, in the order the gallery picks one. */
const GALLERY_HOOKS = ["death", "trigger", "activate"] as const satisfies readonly FxCardHook[];

type GalleryHook = (typeof GALLERY_HOOKS)[number];

/** R2797000: the marker each hook the gallery plays opens its entry with (a Whenever stands for every trigger). */
const GALLERY_HOOK_KINDS = { death: "death", trigger: "whenever", activate: "activate" } as const satisfies Readonly<Record<GalleryHook, string>>;

/**
 * #2793: how the gallery plays `entry`, or null for a `none` entry. A motif on the play wins over the
 * other hooks, any hook over an entrance (R3332000), an entrance over an aura.
 */
export function galleryPlayOf(defId: string, entry: FxCardEntry): GalleryCardPlay | null {
  switch (entry.kind) {
    case "none":
      return null;
    case "bespoke":
      return { kind: "recipe", key: entry.key };
    case "setPiece":
      return { kind: "setPiece" };
    case "hooks": {
      const play = entry.hooks.cast ?? entry.hooks.cry;
      if (play !== undefined && isDrawnMotif(play.motif)) return { kind: "board", motif: play.motif };
      for (const hook of GALLERY_HOOKS) {
        const spec = entry.hooks[hook];
        if (spec === undefined || !isDrawnMotif(spec.motif)) continue;
        return { kind: "hook", hook, motif: spec.motif };
      }
      if (entry.entrance !== undefined) return { kind: "entrance", key: entry.entrance };
      return entry.aura === undefined ? null : { kind: "aura" };
    }
  }
}

/**
 * #2793: each card recipe's fixture events, as `player` would see its play of `instanceId` resolve on
 * `view`: Hinder adds the next-refresh rider to the other player, Blood Ridden Glowy Jelly Bean makes the
 * viewer's first hand card Radiant at a life price, Crushing Walls takes wipe's fixture, Transmogulate
 * turns the viewer's first hand card into a Legendary, Zephrys adds its card to the hand and Craft a
 * Card adds its crafted card, then fuses it. R2795000: Rollback rolls both sides back a turn, KY's
 * Constant changes the cost of the viewer's first hand card, and Book of Pain discards two of the other
 * player's cards. #2796: Auspicious Rock costs its caster 2 and rolls a Jade into the hand, Spiritually
 * 中国 turns every one of the viewer's hand cards into one CN card and summons a CN Unit, and Harmony
 * Spirit sets every unit to 3/3. R2797000: Olivian Expanse adds its Discovered card, Imitated
 * Instrumentality summons its three Field Spells and Drone Core X its Battleship. #2798: iKwzki Duos
 * with the player's first unit, Flash Freeze freezes every unit on both sides and Stratos Swarm fills
 * the player's empty zones with Stratos (its first unit zone, a full board having none). #2799:
 * Celestial Caller summons a Sun in lane 2 and a Moon in lane 4, Dino-size sets the viewer's first unit
 * to 10/10 and Goetzmann Game Design adds its card. R2794000: Classic's Flourishes take their own
 * fixtures (fx/cards/classic/): Book of Heal heals the viewer's first unit, Recycle shuffles two cards
 * into the deck and draws one, and King Wagtoggle swaps the decks.
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
  rollbackSweep: (view, _instanceId, player) => [{ type: "rolledBack", player, turnsAgo: 1, sides: [view.you.player, view.opponent.player] }],
  constantStamp: (view) => {
    const hand = Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
    return hand === undefined ? [] : [{ type: "numberChanged", instanceId: hand.instanceId, defId: hand.defId, key: "cost", value: 3 }];
  },
  inkTear: (_view, _instanceId, player) => {
    const owner = player === "p1" ? "p2" : "p1";
    return [
      { type: "discarded", instanceId: "fx-discarded-1", defId: "core-055", owner },
      { type: "discarded", instanceId: "fx-discarded-2", defId: "core-002", owner },
    ];
  },
  jadeCrack: (_view, _instanceId, player) => [
    { type: "healthLost", player, amount: 2 },
    { type: "addedToHand", player, instanceId: "fx-jade", defId: "meditative-039-2" },
  ],
  // R2803000: Fortify Mind's judged guess, as `glimpse` drew it: the prediction judged on the badge.
  mindJudge: (_view, _instanceId, player) => [
    { type: "predicted", player, secretId: "fx-secret", guess: "attack", outcome: "won" },
    { type: "secretRevealed", player, secretId: "fx-secret", choice: "attack" },
  ],
  cnSweep: (view, _instanceId, player) => {
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
  terminusRift: (view, instanceId, player) => motifFixture("send", view, instanceId, player),
  magiBoot: (_view, _instanceId, player) =>
    (["stinkyscape-018-1", "stinkyscape-018-2", "stinkyscape-018-3"] as const).map((defId, k) => ({
      type: "summoned" as const,
      player,
      instanceId: `fx-magi-${k + 1}`,
      defId,
      row: "backrow" as const,
      lane: GALLERY_MAGI_LANE + k,
    })),
  // R3336000: the Drone Battleship's three layers, summoned bottom first into one zone (R1842000).
  battleshipWarp: (_view, _instanceId, player) =>
    (["stinkyscape-t-meltdown-layer", "stinkyscape-t-propagation-layer", "stinkyscape-t-shield-layer"] as const).map((defId, k) => ({
      type: "summoned" as const,
      player,
      instanceId: `fx-battleship-${k + 1}`,
      defId,
      row: "units" as const,
      lane: GALLERY_WARP_LANE,
    })),
  duoBond: (view, _instanceId, player) => {
    const mine = unitsOf(player === view.viewer ? view.you : view.opponent)[0];
    return mine === undefined ? [] : [{ type: "marked", instanceId: mine.instanceId, mark: DUO_MARK, color: DUO_MARK_COLOR, added: true }];
  },
  flashFreeze: (view) =>
    [...unitsOf(view.you), ...unitsOf(view.opponent)].map((unit) => ({ type: "marked", instanceId: unit.instanceId, mark: FROZEN_MARK, color: FROZEN_MARK_COLOR, added: true })),
  stratosSwarm: (view, _instanceId, player) => {
    const side = player === view.viewer ? view.you : view.opponent;
    const open = (row: "units" | "backrow", zones: readonly unknown[]): { row: "units" | "backrow"; lane: number }[] =>
      zones.flatMap((zone, k) => (zone === null ? [{ row, lane: k + 1 }] : []));
    const zones = [...open("units", side.units), ...open("backrow", side.backrow)];
    return (zones.length === 0 ? [{ row: "units" as const, lane: 1 }] : zones).map(({ row, lane }) => ({
      type: "summoned",
      player,
      instanceId: `fx-stratos-${row}-${String(lane)}`,
      defId: STRATOS,
      row,
      lane,
    }));
  },
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
  pageMend: pageMendFixture,
  recycleLoop: recycleLoopFixture,
  crownSwap: crownSwapFixture,
};

/** R2794000: the lane of the viewer's unit row a gallery entrance lands in. */
export const GALLERY_ENTRANCE_LANE = 3;

/** R3332000: the backrow lane a Field Spell's gallery entrance lands in, the first, where its aura stands (`galleryFieldView`). */
const GALLERY_FIELD_ENTRANCE_LANE = 1;

/** R3332001: the board an entrance reads, for one that reads it (Felinor Fiender's Felinors). */
const ENTRANCE_BOARDS: Partial<Record<EntranceKey, (view: PlayerView) => PlayerView>> = { felinorAssembly: felinorBoard };

/** R3332001: the board the gallery plays `defId` over: its entrance's own (`ENTRANCE_BOARDS`), else `view` itself. */
export function galleryBoardOf(defId: string, view: PlayerView): PlayerView {
  const key = entranceKeyOf(defId);
  const board = key === undefined ? undefined : ENTRANCE_BOARDS[key];
  return board === undefined ? view : board(view);
}

/**
 * #2793: a card's table entry played out: the viewer plays the n-th play's id, its play's fixture
 * resolves under it (a set piece's, and R2797000 a hook's, on its hook's marker, and a Trap's after its
 * own `trapFired`), and it resolves. An `entrance` and an `aura` play nothing under it: the card landing
 * draws the one, the card standing on the board the other. R2794000: a marquee Unit lands first, so its
 * entrance plays, then its play's fixture (Nature Titan's Cry); a set piece's play stays the board clear
 * alone. R3332000: a Field Spell with an entrance lands face up in the backrow's first zone.
 */
export function galleryCardEvents(defId: string, play: GalleryCardPlay, view: PlayerView, n = 0): GameEvent[] {
  const instanceId = galleryPlayId(n);
  const player = view.viewer;
  // R3332000: a Field Spell lands face up in the backrow, where its aura stands.
  const field = CATALOG[defId]?.type === "Field Spell";
  const zone = field ? ({ row: "backrow", lane: GALLERY_FIELD_ENTRANCE_LANE } as const) : ({ row: "units", lane: GALLERY_ENTRANCE_LANE } as const);
  const landing: GameEvent[] = play.kind !== "setPiece" && entranceKeyOf(defId) !== undefined ? [{ type: "summoned", player, instanceId, defId, ...zone }] : [];
  const under = ((): GameEvent[] => {
    switch (play.kind) {
      case "board": {
        // R2797000: a Trap's `cast` plays from its own firing, which snap's fixture is in its place.
        if (!isTrapType(CATALOG[defId]?.type)) return motifFixture(play.motif, view, instanceId, player);
        const fired: GameEvent = { type: "trapFired", instanceId, defId, controller: player, row: "backrow", lane: GALLERY_TRAP_LANE };
        return play.motif === "snap" ? [fired] : [fired, ...motifFixture(play.motif, view, instanceId, player)];
      }
      case "recipe":
        return RECIPE_FIXTURES[play.key](view, instanceId, player);
      case "setPiece":
        return [{ type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: "cry" }, ...motifFixture("wipe", view, instanceId, player)];
      case "hook":
        return [
          { type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: GALLERY_HOOK_KINDS[play.hook] },
          ...motifFixture(play.motif, view, instanceId, player),
        ];
      case "entrance":
      case "aura":
        return [];
    }
  })();
  return [
    { type: "cardPlayed", player, instanceId, defId, costPaid: 1 },
    ...landing,
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
