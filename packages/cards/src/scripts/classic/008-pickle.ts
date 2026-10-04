// C #8 Pickle (SPEC §8.6 row 8, §6.3 Choose one, §10.6; R16, R79, R97, R113, R177). Spell, cost 1, Rare.
//   Base:    "Your opponent chooses {choices|time|times}: they discard {discard|card|cards}, they exile
//            the bottom {exile|card|cards} of their deck, or you draw {draw|card|cards}." (3; 1, 1, 1)
//   Radiant: the same words with 3; 2, 2, 2.
//   Engine:  "Three mode prompts held by the opponent (§10.6), one after another, repeats allowed;
//            "discard" discards at random from their hand (R654). Only choices that would do something
//            are offered, and "you draw" always is. The opponent answers during your turn, a non-active
//            player's prompt on its own clock (R79); a timeout answers with the AI policy. Tunes:
//            choices 3 ↑; discard 1 ↑; exile 1 ↑; draw 1 ↑."
//
// THE CHOICES are mode prompts the opponent holds (B5 E18, `chooseMode({ by: "enemy" })`): they read
// the options and answer, and the answered step still runs as this card's controller, so "you draw"
// draws for you. Each answer asks the next question, so the chain is a named step (`chosen`) and a
// count carried in the prompt's data (§10.6): `n` is the question being asked, and the numbers the
// card declares (`param`) are read once as the Spell resolves and carried with it, so every question
// of one resolution reads the same numbers.
//
// ONLY CHOICES THAT WOULD DO SOMETHING are offered, read as each question is asked: "discard" while
// their hand holds a card, "exile" while their deck does, and "you draw" always — a draw from an
// empty deck is fatigue (§2.4), which still does something. The options name no card.
//
// "DISCARD" is random from their hand (R654: no "of your choice"), so no prompt opens and the next
// question follows at once. You read only the discard events, never the cards (R177).
//
// THE CLOCK is not the card's: the engine runs a non-active player's prompt on its own clock, and a
// timeout answers it with the AI policy (R79).

import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { param, zoneCount } from "@jackioh/engine";
import { chooseMode, chosenOptions, discardRandom, draw, exileBottomOfLibrary } from "@jackioh/engine/effects";
import { opponentOf } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-008");

const DISCARD = "discard";
const EXILE = "exile";
const DRAW = "draw";

/** The card's numbers, read once as it resolves and carried through every question (§10.6). */
type Numbers = { choices: number; discard: number; exile: number; draw: number };

/** The chain's data: the numbers, and which question is being asked (1-based). */
type ChainData = Numbers & { n: number };

function readNumbers(ctx: EffectContext): Numbers {
  return {
    choices: param(ctx, "choices"),
    discard: param(ctx, "discard"),
    exile: param(ctx, "exile"),
    draw: param(ctx, "draw"),
  };
}

function numberIn(data: Record<string, unknown>, key: keyof ChainData): number | null {
  const value = data[key];
  return typeof value === "number" && Number.isInteger(value) ? value : null;
}

/** §10.6's data is JSON, so what comes back is narrowed, never cast. */
function chainOf(ctx: EffectContext): ChainData | null {
  const n = numberIn(ctx.data, "n");
  const choices = numberIn(ctx.data, "choices");
  const discardCount = numberIn(ctx.data, "discard");
  const exileCount = numberIn(ctx.data, "exile");
  const drawCount = numberIn(ctx.data, "draw");
  if (n === null || choices === null || discardCount === null || exileCount === null || drawCount === null) return null;
  return { n, choices, discard: discardCount, exile: exileCount, draw: drawCount };
}

function cards(count: number): string {
  return count === 1 ? "1 card" : `${count} cards`;
}

/** The captions the opponent reads: from their seat, and naming no card. */
function labelsFor(numbers: Numbers): Record<string, string> {
  return {
    [DISCARD]: `Discard ${cards(numbers.discard)}`,
    [EXILE]: `Exile the bottom ${cards(numbers.exile)} of your deck`,
    [DRAW]: `Your opponent draws ${cards(numbers.draw)}`,
  };
}

/**
 * What an effect built in the same list as the next question will have taken from their piles by
 * the time that question opens: a question's options are read as its list is built, before the
 * discards or the exile ahead of it in that list apply, so the count they take is subtracted here.
 */
type Pending = { handLoss?: number; deckLoss?: number };

/** Question `chain.n`, offering only what would do something; nothing once every question is asked. */
function ask(ctx: EffectContext, chain: ChainData, pending: Pending = {}): Effect[] {
  if (chain.n > chain.choices) return [];
  const enemy = opponentOf(ctx.controller);
  const options: string[] = [];
  if (zoneCount(ctx.state, enemy, "hand") - (pending.handLoss ?? 0) > 0) options.push(DISCARD);
  if (zoneCount(ctx.state, enemy, "library") - (pending.deckLoss ?? 0) > 0) options.push(EXILE);
  options.push(DRAW);
  return [
    chooseMode({
      by: "enemy",
      options,
      labels: labelsFor(chain),
      step: "chosen",
      prompt: `Pickle: your choice (${chain.n} of ${chain.choices})`,
      data: { ...chain },
    }),
  ];
}

const pickle: Script = {
  cry: (ctx) => ask(ctx, { ...readNumbers(ctx), n: 1 }),
  resume: {
    chosen: (ctx) => {
      const chain = chainOf(ctx);
      if (chain === null) return [];
      const next = { ...chain, n: chain.n + 1 };
      const enemy = opponentOf(ctx.controller);
      switch (chosenOptions(ctx)[0]) {
        case DISCARD: {
          // A hand emptied since the question was asked has nothing to discard: on to the next one.
          // The random discard lands first in the list, so the next question subtracts it (Pending).
          const handLoss = Math.min(chain.discard, zoneCount(ctx.state, enemy, "hand"));
          if (handLoss === 0) return ask(ctx, next);
          return [discardRandom({ count: chain.discard, player: "enemy" }), ...ask(ctx, next, { handLoss })];
        }
        case EXILE: {
          const deckLoss = Math.min(chain.exile, zoneCount(ctx.state, enemy, "library"));
          return [exileBottomOfLibrary({ player: "enemy", count: chain.exile }), ...ask(ctx, next, { deckLoss })];
        }
        case DRAW:
          return [draw({ count: chain.draw }), ...ask(ctx, next)];
        default:
          return [];
      }
    },
  },
};

export const base: Script = pickle;

// The same script: the Radiant face differs only in its declared numbers (discard, exile and draw 2),
// which `param` reads off the running face.
export const radiant: Script = pickle;
