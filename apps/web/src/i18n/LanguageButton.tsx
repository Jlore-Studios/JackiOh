// The language button (#711, R711000): the top bar's control beside the settings gear, and the
// board's. One click switches to the next of LOCALES, live. It stays only until the device has
// settled on its language (R1916000); settings' Language row is always there.

import type { ReactElement } from "react";

import { LOCALES, nextLocale, setLocale, t, useLocale, useLocaleSettled } from "./i18n.ts";
import "./language.css";

export type LanguageButtonProps = {
  /** Where it sits, as the settings gear's `placement` does: the top bar, or the board's control bar. */
  placement: "game" | "nav";
};

export default function LanguageButton({ placement }: LanguageButtonProps): ReactElement | null {
  const locale = useLocale();
  const settled = useLocaleSettled();
  if (settled) return null;
  const next = nextLocale(locale);
  return (
    <button
      type="button"
      className={`language-button language-button--${placement}`}
      data-testid={`language-${placement}`}
      data-locale={locale}
      // WCAG 2.5.3 (#757): the name starts with the shown short name, so a voice user can say what they see.
      aria-label={`${LOCALES[locale].short}. ${t("Language: {name}. Switch to {next}.", { name: LOCALES[locale].name, next: LOCALES[next].name })}`}
      title={t("Language")}
      onClick={() => {
        void setLocale(next);
      }}
    >
      <span lang={locale} aria-hidden="true">
        {LOCALES[locale].short}
      </span>
    </button>
  );
}
