// The language button (#711, R711000): the top bar's control beside the settings gear, and the
// board's, in the same corner. It shows the current language's short name ("EN", "中文") and one
// click switches to the next of LOCALES, live: the store notifies and the screen redraws in it.

import type { ReactElement } from "react";

import { LOCALES, nextLocale, setLocale, t, useLocale } from "./i18n.ts";
import "./language.css";

export type LanguageButtonProps = {
  /** Where it sits, as the settings gear's `placement` does: the top bar, or the board's control bar. */
  placement: "game" | "nav";
};

export default function LanguageButton({ placement }: LanguageButtonProps): ReactElement {
  const locale = useLocale();
  const next = nextLocale(locale);
  return (
    <button
      type="button"
      className={`language-button language-button--${placement}`}
      data-testid={`language-${placement}`}
      data-locale={locale}
      aria-label={t("Language: {name}. Switch to {next}.", { name: LOCALES[locale].name, next: LOCALES[next].name })}
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
