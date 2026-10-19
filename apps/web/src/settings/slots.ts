// Integration seam for feature-owned settings (docs/polish/7-mobile-ux.md S8).
// Mount each store in its section; panel-level hover and motion switches remain their only handles.

import { createElement, type ReactNode } from "react";

import AudioControls from "../audio/AudioControls.tsx";
import { DEFAULT_AUDIO_SETTINGS, writeAudioSettings } from "../audio/settings.ts";
import { DEFAULT_CARD_COSMETICS, writeCosmetics } from "../cards/cosmetics.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../fx/settings.ts";
import { DEFAULT_HAPTICS_SETTINGS, writeHapticsSettings } from "../haptics/settings.ts";
import { resetLocale } from "../i18n/i18n.ts";
import {
  AnimatedFoilSwitch,
  CardCosmeticsControls,
  DeckCosmeticsControls,
  FullArtSwitch,
  FxControls,
  LanguageSelect,
  VibrationSwitch,
} from "./controls.tsx";

export type SettingsSectionId = "gameplay" | "visuals" | "audio";

export type SettingsSlot = {
  section: SettingsSectionId;
  /** Unique within the list: the React key and the slot wrapper's `data-settings-slot`. */
  id: string;
  render: () => ReactNode;
  reset?: () => void;
};

/** Settings slots (R669). */
export const SETTINGS_SLOTS: readonly SettingsSlot[] = [
  {
    section: "gameplay",
    id: "haptics",
    render: () => createElement(VibrationSwitch),
    reset: () => {
      writeHapticsSettings({ ...DEFAULT_HAPTICS_SETTINGS });
    },
  },
  {
    section: "visuals",
    id: "fx",
    render: () => createElement(FxControls),
    reset: () => {
      setFxSettings({ ...DEFAULT_FX_SETTINGS });
    },
  },
  {
    section: "visuals",
    id: "card-foil",
    render: () => createElement(AnimatedFoilSwitch),
    reset: () => {
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    },
  },
  {
    section: "visuals",
    id: "card-full-art",
    render: () => createElement(FullArtSwitch),
    reset: () => {
      writeCardSettings({ fullArt: CARD_SETTINGS_DEFAULTS.fullArt });
    },
  },
  {
    // R1295000: the foil, finish and border your own cards wear.
    section: "visuals",
    id: "card-cosmetics",
    render: () => createElement(CardCosmeticsControls),
    reset: () => {
      writeCosmetics({
        foil: DEFAULT_CARD_COSMETICS.foil,
        finish: DEFAULT_CARD_COSMETICS.finish,
        border: DEFAULT_CARD_COSMETICS.border,
      });
    },
  },
  {
    // R1297000: the playmat, backdrop and card back your own side wears.
    section: "visuals",
    id: "deck-cosmetics",
    render: () => createElement(DeckCosmeticsControls),
    reset: () => {
      writeCosmetics({
        table: DEFAULT_CARD_COSMETICS.table,
        field: DEFAULT_CARD_COSMETICS.field,
        cardBack: DEFAULT_CARD_COSMETICS.cardBack,
      });
    },
  },
  {
    section: "audio",
    id: "audio",
    render: () => createElement(AudioControls, { className: "settings-audio" }),
    reset: () => {
      writeAudioSettings({ ...DEFAULT_AUDIO_SETTINGS });
    },
  },
  {
    section: "gameplay",
    id: "language",
    render: () => createElement(LanguageSelect),
    reset: () => {
      void resetLocale();
    },
  },
];
