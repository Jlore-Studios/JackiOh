// Fixed "basics" coach script (SPEC §9.10): view-only steps use legal actions (CLAUDE.md rule 7)
// and retire when players deviate from the scripted line.

import type { PlayerView, UnitView } from "@jackioh/shared";
import { MAX_MANA } from "@jackioh/engine/config";

import { t } from "../../i18n/i18n.ts";
import { cardName, goodTrade, heroAttack, legalAttacksOf, readerCardName, sameMove, yourMove, zoneOf } from "../advice.ts";
import type { CoachAnchor, CoachCtx, CoachStep, LessonScript } from "../coach.ts";
import {
  freshOf,
  heroTargetId,
  info,
  inHand,
  isMyTurn,
  keepHand,
  mulliganOpen,
  myMain,
  myTurnNumber,
  playCard,
  tip,
  unitOf,
  unitsOf,
} from "../steps.ts";

const VANILLA = "core-008";
const FELINORS = "core-012";
const SHREDDER = "core-013";
const GARY = "core-004";
const MENACE = "core-019";
const SEVEN = "core-025";
const MOTHS = "core-009";
const POSTDOC = "core-061";
const COIN = "core-t-coin";

/** The enemy hero's health at or under which the coach says it is nearly beaten. */
const ENEMY_LOW_HEALTH = 8;

function attacked(ctx: CoachCtx, at: "hero" | "unit"): boolean {
  const hero = heroTargetId(ctx.view, "opponent");
  const own = (instanceId: string): boolean =>
    unitsOf(ctx.view, "you").some((unit) => unit.instanceId === instanceId) || ctx.view.you.graveyard.some((card) => card.instanceId === instanceId);
  return freshOf(ctx, "attackDeclared").some(
    (event) => !event.forced && own(event.attackerId) && (at === "hero" ? event.targetId === hero : event.targetId !== hero),
  );
}

function movedThisTurn(view: PlayerView): boolean {
  for (let index = view.events.length - 1; index >= 0; index -= 1) {
    const event = view.events[index];
    if (event === undefined || event.type === "turnStarted") return false;
    if (event.type === "cardPlayed" && event.player === view.viewer) return true;
    if (event.type === "attackDeclared" && !event.forced && unitsOf(view, "you").some((unit) => unit.instanceId === event.attackerId)) return true;
  }
  return false;
}

function plainCopy(view: PlayerView): UnitView | undefined {
  return unitsOf(view, "opponent").find((unit) => unit.vanilla === true);
}

/** Retires an end-turn prompt once its turn is past. */
function endTurnOn(n: number, options: { id: string; title: string; text: string; when?: (ctx: CoachCtx) => boolean }): CoachStep {
  const { when, ...rest } = options;
  return {
    kind: "act",
    anchor: { kind: "endTurn" },
    ...rest,
    when: (ctx) => myMain(ctx) && myTurnNumber(ctx.view) === n && (when === undefined || when(ctx)),
    done: (ctx, since) => ctx.view.turn !== since.turn,
    moot: (ctx, since) => since === null && myTurnNumber(ctx.view) > n,
    expect: (action) => action.type === "endTurn",
  };
}

/** Retires a play prompt after its deadline or when the card has left hand. */
function playBy(byTurn: number, options: Parameters<typeof playCard>[0]): CoachStep {
  const step = playCard(options);
  return {
    ...step,
    moot: (ctx, since) =>
      since === null &&
      (myTurnNumber(ctx.view) > byTurn || (inHand(ctx.view, options.defId) === undefined && unitOf(ctx.view, "you", options.defId) !== undefined)),
  };
}

const attackHero: CoachStep = {
  id: "attack-hero",
  kind: "act",
  title: "Attack the hero",
  text: (ctx) => {
    const choice = heroAttack(ctx);
    if (choice === undefined) {
      return t("Your unit is ready: it glows green. Drag it onto the enemy hero to deal its attack damage. Any unit can attack any target, whatever its lane.");
    }
    return t("{name} is ready: it glows green. Drag it onto the enemy hero to deal {damage} damage. Any unit can attack any target, whatever its lane.", {
      name: cardName(ctx, choice.attacker.defId),
      damage: String(choice.attacker.attack),
    });
  },
  anchor: { kind: "hero", side: "opponent" },
  when: (ctx) => myMain(ctx) && myTurnNumber(ctx.view) >= 2 && heroAttack(ctx) !== undefined,
  done: (ctx, since) => since !== ctx.view && attacked(ctx, "hero"),
  // Retire while Taunt blocks the hero; the final step resumes when it is clear.
  moot: (ctx, since) => {
    if (since !== null) return ctx.view.turn !== since.turn;
    const blocked = myMain(ctx) && heroAttack(ctx) === undefined && legalAttacksOf(ctx).length > 0;
    return myTurnNumber(ctx.view) > 4 || (myTurnNumber(ctx.view) >= 2 && blocked);
  },
  expect: (action, ctx) => sameMove(action, heroAttack(ctx)?.action),
};

const trade: CoachStep = {
  id: "trade",
  kind: "act",
  title: "Trade units",
  text: (ctx) => {
    const choice = goodTrade(ctx);
    if (choice === undefined) return t("Attack an enemy unit: both units hit each other at the same time.");
    return t("Attack a unit this time: drag {attacker} onto the highlighted {target}. Both hit each other at once, {dealt} damage one way and {taken} back.", {
      attacker: cardName(ctx, choice.attacker.defId),
      target: cardName(ctx, choice.target.defId),
      dealt: String(choice.dealt),
      taken: String(choice.taken),
    });
  },
  anchor: (ctx): CoachAnchor | null => {
    const choice = goodTrade(ctx);
    return (choice === undefined ? null : zoneOf(ctx, "opponent", choice.target.instanceId)) ?? { kind: "units", side: "opponent" };
  },
  when: (ctx) => myMain(ctx) && myTurnNumber(ctx.view) >= 3 && goodTrade(ctx) !== undefined,
  done: (ctx, since) => since !== ctx.view && attacked(ctx, "unit"),
  moot: (ctx, since) => (since === null ? myTurnNumber(ctx.view) > 6 : ctx.view.turn !== since.turn),
  expect: (action, ctx) => sameMove(action, goodTrade(ctx)?.action),
};

export const script: LessonScript = {
  lessonId: "basics",
  steps: [
    info({
      id: "welcome",
      title: "Welcome to JackiOh",
      text: "You'll learn by playing a real game against a gentle opponent. These three cards are your opening hand.",
      anchor: { kind: "prompt" },
      when: (ctx) => mulliganOpen(ctx.view),
      moot: (ctx, since) => since === null && ctx.view.phase !== "mulligan" && ctx.view.phase !== "setup",
    }),
    keepHand({
      id: "keep",
      title: "Keep your hand",
      text: "You could swap cards here (lesson 4 shows how), but these are good ones. Press Ready to keep all three.",
    }),
    info({
      id: "your-hero",
      title: "Your hero",
      text: (ctx) => t("This is you: your hero, with {health} health. If it ever drops to 0, you lose.", { health: String(ctx.view.you.hero.health) }),
      anchor: { kind: "hero", side: "you" },
      // The first turn the player can act on: turn 1, unless a hand with nothing to play ended it (R82).
      when: (ctx) => myMain(ctx),
    }),
    info({
      id: "enemy-hero",
      title: "The enemy hero",
      text: (ctx) =>
        t("Across the board is the enemy hero. In this lesson it starts with {health} health. Bring it down to 0 and you win.", {
          health: String(ctx.view.opponent.hero.health),
        }),
      anchor: { kind: "hero", side: "opponent" },
      when: (ctx) => myMain(ctx),
    }),
    playBy(2, {
      id: "play-vanilla",
      title: "Play a unit",
      text: () =>
        t("{name} costs (1): the blue number at its top left. Drag it into any of your five unit zones, or click it and then a zone. Each column is a lane.", {
          name: readerCardName(VANILLA, "Mr. Vanilla"),
        }),
      defId: VANILLA,
    }),
    // Follow the first play so this turn has at most two "Got it"s.
    info({
      id: "mana",
      title: "Mana",
      text: (ctx) => {
        const { current, max } = ctx.view.you.mana;
        return t("This is your mana: {current} of {max} left this turn. Cards cost mana. You get one more crystal each turn, up to {cap}, and your mana refills every turn.", {
          current: String(current),
          max: String(max),
          cap: String(MAX_MANA),
        });
      },
      anchor: { kind: "mana" },
      when: (ctx) => myMain(ctx),
    }),
    info({
      id: "sick",
      title: "Not ready yet",
      text: (ctx) =>
        unitOf(ctx.view, "you", VANILLA) === undefined
          ? t("Units can't attack on the turn they arrive. Your unit will be ready on your next turn.")
          : t("Units can't attack on the turn they arrive. {name} will be ready on your next turn.", { name: readerCardName(VANILLA, "Mr. Vanilla") }),
      anchor: (ctx) => (unitOf(ctx.view, "you", VANILLA) === undefined ? { kind: "units", side: "you" } : { kind: "unit", side: "you", defId: VANILLA }),
      when: (ctx) => myMain(ctx) && unitsOf(ctx.view, "you").length > 0 && legalAttacksOf(ctx).length === 0,
      moot: (ctx, since) => since === null && (legalAttacksOf(ctx).length > 0 || myTurnNumber(ctx.view) > 2),
    }),
    endTurnOn(1, {
      id: "end-1",
      title: "End the turn",
      text: "Your mana is spent. Press End turn: the enemy plays next, then you draw a card and your mana refills.",
    }),
    attackHero,
    playBy(3, {
      id: "play-felinors",
      title: "Two for one",
      text: (ctx) => {
        const cost = inHand(ctx.view, FELINORS)?.cost;
        const max = String(ctx.view.you.mana.max);
        const name = readerCardName(FELINORS, "Duplicating Felinors");
        if (cost === undefined) {
          return t("Your mana refilled with one more crystal: {max} now. {name}: when you play it, it brings a copy of itself, two units from one card.", { max, name });
        }
        return t("Your mana refilled with one more crystal: {max} now. {name} costs ({cost}): when you play it, it brings a copy of itself, two units from one card.", {
          max,
          name,
          cost: String(cost),
        });
      },
      defId: FELINORS,
    }),
    endTurnOn(2, {
      id: "end-2",
      title: "End the turn",
      text: "Your mana is spent, and the Felinors need a turn before they can attack. Press End turn.",
      when: (ctx) => !ctx.legal.some((action) => action.type === "play" || action.type === "attack"),
    }),
    playBy(4, {
      id: "play-shredder",
      title: "A big unit",
      text: () =>
        t("{name} costs (3). Play it: it's the biggest unit on the board, and it has a trick you'll see at the end of turn.", {
          name: readerCardName(SHREDDER, "Jlockeed Shredder-10"),
        }),
      defId: SHREDDER,
    }),
    endTurnOn(3, {
      id: "end-3",
      title: "Watch the Shredder",
      text: "Shredder can't attack yet, but press End turn and watch: at the end of each of your turns it hits every enemy for 2.",
    }),
    trade,
    info({
      id: "trade-result",
      title: "How trades work",
      text: "Both units took their damage at once. A unit at 0 health is destroyed and goes to the graveyard; one that lives keeps its damage.",
      anchor: { kind: "graveyard", side: "opponent" },
      when: (ctx) => myMain(ctx),
      moot: (ctx, since) => since === null && !attacked(ctx, "unit"),
    }),
    info({
      id: "rule-of-thumb",
      title: "Unit or hero?",
      text: "A good rule: if your unit can destroy an enemy unit and survive, trade. Otherwise attack the enemy hero: every point counts.",
      anchor: { kind: "units", side: "opponent" },
      when: (ctx) => myMain(ctx) && myTurnNumber(ctx.view) >= 4,
    }),
    yourMove({ id: "win", title: "Win the game", final: true }),
  ],
  tips: [
    tip({
      id: "coin",
      title: () => readerCardName(COIN, "The Coin"),
      text: () =>
        t("The enemy played {name}: a bonus card for the player who goes second. It gives 1 extra mana, once.", { name: readerCardName(COIN, "The Coin") }),
      anchor: { kind: "hero", side: "opponent" },
      when: (ctx) => freshOf(ctx, "cardPlayed").some((event) => event.player !== ctx.view.viewer && event.defId === COIN),
      holdAi: true,
    }),
    tip({
      id: "enemy-unit",
      title: "Attack and health",
      text: "The enemy played a unit. Every unit shows its attack in the yellow gem and its health in the red one. Hover a card to read it (press and hold on a phone).",
      anchor: { kind: "units", side: "opponent" },
      when: (ctx) => freshOf(ctx, "summoned").some((event) => event.player !== ctx.view.viewer && event.row === "units"),
      holdAi: true,
    }),
    tip({
      id: "enemy-attacks",
      title: "The enemy attacks",
      // AI attacks resolve with their losses, so one tip covers both.
      text: (ctx) => {
        const lost = freshOf(ctx, "destroyed").find((event) => event.owner === ctx.view.viewer);
        if (lost === undefined) {
          return t("The enemy attacked too. Its units follow the same rules as yours: one attack each per turn, never on the turn they arrive.");
        }
        return t(
          "The enemy attacked and destroyed your {name}: it went to your graveyard. Its units follow the same rules as yours: one attack each per turn, never on the turn they arrive.",
          { name: cardName(ctx, lost.defId) },
        );
      },
      anchor: (ctx) =>
        freshOf(ctx, "destroyed").some((event) => event.owner === ctx.view.viewer) ? { kind: "graveyard", side: "you" } : { kind: "hero", side: "you" },
      when: (ctx) =>
        !isMyTurn(ctx.view) && freshOf(ctx, "attackDeclared").some((event) => !event.forced && unitsOf(ctx.view, "opponent").some((unit) => unit.instanceId === event.attackerId)),
    }),
    tip({
      id: "defense",
      title: "Defense Position",
      text: (ctx) => {
        const unit = unitsOf(ctx.view, "opponent").find((candidate) => candidate.position === "DEF");
        if (unit === undefined || unit.armor === 0) {
          return t("The enemy turned a unit sideways: Defense Position. It now has Taunt, so your attacks must hit it first. More in lesson 2.");
        }
        return t(
          "The enemy turned a unit sideways: Defense Position. It now has Taunt, so your attacks must hit it first, and it takes {armor} less damage. More in lesson 2.",
          { armor: String(unit.armor) },
        );
      },
      anchor: (ctx) => {
        const unit = unitsOf(ctx.view, "opponent").find((candidate) => candidate.position === "DEF");
        return (unit === undefined ? null : zoneOf(ctx, "opponent", unit.instanceId)) ?? { kind: "units", side: "opponent" };
      },
      when: (ctx) =>
        freshOf(ctx, "positionSwitched").some(
          (event) => event.position === "DEF" && unitsOf(ctx.view, "opponent").some((unit) => unit.instanceId === event.instanceId),
        ),
      holdAi: true,
    }),
    tip({
      id: "postdoc",
      title: "A plain copy",
      text: (ctx) => {
        const copy = plainCopy(ctx.view);
        const maker = readerCardName(POSTDOC, "Prejudiced Postdoc");
        if (copy === undefined) {
          return t("That enemy unit is a plain copy {maker} made: the same stats, none of its text. One more body for you to deal with.", { maker });
        }
        return t("That enemy {name} is a plain copy {maker} made: the same stats, none of its text. One more body for you to deal with.", {
          name: cardName(ctx, copy.defId),
          maker,
        });
      },
      anchor: (ctx) => {
        const copy = plainCopy(ctx.view);
        return (copy === undefined ? null : zoneOf(ctx, "opponent", copy.instanceId)) ?? { kind: "units", side: "opponent" };
      },
      // Wait for the player's move; the AI turn already has two tips.
      when: (ctx) => myMain(ctx) && movedThisTurn(ctx.view) && plainCopy(ctx.view) !== undefined,
    }),
    tip({
      id: "shredder-fired",
      title: "Shredder at work",
      text: () =>
        t("At the end of turn, {name} hit every enemy unit and the enemy hero for 2. It does that every turn it stays.", {
          name: readerCardName(SHREDDER, "Jlockeed Shredder-10"),
        }),
      anchor: { kind: "unit", side: "you", defId: SHREDDER },
      when: (ctx) => {
        const shredder = unitOf(ctx.view, "you", SHREDDER);
        return shredder !== undefined && freshOf(ctx, "damage").some((event) => event.sourceId === shredder.instanceId && !event.combat);
      },
    }),
    tip({
      id: "gary",
      title: "Heads or tails",
      text: (ctx) => {
        const buff = freshOf(ctx, "buffed").find((event) => [...unitsOf(ctx.view, "you"), ...unitsOf(ctx.view, "opponent")].some((unit) => unit.instanceId === event.instanceId && unit.defId === GARY));
        const name = readerCardName(GARY, "Gary the Gambler");
        if (buff === undefined) return t("{name} flipped five coins as it arrived: +1 attack for each heads, +1 health for each tails.", { name });
        return t(
          "{name} flipped five coins as it arrived: +1 attack for each heads, +1 health for each tails. This one got +{attack} attack and +{health} health.",
          { name, attack: String(buff.attack), health: String(buff.health) },
        );
      },
      anchor: (ctx) => (unitOf(ctx.view, "you", GARY) === undefined ? { kind: "unit", side: "opponent", defId: GARY } : { kind: "unit", side: "you", defId: GARY }),
      when: (ctx) => freshOf(ctx, "summoned").some((event) => event.defId === GARY && event.row === "units"),
    }),
    tip({
      id: "menace",
      title: "A bodyguard",
      text: () =>
        t("{name} has Taunt, like a unit in Defense Position: enemy attacks must go at it first, so it guards your hero. It heals to full at the end of turn.", {
          name: readerCardName(MENACE, "Midrange Menace"),
        }),
      anchor: { kind: "unit", side: "you", defId: MENACE },
      when: (ctx) => freshOf(ctx, "summoned").some((event) => event.player === ctx.view.viewer && event.defId === MENACE),
    }),
    tip({
      id: "armor",
      title: "Armor",
      text: () => t("{name} has Armor 7: every hit on it is 7 smaller, so most units can't hurt it at all.", { name: readerCardName(SEVEN, "4-mana 7/7") }),
      anchor: { kind: "unit", side: "you", defId: SEVEN },
      when: (ctx) => freshOf(ctx, "summoned").some((event) => event.player === ctx.view.viewer && event.defId === SEVEN),
    }),
    tip({
      id: "moths",
      title: () => readerCardName(MOTHS, "Moths to the Flame"),
      text: () =>
        t("{name} makes every one of your units attack it at the start of the enemy's turn. They still attack as usual on yours.", {
          name: readerCardName(MOTHS, "Moths to the Flame"),
        }),
      anchor: { kind: "unit", side: "opponent", defId: MOTHS },
      when: (ctx) => freshOf(ctx, "attackDeclared").some((event) => event.forced),
    }),
    tip({
      id: "enemy-low",
      title: "Almost there",
      text: (ctx) =>
        t("The enemy hero is down to {health} health. Look for units that can reach it this turn.", { health: String(ctx.view.opponent.hero.health) }),
      anchor: { kind: "hero", side: "opponent" },
      // Only show while the player can still attack this turn.
      when: (ctx) => myMain(ctx) && ctx.view.opponent.hero.health <= ENEMY_LOW_HEALTH && legalAttacksOf(ctx).length > 0,
    }),
    tip({
      id: "enemy-fatigue",
      title: "Out of cards",
      text: "The enemy's deck is empty. Every draw from an empty deck hurts its hero, a little more each time.",
      anchor: { kind: "library", side: "opponent" },
      when: (ctx) => ctx.view.opponent.fatigueCount > 0,
    }),
  ],
};
