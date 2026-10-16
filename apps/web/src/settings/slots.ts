// Integration seam for feature-owned settings (docs/polish/7-mobile-ux.md S8).
// Mount each store in its section; panel-level hover and motion switches remain their only handles.

import { createElement, type ReactNode } from "react";

import AudioControls from "../audio/AudioControls.tsx";
import { DEFAULT_AUDIO_SETTINGS, writeAudioSettings } from "../audio/settings.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../fx/settings.ts";
import { DEFAULT_HAPTICS_SETTINGS, writeHapticsSettings } from "../haptics/settings.ts";
import { resetLocale } from "../i18n/i18n.ts";
import { AnimatedFoilSwitch, FullArtSwitch, FxControls, LanguageSelect, VibrationSwitch } from "./controls.tsx";

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
