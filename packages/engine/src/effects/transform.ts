// Transform and Vanilla (SPEC §6.3): the two verbs that rewrite what a card is, so Immutable
// refuses both (R23). A Transform puts a new instance of another definition in the same zone and
// position, with no Cry (R1); the card it replaces ceases to exist rather than reaching a graveyard
// (R35). Vanilla is a flag the layers read (§10.4): stats, buffs and damage are other layers and
// must survive it, and the definition is shared by every copy of the card, so it is never edited.

import type { CardDef, CardType, PlayerId, Row } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { defOf, excludingDefId, pickGenerated, query, type CatalogQueryArgs } from "../catalog";
import { addEnchantment, enchantmentsOfKind } from "../enchantments";
import { cardTypeOf } from "../faces";
import { unitHas } from "../layers";
import { isTemporaryCard } from "../temporary";
import type { Effect, EffectContext } from "../script";
import { newInstance, type CardInstance } from "../state";
import { ceaseToExist, isCarried, moveToZone, pileAt, replaceInZone, slotOf, zoneOf, type OffFieldZone } from "../zones";
import { cloneOf } from "./summon";
import { instanceOnItsStay, resolveTarget, type TargetSpec } from "./targets";

/**
 * Which card to rewrite: the pick the play or a prompt carried (R81), or an instance id a trigger
 * read off the event it answers — Sheepish transforms the unit the opponent just played (#41).
 */
export type TransformTarget = { target?: TargetSpec; instanceId?: string };

function instanceOf(ctx: EffectContext, args: TransformTarget): CardInstance | null {
  // R174: a card named by id is aimed at the stay it had when the run began (`instanceOnItsStay`).
  if (args.instanceId !== undefined) return instanceOnItsStay(ctx, args.instanceId);
  const target = resolveTarget(ctx, args.target ?? { of: "chosen" });
  if (target === null || target.kind !== "unit") return null;
  return target.instance;
}

/** §5.1: the row a type lives in; a Spell is never a permanent, so it can never replace one. */
function rowFor(type: CardType): Row | null {
  if (type === "Unit") return "units";
  if (type === "Spell") return null;
  return "backrow";
}

/**
 * §6.3 Replace: the same zone and position, with the old card's owner and controller. The new card
 * takes the old one's place (`zones.replaceInZone`) rather than being summoned into an emptied zone:
 * a Transform result is no summon (§6.2), so the Lock §3.2 puts on a zone — "accepts no summons … the
 * current occupant is unaffected" — does not refuse it, and neither does a reservation (R64). #36
 * Magic Jammed locks the zone of a Heroic Power it could not destroy (R46), and radiant #36 locks
 * the zone of a card it could not steal (R15); R35 still replaces either. A Stack pile keeps its
 * dormant cards beneath the replacement (§3.2).
 */
function replaceOnField(ctx: EffectContext, old: CardInstance, def: CardDef, radiant: boolean): CardInstance | null {
  const at = slotOf(ctx.state, old);
  if (at === null) return null;
  // R446: a Unit a carrier holds stands in a backrow zone as a Unit, and a Unit may take its place there.
  if (rowFor(def.type) !== (isCarried(ctx.state, old) ? "units" : at.row)) return null;

  const replacement = newInstance(ctx.state, def.id, old.owner, zoneOf(at));
  replacement.radiant = radiant;
  if (old.position !== undefined) replacement.position = old.position;
  // R659: a new body enters the field this turn, so it is summoning sick like a summoned card (§4.1);
  // only text that says otherwise lifts it (R424).
  replacement.summonedTurn = ctx.state.turn;

  if (!replaceInZone(ctx.state, old, replacement)) return null;
  // The replaced card ceases to exist: no graveyard, no exile pile, no Death trigger (§6.3, R35) —
  // and it has left the field, which R174 counts like any departure (`zones.ceaseToExist`).
  ceaseToExist(ctx.state, old);
  // §3.2: a Field Spell is public where a Trap stays face-down until it fires (R33).
  if (def.type === "Field Spell") replacement.faceUp = true;
  return replacement;
}

/**
 * R35's other zones: Transmogulate replaces hand, library, graveyard and exile cards too, "same
 * counts" per zone. A library and a hand keep their order, so the replacement takes the old card's
 * index there; `moveToZone` appends in the other piles.
 */
function replaceOffField(ctx: EffectContext, old: CardInstance, def: CardDef, radiant: boolean): CardInstance | null {
  const zone = old.zone.z;
  // A card mid-resolution, or one that has ceased to exist (R11), is in no pile to replace it in.
  if (zone === "field" || zone === "resolving" || zone === "gone") return null;
  const at: OffFieldZone = zone;
  // R11: a unit-token card cannot sit in a graveyard or exile, so a replacement that would cease
  // to exist on arrival is refused instead of thinning the zone.
  if ((at === "graveyard" || at === "exile") && def.token && def.type === "Unit") return null;

  const owner = old.owner;
  const pile = ctx.state.players[owner][at];
  const index = pile.findIndex((card) => card.id === old.id);
  if (index < 0) return null;

  ceaseToExist(ctx.state, old);
  // R312: a new instance with no `knownAs`, so a library replacement is a card its owner was never
  // shown, and their list counts it unknown (the `transformed` event names it to nobody, R177).
  const replacement = newInstance(ctx.state, def.id, owner, { z: at, player: owner });
  replacement.radiant = radiant;
  moveToZone(ctx.state, replacement, at, { position: index });
  // R671: a hand keeps its order too, so a card swapped in hand stays where its owner put it.
  if (at === "hand") {
    const hand = ctx.state.players[owner].hand;
    hand.splice(hand.indexOf(replacement), 1);
    hand.splice(index, 0, replacement);
  }
  return replacement;
}

/**
 * §6.3 Transform: a new instance of `defId` where the old card was, no Cry (R1), and the old card
 * ceases to exist. Immutable refuses it (R23), and so does a definition that cannot live in the
 * zone the old card occupies (§5.1).
 */
export function transform(args: TransformTarget & { defId: string; radiant?: boolean }): Effect {
  return {
    kind: "transform",
    apply(ctx): void {
      const old = instanceOf(ctx, args);
      if (old === null || !transformable(ctx, old)) return;
      replaceCard(ctx, old, defOf(ctx.state, args.defId), args.radiant === true);
    },
  };
}

/**
 * R23: Immutable blocks a Transform, which is what a Replace is on the field (§6.3). Off the field a
 * Replace is no Transform — the card is not rewritten, it ceases to exist and another takes its
 * place — so R35's "other zones: any card from the pool, same counts" replaces an Immutable card too,
 * and a library keeps its count whatever it held (§9.1).
 */
function transformable(ctx: EffectContext, old: CardInstance): boolean {
  return !(old.zone.z === "field" && unitHas(ctx.state, old, "Immutable"));
}

/** Replace `old` with a new card of `def` where it is, and report it; null when the zone refuses it. */
function replaceCard(ctx: EffectContext, old: CardInstance, def: CardDef, radiant: boolean): CardInstance | null {
  const fromDefId = old.defId;
  const hiddenFrom = unreadableBy(ctx, old);
  const replacement =
    old.zone.z === "field" ? replaceOnField(ctx, old, def, radiant) : replaceOffField(ctx, old, def, radiant);
  if (replacement === null) return null;

  ctx.events.push({
    type: "transformed",
    instanceId: old.id,
    fromDefId,
    toDefId: replacement.defId,
    newInstanceId: replacement.id,
    ...(hiddenFrom.length === 0 ? {} : { hiddenFrom }),
  });
  return replacement;
}

/**
 * R177: who could not read this card where it is, read before it ceases to exist there — a library
 * card is hidden from both players (§9.1), a hand card from the other one, and a face-down trap from
 * everyone but its controller (R33). A card that ceases to exist leaves no zone of its own to be
 * judged by later, so the event records this for the view.
 */
function unreadableBy(ctx: EffectContext, card: CardInstance): PlayerId[] {
  const zone = card.zone;
  if (zone.z === "library") return [...PLAYER_IDS];
  if (zone.z === "hand") return [opponentOf(zone.player)];
  if (zone.z !== "field" || zone.row !== "backrow" || card.faceUp === true) return [];
  const type = cardTypeOf(ctx.state, card);
  return type === "Trap" || type === "Field Trap" ? [opponentOf(card.controller)] : [];
}

/**
 * §6.3 Vanilla: the card's text stops applying — its printed keywords and its scripts — while its
 * stats, buffs and damage stay. Immutable refuses it (R23), and a card that is already Vanilla is
 * unchanged. The flag lives on the instance; the layers read it (§10.4) and R78 clears it when the
 * card leaves the field.
 */
export function vanilla(args: TransformTarget = {}): Effect {
  return {
    kind: "vanilla",
    apply(ctx): void {
      const card = instanceOf(ctx, args);
      if (card === null) return;
      if (unitHas(ctx.state, card, "Immutable")) return;
      if (card.vanilla) return;

      card.vanilla = true;
      // No instance is created and no definition changes, so both sides of `transformed` name the
      // same card and the same def: the event the client animates is the text going away.
      ctx.events.push({
        type: "transformed",
        instanceId: card.id,
        fromDefId: card.defId,
        toDefId: card.defId,
        newInstanceId: card.id,
      });
    },
  };
}

// ---------------------------------------------------------------------------------------------
// Patch v0.2.0: the Transform variants (docs/classic-sets.md B5 E24; §6.3 Transform, R23, R35).
// ---------------------------------------------------------------------------------------------

/**
 * Whether `def` could take `old`'s place: on the field only a definition of the same row, as R35
 * keeps a board card's type (a Unit for a Unit); in a graveyard or exile never a unit token (R11).
 */
function canReplace(old: CardInstance, def: CardDef): boolean {
  const zone = old.zone;
  if (zone.z === "field") return rowFor(def.type) === zone.row;
  if (zone.z === "graveyard" || zone.z === "exile") return !(def.token && def.type === "Unit");
  return zone.z === "hand" || zone.z === "library";
}

/**
 * E24, Classic+ #73.1 Classic Golem: "it transforms into a random Classic or Classic+ Unit". A §6.3
 * Transform into a definition drawn with `ctx.rng` from `query` (§5.1's one pool, never the running
 * card or its ingredients, B4.1), narrowed to the definitions that could take the card's place — on
 * the field, one of its row (R35) — so the draw never lands on one the zone would refuse. Immutable
 * refuses it (R23), and a refusal or an empty pool draws nothing (R129). `radiant` is the new card's
 * face: `true`, `false` (the default), or `"keep"` the old card's. `readyToAttack` is Classic Golem's
 * "the new Unit may attack again this turn" (R424): the new body on the field is not summoning sick
 * this turn (a new instance's exertion is already fresh).
 */
export function transformRandom(
  args: TransformTarget & { query?: CatalogQueryArgs; radiant?: boolean | "keep"; readyToAttack?: boolean },
): Effect {
  return {
    kind: "transformRandom",
    apply(ctx): void {
      const old = instanceOf(ctx, args);
      if (old === null || !transformable(ctx, old)) return;
      const pool = query(excludingDefId(args.query ?? {}, ctx.self?.defId ?? ctx.defId)).filter((def) =>
        canReplace(old, def),
      );
      const def = pickGenerated(ctx.rng, pool);
      if (def === undefined) return;
      const radiant = args.radiant === "keep" ? old.radiant : args.radiant === true;
      const replacement = replaceCard(ctx, old, def, radiant);
      if (replacement !== null && args.readyToAttack === true && replacement.zone.z === "field") {
        delete replacement.summonedTurn;
      }
    },
  };
}

/**
 * E24, Classic+ #4 Juhan Biggest Bat: "the cards beneath it become copies of this". Every card dormant
 * beneath the top of the unit pile `of` names (default the running card, which must be that top) is
 * Replaced (§6.3) by a copy of the top — R57's copy, with the top's face, buffs, granted keywords,
 * Vanilla state and X/X — that keeps the old card's owner and controller, its position and its place
 * in the pile, and stays dormant (R13): when the top leaves, the next copy resumes. An Immutable card
 * stays as it is (R23). Each replaced card ceases to exist (R35) with a `transformed` event.
 */
export function transformBeneath(args: { of?: TargetSpec } = {}): Effect {
  return {
    kind: "transformBeneath",
    apply(ctx): void {
      const target = resolveTarget(ctx, args.of ?? { of: "self" });
      if (target === null || target.kind !== "unit") return;
      const top = target.instance;
      const at = slotOf(ctx.state, top);
      if (at === null || at.row !== "units") return;
      const pile = pileAt(ctx.state, at);
      if (pile === null || pile[0]?.id !== top.id) return;
      for (const old of pile.slice(1)) {
        if (unitHas(ctx.state, old, "Immutable")) continue;
        const copy = cloneOf(ctx, top, old.owner, { of: { of: "self" } });
        if (old.position !== undefined) copy.position = old.position;
        // A new body in the pile, like any Transform result (§4.1).
        copy.summonedTurn = ctx.state.turn;
        const hiddenFrom = unreadableBy(ctx, old);
        if (!replaceInZone(ctx.state, old, copy)) continue;
        copy.controller = old.controller;
        ceaseToExist(ctx.state, old);
        ctx.events.push({
          type: "transformed",
          instanceId: old.id,
          fromDefId: old.defId,
          toDefId: copy.defId,
          newInstanceId: copy.id,
          ...(hiddenFrom.length === 0 ? {} : { hiddenFrom }),
        });
      }
    },
  };
}

/**
 * Classic #55 Book of Wildfire, R671: "Becomes a different Book at the end of your turn" — the card
 * `instanceId` names, while it is in a hand, is Replaced (§6.3, R35) by a Book drawn with `ctx.rng`
 * from §5.1's pool: every non-token Book of every set (R380) but the one it is now and the card that
 * started the swap (`from`, Wildfire itself). The new Book keeps the old card's face (a Radiant card
 * becomes the Radiant face of the Book) and its place in the hand, and carries the swap on as the
 * `swapsBook` enchantment, so it changes again at its owner's next end of turn. It is otherwise a new
 * card, as any Transform result is (its cost changes and its other enchantments stay behind), except
 * that a Temporary card's new Book is Temporary too (R637): the swap comes before cleanup, and a
 * Temporary card must not leave the turn as a card that stays.
 */
export function swapBook(args: { instanceId: string; from: string }): Effect {
  return {
    kind: "swapBook",
    apply(ctx): void {
      const old = instanceOnItsStay(ctx, args.instanceId);
      if (old === null || old.zone.z !== "hand") return;
      const pool = query({ tags: ["Book"] }).filter((def) => def.id !== old.defId && def.id !== args.from);
      const def = pickGenerated(ctx.rng, pool);
      if (def === undefined) return;
      const temporary = isTemporaryCard(ctx.state, old);
      const replacement = replaceCard(ctx, old, def, old.radiant);
      if (replacement === null) return;
      addEnchantment(replacement, { kind: "swapsBook", from: args.from });
      if (temporary) replacement.grantedKeywords.push({ kind: "Temporary" });
    },
  };
}

/** R671: the card that started a card's Book swap, or null when it carries none. */
export function bookSwapSourceOf(card: CardInstance): string | null {
  return enchantmentsOfKind(card, "swapsBook")[0]?.from ?? null;
}
