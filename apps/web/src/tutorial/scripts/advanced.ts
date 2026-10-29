// Advanced lesson script (SPEC §9.10): fixed seed and decks permit named cards.
// Covers mulligan (§2.1 step 3), Coin (R244, R245), tokens (§7, #62, #92), Radiant (§5.2, #26),
// Tribute (§6.3, #66), and yellow glow (R195); only legal views/actions are read (CLAUDE.md rule 7).

import type { ActionBody, PlayerView } from "@jackioh/shared";

import { t } from "../../i18n/i18n.ts";
import { moveText, nextMove, readerCardName, yourMove } from "../advice.ts";
import type { CoachCtx, CoachStep, LessonScript } from "../coach.ts";
import {
  freshOf,
  info,
  inHand,
  legalPlays,
  mulliganAway,
  mulliganOpen,
  myHand,
  myMain,
  myTurnNumber,
  playCard,
  tip,
  unitOf,
  unitsOf,
} from "../steps.ts";

const COIN = "core-t-coin";
const FELINOR_TOKEN = "core-t-felinor";
const FRIEND_OF_FELINORS = "core-062";
const FELINOR_FIENDER = "core-092";
const GLOWY_JELLY_BEAN = "core-026";
const THE_ROCK = "core-066";
const SEVEN_SEVEN = "core-025";
const RADIANT_SAINTESS = "core-081";
const RENO = "core-053";

type Play = Extract<ActionBody, { type: "play" }>;

/** The lesson's cards as the reader reads their names (the English is what the coach's lines print). */
const rock = (): string => readerCardName(THE_ROCK, "The Rock");
const coin = (): string => readerCardName(COIN, "The Coin");
const fiender = (): string => readerCardName(FELINOR_FIENDER, "Felinor Fiender");
const token = (): string => readerCardName(FELINOR_TOKEN, "Felinor Token");
const saintess = (): string => readerCardName(RADIANT_SAINTESS, "Radiant Saintess");

function pastMulligan(view: PlayerView): boolean {
  return view.phase !== "mulligan" && view.phase !== "setup";
}

function myTokens(view: PlayerView): string[] {
  return unitsOf(view, "you")
    .filter((unit) => unit.defId === FELINOR_TOKEN)
    .map((unit) => unit.instanceId);
}

function fienderOnField(ctx: CoachCtx): boolean {
  return unitOf(ctx.view, "you", FELINOR_FIENDER) !== undefined;
}

function rockRadiant(view: PlayerView): boolean {
  return inHand(view, THE_ROCK)?.radiant === true || unitOf(view, "you", THE_ROCK)?.radiant === true;
}

function sameAction(a: ActionBody, b: ActionBody): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function beanOnRock(ctx: CoachCtx): Play[] {
  const rock = inHand(ctx.view, THE_ROCK);
  if (rock === undefined) return [];
  return legalPlays(ctx, GLOWY_JELLY_BEAN).filter((play) =>
    (play.targets ?? []).some((target) => target.pick === "instance" && target.instanceId === rock.instanceId),
  );
}

/** Prefer a token tribute; otherwise preserve the most valuable unit. */
function rockPlays(ctx: CoachCtx): Play[] {
  const plays = legalPlays(ctx, THE_ROCK);
  const tokens = new Set(myTokens(ctx.view));
  const onTokens = plays.filter((play) => (play.tributes ?? []).length > 0 && (play.tributes ?? []).every((id) => tokens.has(id)));
  if (onTokens.length > 0) return onTokens;
  const worth = (id: string): number => {
    const unit = unitsOf(ctx.view, "you").find((candidate) => candidate.instanceId === id);
    return unit === undefined ? Number.MAX_SAFE_INTEGER : unit.attack + unit.health;
  };
  const cost = (play: Play): number => (play.tributes ?? []).reduce((sum, id) => sum + worth(id), 0);
  const cheapest = Math.min(...plays.map(cost));
  return plays.filter((play) => cost(play) === cheapest);
}

function renoReady(ctx: CoachCtx): boolean {
  return inHand(ctx.view, RENO)?.conditionActive === true && legalPlays(ctx, RENO).length > 0;
}

/** Drop unreachable steps so the coach never waits for an illegal play. */
function outOfReach(ctx: CoachCtx, possible: (ctx: CoachCtx) => boolean): boolean {
  return myMain(ctx) && !possible(ctx);
}

function restOfTurn(id: string): CoachStep {
  return yourMove({ id, title: "Your move" });
}

function playStep(options: Parameters<typeof playCard>[0] & { possible?: (ctx: CoachCtx) => boolean }): CoachStep {
  const { possible, ...rest } = options;
  const reachable = possible ?? ((ctx: CoachCtx): boolean => legalPlays(ctx, rest.defId).length > 0);
  const step = playCard({ ...rest, when: (ctx) => reachable(ctx) && (rest.when === undefined || rest.when(ctx)) });
  return {
    ...step,
    moot: (ctx, since) => (since === null && inHand(ctx.view, rest.defId) === undefined) || outOfReach(ctx, reachable),
  };
}

const makeRadiant: CoachStep = {
  id: "make-radiant",
  title: "Make it Radiant",
  text: () =>
    t("Play {bean}, then pick {rock} in your hand. The card you pick becomes Radiant.", {
      bean: readerCardName(GLOWY_JELLY_BEAN, "Glowy Jelly Bean"),
      rock: rock(),
    }),
  kind: "act",
  anchor: { kind: "handCard", defId: GLOWY_JELLY_BEAN },
  when: (ctx) => myMain(ctx) && beanOnRock(ctx).length > 0,
  done: (ctx) => inHand(ctx.view, GLOWY_JELLY_BEAN) === undefined || rockRadiant(ctx.view),
  moot: (ctx) => outOfReach(ctx, (now) => beanOnRock(now).length > 0),
  expect: (action, ctx) => beanOnRock(ctx).some((play) => sameAction(play, action)),
};

const playRock: CoachStep = {
  id: "play-rock",
  title: "Pay the Tribute",
  text: (ctx) =>
    myTokens(ctx.view).length > 0
      ? t("Play {rock}: pick a {token} as its Tribute, then an empty zone. The token is tributed to pay for it.", { rock: rock(), token: token() })
      : t("Play {rock}: pick your weakest unit as its Tribute, then an empty zone. That unit is tributed to pay for it.", { rock: rock() }),
  kind: "act",
  anchor: { kind: "handCard", defId: THE_ROCK },
  when: (ctx) => myMain(ctx) && rockPlays(ctx).length > 0,
  done: (ctx) => inHand(ctx.view, THE_ROCK) === undefined,
  moot: (ctx, since) => (since === null && inHand(ctx.view, THE_ROCK) === undefined) || outOfReach(ctx, (now) => rockPlays(now).length > 0),
  expect: (action, ctx) => rockPlays(ctx).some((play) => sameAction(play, action)),
};

export const script: LessonScript = {
  lessonId: "advanced",
  steps: [
    // Before the first turn: mulligan.
    info({
      id: "welcome",
      title: "Tricks of the trade",
      text: "Your last lesson: the tricks good players use. The first one comes before the game even starts.",
      anchor: { kind: "prompt" },
      when: (ctx) => mulliganOpen(ctx.view),
      moot: (ctx) => pastMulligan(ctx.view),
    }),
    info({
      id: "mulligan-what",
      title: "The mulligan",
      text: "This is your opening hand. You may send cards back: you draw that many new ones, and the ones you sent back are shuffled into your deck.",
      anchor: { kind: "prompt" },
      when: (ctx) => mulliganOpen(ctx.view),
      moot: (ctx) => pastMulligan(ctx.view),
    }),
    mulliganAway({
      id: "mulligan",
      title: "Send one back",
      text: "Rule of thumb: send back cards you can't play in your first turns. The 7/7 costs (4), so tap it to mark it Redraw, then press Ready.",
      defIds: [SEVEN_SEVEN],
    }),

    // First turn: The Coin.
    info({
      id: "coin",
      title: coin,
      text: () =>
        t("The AI went first. To make up for it, you started with one extra card and got {coin}: a free spell that gives 1 extra mana, this turn only.", {
          coin: coin(),
        }),
      anchor: { kind: "handCard", defId: COIN },
      when: (ctx) => myMain(ctx) && inHand(ctx.view, COIN) !== undefined,
      moot: (ctx, since) => since === null && pastMulligan(ctx.view) && inHand(ctx.view, COIN) === undefined,
    }),
    // Both belong to the first turn: once it has passed, "a turn early" is no longer true.
    {
      ...playStep({
        id: "play-coin",
        title: () => t("Play {coin}", { coin: coin() }),
        text: () => t("Play {coin} now, so you have 2 mana on your very first turn.", { coin: coin() }),
        defId: COIN,
      }),
      moot: (ctx, since) => (since === null && inHand(ctx.view, COIN) === undefined) || myTurnNumber(ctx.view) > 1,
    },
    {
      ...playStep({
        id: "play-fiender",
        title: "A turn early",
        text: () => t("With 2 mana you can play {name}, a (2) Cost unit, a whole turn before you normally could.", { name: fiender() }),
        defId: FELINOR_FIENDER,
      }),
      moot: (ctx, since) =>
        (since === null && inHand(ctx.view, FELINOR_FIENDER) === undefined) ||
        myTurnNumber(ctx.view) > 1 ||
        outOfReach(ctx, (now) => legalPlays(now, FELINOR_FIENDER).length > 0),
    },
    {
      ...restOfTurn("end-first"),
      text: (ctx) => {
        const move = nextMove(ctx);
        return move?.kind === "end" && ctx.view.you.mana.current === 0 ? t("That is all your mana. End the turn.") : moveText(ctx, move);
      },
    },

    // Second turn: tribes and tokens.
    {
      ...info({
        id: "tribes",
        title: "Tribes",
        text: () =>
          t("Some cards belong to a tribe, named at the bottom of the card, like Felinor. {name} adds the stats of all your Felinors to its own.", {
            name: fiender(),
          }),
        anchor: { kind: "unit", side: "you", defId: FELINOR_FIENDER },
        when: (ctx) => myMain(ctx) && fienderOnField(ctx),
      }),
      moot: (ctx) => outOfReach(ctx, fienderOnField),
    },
    playStep({
      id: "play-friend",
      title: "Felinor Tokens",
      text: () =>
        t("Play {name}: it fills every empty zone on your side with a {token}, a small unit the card makes.", {
          name: readerCardName(FRIEND_OF_FELINORS, "Friend of Felinors"),
          token: token(),
        }),
      defId: FRIEND_OF_FELINORS,
    }),
    info({
      id: "fiender-grew",
      title: "It grew",
      text: () =>
        t("Every {token} is a Felinor, so {name} counts them all. Look how much bigger it is now.", { token: token(), name: fiender() }),
      anchor: { kind: "unit", side: "you", defId: FELINOR_FIENDER },
      when: (ctx) => fienderOnField(ctx) && myTokens(ctx.view).length > 0,
      moot: (ctx, since) => since === null && (!fienderOnField(ctx) || myTokens(ctx.view).length === 0),
    }),
    restOfTurn("move-2"),

    // Third turn: Radiant.
    info({
      id: "radiant",
      title: "Radiant cards",
      text: () => t("Every card has a stronger Radiant form with a gold face. Look at {rock} in your hand and note its stats.", { rock: rock() }),
      anchor: { kind: "handCard", defId: THE_ROCK },
      when: (ctx) => myMain(ctx) && beanOnRock(ctx).length > 0,
      moot: (ctx) => outOfReach(ctx, (now) => beanOnRock(now).length > 0),
    }),
    makeRadiant,
    info({
      id: "radiant-after",
      title: "Stronger",
      text: () =>
        t("{rock} is Radiant now: a gold face and stronger stats. Compare them with before. It stays Radiant for the rest of the game.", {
          rock: rock(),
        }),
      anchor: { kind: "handCard", defId: THE_ROCK },
      when: (ctx) => rockRadiant(ctx.view),
      moot: (ctx, since) => since === null && !rockRadiant(ctx.view),
    }),
    restOfTurn("move-3"),

    // Fourth turn: Tribute.
    info({
      id: "tribute",
      title: "Tribute",
      text: () =>
        t("{rock} has Tribute 1: besides its mana, playing it costs one of your own units, which is tributed. A token is perfect for that.", {
          rock: rock(),
        }),
      anchor: { kind: "handCard", defId: THE_ROCK },
      when: (ctx) => myMain(ctx) && rockPlays(ctx).length > 0,
      moot: (ctx) => outOfReach(ctx, (now) => rockPlays(now).length > 0),
    }),
    playRock,
    info({
      id: "indestructible",
      title: "Indestructible",
      text: () => t("{rock} is Indestructible: it never takes damage and can't be destroyed. Next turn it can attack.", { rock: rock() }),
      anchor: { kind: "unit", side: "you", defId: THE_ROCK },
      when: (ctx) => unitOf(ctx.view, "you", THE_ROCK) !== undefined,
      moot: (ctx, since) => since === null && unitOf(ctx.view, "you", THE_ROCK) === undefined,
    }),
    restOfTurn("move-4"),

    // Fifth turn: yellow glow, taught only once Reno can be played.
    playStep({
      id: "play-reno",
      title: "Glowing yellow",
      text: () =>
        t("{name} glows yellow: its condition is met right now. Your hero is below 30, so its Cry sets it back to 30. Play it.", {
          name: readerCardName(RENO, "Reno"),
        }),
      defId: RENO,
      possible: renoReady,
    }),

    // The rest of the game.
    yourMove({ id: "win", title: "Win the game", final: true }),
  ],
  tips: [
    tip({
      id: "fewer-felinors",
      title: "Fewer Felinors",
      text: () =>
        t("A {token} is gone, so {name} shrank: it counts only the Felinors you have right now. Tokens never go to the graveyard.", {
          token: token(),
          name: fiender(),
        }),
      anchor: { kind: "unit", side: "you", defId: FELINOR_FIENDER },
      when: (ctx) =>
        fienderOnField(ctx) && freshOf(ctx, "destroyed").some((event) => event.defId === FELINOR_TOKEN && event.owner === ctx.view.viewer),
    }),
    tip({
      id: "yellow-glow",
      title: "Glowing yellow",
      text: "A card in your hand glows yellow: its special condition is met right now, so it does its extra effect if you play it.",
      anchor: (ctx) => {
        const card = myHand(ctx.view).find((candidate) => candidate.defId !== RENO && candidate.conditionActive === true);
        return card === undefined ? null : { kind: "handCard", defId: card.defId };
      },
      when: (ctx) =>
        myMain(ctx) &&
        myHand(ctx.view).some((card) => card.defId !== RENO && card.conditionActive === true && legalPlays(ctx, card.defId).length > 0),
    }),
    tip({
      id: "saintess",
      title: saintess,
      text: () => t("{name} died, so all your other units became Radiant: gold faces and stronger stats.", { name: saintess() }),
      anchor: { kind: "units", side: "you" },
      when: (ctx) =>
        freshOf(ctx, "destroyed").some((event) => event.defId === RADIANT_SAINTESS && event.owner === ctx.view.viewer) &&
        freshOf(ctx, "radiantSet").some((event) => event.zone.z === "field" && event.zone.player === ctx.view.viewer),
    }),
    tip({
      id: "board-full",
      title: "Board full",
      text: "All five of your zones are full, so you can't play another unit until one of them opens up.",
      anchor: { kind: "units", side: "you" },
      when: (ctx) =>
        myMain(ctx) &&
        unitsOf(ctx.view, "you").length === ctx.view.you.units.length &&
        // Cards with stats are Units (CardView.attack, R243); require enough mana.
        myHand(ctx.view).some(
          (card) => card.attack !== undefined && card.cost <= ctx.view.you.mana.current && legalPlays(ctx, card.defId).length === 0,
        ),
    }),
  ],
};
