// Integration seam for feature-owned settings (docs/polish/7-mobile-ux.md S8).
// Mount each store in its section; panel-level hover and motion switches remain their only handles.

import { createElement, type ReactNode } from "react";

import AudioControls from "../audio/AudioControls.tsx";
import { DEFAULT_AUDIO_SETTINGS, writeAudioSettings } from "../audio/settings.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../cards/settings.ts";
import { DEFAULT_FX_SETTINGS, setFxSettings } from "../fx/settings.ts";
import { DEFAULT_HAPTICS_SETTINGS, writeHapticsSettings } from "../haptics/settings.ts";
import { DEFAULT_LOCALE, setLocale, t, useLocale } from "../i18n/i18n.ts";
import { AnimatedFoilSwitch, FxControls, VibrationSwitch } from "./controls.tsx";

export type SettingsSectionId = "gameplay" | "visuals" | "audio";

export type SettingsSlot = {
  section: SettingsSectionId;
  /** Unique within the list: the React key and the slot wrapper's `data-settings-slot`. */
  id: string;
  render: () => ReactNode;
  reset?: () => void;
};

function LanguageSwitch(): ReactNode {
  const locale = useLocale();
  return createElement(
    "div",
    { className: "settings-row", "data-testid": "setting-language" },
    createElement(
      "div",
      { className: "settings-control" },
      createElement("span", { className: "settings-label" }, t("Language")),
      createElement(
        "div",
        {
          className: "settings-radio-group",
          role: "radiogroup",
          "aria-label": t("Language"),
          style: { display: "flex", gap: "12px", alignItems: "center" },
        },
        createElement(
          "label",
          { style: { display: "inline-flex", alignItems: "center", gap: "6px", cursor: "pointer" } },
          createElement("input", {
            type: "radio",
            name: "setting-language-choice",
            value: "en",
            checked: locale === "en",
            onChange: () => {
              void setLocale("en");
            },
          }),
          t("English"),
        ),
        createElement(
          "label",
          { style: { display: "inline-flex", alignItems: "center", gap: "6px", cursor: "pointer" } },
          createElement("input", {
            type: "radio",
            name: "setting-language-choice",
            value: "zh-CN",
            checked: locale === "zh-CN",
            onChange: () => {
              void setLocale("zh-CN");
            },
          }),
          t("简体中文"),
        ),
      ),
    ),
  );
}

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
    render: () => createElement(LanguageSwitch),
    reset: () => {
      void setLocale(DEFAULT_LOCALE);
    },
  },
];
