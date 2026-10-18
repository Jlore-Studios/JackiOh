// What a "Choose one" option says to the player (presentation only, CLAUDE.md rule 7).
//
// A card script's mode options are its public interface (`crates/cards` owns them, e2e answers
// with them): short machine words such as Pocket Chaos's "health", "board" and "library". The
// picker used to print those words bare under "Choose one", with nothing saying which card asked or
// what each option does. This map gives each one a label and a line of detail, written from the
// card's §8 text, keyed by the card's id and the option string. The option string stays the key the
// answer sends; an option this map does not know is printed with a capital letter and no detail.
// The words are English here and read in the reader's language where they are handed out
// (`modeText`, through `t()`): the Chinese card table has each card's text, not its options' words.

import { t } from "../i18n/i18n.ts";

type ModeText = { label: string; detail: string };

export const MODE_TEXT: Readonly<Record<string, Readonly<Record<string, ModeText>>>> = {
  // #17 Flood (radiant): "Choose one: Bounce all Units, Bounce all enemy Units, or destroy all enemy Units. Then draw 1."
  "core-017": {
    "bounce all units": { label: "Bounce all units", detail: "Every unit returns to its controller's hand. Then draw 1." },
    "bounce all enemy units": { label: "Bounce enemy units", detail: "Your opponent's units return to their hand. Then draw 1." },
    "destroy all enemy units": { label: "Destroy enemy units", detail: "Destroy every enemy unit. Then draw 1." },
  },
  // #24 Efficiency Dividend: "Deal X+1 damage, heal a target 4X, or gain X mana next turn".
  "core-024": {
    damage: { label: "Deal X+1 damage", detail: "Deal X plus 1 damage." },
    heal: { label: "Heal 4X", detail: "Heal a target by four times X." },
    mana: { label: "Mana next turn", detail: "Gain X mana next turn." },
  },
  // #30 Archivist: "draw the highest-cost card in your library, or the lowest".
  "core-030": {
    highest: { label: "Highest cost", detail: "Draw the highest-cost card in your deck." },
    lowest: { label: "Lowest cost", detail: "Draw the lowest-cost card in your deck." },
  },
  // #48 5pek Controller (radiant): "Choose: all enemy units, or all units".
  "core-048": {
    enemy: { label: "Enemy units", detail: "Switch the position of every enemy unit." },
    all: { label: "All units", detail: "Switch the position of every unit." },
  },
  // #87 Pocket Chaos: "swap hero health, swap boards (every zone, lane-preserving), or swap libraries";
  // radiant: "You may skip adding it".
  "core-087": {
    health: { label: "Swap hero Health", detail: "You and your opponent trade hero Health." },
    board: { label: "Swap boards", detail: "Every zone changes sides, each card keeping its lane." },
    library: { label: "Swap decks", detail: "You and your opponent trade decks." },
    gift: { label: "Give a copy", detail: "Add a Pocket Chaos to your opponent's hand." },
    skip: { label: "Keep it to yourself", detail: "Your opponent gets no Pocket Chaos." },
  },
  // #88 Twisting Nether (radiant): "Choose: all enemy permanents, or all".
  "core-088": {
    enemy: { label: "Enemy permanents", detail: "Destroy every enemy permanent." },
    all: { label: "All permanents", detail: "Destroy every permanent on both sides." },
  },
  // C+ #40 Appropriations (X spell): "Choose one: Military, Education, Culture, or Healthcare" —
  // the effects the Discover menu shows, X filled in on play.
  "classicplus-040": {
    Military: { label: "Military", detail: "Your Units on the field, in your hand and in your deck get +2X Attack and Rush." },
    Education: { label: "Education", detail: "Shuffle 2X random Radiant Books into your deck. They have Cast on draw." },
    Culture: { label: "Culture", detail: "Each card on your field, in your hand and in your deck has an 8X% chance to become Radiant." },
    Healthcare: { label: "Healthcare", detail: "Your Units on the field, in your hand and in your deck get +2X Health and Armor X." },
  },
  // M #22 Mind Games: "Choose one in secret: Greed, Attack or Defend." The detail names no
  // numbers — the reward waits until the start of the caster's next turn (R861).
  "meditative-022": {
    greed: { label: "Greed", detail: "In secret: at the start of your next turn, gain mana and draw cards." },
    attack: { label: "Attack", detail: "In secret: at the start of your next turn, deal damage to each enemy." },
    defend: { label: "Defend", detail: "In secret: at the start of your next turn, heal your side and grant Armor." },
  },
  // M #22.1 Fortify Mind: "Choose one: Greed, Attack or Defend, to guess your opponent's Mind
  // Games." Rock beats scissors here: Attack beats Greed, Greed beats Defend, Defend beats Attack.
  "meditative-022-1": {
    greed: { label: "Guess Greed", detail: "Beats Defend; loses to Attack." },
    attack: { label: "Guess Attack", detail: "Beats Greed; loses to Defend." },
    defend: { label: "Guess Defend", detail: "Beats Attack; loses to Greed." },
  },
  // M #98 Showdown: "Choose a lane. Lock every other lane until the start of your next turn." The
  // board draws lane 1 on the left for both seats (Board.tsx), so each option says where it is.
  "meditative-098": lanes("Lock the other four lanes until the start of your next turn."),
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
  // C+ #40 Appropriations, radiant: Attack and Armor doubled, the Books and the Culture chance 2.5 times, Health tripled.
  "classicplus-040": {
    Military: { label: "Military", detail: "Your Units on the field, in your hand and in your deck get +4X Attack and Rush." },
    Education: { label: "Education", detail: "Shuffle 5X random Radiant Books into your deck. They have Cast on draw." },
    Culture: { label: "Culture", detail: "Each card on your field, in your hand and in your deck has a 20X% chance to become Radiant." },
    Healthcare: { label: "Healthcare", detail: "Your Units on the field, in your hand and in your deck get +6X Health and Armor 2X." },
  },
  // M #98 Showdown, radiant: "This turn, after you put a card into that lane, cast Book of Buff on it."
  "meditative-098": lanes("Lock the other four lanes until the start of your next turn. This turn, Book of Buff is cast on each card you put into this one."),
  // #24 Efficiency Dividend, radiant: "Deal 2X+1 damage, heal a target 8X, or gain X+1 mana next turn"
  // (§8 #24, R275).
  "core-024": {
    damage: { label: "Deal 2X+1 damage", detail: "Deal twice X plus 1 damage." },
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
