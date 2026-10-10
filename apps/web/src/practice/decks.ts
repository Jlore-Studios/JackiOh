// Practice presets are token-free Core decks, checked under §2.6 and outside AI's shadow ban (R186).

import { DECK_SIZE } from "@jackioh/engine/config";

import type { PracticeDeckChoice } from "./protocol.ts";

export type PracticePreset = {
  id: string;
  name: string;
  identity: string;
  /** `DECK_SIZE` distinct, non-token Core IDs, cheapest first. */
  cards: readonly string[];
};

export const PRACTICE_PRESETS: readonly PracticePreset[] = [
  {
    id: "humans",
    name: "Human Vanguard",
    identity: "Humans hold the line: cheap bodies, Taunt and armor, then The Rock and friends.",
    cards: [
      "core-004", // Gary the Gambler
      "core-005", // Stockpile
      "core-008", // Mr. Vanilla
      "core-011", // Tempo Timmy
      "core-015", // Me and Mr Token
      "core-081", // Radiant Saintess
      "core-001", // Big D-fender
      "core-016", // Hit Job
      "core-020", // Pointmaster
      "core-045", // Deft Duelist
      "core-061", // Prejudiced Postdoc
      "core-069", // Call to Arms
      "core-077", // Professor Curvature
      "core-030", // Archivist
      "core-013", // Jlockeed Shredder-10
      "core-019", // Midrange Menace
      "core-053", // Reno
      "core-025", // 4-mana 7/7
      "core-054", // Straaza
      "core-066", // The Rock
    ],
  },
  {
    id: "blitz",
    name: "Blitz",
    identity: "Rush, Charge and burn: hit hard early and finish the job with spells.",
    cards: [
      "core-004", // Gary the Gambler
      "core-008", // Mr. Vanilla
      "core-011", // Tempo Timmy
      "core-015", // Me and Mr Token
      "core-035", // Lunar Eclipse
      "core-044", // True Strike
      "core-063", // Plastic Surgery
      "core-074", // Adaptive UI
      "core-012", // Duplicating Felinors
      "core-020", // Pointmaster
      "core-032", // Prem Panther
      "core-045", // Deft Duelist
      "core-056", // Jilliax
      "core-058", // Rush Token Farm
      "core-068", // Twisted Sorcerer
      "core-013", // Jlockeed Shredder-10
      "core-070", // Spiteful Stab
      "core-014", // Jlockeed's Weapons
      "core-025", // 4-mana 7/7
      "core-054", // Straaza
    ],
  },
  {
    id: "fortress",
    name: "Fortress",
    identity: "Removal, Taunts and card draw: weather the storm, then win with giants.",
    cards: [
      "core-005", // Stockpile
      "core-035", // Lunar Eclipse
      "core-036", // Magic Jammed
      "core-041", // Sheepish
      "core-044", // True Strike
      "core-067", // Zoomerbin Oomen
      "core-009", // Moths to the Flame
      "core-016", // Hit Job
      "core-030", // Archivist
      "core-037", // Gravedigger
      "core-056", // Jilliax
      "core-073", // Anti-oneshot Armor
      "core-013", // Jlockeed Shredder-10
      "core-019", // Midrange Menace
      "core-049", // Snom Bunny Mind Control
      "core-053", // Reno
      "core-088", // Twisting Nether
      "core-025", // 4-mana 7/7
      "core-054", // Straaza
      "core-066", // The Rock
    ],
  },
];

export function presetById(id: string): PracticePreset | undefined {
  return PRACTICE_PRESETS.find((preset) => preset.id === id);
}

export const RANDOM_DECK_IDENTITY =
  "A fresh twenty-card deck every game, dealt with a sensible mana curve. You meet it in your opening hand.";

/** Oldest-first, named `GET /api/decks` result; it may be an incomplete draft (R250). */
export type PracticeSavedDeck = { name: string; cards: readonly string[]; portrait?: string | null };

/** R250 drafts need `DECK_SIZE` cards to be offered; `createGame` validates every deck (§2.6). */
export function isPlayableSavedDeck(deck: PracticeSavedDeck): boolean {
  return deck.cards.length === DECK_SIZE;
}

const RANDOM_VALUE = "random";
const PRESET_PREFIX = "preset:";
const SAVED_PREFIX = "saved:";

export function deckChoiceValue(choice: PracticeDeckChoice): string {
  switch (choice.kind) {
    case "random":
      return RANDOM_VALUE;
    case "preset":
      return `${PRESET_PREFIX}${choice.id}`;
    case "saved":
      return `${SAVED_PREFIX}${String(choice.index)}`;
  }
}

export function deckChoiceFromValue(
  value: string,
  saved: readonly PracticeSavedDeck[] | null,
): PracticeDeckChoice | null {
  if (value === RANDOM_VALUE) return { kind: "random" };

  if (value.startsWith(PRESET_PREFIX)) {
    const id = value.slice(PRESET_PREFIX.length);
    return presetById(id) === undefined ? null : { kind: "preset", id };
  }

  if (value.startsWith(SAVED_PREFIX)) {
    if (saved === null) return null;
    const digits = value.slice(SAVED_PREFIX.length);
    if (!/^[0-9]+$/.test(digits)) return null;
    const index = Number(digits);
    const deck = saved[index - 1];
    if (index < 1 || deck === undefined || !isPlayableSavedDeck(deck)) return null;
    return { kind: "saved", index, cards: [...deck.cards], portrait: deck.portrait ?? null };
  }

  return null;
}

export function isDeckValue(value: string): boolean {
  if (value === RANDOM_VALUE) return true;
  if (value.startsWith(PRESET_PREFIX)) return presetById(value.slice(PRESET_PREFIX.length)) !== undefined;
  return /^saved:[1-9][0-9]*$/.test(value);
}

export function isAutostartDeckValue(value: string): boolean {
  return value === RANDOM_VALUE || (value.startsWith(PRESET_PREFIX) && isDeckValue(value));
}

type DeckOption = { value: string; label: string; disabled: boolean };

export function savedDeckLabel(deck: PracticeSavedDeck): string {
  if (isPlayableSavedDeck(deck)) return deck.name;
  return `${deck.name} (${String(deck.cards.length)} of ${String(DECK_SIZE)} cards, not complete)`;
}

export function deckOptions(saved: readonly PracticeSavedDeck[] | null): DeckOption[] {
  const options: DeckOption[] = [{ value: RANDOM_VALUE, label: "Random deck", disabled: false }];
  for (const preset of PRACTICE_PRESETS) {
    options.push({ value: `${PRESET_PREFIX}${preset.id}`, label: preset.name, disabled: false });
  }
  if (saved !== null) {
    saved.forEach((deck, i) => {
      options.push({
        value: `${SAVED_PREFIX}${String(i + 1)}`,
        label: savedDeckLabel(deck),
        disabled: !isPlayableSavedDeck(deck),
      });
    });
  }
  return options;
}
