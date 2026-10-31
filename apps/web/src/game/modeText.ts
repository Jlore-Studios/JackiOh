// What a "Choose one" option says to the player (presentation only, CLAUDE.md rule 7).
//
// A card script's mode options are its public interface (`crates/cards` owns them, e2e answers
// with them). This map gives each one a label and a line of detail, written from the card's §8 text.
// The option string stays the key the answer sends. The words are English here and read in the
// reader's language through `t()`: the Chinese card table has each card's text, not its options' words.

import { t } from "../i18n/i18n.ts";

type ModeText = { label: string; detail: string };

export const MODE_TEXT: Readonly<Record<string, Readonly<Record<string, ModeText>>>> = {
  // #17 Flood (radiant).
  "core-017": {
    "bounce all units": { label: "Bounce all units", detail: "Every unit returns to its controller's hand. Then draw 1." },
    "bounce all enemy units": { label: "Bounce enemy units", detail: "Your opponent's units return to their hand. Then draw 1." },
    "destroy all enemy units": { label: "Destroy enemy units", detail: "Destroy every enemy unit. Then draw 1." },
  },
  // #24 Efficiency Dividend.
  "core-024": {
    damage: { label: "Deal X damage", detail: "Deal X damage." },
    heal: { label: "Heal 4X", detail: "Heal a target by four times X." },
    mana: { label: "Mana next turn", detail: "Gain X mana next turn." },
  },
  // #30 Archivist.
  "core-030": {
    highest: { label: "Highest cost", detail: "Draw the highest-cost card in your deck." },
    lowest: { label: "Lowest cost", detail: "Draw the lowest-cost card in your deck." },
  },
  // #48 5pek Controller (radiant).
  "core-048": {
    enemy: { label: "Enemy units", detail: "Switch the position of every enemy unit." },
    all: { label: "All units", detail: "Switch the position of every unit." },
  },
  // #87 Pocket Chaos; radiant: "You may skip adding it".
  "core-087": {
    health: { label: "Swap hero Health", detail: "You and your opponent trade hero Health." },
    board: { label: "Swap boards", detail: "Every zone changes sides, each card keeping its lane." },
    library: { label: "Swap decks", detail: "You and your opponent trade decks." },
    gift: { label: "Give a copy", detail: "Add a Pocket Chaos to your opponent's hand." },
    skip: { label: "Keep it to yourself", detail: "Your opponent gets no Pocket Chaos." },
  },
  // #88 Twisting Nether (radiant).
  "core-088": {
    enemy: { label: "Enemy permanents", detail: "Destroy every enemy permanent." },
    all: { label: "All permanents", detail: "Destroy every permanent on both sides." },
  },
  // C+ #40 Appropriations (X spell): the effects the Discover menu shows, X filled in on play.
  "classicplus-040": {
    Military: { label: "Military", detail: "Your Units on the field, in your hand and in your deck get +(2X−1) Attack and Rush." },
    Education: { label: "Education", detail: "Shuffle 2X−1 random Radiant Books into your deck. They have Cast on draw." },
    Culture: { label: "Culture", detail: "Each card on your field, in your hand and in your deck has a 7X% chance to become Radiant." },
    Healthcare: { label: "Healthcare", detail: "Your Units on the field, in your hand and in your deck get +(2X−1) Health and Armor X." },
  },
  // C+ #7 The House: the twin is the Activate's mode (R1704002).
  "classicplus-007": {
    defender: { label: "Right-house defender", detail: "Spend a Plague Counter to summon a Right-house defender." },
    attacker: { label: "Wrong-House Attacker", detail: "Spend a Plague Counter to summon a Wrong-House Attacker." },
  },
  // M #22 Mind Games: the detail names no numbers — the reward waits until the start of the caster's
  // next turn (R861).
  "meditative-022": {
    greed: { label: "Greed", detail: "In secret: at the start of your next turn, gain mana and draw cards." },
    attack: { label: "Attack", detail: "In secret: at the start of your next turn, deal damage to each enemy." },
    defend: { label: "Defend", detail: "In secret: at the start of your next turn, heal your side and grant Armor." },
  },
  // M #22.1 Fortify Mind: a guess at the opponent's Mind Games.
  "meditative-022-1": {
    greed: { label: "Guess Greed", detail: "Beats Defend; loses to Attack." },
    attack: { label: "Guess Attack", detail: "Beats Greed; loses to Defend." },
    defend: { label: "Guess Defend", detail: "Beats Attack; loses to Greed." },
  },
  // M #98 Showdown: the board draws lane 1 on the left for both seats (Board.tsx), so each option
  // says where it is.
  "meditative-098": lanes("Lock the other four lanes until the start of your next turn."),
  // M+ #4 Supply Drop: the wait and the numbers are the card's and its Embiggened version's, so the
  // detail names none.
  "meditativeplus-004": {
    combat: { label: "Combat", detail: "In a few turns, deal damage to each enemy, with Pierce and Lifesteal." },
    supplies: { label: "Supplies", detail: "In a few turns, add Radiant Jlockeed cards to your hand." },
  },
  // M+ #24 Bob, Wisconsin's Strongest Soldier: the Coins' count is the card's, so the detail names none.
  "meditativeplus-024": {
    freeze: { label: "Freeze", detail: "Freeze every enemy Unit." },
    take: { label: "Take a Unit", detail: "Put the enemy Unit you choose into your hand. Your opponent gets Coins." },
    discover: { label: "Discover a Unit", detail: "Discover a Unit and make it Radiant." },
    copies: { label: "Draw and copy", detail: "Draw 1, then fill your hand with copies of it." },
  },
};

/** Showdown's five lanes, `lane-1` to `lane-5`, each named by where it sits on the board. */
function lanes(detail: string): Record<string, ModeText> {
  const where = ["on the left", "second from the left", "in the middle", "second from the right", "on the right"];
  return Object.fromEntries(where.map((place, index) => [`lane-${String(index + 1)}`, { label: `Lane ${String(index + 1)}`, detail: `The lane ${place}. ${detail}` }]));
}

/**
 * The options whose words change on the card's Radiant face, keyed like `MODE_TEXT`. Any option
 * missing here reads the same on both faces.
 */
export const RADIANT_MODE_TEXT: Readonly<Record<string, Readonly<Record<string, ModeText>>>> = {
  // C+ #40 Appropriations, radiant.
  "classicplus-040": {
    Military: { label: "Military", detail: "Your Units on the field, in your hand and in your deck get +(4X−2) Attack and Rush." },
    Education: { label: "Education", detail: "Shuffle 4X random Radiant Books into your deck. They have Cast on draw." },
    Culture: { label: "Culture", detail: "Each card on your field, in your hand and in your deck has a 17X% chance to become Radiant." },
    Healthcare: { label: "Healthcare", detail: "Your Units on the field, in your hand and in your deck get +5X Health and Armor 2X." },
  },
  // C+ #7 The House, radiant: the twins it summons are Radiant.
  "classicplus-007": {
    defender: { label: "Radiant Right-house defender", detail: "Spend a Plague Counter to summon a Radiant Right-house defender." },
    attacker: { label: "Radiant Wrong-House Attacker", detail: "Spend a Plague Counter to summon a Radiant Wrong-House Attacker." },
  },
  // M #98 Showdown, radiant.
  "meditative-098": lanes("Lock the other four lanes until the start of your next turn. This turn, Book of Buff is cast on each card you put into this one."),
  // #24 Efficiency Dividend, radiant (§8 #24, R275).
  "core-024": {
    damage: { label: "Deal 2X damage", detail: "Deal twice X damage." },
    heal: { label: "Heal 8X", detail: "Heal a target by eight times X." },
    mana: { label: "Mana next turn", detail: "Gain X plus 1 mana next turn." },
  },
};

/** A mode's words in the reader's language. */
function read(text: ModeText): ModeText {
  return { label: t(text.label), detail: t(text.detail) };
}

/**
 * An option's label and detail for card `defId` on the face it will resolve with (`radiant`), in the
 * reader's language; an unknown one is its own word, capitalised.
 */
export function modeText(
  defId: string | undefined,
  option: string,
  radiant = false,
): { label: string; detail?: string } {
  const face = radiant && defId !== undefined ? RADIANT_MODE_TEXT[defId]?.[option] : undefined;
  if (face !== undefined) return read(face);
  const known = defId === undefined ? undefined : MODE_TEXT[defId]?.[option];
  if (known !== undefined) return read(known);
  if (option.length === 0) return { label: option };
  // The option's own word first, where the language has one: "draw" is not the result word "Draw".
  const own = t(option);
  return { label: own !== option ? own : t(`${option.charAt(0).toUpperCase()}${option.slice(1)}`) };
}
