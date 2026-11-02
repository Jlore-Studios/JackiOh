// The stores that make up a player's settings, as the account sync sees them (issue #129, R633). A
// group is one store, read as a flat object of booleans, numbers and short texts, applied back through
// the store's own writer and watched through its own subscription. The sync never reads a store's
// storage key, so a store stays free to change how it saves; a new store joins by being listed here.
// R2718000: a card setting kept to the device (`DEVICE_CARD_SETTINGS`) is out of the `cards` group both
// ways (never read for the account, never applied from it) and changing it does not wake the sync.

import { readAudioSettings, subscribeAudioSettings, writeAudioSettings } from "../audio/settings.ts";
import type { AudioSettings } from "../audio/types.ts";
import { readCosmetics, subscribeCosmetics, writeCosmetics } from "../cards/cosmetics.ts";
import {
  DEVICE_CARD_SETTINGS,
  readCardSettings,
  subscribeCardSettings,
  writeCardSettings,
  type CardSettings,
} from "../cards/settings.ts";
import { getFxSettings, setFxSettings, subscribeFxSettings, type FxSettings } from "../fx/settings.ts";
import type { PlayerSettingValue } from "../net/api.ts";
import { readSettings, subscribeSettings, writeSettings, type Settings } from "./store.ts";

export type SettingsGroup = {
  /** The id the account keeps it under: a lower-case slug (`^[a-z][a-z0-9]*$`). */
  id: string;
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

/** R2718000: the card settings the account keeps: all but the device's own. */
function accountCards(record: object): Record<string, PlayerSettingValue> {
  const device: readonly string[] = DEVICE_CARD_SETTINGS;
  return Object.fromEntries(Object.entries(flat(record)).filter(([key]) => !device.includes(key)));
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
    read: () => accountCards(readCardSettings()),
    apply: (values) => {
      writeCardSettings(accountCards(values) as Partial<CardSettings>);
    },
    subscribe: (listener) => {
      // The device's own settings change the store but not what the account keeps: no sync for them.
      let last = JSON.stringify(accountCards(readCardSettings()));
      return subscribeCardSettings(() => {
        const next = JSON.stringify(accountCards(readCardSettings()));
        if (next === last) return;
        last = next;
        listener();
      });
    },
  },
  {
    // R1295000: the store keeps an id it does not know, so a newer client's pick comes back intact.
    id: "cosmetics",
    read: () => flat(readCosmetics()),
    apply: (values) => {
      writeCosmetics(values);
    },
    subscribe: subscribeCosmetics,
  },
];
