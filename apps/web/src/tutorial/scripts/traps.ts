// Lesson "traps" script (SPEC §9.10): it reads only the view and legal actions (CLAUDE.md rule 7).
// Scheduled cards wait for their draw and missed moments retire; an automatic turn can end (R82).
// The Tokens tip waits for the player's turn so no more than two "Got it" bubbles stack.

import { COIN_DEF_ID } from "@jackioh/engine/config";
import type { ActionBody, CardView, PlayerView } from "@jackioh/shared";

import { yourMove } from "../advice.ts";
import type { CoachCtx, CoachStep, LessonScript } from "../coach.ts";
import {
  freshOf,
  heroTargetId,
  info,
  inHand,
  keepHand,
  legalPlays,
  myHand,
  myMain,
  playCard,
  tip,
  unitOf,
} from "../steps.ts";

const GOING_LONG = "core-084";
const HONEYPOT = "core-060";
const SHEEPISH = "core-041";
const FARM = "core-058";
const RUSH_TOKEN = "core-t-rush";
const SHEEP = "core-t-sheep";

const CHEAP_UNITS: Readonly<Record<string, string>> = {
  "core-011": "Tempo Timmy",
  "core-015": "Me and Mr Token",
  "core-003": "Right-house defender",
};

function enemyFaceDown(ctx: CoachCtx): boolean {
  return ctx.view.opponent.backrow.some((card) => card !== null && card.faceDown);
}

function myBackrowLane(ctx: CoachCtx, defId: string): number | undefined {
  const index = ctx.view.you.backrow.findIndex((card) => card !== null && !card.faceDown && card.defId === defId);
  return index < 0 ? undefined : index + 1;
}

function played(ctx: CoachCtx, defId: string): boolean {
  const you = ctx.view.you;
  return (
    myBackrowLane(ctx, defId) !== undefined ||
    unitOf(ctx.view, "you", defId) !== undefined ||
    you.graveyard.some((card) => card.defId === defId) ||
    you.exile.some((card) => card.defId === defId)
  );
}

function trapFiredBy(ctx: CoachCtx, mine: boolean, defId?: string): boolean {
  return freshOf(ctx, "trapFired").some(
    (event) => (event.controller === ctx.view.viewer) === mine && (defId === undefined || event.defId === defId),
  );
}

/** Wait for a scheduled draw; retire only after the card has been played without the coach. */
function playWhenDrawn(options: Parameters<typeof playCard>[0]): CoachStep {
  const step = playCard(options);
  return {
    ...step,
    moot: (ctx, since) => since === null && inHand(ctx.view, options.defId) === undefined && played(ctx, options.defId),
  };
}

function baitIn(ctx: CoachCtx): CardView | undefined {
  const hand = myHand(ctx.view);
  for (const defId of Object.keys(CHEAP_UNITS)) {
    const card = hand.find((candidate) => candidate.defId === defId);
    if (card !== undefined && ctx.legal.some((action) => action.type === "play" && action.instanceId === card.instanceId)) return card;
  }
  return undefined;
}

function baitPlayed(ctx: CoachCtx): boolean {
  return (
    trapFiredBy(ctx, false) ||
    freshOf(ctx, "summoned").some((event) => event.player === ctx.view.viewer && event.row === "units" && CHEAP_UNITS[event.defId] !== undefined)
  );
}

function baitMoment(ctx: CoachCtx): boolean {
  return enemyFaceDown(ctx) && baitIn(ctx) !== undefined;
}

/** Retire a missed bait moment so the turn's next advice is never hidden. */
const bait: CoachStep = {
  id: "bait",
  kind: "act",
  title: "Test the trap",
  text: (ctx) => {
    const card = baitIn(ctx);
    const name = card === undefined ? "a cheap unit" : (CHEAP_UNITS[card.defId] ?? "a cheap unit");
    return `That face-down card could be a trap. Test it with a cheap unit before you risk a good one: play ${name}.`;
  },
  anchor: (ctx) => {
    const card = baitIn(ctx);
    return card === undefined ? { kind: "backrow", side: "opponent" } : { kind: "handCard", defId: card.defId };
  },
  when: (ctx) => myMain(ctx) && baitMoment(ctx),
  done: (ctx) => baitPlayed(ctx),
  moot: (ctx, since) => (since === null ? myMain(ctx) && !baitMoment(ctx) : ctx.view.turn !== since.turn && !baitPlayed(ctx)),
  expect: (action: ActionBody, ctx) => {
    const card = baitIn(ctx);
    return card !== undefined && action.type === "play" && action.instanceId === card.instanceId;
  },
};

function farmWorked(ctx: CoachCtx): boolean {
  return (
    myBackrowLane(ctx, FARM) !== undefined &&
    freshOf(ctx, "summoned").some((event) => event.player === ctx.view.viewer && event.defId === RUSH_TOKEN)
  );
}

function myTokenIds(view: PlayerView): Set<string> {
  return new Set(view.you.units.filter((unit) => unit !== null && unit.defId === RUSH_TOKEN).map((unit) => unit?.instanceId ?? ""));
}

function tokenCanAttack(ctx: CoachCtx): boolean {
  const tokens = myTokenIds(ctx.view);
  return ctx.legal.some((action) => action.type === "attack" && tokens.has(action.attackerId));
}

function tokenAttacked(ctx: CoachCtx, since: PlayerView): boolean {
  const tokens = myTokenIds(since);
  return freshOf(ctx, "attackDeclared").some((event) => !event.forced && tokens.has(event.attackerId));
}

/** Remember the shown instances: either token can complete this step, even after an automatic turn end (R82). */
const tokenAttack: CoachStep = {
  id: "token-attack",
  kind: "act",
  title: "Tokens fight too",
  text: "A token fights like any other unit. Attack with a Rush Token.",
  anchor: { kind: "unit", side: "you", defId: RUSH_TOKEN },
  when: (ctx) => myMain(ctx) && tokenCanAttack(ctx),
  done: (ctx, since) => tokenAttacked(ctx, since),
  moot: (ctx, since) => {
    if (since === null) return myMain(ctx) && !tokenCanAttack(ctx);
    return ctx.view.turn !== since.turn && !tokenAttacked(ctx, since);
  },
  expect: (action: ActionBody, ctx) => {
    const tokens = myTokenIds(ctx.view);
    return action.type === "attack" && tokens.has(action.attackerId);
  },
};

// Tips re-read each view, so their text derives from state that persists beyond the triggering event.

/** Names The Coin when the AI played it into the trap (§7: it costs 0). */
function honeypotText(ctx: CoachCtx): string {
  const coin = ctx.view.opponent.graveyard.some((card) => card.defId === COIN_DEF_ID);
  const what = coin ? "The AI played The Coin, which costs (0)" : "The AI played a (1) Cost or less card";
  return `${what}, so your Bear Honeypot sprang on its turn and made two Rush Tokens. A trap that has fired goes to the graveyard.`;
}

function enemyTrapText(ctx: CoachCtx): string {
  const sheepish = ctx.view.opponent.graveyard.some((card) => card.defId === SHEEPISH) || unitOf(ctx.view, "you", SHEEP) !== undefined;
  if (!sheepish) return "The AI's face-down card was a trap, and your play set it off. Its zone is empty again.";
  const sheeped = [...ctx.view.events].reverse().find((event) => event.type === "transformed" && event.toDefId === SHEEP);
  const name = sheeped?.type === "transformed" ? CHEAP_UNITS[sheeped.fromDefId] : undefined;
  return `It was Sheepish, a trap: it turned ${name ?? "your unit"} into a 1/1 Sheep. Better to lose your cheapest unit than your best one.`;
}

export const script: LessonScript = {
  lessonId: "traps",
  steps: [
    keepHand({ id: "keep", title: "Keep your hand", text: "Press Ready to keep all of these cards." }),
    info({
      id: "welcome",
      title: "The backrow",
      text: "Behind your units is a second row of five zones: the backrow. Traps and Field Spells go there, never units.",
      anchor: { kind: "backrow", side: "you" },
      when: (ctx) => myMain(ctx),
    }),
    info({
      id: "quickdraw",
      title: "Quickdraw",
      text: "Going Long has Quickdraw: a Quickdraw card always starts in your opening hand, so you can plan on it.",
      anchor: { kind: "handCard", defId: GOING_LONG },
      when: (ctx) => myMain(ctx),
      moot: (ctx, since) => since === null && inHand(ctx.view, GOING_LONG) === undefined,
    }),
    playCard({
      id: "set-trap",
      title: "Set a trap",
      text: "Bear Honeypot is a Trap. Pay 1 mana and drag it into your backrow. It goes in face-down, so the AI can't see what it is.",
      defId: HONEYPOT,
    }),
    info({
      id: "trap-waits",
      title: "A hidden trap",
      text: "The AI sees only the back of your card. Bear Honeypot springs by itself when the AI plays a (1) Cost or less card, even on its turn.",
      anchor: (ctx) => {
        const lane = myBackrowLane(ctx, HONEYPOT);
        return lane === undefined ? { kind: "backrow", side: "you" } : { kind: "backrow", side: "you", lane };
      },
      when: (ctx) => myBackrowLane(ctx, HONEYPOT) !== undefined,
      // A missed or sprung trap expires; the turn may already have ended automatically (R82).
      moot: (ctx, since) =>
        since === null && myBackrowLane(ctx, HONEYPOT) === undefined && (myMain(ctx) || played(ctx, HONEYPOT)),
      holdAi: true,
    }),
    playWhenDrawn({
      id: "field-spell",
      title: "A Field Spell",
      text: "Rush Token Farm is a Field Spell. It stays face-up in your backrow and makes a Rush Token at the start of each of your turns.",
      defId: FARM,
    }),
    tokenAttack,
    yourMove({ id: "move-2", title: "Your move" }),
    info({
      id: "farm-works",
      title: "The farm at work",
      text: "At the start of turn, Rush Token Farm made a new Rush Token. A Field Spell keeps working, turn after turn.",
      anchor: (ctx) =>
        unitOf(ctx.view, "you", RUSH_TOKEN) === undefined ? { kind: "backrow", side: "you" } : { kind: "unit", side: "you", defId: RUSH_TOKEN },
      when: (ctx) => myMain(ctx) && farmWorked(ctx),
      // Without the next-turn token, retire it so the turn's own steps follow.
      moot: (ctx, since) => since === null && myMain(ctx) && !farmWorked(ctx),
    }),
    playCard({
      id: "going-long",
      title: "Armor for your hero",
      text: "Going Long is a Field Spell too. While it stays in your backrow, your hero has Armor 2: every hit on it is 2 smaller.",
      defId: GOING_LONG,
    }),
    yourMove({ id: "move-3", title: "Your move" }),
    bait,
    yourMove({ id: "move-4", title: "Your move" }),
    yourMove({ id: "win", title: "Win the game", final: true }),
  ],
  tips: [
    tip({
      id: "honeypot-fired",
      title: "Your trap sprang",
      text: honeypotText,
      anchor: { kind: "units", side: "you" },
      when: (ctx) => trapFiredBy(ctx, true, HONEYPOT),
      holdAi: true,
    }),
    tip({
      id: "tokens",
      title: "Tokens",
      text: "A Rush Token is a token: a unit another card makes. It is 3/3 with Rush. A token vanishes for good when it leaves the field.",
      anchor: { kind: "unit", side: "you", defId: RUSH_TOKEN },
      // Show before the token prompt, not behind the trap bubbles on the AI's turn.
      when: (ctx) => myMain(ctx) && tokenCanAttack(ctx) && legalPlays(ctx, FARM).length === 0,
    }),
    tip({
      id: "enemy-facedown",
      title: "A face-down card",
      text: "The AI set a face-down card in its backrow. It could be a trap, and only the AI knows what it is.",
      anchor: { kind: "backrow", side: "opponent" },
      when: (ctx) => enemyFaceDown(ctx),
      holdAi: true,
    }),
    tip({
      id: "armor",
      title: "Armor at work",
      text: "Going Long's Armor 2 made that hit on your hero 2 smaller. It works on every hit, for as long as Going Long stays.",
      anchor: { kind: "hero", side: "you" },
      when: (ctx) =>
        myBackrowLane(ctx, GOING_LONG) !== undefined &&
        freshOf(ctx, "damage").some((event) => event.targetId === heroTargetId(ctx.view, "you")),
    }),
    tip({
      id: "enemy-trap",
      title: "The AI's trap",
      text: enemyTrapText,
      anchor: { kind: "units", side: "you" },
      when: (ctx) => trapFiredBy(ctx, false),
    }),
  ],
};
