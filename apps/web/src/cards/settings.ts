// Card presentation settings (B28). One localStorage key; every storage access sits in try/catch,
// because storage can be missing, full, or throw on access (private mode, sandboxed frames). When
// storage refuses a write the value lives on in memory, so the toggle still works for this page.
//
// R1834004: `opponentCosmetics` on, an opponent's cards and side wear the loadout their seat froze
// when the match began (game/wearer.ts); off, the defaults, on a live, watched or replayed board alike.
// R2721000: `rarityFoil`, off by default, gives a Mythic face its rarity foil (model.ts foilFor).
// R2718000, R2721000: `setWatermarks` (setMark.ts) and `rarityFoil` stay on the device
// (`DEVICE_CARD_SETTINGS`), never the account.
// R3344001: `deviceTilt`, off by default, lets a card in the card views tilt with the phone's orientation (tilt.ts); the
// switch asks the phone's leave once where it asks (settings/controls.tsx), and it stays on the device too.

import { useSyncExternalStore } from "react";
import { setStored } from "../net/storage.ts";

export type BoardNamesOption = "artFirst" | "always" | "never";

export const BOARD_NAMES_DEFAULT: BoardNamesOption = "artFirst";
export const BOARD_NAMES_VALUES: readonly BoardNamesOption[] = ["artFirst", "always", "never"] as const;

export type CardSettings = {
  hoverPreviews: boolean;
  animatedFoil: boolean;
  fullArt: boolean;
  opponentCosmetics: boolean;
  setWatermarks: boolean;
  rarityFoil: boolean;
  deviceTilt: boolean;
  boardNames: BoardNamesOption;
};

export const CARD_SETTINGS_KEY = "jackioh.cards.settings.v1";

export const CARD_SETTINGS_DEFAULTS: Readonly<CardSettings> = Object.freeze({
  hoverPreviews: true,
  animatedFoil: true,
  fullArt: true,
  opponentCosmetics: true,
  setWatermarks: true,
  rarityFoil: false,
  deviceTilt: false,
  boardNames: "artFirst",
});

type CardSettingsField = {
  key: keyof CardSettings;
  section: "gameplay" | "visuals";
  label: string;
  description: string;
};

export const CARD_SETTINGS_FIELDS: readonly CardSettingsField[] = [
  {
    key: "hoverPreviews",
    section: "gameplay",
    label: "Hover previews",
    description: "Rest the pointer on a card to see it enlarged, with its keywords explained.",
  },
  {
    key: "rarityFoil",
    section: "visuals",
    label: "Rarity foil",
    description: "Mythic cards shine with a rainbow foil and edge. Off, the crest at the top marks Legendary and Mythic cards.",
  },
  {
    key: "animatedFoil",
    section: "visuals",
    label: "Animated foil",
    description: "Radiant cards and foils shimmer. Off shows the foil standing still.",
  },
  {
    key: "fullArt",
    section: "visuals",
    label: "Full-art cards",
    description: "The heroes' cards show only their art and name; hover or click one to read it. Off shows their text like any card.",
  },
  {
    key: "setWatermarks",
    section: "visuals",
    label: "Set watermarks",
    description: "Each card shows its set's symbol, faint, behind its text. Off prints the text on a plain box.",
  },
  {
    key: "opponentCosmetics",
    section: "visuals",
    label: "Show opponents' cosmetics",
    description: "Your opponents' cards and side wear what they equipped. Off shows them plain.",
  },
  {
    key: "boardNames",
    section: "visuals",
    label: "Names on the board",
    description: "Full-art and Legendary/Mythic cards hide their name and words in play; hover or press to read. Always shows every name; Never hides all names.",
  },
  {
    key: "deviceTilt",
    section: "visuals",
    label: "Tilt with my phone",
    description: "Cards tilt as you tilt your phone, in the card view. Your phone asks once to allow it. Off: drag the card instead.",
  },
];

/** R2718000, R2721000: card settings that stay on the device; the account's settings (R633, settings/groups.ts) never carry them. */
export const DEVICE_CARD_SETTINGS: readonly (keyof CardSettings)[] = ["setWatermarks", "rarityFoil", "deviceTilt"];

const SETTING_KEYS: readonly (keyof CardSettings)[] = [
  "hoverPreviews",
  "animatedFoil",
  "fullArt",
  "opponentCosmetics",
  "setWatermarks",
  "rarityFoil",
  "boardNames",
  "deviceTilt",
];

/**
 * The value of a write that storage refused, and what storage held at that moment (`undefined`
 * when it could not be read at all). Storage wins again as soon as it holds something else.
 */
let fallback: { value: CardSettings; raw: string | null | undefined } | null = null;

const listeners = new Set<() => void>();

/** What storage holds under the key: `null` when empty, `undefined` when storage is unusable. */
function readRaw(): string | null | undefined {
  try {
    return window.localStorage.getItem(CARD_SETTINGS_KEY);
  } catch {
    return undefined;
  }
}

/** Parsed settings with every missing or mistyped key taken from the defaults. */
function parse(raw: string): CardSettings {
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return CARD_SETTINGS_DEFAULTS;
  }
  if (typeof data !== "object" || data === null || Array.isArray(data)) return CARD_SETTINGS_DEFAULTS;
  const record = data as Record<string, unknown>;
  const settings: CardSettings = { ...CARD_SETTINGS_DEFAULTS };
  for (const key of SETTING_KEYS) {
    const value = record[key];
    if (typeof value === "boolean") (settings as Record<string, unknown>)[key] = value;
  }
  if (record.boardNames === "artFirst" || record.boardNames === "always" || record.boardNames === "never") {
    settings.boardNames = record.boardNames;
  }
  return settings;
}

export function readCardSettings(): CardSettings {
  const raw = readRaw();
  if (fallback !== null) {
    if (raw === undefined || raw === fallback.raw) return fallback.value;
    fallback = null;
  }
  if (raw === undefined || raw === null) return CARD_SETTINGS_DEFAULTS;
  return parse(raw);
}

export function writeCardSettings(patch: Partial<CardSettings>): CardSettings {
  const next: CardSettings = { ...readCardSettings() };
  for (const key of SETTING_KEYS) {
    const value = patch[key];
    if (typeof value === "boolean") (next as Record<string, unknown>)[key] = value;
  }
  if (patch.boardNames === "artFirst" || patch.boardNames === "always" || patch.boardNames === "never") {
    next.boardNames = patch.boardNames;
  }
  fallback = setStored(CARD_SETTINGS_KEY, JSON.stringify(next)) ? null : { value: next, raw: readRaw() };
  for (const listener of [...listeners]) listener();
  return next;
}

export function subscribeCardSettings(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The last snapshot handed to React: unchanged values keep the same object, as useSyncExternalStore needs. */
let snapshot: CardSettings = CARD_SETTINGS_DEFAULTS;

function getSnapshot(): CardSettings {
  const next = readCardSettings();
  if (SETTING_KEYS.some((key) => next[key] !== snapshot[key])) {
    snapshot = next;
  }
  return snapshot;
}

export function useCardSettings(): CardSettings {
  return useSyncExternalStore(subscribeCardSettings, getSnapshot);
}
