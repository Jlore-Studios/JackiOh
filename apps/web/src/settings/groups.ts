// The stores that make up a player's settings, as the account sync sees them (issue #129, R633).
//
// The settings live in four stores, each in its own module and its own `localStorage` key: the
// gameplay switches (`store.ts`), the audio mixer (`audio/settings.ts`), the effects (`fx/settings.ts`)
// and the card display (`cards/settings.ts`). A group is one of them, read as a flat object of
// booleans, numbers and short texts, applied back through the store's own writer (which drops
// anything it does not know and clamps anything out of range), and watched through the store's own
// subscription. The sync never reads a store's storage key, so a store stays free to change how it
// saves; a store added later joins the account by being listed here.

import { readAudioSettings, subscribeAudioSettings, writeAudioSettings } from "../audio/settings.ts";
import type { AudioSettings } from "../audio/types.ts";
import { readCardSettings, subscribeCardSettings, writeCardSettings, type CardSettings } from "../cards/settings.ts";
import { getFxSettings, setFxSettings, subscribeFxSettings, type FxSettings } from "../fx/settings.ts";
import type { PlayerSettingValue } from "../net/api.ts";
import { readSettings, subscribeSettings, writeSettings, type Settings } from "./store.ts";

export type SettingsGroup = {
  /** The id the account keeps it under: a lower-case slug (`^[a-z][a-z0-9]*$`). */
  id: string;
  /** The store's values now, as a flat object. */
  read: () => Record<string, PlayerSettingValue>;
  /** Writes values the account sent; the store keeps what it knows and clamps the rest. */
  apply: (values: Record<string, unknown>) => void;
  /** Calls `listener` after every change to the store, whoever made it. Returns the unsubscribe. */
  subscribe: (listener: () => void) => () => void;
};

/** A flat object of the primitives the account keeps, from a store's own typed object. */
function flat(record: object): Record<string, PlayerSettingValue> {
  const out: Record<string, PlayerSettingValue> = {};
  for (const [key, value] of Object.entries(record)) {
    if (typeof value === "boolean" || typeof value === "number" || typeof value === "string") out[key] = value;
  }
  return out;
}

export const SETTINGS_GROUPS: readonly SettingsGroup[] = [
  {
    id: "gameplay",
    read: () => flat(readSettings()),
    apply: (values) => {
      // The store keeps the booleans it knows and drops the rest.
      writeSettings(values as Partial<Settings>);
    },
    subscribe: subscribeSettings,
  },
  {
    id: "audio",
    read: () => flat(readAudioSettings()),
    apply: (values) => {
      writeAudioSettings(values as Partial<AudioSettings>);
    },
    subscribe: (listener) => subscribeAudioSettings(() => {
      listener();
    }),
  },
  {
    id: "fx",
    read: () => flat(getFxSettings()),
    apply: (values) => {
      setFxSettings(values as Partial<FxSettings>);
    },
    subscribe: (listener) => subscribeFxSettings(() => {
      listener();
    }),
  },
  {
    id: "cards",
    read: () => flat(readCardSettings()),
    apply: (values) => {
      writeCardSettings(values as Partial<CardSettings>);
    },
    subscribe: subscribeCardSettings,
  },
];
