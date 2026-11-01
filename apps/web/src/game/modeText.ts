// What a "Choose one" option says to the player (presentation only, CLAUDE.md rule 7).
//
// A card script's mode options are its public interface (`crates/cards` owns them, e2e answers
// with them): short machine words such as Pocket Chaos's "health", "board" and "library". The
// picker used to print those words bare under "Choose one", with nothing saying which card asked or
// what each option does. This map gives each one a label and a line of detail, written from the
// card's §8 text, keyed by the card's id and the option string. The option string stays the key the
// answer sends; an option this map does not know is printed with a capital letter and no detail.
// #2646: a short choice's options are choice cards, whose text is the detail, so modeText.test.ts holds
// this map to every option a card offers in a Choose one of DISCOVER_OPTION_LIMIT options or fewer: the
// ones the scripts declare (cardModes.json) and the ones an engine prompt asks (its `ENGINE_MODES`).
// The details name no number a card declares (its `params`), which a patch may tune.
// The words are English here and read in the reader's language where they are handed out
// (`modeText`, through `t()`): the Chinese card table has each card's text, not its options' words.

import { SHIPPED_SETS } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";

type ModeText = { label: string; detail: string };

export const MODE_TEXT: Readonly<Record<string, Readonly<Record<string, ModeText>>>> = {
  // #17 Flood (radiant): "Choose one: Bounce all Units, Bounce all enemy Units, or destroy all enemy Units. Then draw 1."
  "core-017": {
    "bounce all units": { label: "Bounce all units", detail: "Every unit returns to its controller's hand. Then draw 1." },
    "bounce all enemy units": { label: "Bounce enemy units", detail: "Your opponent's units return to their hand. Then draw 1." },
    "destroy all enemy units": { label: "Destroy enemy units", detail: "Destroy every enemy unit. Then draw 1." },
  },
  // #24 Efficiency Dividend: "Deal X damage, heal a target 4X, or gain X mana next turn".
  "core-024": {
    damage: { label: "Deal X damage", detail: "Deal X damage." },
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
    Military: { label: "Military", detail: "Your Units on the field, in your hand and in your deck get +(2X−1) Attack and Rush." },
    Education: { label: "Education", detail: "Shuffle 2X−1 random Radiant Books into your deck. They have Cast on draw." },
    Culture: { label: "Culture", detail: "Each card on your field, in your hand and in your deck has a 7X% chance to become Radiant." },
    Healthcare: { label: "Healthcare", detail: "Your Units on the field, in your hand and in your deck get +(2X−1) Health and Armor X." },
  },
  // C+ #7 The House: "Activate: Spend a Plague Counter from this to summon a Right-house defender or a
  // Wrong-House Attacker" — the twin is the Activate's mode (R1704002).
  "classicplus-007": {
    defender: { label: "Right-house defender", detail: "Spend a Plague Counter to summon a Right-house defender." },
    attacker: { label: "Wrong-House Attacker", detail: "Spend a Plague Counter to summon a Wrong-House Attacker." },
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
  // M+ #4 Supply Drop: "Choose one: In 3 turns, deal 4 damage to each enemy, with Pierce and Lifesteal;
  // or in 3 turns, add 1 random Radiant Jlockeed card to your hand, and each costs (2)." The wait and
  // the numbers are the card's and its Embiggened version's, so the detail names none.
  "meditativeplus-004": {
    combat: { label: "Combat", detail: "In a few turns, deal damage to each enemy, with Pierce and Lifesteal." },
    supplies: { label: "Supplies", detail: "In a few turns, add Radiant Jlockeed cards to your hand." },
  },
  // M+ #24 Bob, Wisconsin's Strongest Soldier: "Cry: Choose one: Freeze all enemy Units; put an enemy
  // Unit into your hand, and your opponent gets 3 Coins; Discover a Unit and make it Radiant; or draw 1
  // and fill your hand with copies of it." The Coins' count is the card's, so the detail names none.
  "meditativeplus-024": {
    freeze: { label: "Freeze", detail: "Freeze every enemy Unit." },
    take: { label: "Take a Unit", detail: "Put the enemy Unit you choose into your hand. Your opponent gets Coins." },
    discover: { label: "Discover a Unit", detail: "Discover a Unit and make it Radiant." },
    copies: { label: "Draw and copy", detail: "Draw 1, then fill your hand with copies of it." },
  },
  // C #8 Pickle: "At the start of your opponent's next turn, they choose three times: they discard a
  // card, they exile the bottom card of their deck, or you draw a card." The opponent answers, so the
  // words are theirs; the engine's labels carry the counts.
  "classic-008": {
    discard: { label: "Discard", detail: "You discard at random from your hand." },
    exile: { label: "Exile bottom", detail: "You exile the bottom of your deck." },
    draw: { label: "They draw", detail: "Your opponent draws." },
  },
  // C #20 The Power to Punish: "Activate: choose one: deal 2 Pierce damage to a target; your opponent
  // discards a card; or choose a Unit, which is destroyed at the start of your next turn"; the Radiant's
  // third: "all enemy Units are destroyed at the start of your next turn".
  "classic-020": {
    "deal damage": { label: "Deal damage", detail: "Deal Pierce damage to a target." },
    "opponent discards": { label: "Opponent discards", detail: "Your opponent discards at random from their hand." },
    "destroy a Unit at the start of your next turn": { label: "Doom a Unit", detail: "Choose a Unit. It is destroyed at the start of your next turn." },
    "destroy all enemy Units at the start of your next turn": { label: "Doom enemy Units", detail: "Every enemy Unit is destroyed at the start of your next turn." },
  },
  // C #81 The Power to Thrive: "Activate: choose one: heal your hero 3; draw 1; or reduce the Cost of a
  // card in your hand by (1)".
  "classic-081": {
    heal: { label: "Heal", detail: "Heal your hero." },
    draw: { label: "Draw cards", detail: "Draw from your deck." },
    reduce: { label: "Discount", detail: "Reduce the Cost of a card in your hand." },
  },
  // C+ #34 Memory Leak: "Choose one: 'End of turn: Lock a random zone on your opponent's side'; or 'After
  // your opponent plays a Unit or Field Spell, Lock its zone'".
  "classicplus-034": {
    endOfTurn: { label: "Lock at end of turn", detail: "End of turn: Lock a random zone on your opponent's side." },
    afterPlay: { label: "Lock what they play", detail: "After your opponent plays a Unit or Field Spell, Lock its zone." },
  },
  // C+ #35 Rollback (radiant): "your side, your opponent's side or both. Return that part of the board to
  // how it was that many turns ago".
  "classicplus-035": {
    "your side": { label: "Your side", detail: "Return your side of the board to how it was." },
    "your opponent's side": { label: "Their side", detail: "Return your opponent's side of the board to how it was." },
    "both sides": { label: "Both sides", detail: "Return the whole board to how it was." },
  },
  // C+ #42 KY's Test: "Offer an easy, a medium, and a hard problem, each with a random reward." The
  // engine's labels name the rolled rewards.
  "classicplus-042": {
    Easy: { label: "Easy", detail: "An easy problem. Answer it right for its reward." },
    Medium: { label: "Medium", detail: "A medium problem. Answer it right for its reward." },
    Hard: { label: "Hard", detail: "A hard problem. Answer it right for its reward." },
  },
  // C+ #44 Simplicity Audit (radiant): "Choose all permanents or only your opponent's; exile each of them
  // whose card has fewer lines of code than this one".
  "classicplus-044": {
    "All permanents": { label: "All permanents", detail: "Exile every permanent whose card has fewer lines of code than this one." },
    "Only your opponent's": { label: "Only your opponent's", detail: "Exile each enemy permanent whose card has fewer lines of code than this one." },
  },
  // C+ #45 Complexity Audit (radiant): C+ #44 with "more lines of code".
  "classicplus-045": {
    "All permanents": { label: "All permanents", detail: "Exile every permanent whose card has more lines of code than this one." },
    "Only your opponent's": { label: "Only your opponent's", detail: "Exile each enemy permanent whose card has more lines of code than this one." },
  },
  // C+ #62 KY's Papaya: "Discover 4 cells … the curve goes through them … destroy every card on the
  // curve". Its cells are `MODE_SHAPES`'; the engine's labels say where each is.
  "classicplus-062": {
    done: { label: "Done", detail: "Pick no more cells: the curve goes through the ones picked." },
  },
  // C+ #77 Anti-Softlock (radiant): "choose all cards or only yours: each of them on the field, in hands
  // and in decks gains Stack and Pierce".
  "classicplus-077": {
    "All cards": { label: "All cards", detail: "Every card on the field, in hands and in decks gains Stack and Pierce." },
    "Only yours": { label: "Only yours", detail: "Your cards on the field, in your hand and in your deck gain Stack and Pierce." },
  },
  // #51 KY's Private Tutor: "Choose a type (Spell, Unit, Field Spell, Trap), then a cost bracket (0–1, 2,
  // 3, 4+) … reveal 3 random matching library cards".
  "core-051": {
    Spell: { label: "Spell", detail: "Look for a Spell in your deck." },
    Unit: { label: "Unit", detail: "Look for a Unit in your deck." },
    "Field Spell": { label: "Field Spell", detail: "Look for a Field Spell in your deck." },
    Trap: { label: "Trap", detail: "Look for a Trap or a Field Trap in your deck." },
    "0-1": { label: "0–1", detail: "Look for one that costs (0) or (1)." },
    "2": { label: "2", detail: "Look for one that costs (2)." },
    "3": { label: "3", detail: "Look for one that costs (3)." },
    "4+": { label: "4+", detail: "Look for one that costs (4) or more." },
  },
  // #65 Masochism Mask: "Start of turn: choose one: exile the bottom card of your library, lose 3 health,
  // or summon a Spikey Pillow"; radiant: "Choose twice from: nothing, exile bottom, lose 3, summon Spikey
  // Pillow". "Lose" is not damage.
  "core-065": {
    "exile bottom": { label: "Exile bottom", detail: "Exile the bottom card of your deck." },
    "lose 3": { label: "Lose 3", detail: "Lose that much Health. It is not damage." },
    "summon Spikey Pillow": { label: "Spikey Pillow", detail: "Summon a Spikey Pillow." },
    nothing: { label: "Nothing", detail: "Nothing happens." },
  },
  // #82 KY's Trial: "Discover one of 3 random sets, then one of 3 random numbers in it; add the Radiant
  // card with that number to your hand" (R831004: a set is offered by its name).
  "core-082": trialSets(),
  // M #20 Aestheticize the Game: "Choose one; for the rest of the game you also win while it is true".
  // The thresholds are the card's, so the detail names none.
  "meditative-020": {
    health: { label: "Health", detail: "For the rest of the game, you also win while your hero has enough Health." },
    graveyard: { label: "Graveyard", detail: "For the rest of the game, you also win while your graveyard holds enough cards." },
    board: { label: "Board", detail: "For the rest of the game, you also win while your Units have enough Attack and Health in total." },
  },
  // M #85 Playtester, as v0.5.2 played it (R1161, kept for its replays): "Start of turn: add a Book of
  // Buff or a Book of Nerf to your hand".
  "meditative-085": {
    "Book of Buff": { label: "Book of Buff", detail: "Add a Book of Buff to your hand." },
    "Book of Nerf": { label: "Book of Nerf", detail: "Add a Book of Nerf to your hand." },
  },
  // S #2 Absolute Zero (radiant): "Freeze a Unit forever. Also: you may give it Indestructible".
  "stinkyscape-002": {
    "give Indestructible": { label: "Indestructible too", detail: "Freeze the Unit forever, and give it Indestructible." },
    "freeze only": { label: "Freeze only", detail: "Freeze the Unit forever." },
  },
  // S #18.1 MELCHIOR: "Activate: Discover a combat keyword and give it to each of your Units" (R21's pool).
  "stinkyscape-018-1": combatKeywords(),
  // S #26 ZolarKeth (#2657): "Cry: Choose one: summon a University, or this gains 'Activate: Summon a random
  // Starship Piece.'" `universities` is v0.5.9f's Activate's other option, kept for its replays.
  "stinkyscape-026": {
    university: { label: "University", detail: "Summon a University." },
    piece: { label: "Starship Piece", detail: "This gains “Activate: Summon a random Starship Piece.”" },
    universities: { label: "Universities", detail: "Activate your Universities." },
  },
  // S #112 Job Application, Stage 3: "Activate to accept (a Paycheck each start of turn from the next) or
  // re-apply with every reward tripled".
  "stinkyscape-112": {
    "Accept the offer": { label: "Accept the offer", detail: "Get a Paycheck at the start of each of your turns, from the next one." },
    "Send in another application": { label: "Apply again", detail: "Re-apply, with every reward tripled." },
  },
  // S #112.1 Paycheck: "Choose one: gain 2 mana, draw 1 and gain 20 Credits; or go all in on red, an 18/39
  // chance at twice that".
  "stinkyscape-112-1": {
    Payday: { label: "Payday", detail: "Gain mana this turn, draw and gain Credits." },
    "All in on red": { label: "All in on red", detail: "A spin of the wheel: a win gives twice the Payday, a loss nothing." },
  },
  // S #116 Megabase: "Activate: both players gain 5, 10 or 15 Credits, your choice".
  "stinkyscape-116": {
    "5 Credits": { label: "5 Credits", detail: "Both players gain 5 Credits." },
    "10 Credits": { label: "10 Credits", detail: "Both players gain 10 Credits." },
    "15 Credits": { label: "15 Credits", detail: "Both players gain 15 Credits." },
  },
  // S #121 System Schematic: "Add your choice of a Core System, Secure System, Unsecure System or Wild
  // System (S #117 to #120) to your hand".
  "stinkyscape-121": {
    "Core System": { label: "Core System", detail: "Add a Core System to your hand." },
    "Secure System": { label: "Secure System", detail: "Add a Secure System to your hand." },
    "Unsecure System": { label: "Unsecure System", detail: "Add an Unsecure System to your hand." },
    "Wild System": { label: "Wild System", detail: "Add a Wild System to your hand." },
  },
  // S #148.1 Edge of the Universe (radiant): "You choose whether it is a draw or a loss for your opponent".
  "stinkyscape-148-1": {
    Draw: { label: "Draw", detail: "Escape the Universe: you win, and it is a draw for your opponent." },
    Loss: { label: "Loss", detail: "Escape the Universe: you win, and your opponent loses." },
  },
  // S #157 The Gauntlet (radiant): "Activate: choose one: +2 Brittle, or Tribute this".
  "stinkyscape-157": {
    "gain Brittle": { label: "More Brittle", detail: "This gains more Brittle." },
    "Tribute this": { label: "Tribute this", detail: "Tribute this card." },
  },
};

/** Showdown's five lanes, `lane-1` to `lane-5`, each named by where it sits on the board. */
function lanes(detail: string): Record<string, ModeText> {
  const where = ["on the left", "second from the left", "in the middle", "second from the right", "on the right"];
  return Object.fromEntries(where.map((place, index) => [`lane-${String(index + 1)}`, { label: `Lane ${String(index + 1)}`, detail: `The lane ${place}. ${detail}` }]));
}

/**
 * The options a card offers by their shape rather than by name, keyed by the card's id: C+ #62 KY's
 * Papaya's cells, each `x,y` on the curve's grid from the caster's seat (`papaya.rs`).
 */
const MODE_SHAPES: Readonly<Record<string, { shape: RegExp; text: ModeText }>> = {
  "classicplus-062": { shape: /^\d+,\d+$/, text: { label: "Cell", detail: "The curve goes through this cell." } },
};

/** #82 KY's Trial's sets, as the engine names them (`SHIPPED_SETS`). */
function trialSets(): Record<string, ModeText> {
  return Object.fromEntries(
    SHIPPED_SETS.map((set) => [set, { label: set, detail: `Then Discover a number in the ${set} set, and add the Radiant card with that number to your hand.` }]),
  );
}

/** R21's combat keywords, the pool S #18.1 MELCHIOR Discovers from (`RANDOM_KEYWORD_POOL`). */
function combatKeywords(): Record<string, ModeText> {
  const pool = ["Taunt", "Rush", "First Strike", "Flying", "Ranged", "Poisonous", "Reborn", "Divine Shield", "Windfury", "Cleave", "Deft", "Lifesteal", "Pierce", "Trample"];
  return Object.fromEntries(pool.map((keyword) => [keyword, { label: keyword, detail: `Each of your Units gains ${keyword}.` }]));
}

/**
 * The options whose words change on the card's Radiant face, keyed like `MODE_TEXT`. Any option
 * missing here reads the same on both faces.
 */
export const RADIANT_MODE_TEXT: Readonly<Record<string, Readonly<Record<string, ModeText>>>> = {
  // C+ #40 Appropriations, radiant: Attack and Armor doubled, and more Books, Culture chance and Health.
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
  // M #98 Showdown, radiant: "This turn, after you put a card into that lane, cast Book of Buff on it."
  "meditative-098": lanes("Lock the other four lanes until the start of your next turn. This turn, Book of Buff is cast on each card you put into this one."),
  // #24 Efficiency Dividend, radiant: "Deal 2X damage, heal a target 8X, or gain X+1 mana next turn"
  // (§8 #24, R275).
  "core-024": {
    damage: { label: "Deal 2X damage", detail: "Deal twice X damage." },
    heal: { label: "Heal 8X", detail: "Heal a target by eight times X." },
    mana: { label: "Mana next turn", detail: "Gain X plus 1 mana next turn." },
  },
  // S #116 Megabase, radiant: "its Activate gives its controller N and the opponent N − 5".
  "stinkyscape-116": {
    "5 Credits": { label: "5 Credits", detail: "You gain 5 Credits, and your opponent 5 fewer." },
    "10 Credits": { label: "10 Credits", detail: "You gain 10 Credits, and your opponent 5 fewer." },
    "15 Credits": { label: "15 Credits", detail: "You gain 15 Credits, and your opponent 5 fewer." },
  },
  // S #26 ZolarKeth, radiant (#2657): the University and the Starship Piece are Radiant.
  "stinkyscape-026": {
    university: { label: "Radiant University", detail: "Summon a Radiant University." },
    piece: { label: "Radiant Starship Piece", detail: "This gains “Activate: Summon a random Radiant Starship Piece.”" },
  },
  // S #121 System Schematic, radiant: "The one chosen Radiant".
  "stinkyscape-121": {
    "Core System": { label: "Core System", detail: "Add a Radiant Core System to your hand." },
    "Secure System": { label: "Secure System", detail: "Add a Radiant Secure System to your hand." },
    "Unsecure System": { label: "Unsecure System", detail: "Add a Radiant Unsecure System to your hand." },
    "Wild System": { label: "Wild System", detail: "Add a Radiant Wild System to your hand." },
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
  const shaped = defId === undefined ? undefined : MODE_SHAPES[defId];
  if (shaped?.shape.test(option) === true) return read(shaped.text);
  if (option.length === 0) return { label: option };
  // The option's own word first, where the language has one: "draw" is not the result word "Draw".
  const own = t(option);
  return { label: own !== option ? own : t(`${option.charAt(0).toUpperCase()}${option.slice(1)}`) };
}
