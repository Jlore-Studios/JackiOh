// The Field Guide's entries (SPEC §9.10, R2205000): each mechanic a lesson lets the player find, its
// one-line rule, and how the coach recognizes it from the human's view and the public catalog alone
// (R292, §5.1). A predicate decides nothing; it reads what the board already shows (CLAUDE.md rule 7).

import type { CardDef, GameEvent, PlayerView, UnitView } from "@jackioh/shared";

import type { CoachCtx } from "./coach.ts";
import { freshOf, heroTargetId, myHand, unitsOf } from "./steps.ts";

export type Discovery = {
  /** A stable slug, kept in the device's Field Guide (R2205002). */
  id: string;
  name: string;
  /** The rule in one line, after "You found {name}:". */
  rule: string;
  /** The lesson that sets it up; the Field Guide lists an entry not yet found under it. */
  lesson: string;
  found: (ctx: CoachCtx) => boolean;
};

const MIDLANE_MENACE = "traininggrounds-005";
const CHOSEN_BY_THE_SUN = "traininggrounds-006";
const CHOSEN_BY_THE_MOON = "traininggrounds-007";
const FLAME_TONGUE_TOTEM = "traininggrounds-021";
const RIGHT_HAND_FELINOR = "traininggrounds-022";
const SEANCE = "traininggrounds-037";
const BIG_FELINOR = "traininggrounds-051";
const COIN = "core-t-coin";
const SHOWPIECES: readonly string[] = ["Legendary", "Mythic"];

function defOf(ctx: CoachCtx, defId: string): CardDef | undefined {
  return ctx.defOf?.(defId);
}

function hasKeyword(def: CardDef | undefined, kind: string): boolean {
  return def?.base.keywords.some((keyword) => keyword.kind === kind) === true;
}

function says(def: CardDef | undefined, words: string): boolean {
  return def?.base.text.includes(words) === true;
}

function unitHas(unit: UnitView | undefined, kind: string): boolean {
  return unit?.keywords.some((keyword) => keyword.kind === kind) === true;
}

function myPlays(ctx: CoachCtx): { event: Extract<GameEvent, { type: "cardPlayed" }>; def: CardDef | undefined }[] {
  return freshOf(ctx, "cardPlayed")
    .filter((event) => event.player === ctx.view.viewer)
    .map((event) => ({ event, def: defOf(ctx, event.defId) }));
}

/** The traps this batch reveals: every `trapFired` but a face-up Field Trap's firing again, which reveals nothing (R2657001). */
function trapReveals(ctx: CoachCtx): Extract<GameEvent, { type: "trapFired" }>[] {
  return freshOf(ctx, "trapFired").filter((event) => event.again !== true);
}

function mine(view: PlayerView, instanceId: string): UnitView | undefined {
  return unitsOf(view, "you").find((unit) => unit.instanceId === instanceId);
}

function theirs(view: PlayerView, instanceId: string): UnitView | undefined {
  return unitsOf(view, "opponent").find((unit) => unit.instanceId === instanceId);
}

/** Your un-forced attacks in this batch; the attacker is read where it stands now (it may have died). */
function myAttacks(ctx: CoachCtx): Extract<GameEvent, { type: "attackDeclared" }>[] {
  const ownIds = new Set([
    ...unitsOf(ctx.view, "you").map((unit) => unit.instanceId),
    ...ctx.view.you.graveyard.map((card) => card.instanceId),
  ]);
  return freshOf(ctx, "attackDeclared").filter((event) => !event.forced && ownIds.has(event.attackerId));
}

function attackedDef(ctx: CoachCtx, targetId: string): { unit?: UnitView; def?: CardDef } {
  const unit = theirs(ctx.view, targetId);
  if (unit !== undefined) return { unit, def: defOf(ctx, unit.defId) };
  const gone = freshOf(ctx, "destroyed").find((event) => event.instanceId === targetId);
  return gone === undefined ? {} : { def: defOf(ctx, gone.defId) };
}

/** Effect events: what a Cry or a Death did once its card was played or died. */
const EFFECTS: ReadonlySet<GameEvent["type"]> = new Set<GameEvent["type"]>([
  "damage",
  "buffed",
  "destroyed",
  "addedToHand",
  "keywordGranted",
  "healed",
  "exiled",
  "transformed",
  "drawn",
]);

function anyEffect(ctx: CoachCtx): boolean {
  return ctx.fresh.some((event) => EFFECTS.has(event.type)) || freshOf(ctx, "summoned").length > 1;
}

function aboveFace(ctx: CoachCtx, unit: UnitView): boolean {
  const def = defOf(ctx, unit.defId);
  const face = unit.radiant ? def?.radiant : def?.base;
  return face?.attack !== undefined && unit.attack > face.attack;
}

function adjacentToTotem(ctx: CoachCtx): boolean {
  const lanes = ctx.view.you.units;
  return lanes.some((unit, lane) => {
    if (unit === null || unit.defId !== FLAME_TONGUE_TOTEM) return false;
    return [lanes[lane - 1], lanes[lane + 1]].some((next) => next != null && aboveFace(ctx, next));
  });
}

export const FIELD_GUIDE: readonly Discovery[] = [
  // Lesson 1, First steps.
  {
    id: "mana",
    name: "Mana",
    rule: "each card costs mana, and you get one more crystal every turn.",
    lesson: "basics",
    // A play's price is on its `cardPlayed`; `manaSpent` is reported only while a card listens for it (MD-D26).
    found: (ctx) =>
      myPlays(ctx).some(({ event }) => event.costPaid > 0) || freshOf(ctx, "manaSpent").some((event) => event.player === ctx.view.viewer),
  },
  {
    id: "play-unit",
    name: "Playing units",
    rule: "a unit you play stands in one of your three lanes until it dies.",
    lesson: "basics",
    found: (ctx) => myPlays(ctx).some(({ def }) => def?.type === "Unit"),
  },
  {
    id: "summoning-sickness",
    name: "Summoning sickness",
    rule: "a unit can't attack on the turn it arrives, but it is ready on your next turn.",
    lesson: "basics",
    found: (ctx) =>
      freshOf(ctx, "turnStarted").some((event) => event.player === ctx.view.viewer) &&
      unitsOf(ctx.view, "you").some((unit) => unit.canAct && !unitHas(unit, "Rush") && !unitHas(unit, "Charge")),
  },
  {
    id: "attack-hero",
    name: "Attacking the hero",
    rule: "any ready unit can hit the enemy hero for its Attack, whatever its lane.",
    lesson: "basics",
    found: (ctx) => myAttacks(ctx).some((event) => event.targetId === heroTargetId(ctx.view, "opponent")),
  },
  {
    id: "trade",
    name: "Trading",
    rule: "when two units fight, each deals its Attack to the other at the same time.",
    lesson: "basics",
    found: (ctx) => myAttacks(ctx).some((event) => event.targetId !== heroTargetId(ctx.view, "opponent")),
  },
  {
    id: "hero-health",
    name: "Hero health",
    rule: "a hero at 0 health loses. Yours starts at 30, the tutorial's enemy at 20.",
    lesson: "basics",
    found: (ctx) => freshOf(ctx, "damage").some((event) => event.targetId === heroTargetId(ctx.view, "you")),
  },
  {
    id: "winning",
    name: "Winning",
    rule: "bring the enemy hero to 0 health and the game is yours.",
    lesson: "basics",
    found: (ctx) => freshOf(ctx, "gameOver").some((event) => event.winner === ctx.view.viewer),
  },
  // Lesson 2, Cries and keywords.
  {
    id: "cry",
    name: "Cry",
    rule: "it happens when you play the unit from your hand.",
    lesson: "keywords",
    found: (ctx) => myPlays(ctx).some(({ def }) => def?.type === "Unit" && says(def, "Cry:")) && anyEffect(ctx),
  },
  {
    id: "taunt",
    name: "Taunt",
    rule: "while an enemy has a Taunt unit, your attacks must go at a Taunt unit first.",
    lesson: "keywords",
    found: (ctx) =>
      myAttacks(ctx).some((event) => {
        const { unit, def } = attackedDef(ctx, event.targetId);
        return unitHas(unit, "Taunt") || hasKeyword(def, "Taunt");
      }),
  },
  {
    id: "divine-shield",
    name: "Divine Shield",
    rule: "the first hit on a shielded unit does nothing but pop the shield.",
    lesson: "keywords",
    found: (ctx) => freshOf(ctx, "divineShieldLost").length > 0,
  },
  {
    id: "armor",
    name: "Armor",
    rule: "Armor soaks up damage before health does.",
    lesson: "keywords",
    found: (ctx) => freshOf(ctx, "damageAbsorbed").length > 0,
  },
  {
    id: "rush",
    name: "Rush",
    rule: "a unit with Rush can attack enemy units on the turn it arrives, though not the hero.",
    lesson: "keywords",
    found: (ctx) => myAttacks(ctx).some((event) => unitHas(mine(ctx.view, event.attackerId), "Rush")),
  },
  {
    id: "pierce",
    name: "Pierce",
    rule: "damage from a Pierce card ignores Armor.",
    lesson: "keywords",
    found: (ctx) =>
      myPlays(ctx).some(({ def }) => hasKeyword(def, "Pierce")) ||
      myAttacks(ctx).some((event) => unitHas(mine(ctx.view, event.attackerId), "Pierce")),
  },
  // Lesson 3, Spells.
  {
    id: "spell",
    name: "Spells",
    rule: "a spell does what it says once and goes to your graveyard. Some ask you for a target.",
    lesson: "casting",
    found: (ctx) => myPlays(ctx).some(({ def }) => def?.type === "Spell"),
  },
  {
    id: "removal",
    name: "Removal",
    rule: "a spell can deal with a unit that is too big to fight.",
    lesson: "casting",
    found: (ctx) =>
      myPlays(ctx).some(({ def }) => def?.type === "Spell") &&
      freshOf(ctx, "destroyed").some((event) => event.owner !== ctx.view.viewer),
  },
  // Lesson 4, Field Spells and Traps.
  {
    id: "trap",
    name: "Traps",
    rule: "you set a Trap face-down, and it springs by itself when its moment comes, on either turn.",
    lesson: "backrow",
    found: (ctx) => trapReveals(ctx).some((event) => event.controller === ctx.view.viewer),
  },
  {
    id: "enemy-backrow",
    name: "Face-down cards",
    rule: "the enemy's face-down cards are hidden until they spring.",
    lesson: "backrow",
    found: (ctx) => trapReveals(ctx).some((event) => event.controller !== ctx.view.viewer),
  },
  {
    id: "field-spell",
    name: "Field Spells",
    rule: "a Field Spell stays in your backrow and keeps working turn after turn.",
    lesson: "backrow",
    found: (ctx) => myPlays(ctx).some(({ def }) => def?.type === "Field Spell"),
  },
  {
    id: "field-trap",
    name: "Field Traps",
    rule: "a Field Trap springs like a Trap, then stays to keep working.",
    lesson: "backrow",
    found: (ctx) => trapReveals(ctx).some((event) => defOf(ctx, event.defId)?.type === "Field Trap"),
  },
  // Lesson 5, The final exam: everything else the game holds.
  {
    id: "defense-position",
    name: "Defense Position",
    rule: "a unit in Defense Position gains Taunt and 1 Armor.",
    lesson: "finale",
    found: (ctx) =>
      freshOf(ctx, "positionSwitched").some((event) => event.position === "DEF" && mine(ctx.view, event.instanceId) !== undefined) ||
      myAttacks(ctx).some((event) => attackedDef(ctx, event.targetId).unit?.position === "DEF"),
  },
  {
    id: "lanes",
    name: "Lanes",
    rule: "where a unit stands can matter: some cards care which lane they are in.",
    lesson: "finale",
    // Midlane Menace's Cry buffs it only in the middle lane (R685), so its buff is the lane found.
    found: (ctx) =>
      freshOf(ctx, "summoned").some(
        (summon) =>
          summon.player === ctx.view.viewer &&
          summon.defId === MIDLANE_MENACE &&
          freshOf(ctx, "buffed").some((buff) => buff.instanceId === summon.instanceId),
      ),
  },
  {
    id: "adjacency",
    name: "Neighbours",
    rule: "some cards care about the units beside them, to the left and the right.",
    lesson: "finale",
    found: (ctx) =>
      freshOf(ctx, "buffed").some((event) => mine(ctx.view, event.instanceId)?.defId === RIGHT_HAND_FELINOR) ||
      freshOf(ctx, "summoned").some(
        (event) => event.player === ctx.view.viewer && (event.defId === CHOSEN_BY_THE_SUN || event.defId === CHOSEN_BY_THE_MOON),
      ) ||
      adjacentToTotem(ctx),
  },
  {
    id: "aura",
    name: "Auras",
    rule: "an Aura works for as long as its card is on the field.",
    lesson: "finale",
    found: (ctx) => {
      const units = unitsOf(ctx.view, "you");
      const aura = units.some((unit) => says(defOf(ctx, unit.defId), "Aura:"));
      return aura && units.some((unit) => aboveFace(ctx, unit));
    },
  },
  {
    id: "token",
    name: "Tokens",
    rule: "a token is a unit a card makes. It fights like any unit and vanishes when it leaves the field.",
    lesson: "finale",
    found: (ctx) => freshOf(ctx, "summoned").some((event) => event.player === ctx.view.viewer && defOf(ctx, event.defId)?.token === true),
  },
  {
    id: "mulligan",
    name: "The mulligan",
    rule: "before the first turn you may swap any of your opening cards for new ones.",
    lesson: "finale",
    // Once both players have answered, the mulligan's view is gone with the phase: the player's own
    // answer at the game's start is the find too.
    found: (ctx) =>
      ctx.view.mulligan?.youReady === true ||
      (ctx.view.turn <= 1 && freshOf(ctx, "promptAnswered").some((event) => event.player === ctx.view.viewer)),
  },
  {
    id: "the-coin",
    name: "The Coin",
    rule: "the player going second gets The Coin: one extra mana, once.",
    lesson: "finale",
    found: (ctx) => myPlays(ctx).some(({ event }) => event.defId === COIN),
  },
  {
    id: "radiant",
    name: "Radiant",
    rule: "a Radiant card is the stronger face of the same card.",
    lesson: "finale",
    found: (ctx) =>
      freshOf(ctx, "radiantSet").some(
        (event) => mine(ctx.view, event.instanceId) !== undefined || myHand(ctx.view).some((card) => card.instanceId === event.instanceId),
      ),
  },
  {
    id: "tribe",
    name: "Tribes",
    rule: "a tag such as Felinor or Human is a tribe, and some cards reward their own tribe.",
    lesson: "finale",
    found: (ctx) =>
      myPlays(ctx).some(({ event }) => event.defId === BIG_FELINOR) &&
      freshOf(ctx, "destroyed").length > 0 &&
      unitsOf(ctx.view, "you").some((unit) => defOf(ctx, unit.defId)?.tags.includes("Felinor") === true),
  },
  {
    id: "tribute",
    name: "Tribute",
    rule: "a big unit can ask you to give up units of your own to play it.",
    lesson: "finale",
    found: (ctx) =>
      myPlays(ctx).some(({ def }) => says(def, "Tribute")) &&
      freshOf(ctx, "destroyed").some((event) => event.owner === ctx.view.viewer),
  },
  {
    id: "death",
    name: "Death",
    rule: "a Death ability happens when its unit dies.",
    lesson: "finale",
    found: (ctx) => freshOf(ctx, "destroyed").some((event) => says(defOf(ctx, event.defId), "Death:")) && anyEffect(ctx),
  },
  {
    id: "temporary-tokens",
    name: "Tokens for one turn",
    rule: "some tokens only last this turn, so use them while they are here.",
    lesson: "finale",
    found: (ctx) =>
      freshOf(ctx, "turnEnded").some((event) => event.player === ctx.view.viewer) &&
      freshOf(ctx, "destroyed").some((event) => event.owner === ctx.view.viewer && defOf(ctx, event.defId)?.token === true),
  },
  {
    id: "yellow-glow",
    name: "The yellow glow",
    rule: "a card in your hand glows yellow while its condition holds, so now is the time to play it.",
    lesson: "finale",
    found: (ctx) => ctx.view.active === ctx.view.viewer && myHand(ctx.view).some((card) => card.conditionActive === true),
  },
  {
    id: "legendary",
    name: "Legendary cards",
    rule: "Legendary and Mythic cards are the rarest, and they bend the rules.",
    lesson: "finale",
    found: (ctx) => myPlays(ctx).some(({ def }) => def !== undefined && SHOWPIECES.includes(def.rarity)),
  },
  {
    id: "random",
    name: "Random effects",
    rule: "a random effect can give you a different card or outcome each time.",
    lesson: "finale",
    found: (ctx) =>
      freshOf(ctx, "chaosRolled").some((event) => event.player === ctx.view.viewer) ||
      (myPlays(ctx).some(({ def }) => says(def, "random")) &&
        freshOf(ctx, "addedToHand").some((event) => event.player === ctx.view.viewer)),
  },
  {
    id: "quickdraw",
    name: "Quickdraw",
    rule: "a Quickdraw card always starts in your opening hand.",
    lesson: "finale",
    found: (ctx) => myPlays(ctx).some(({ def }) => def?.tags.includes("Quickdraw") === true),
  },
  {
    id: "copying",
    name: "Copying",
    rule: "a copy is the same card again, changes and all.",
    lesson: "finale",
    found: (ctx) =>
      myPlays(ctx).some(({ event }) => event.defId === SEANCE) &&
      freshOf(ctx, "addedToHand").some((event) => event.player === ctx.view.viewer),
  },
];

export function discoveryById(id: string, guide: readonly Discovery[] = FIELD_GUIDE): Discovery | undefined {
  return guide.find((entry) => entry.id === id);
}
