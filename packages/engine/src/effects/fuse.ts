// Fuse as a verb (SPEC §6.3 Fuse, R77, R102; §8.4 #85, §8.5 #99).
//
// `subsystems/fuse.ts` is R77 in full — the transient definition with both faces fused, the summed
// stats, the united keywords and tags, the concatenated scripts, `min(sum, FUSE_COST_CAP)`, the
// target instance kept with its zone, damage, exertion, counters and memory, the other ingredients
// ceasing to exist with no death, and Craft a Card's fresh `costOverride` 0 hand card. NONE of that
// is repeated here and none of it may be: it is eighteen kilobytes of rules with its own test file,
// and a second implementation is a second set of rules. This file exists only because `fuse` takes
// an `EngineSink` and mutates state, which a card script may not do (CLAUDE.md rule 5), and because
// `FuseArgs.ingredients` is `readonly CardInstance[]` while a Discover hands over catalog ids.
//
// AN INGREDIENT THAT WAS NEVER A CARD (§8.5 #99). Craft a Card's ingredients are Discovered
// DEFINITIONS: nobody ever saw them on a board, and R77 has them cease to exist the moment the
// fusion is made. An ingredient only ever contributes its definition to the fusion, so the
// cheapest faithful thing is an instance that sits in no pile at all: `{ z: "gone" }`, which
// `@jackioh/shared`'s `Zone` really does offer and which `subsystems/fuse.ts` itself uses for
// "ceased to exist" (R11, R86). No zone event fires for it, `findInstance` cannot reach it, and
// nothing in `viewFor` lists it, so the ingredient is invisible exactly as #99 requires.
//
// The `fused` event, `state.transientDefs` and the script registration are all the subsystem's and
// are deliberately untouched here.

import type { CardType, PlayerId, Selection } from "@jackioh/shared";
import { defOf, excludingDefId, query, type CatalogQueryArgs } from "../catalog";
import { cardTypeOf } from "../faces";
import { unitHas } from "../layers";
import {
  closePrompt,
  inOfferedOrder,
  openPrompt,
  registerPromptAnswerer,
  resumeAt,
  resumeOf,
  whyAnswerRefused,
  type AnswerInput,
} from "../prompts";
import type { EngineSink } from "../resolve";
import { FUSE_MIN_INGREDIENTS, fuse, type HandPrice } from "../subsystems/fuse";
import type { Effect, EffectContext } from "../script";
import { findInstance, newInstance, type CardInstance, type GameState } from "../state";
import { beginWorkCascade, drainWork } from "../work";
import { cardAt, isBuried, slotsOf } from "../zones";
import { exile } from "./move";
import { instanceOf, playerOf, type PlayerSpec, type TargetSpec } from "./targets";

/**
 * R77 and R86: an ingredient that is only a definition. It is created in `{ z: "gone" }` — in no
 * pile, so no zone event fires, nothing can target it and nothing can bring it back — which is the
 * same zone the subsystem moves a consumed ingredient to.
 */
function phantomIngredient(ctx: EffectContext, defId: string, owner: PlayerSpec | undefined): CardInstance {
  const player = owner === undefined ? ctx.controller : playerOf(ctx, owner);
  return newInstance(ctx.state, defId, player, { z: "gone", player });
}

/**
 * §6.3 Fuse per R77, wrapped. The ingredients are named in two ways, in this order: cards that
 * already exist (`instanceIds`, #85 Unlicensed Experimentation's played permanent) and then
 * definitions that never were cards (`defIds`, #99 Craft a Card's Discover picks). No Core card
 * passes both, so the order between the two groups is a convention rather than a rule; within a
 * group the caller's order is kept, because it is the order the fused name, text and scripts are
 * concatenated in.
 *
 * ONE FUSION AT A TIME, AND WHY THE LOOP IS IN HERE (R77, §8.4 #85 radiant). Radiant #85 fuses its
 * played permanent "onto each matching permanent separately", each fusion its own transient
 * definition. That CANNOT be a card emitting one `fuseCards` per target: the subsystem's
 * `ceaseToExist` calls `removeFromAnyZone`, so after the first fusion the played card is in
 * `{ z: "gone" }` and held by no pile — `findInstance` cannot reach it and its id cannot recover
 * it, so every later call would see one ingredient, fall below `FUSE_MIN_INGREDIENTS` and answer
 * `null`. The subsystem anticipates exactly this ("an ingredient that already ceased to exist in an
 * earlier fusion still fuses", because an ingredient only ever contributes its DEFINITION), and the
 * way to honour it is to resolve the ingredients ONCE and reuse those same objects across the
 * fusions — which only something holding them can do. Hence `targetInstanceIds` and the loop here.
 *
 * R86 is about POOLS, not about this: it drops ids whose cards are gone when the card said "a
 * random card you played this turn", so a pool degrades instead of fizzling at random. An
 * `instanceIds` entry is an instruction the caller named outright, so it is resolved once and then
 * kept — dropping it mid-loop would turn a deliberate second fusion into a silent no-op.
 *
 * It fizzles silently and the card still resolves (§6.3): an `instanceIds` entry that never
 * resolved at all drops out, an empty target list after filtering fuses nothing, and the subsystem
 * itself answers `null` — changing nothing — for fewer than `FUSE_MIN_INGREDIENTS`, for a target
 * that is off the field or Immutable (R23), and for a call that names neither a target nor a hand.
 */
export function fuseCards(args: {
  /** Ingredients that are definitions only: #99's Discover picks. */
  defIds?: readonly string[];
  /** Ingredients that already exist as cards: #85's played permanent. */
  instanceIds?: readonly string[];
  /** R77's kept instance: the on-field ingredient the result becomes. #99 never passes one. */
  targetInstanceId?: string;
  /**
   * Several kept instances, one fusion each in the order given (#85 radiant's "onto every such
   * permanent"). `targetInstanceId` is the one-target spelling of the same thing.
   */
  targetInstanceIds?: readonly string[];
  /**
   * #85 base is "a random permanent of yours of that type", so the draw is one `ctx.rng.pick` over
   * the candidates INSIDE apply — never at construction time, or the draw escapes the reducer
   * (§9.3, §10.7). Default "all", which is also the only behaviour a single target can have.
   */
  pick?: "random" | "all";
  /** R77's Craft a Card path: whose hand the fresh `costOverride` 0 result goes to. */
  toHand?: PlayerSpec;
  /** R352: the hand card's price, `"free"` (#99, the default) or R77's `"fused"` cost (#98's Stitching). */
  handPrice?: HandPrice;
  /** R352: the hand card is made Radiant (#98's radiant Stitching). */
  radiant?: boolean;
  // ---- v0.2.0, generation (E23, R469, R470) ----
  /**
   * R470: the kept instance is this card in a hand or a library, which stays where it is (Classic+
   * #31 Fusion Lab's pick, Classic #78 Radiant's). `targetInstanceId` is the field's spelling.
   */
  intoInstanceId?: string;
  /**
   * R469: every ingredient but the kept card goes in on its Radiant face — "a Radiant copy of it is
   * fused into this" (Classic+ #74 Radiant), "fuse 3 random Radiant AI generated cards" (#43 Radiant).
   */
  radiantIngredients?: boolean;
  /** R470: the kept card keeps the cost it had ("its cost doesn't change"). */
  keepCost?: boolean;
}): Effect {
  return {
    kind: "fuseCards",
    apply(ctx): void {
      // Resolved ONCE, before any fusion: these objects are reused for every fusion below, so an
      // ingredient the first fusion consumed still contributes its definition to the second.
      const ingredients: CardInstance[] = [];
      for (const instanceId of args.instanceIds ?? []) {
        const found = findInstance(ctx.state, instanceId);
        if (found !== undefined) ingredients.push(found);
      }
      for (const defId of args.defIds ?? []) {
        ingredients.push(phantomIngredient(ctx, defId, args.toHand));
      }

      const targetIds =
        args.targetInstanceIds ??
        (args.targetInstanceId === undefined ? [] : [args.targetInstanceId]);
      const targets: CardInstance[] = [];
      for (const instanceId of targetIds) {
        const found = findInstance(ctx.state, instanceId);
        if (found !== undefined) targets.push(found);
      }

      const toHand =
        args.toHand === undefined
          ? {}
          : {
              toHand: playerOf(ctx, args.toHand),
              ...(args.handPrice === undefined ? {} : { handPrice: args.handPrice }),
              ...(args.radiant === true ? { radiant: true } : {}),
            };
      // R469, R470: which ingredients go in Radiant, and whether the kept card keeps its cost.
      const terms = {
        ...(args.radiantIngredients === true ? { radiantIngredients: ingredients.map((card) => card.id) } : {}),
        ...(args.keepCost === true ? { keepCost: true } : {}),
      };

      // R470: a kept card in a hand or a library, which stays where it is.
      if (args.intoInstanceId !== undefined) {
        const into = findInstance(ctx.state, args.intoInstanceId);
        if (into === undefined) return;
        fuse(ctx, { ingredients, into, ...terms });
        return;
      }

      // #99's path: no kept instance at all, so one fusion into a hand.
      if (targetIds.length === 0) {
        // `EffectContext` satisfies `EngineSink`, so the subsystem takes the context as it stands.
        fuse(ctx, { ingredients, ...toHand, ...terms });
        return;
      }

      // Nothing left to fuse onto: the trap fired and did nothing (R61), taking no rng draw.
      if (targets.length === 0) return;

      const chosen =
        args.pick === "random" ? [ctx.rng.pick(targets)].flatMap((card) => card ?? []) : targets;

      // R77: each target is its own fusion, in order, each with its own transient definition.
      for (const target of chosen) {
        fuse(ctx, { ingredients, target, ...toHand, ...terms });
      }
    },
  };
}

// ---------------------------------------------------------------------------------------------
// Patch v0.2.0: the Fuse variants (docs/classic-sets.md B5 E23; R77, R102, R468–R470).
// ---------------------------------------------------------------------------------------------

/**
 * B4.1, R387: a random pool never offers the card generating from it — nor, for a fused card, any of
 * its ingredients (`catalog.selfDefIds`).
 */
function poolFor(ctx: EffectContext, asked: CatalogQueryArgs | undefined): ReturnType<typeof query> {
  return query(excludingDefId(asked ?? {}, ctx.self?.defId ?? ctx.defId));
}

/**
 * R23, R470: a card a Fuse may keep — on the field and acting (R77's target), or in a hand or a
 * library (`into`) — and not Immutable.
 */
function keepable(state: GameState, card: CardInstance): boolean {
  const zone = card.zone.z;
  if (zone === "field" ? isBuried(state, card) : zone !== "hand" && zone !== "library") return false;
  return !unitHas(state, card, "Immutable");
}

/** Where a random ingredient is fused: one named card, or every card of a hand or a library. */
export type FuseInto = { target: TargetSpec } | { pile: "hand" | "library"; player?: PlayerSpec };

/** The kept cards `into` names, read once as the effect applies, in pile order (a library top down). */
function keptCards(ctx: EffectContext, into: FuseInto): CardInstance[] {
  if ("target" in into) {
    const card = instanceOf(ctx, into.target);
    return card === null ? [] : [card];
  }
  const side = ctx.state.players[playerOf(ctx, into.player ?? "self")];
  return [...(into.pile === "hand" ? side.hand : side.library)];
}

/**
 * E23: "fuse a random card into a card in your hand; its cost doesn't change" (Classic+ #31 Fusion
 * Lab, whose Radiant face fuses "a random Radiant card") and "fuse all cards in your deck with a
 * random card, they maintain their original cost" (Classic+ #73). For each kept card — the one
 * `into.target` names, or every card of `into.pile`, read once, a library top down — one random card
 * of the pool (§5.1 through `catalog.query`, never the running card or its ingredients, B4.1) is fused
 * into it per R77: the kept card is the instance, keeps its zone and its type, and keeps the cost it
 * had (R470, `keepCost`, default on); a Radiant pick goes in on its Radiant face (R469). Each fusion is
 * its own transient definition (R102) and draws its own pick; with no kept card or an empty pool the
 * effect does nothing and draws nothing (R129). A kept card on the field is R77's `target`; a hand or
 * library card stays where it is (R470) and its `fused` event is hidden as the card is (§10.8).
 */
export function fuseRandomInto(args: {
  into: FuseInto;
  query?: CatalogQueryArgs;
  radiant?: boolean;
  keepCost?: boolean;
}): Effect {
  return {
    kind: "fuseRandomInto",
    apply(ctx): void {
      const kept = keptCards(ctx, args.into);
      if (kept.length === 0) return;
      const pool = poolFor(ctx, args.query);
      if (pool.length === 0) return;
      for (const card of kept) {
        // A card an earlier fusion of this list has taken off its pile is no longer one to fuse into,
        // and one the Fuse would refuse (R23, R470) draws nothing for it (R129).
        if (findInstance(ctx.state, card.id) === undefined || !keepable(ctx.state, card)) continue;
        const picked = ctx.rng.pick(pool);
        if (picked === undefined) return;
        const ingredient = newInstance(ctx.state, picked.id, card.owner, { z: "gone", player: card.owner });
        const onField = card.zone.z === "field";
        fuse(ctx, {
          ingredients: [ingredient],
          ...(onField ? { target: card } : { into: card }),
          ...(args.radiant === true ? { radiantIngredients: [ingredient.id] } : {}),
          ...(args.keepCost === false ? {} : { keepCost: true }),
        });
      }
    },
  };
}

/**
 * E23: "fuse 3 random AI generated cards and add the result to your hand; it costs (0)" (Classic+
 * #43 AI Slop; Radiant: "3 random Radiant AI generated cards"). `count` independent picks from the
 * pool (generated cards may repeat, R60; never the running card or its ingredients, B4.1), fused per
 * R77 with no target: the shared type, else the first pick's (R102); a Token when every pick is one;
 * the result a fresh card in the hand, at `handPrice` (default `"free"`, a `costOverride` of 0, R352)
 * and burned by a full hand (R4). A Radiant pick goes in on its Radiant face (R469). Fewer than two
 * picks is no fusion, so it draws nothing (R77, R129), and neither does an empty pool.
 */
export function fuseGenerated(args: {
  count: number;
  query?: CatalogQueryArgs;
  radiant?: boolean;
  toHand?: PlayerSpec;
  handPrice?: HandPrice;
}): Effect {
  return {
    kind: "fuseGenerated",
    apply(ctx): void {
      const count = Math.trunc(args.count);
      if (count < FUSE_MIN_INGREDIENTS) return;
      const pool = poolFor(ctx, args.query);
      if (pool.length === 0) return;
      const player = playerOf(ctx, args.toHand ?? "self");
      const ingredients: CardInstance[] = [];
      for (let at = 0; at < count; at += 1) {
        const picked = ctx.rng.pick(pool);
        if (picked === undefined) return;
        ingredients.push(newInstance(ctx.state, picked.id, player, { z: "gone", player }));
      }
      fuse(ctx, {
        ingredients,
        toHand: player,
        handPrice: args.handPrice ?? "free",
        ...(args.radiant === true ? { radiantIngredients: ingredients.map((card) => card.id) } : {}),
      });
    },
  };
}

/** The piles `fuseOntoYourCard` may offer the controller's cards from. */
export type FuseOntoPile = "field" | "hand" | "library";

/** R77, R35, R61: "of its type" — a Field Trap counts as a Trap, and a Trap as a Field Trap. */
function sameFuseType(a: CardType, b: CardType): boolean {
  const trap = (type: CardType): boolean => type === "Trap" || type === "Field Trap";
  return a === b || (trap(a) && trap(b));
}

/** Plain code-unit order: the same in every runtime, unlike `localeCompare`. */
function compareText(a: string, b: string): number {
  if (a === b) return 0;
  return a < b ? -1 : 1;
}

/**
 * The controller's cards of the ingredient's type, in the piles named: the field in lane order
 * (units, then backrow; the top of a pile only, R13), the hand in hand order, and the library in an
 * order its cards alone decide — name, definition, face, id — never its own, which is hidden from
 * everyone (§9.1, R310). An Immutable card is never a Fuse target (R23), and the ingredient itself is
 * not one of its own candidates.
 */
function fuseCandidates(
  state: GameState,
  player: PlayerId,
  ingredient: CardInstance,
  piles: readonly FuseOntoPile[],
): CardInstance[] {
  const type = cardTypeOf(state, ingredient);
  const fits = (card: CardInstance): boolean =>
    card.id !== ingredient.id && sameFuseType(cardTypeOf(state, card), type) && !unitHas(state, card, "Immutable");
  const out: CardInstance[] = [];
  if (piles.includes("field")) {
    for (const row of ["units", "backrow"] as const) {
      for (const ref of slotsOf(player, row)) {
        const card = cardAt(state, ref);
        if (card !== null && fits(card)) out.push(card);
      }
    }
  }
  const side = state.players[player];
  if (piles.includes("hand")) out.push(...side.hand.filter(fits));
  if (piles.includes("library")) {
    const library = side.library.filter(fits).sort((a, b) => {
      const left = defOf(state, a.defId);
      const right = defOf(state, b.defId);
      return (
        compareText(left.name, right.name) ||
        compareText(a.defId, b.defId) ||
        Number(a.radiant) - Number(b.radiant) ||
        compareText(a.id, b.id)
      );
    });
    out.push(...library);
  }
  return out;
}

/** The prompt `fuseOntoYourCard` opens, answered by `answerFuseOnto` below (R122). */
export const FUSE_ONTO_HOOK = "fuse:onto";

/** What the prompt carries to its answer: the ingredient to fuse onto the pick. */
type FuseOntoData = { ingredient: string };

function fuseOntoData(data: Record<string, unknown>): FuseOntoData | null {
  return typeof data.ingredient === "string" ? { ingredient: data.ingredient } : null;
}

/**
 * E23, Classic #78 Mutate Spell's Radiant face: "fuse it onto a card of yours of its type on your
 * field, in your hand or in your deck, or exile it if you have none". The card `target` names — on
 * the field, on the stay the run aimed at (R174) — is the ingredient; the controller picks the card
 * it is fused onto from their own cards of its type (`fuseCandidates`) in a `target` prompt of their
 * own, which the other player sees only as open (§10.6, R81): the deck's cards are shown to the
 * chooser and nobody else (§10.8, as KY's Private Tutor shows library cards). The answer fuses per
 * R77 onto the pick — kept on the field as R77's target, or in the hand or the deck where it is
 * (R470) — and the ingredient ceases to exist (R77, R86). With no candidate the ingredient is exiled
 * (`otherwise: "exile"`, the default) or left alone (`"nothing"`), with no prompt. A prompt splits
 * the list the effect stands in: the rest is parked and resumes after the answer (R113).
 */
export function fuseOntoYourCard(args: {
  target: TargetSpec;
  from?: readonly FuseOntoPile[];
  otherwise?: "exile" | "nothing";
}): Effect {
  return {
    kind: "fuseOntoYourCard",
    apply(ctx): void {
      const ingredient = instanceOf(ctx, args.target);
      if (ingredient === null || ingredient.zone.z !== "field" || isBuried(ctx.state, ingredient)) return;
      const candidates = fuseCandidates(ctx.state, ctx.controller, ingredient, args.from ?? ["field", "hand", "library"]);
      if (candidates.length === 0) {
        if (args.otherwise !== "nothing") exile({ target: { of: "instance", instanceId: ingredient.id } }).apply(ctx);
        return;
      }
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: "Choose a card of yours to fuse it onto",
        options: candidates.map((card) => ({
          key: `instance:${card.id}`,
          label: defOf(ctx.state, card.defId).name,
          selection: { pick: "instance", instanceId: card.id },
        })),
        resume: resumeAt({
          defId: ctx.self?.defId ?? ctx.defId ?? "",
          hook: FUSE_ONTO_HOOK,
          step: "onto",
          radiant: ctx.radiant,
          ...(ctx.self === null ? {} : { instanceId: ctx.self.id }),
          data: { ingredient: ingredient.id },
        }),
      });
    },
  };
}

/**
 * R122: the answer to `fuseOntoYourCard`'s prompt — validated as any prompt's, then the fusion onto
 * the pick, then what the prompt interrupted (R113). An ingredient no longer on the field, or a pick
 * that has moved to a pile it may not be kept in, fuses nothing.
 */
function answerFuseOnto(sink: EngineSink, answer: AnswerInput): string | null {
  const pending = sink.state.pending;
  if (pending === null) return "no prompt is open";
  const refused = whyAnswerRefused(pending, answer);
  if (refused !== null) return refused;
  const data = fuseOntoData(resumeOf(pending).data);
  if (data === null) return "that prompt carries no card to fuse";
  const picked: Selection | undefined = inOfferedOrder(pending, answer.selection)[0];

  closePrompt(sink);
  beginWorkCascade(sink);
  const ingredient = findInstance(sink.state, data.ingredient);
  const onto = picked?.pick === "instance" ? findInstance(sink.state, picked.instanceId) : undefined;
  if (ingredient !== undefined && ingredient.zone.z === "field" && onto !== undefined) {
    fuse(sink, { ingredients: [ingredient], ...(onto.zone.z === "field" ? { target: onto } : { into: onto }) });
  }
  drainWork(sink);
  return null;
}

registerPromptAnswerer(FUSE_ONTO_HOOK, answerFuseOnto);
