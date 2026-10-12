// R711000: client-wide internationalisation (#711).
// Covers:
// 1. t() with fallback, params, and plurals;
// 2. the card rule and the easter egg;
// 3. the language button switching live, persisting, and updating <html lang>;
// 4. every zh-CN.json dictionary entry's {params} matching its English key.

import { createElement } from "react";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { faceModel } from "../cards/model.ts";
import { drawnInChinese } from "../cards/chinese.ts";
import { CardDetail } from "../cards/inspect/CardDetail.tsx";
import { Flavour, showsEasterEgg } from "../cards/inspect/Flavour.tsx";
import { SETTINGS_SLOTS } from "../settings/slots.ts";
import {
  formatDateTime,
  formatDay,
  formatNumber,
  getLocale,
  resetLocaleForTests,
  setLocale,
  t,
} from "./i18n.ts";
import LanguageButton from "./LanguageButton.tsx";
import zhCN from "./zh-CN.json";

describe("R711000 internationalisation and reader language", () => {
  beforeEach(() => {
    resetLocaleForTests();
  });

  afterEach(() => {
    cleanup();
    resetLocaleForTests();
  });

  it("R711000 t() translates with fallback, parameter interpolation, and plural selection", async () => {
    // 1. English (default)
    expect(getLocale()).toBe("en");
    expect(t("Your turn")).toBe("Your turn");
    expect(t("Looking for a {mode} opponent…", { mode: "Best of 1" })).toBe("Looking for a Best of 1 opponent…");
    // Missing entries fall back to the English key
    expect(t("An untranslated engine sentence")).toBe("An untranslated engine sentence");
    expect(t("Hello {name}!", { name: "Player" })).toBe("Hello Player!");

    expect(t("The English text governs.")).toBe("The English text governs.");
    expect(formatDay("2026-10-10")).toBe("2026-10-10");
    expect(formatNumber(1234)).toBe("1,234");
    expect(formatDateTime(1900000000000)).toBe(new Date(1900000000000).toLocaleString("en"));

    // 2. Switch to zh-CN
    await setLocale("zh-CN");
    expect(getLocale()).toBe("zh-CN");
    expect(t("Your turn")).toBe("你的回合");
    expect(t("Looking for a {mode} opponent…", { mode: "单局" })).toBe("正在寻找单局对手…");
    // Missing entries fall back to English
    expect(t("An untranslated engine sentence")).toBe("An untranslated engine sentence");
    expect(t("Hello {name}!", { name: "Player" })).toBe("Hello Player!");
    // Plurals / parameter formatting
    expect(t("{count} waiting", { count: 3 })).toBe("3人在等待");
    // Governing legal notice and date helpers
    expect(t("The English text governs.")).toBe("以英文版本为准。");
    expect(formatDay("2026-10-10")).toBe("2026年10月10日");
    expect(formatNumber(1234)).toBe("1,234");
    expect(formatDateTime(1900000000000)).toBe(new Date(1900000000000).toLocaleString("zh-CN"));
  });

  it("R711000 cards print in reader language, Meditative-flagged cards print in Chinese, and the easter egg triggers only for Chinese readers", async () => {
    const sampleDefId = Object.keys(CATALOG)[0] ?? "c-01";
    const sampleDef = CATALOG[sampleDefId]!;

    // Reader in English:
    expect(getLocale()).toBe("en");
    expect(drawnInChinese(false)).toBe(false);
    expect(drawnInChinese(true)).toBe(true);

    // Normal card for English reader: English face, not flagged, no easter egg
    const enFaceNormal = faceModel({ defId: sampleDefId, def: sampleDef, radiant: false });
    expect(enFaceNormal.chinese).toBeUndefined();
    expect(enFaceNormal.flagged).toBeUndefined();
    expect(showsEasterEgg(enFaceNormal)).toBe(false);

    // Meditative-flagged card for English reader: Chinese face, flagged, but NO easter egg (reader is English)
    const enFaceFlagged = faceModel({ defId: sampleDefId, def: sampleDef, radiant: false, chinese: true });
    expect(enFaceFlagged.chinese).toBe(true);
    expect(enFaceFlagged.flagged).toBe(true);
    expect(showsEasterEgg(enFaceFlagged)).toBe(false);

    // Reader in Simplified Chinese:
    await setLocale("zh-CN");
    expect(getLocale()).toBe("zh-CN");
    expect(drawnInChinese(false)).toBe(true);
    expect(drawnInChinese(true)).toBe(true);

    // Normal card for Chinese reader: Chinese face, not flagged, NO easter egg
    const zhFaceNormal = faceModel({ defId: sampleDefId, def: sampleDef, radiant: false });
    expect(zhFaceNormal.chinese).toBe(true);
    expect(zhFaceNormal.flagged).toBeUndefined();
    expect(showsEasterEgg(zhFaceNormal)).toBe(false);

    // Meditative-flagged card for Chinese reader: Chinese face, flagged, and DOES show easter egg!
    const zhFaceFlagged = faceModel({ defId: sampleDefId, def: sampleDef, radiant: false, chinese: true });
    expect(zhFaceFlagged.chinese).toBe(true);
    expect(zhFaceFlagged.flagged).toBe(true);
    expect(showsEasterEgg(zhFaceFlagged)).toBe(true);

    // Verify Flavour component rendering the easter egg text:
    const { container } = render(createElement(Flavour, { defId: sampleDefId, easter: true }));
    expect(container).toHaveTextContent("本来要把它翻译成中文的——可它已经是中文了！");

    // Verify CardDetail modal inspect view also renders the easter egg line for Chinese reader:
    render(createElement(CardDetail, { def: sampleDef, onClose: () => {}, easter: true }));
    expect(document.body).toHaveTextContent("本来要把它翻译成中文的——可它已经是中文了！");
  });

  it("R711000 language button switches live without reload, sets <html lang> and persists in localStorage", async () => {
    expect(document.documentElement.lang).toBe("en");
    expect(window.localStorage.getItem("jackioh.locale")).toBeNull();

    // SETTINGS_SLOTS mounts language under gameplay and resets to English (R711000):
    const langSlot = SETTINGS_SLOTS.find((slot) => slot.id === "language");
    expect(langSlot).toBeDefined();
    expect(langSlot?.section).toBe("gameplay");
    await setLocale("zh-CN");
    expect(getLocale()).toBe("zh-CN");
    langSlot?.reset?.();
    await vi.waitFor(() => {
      expect(getLocale()).toBe("en");
    });

    const { getByTestId } = render(createElement(LanguageButton, { placement: "nav" }));
    const button = getByTestId("language-nav");

    expect(button).toHaveTextContent("EN");
    expect(button).toHaveAttribute("data-locale", "en");

    // Click button to switch to Chinese:
    fireEvent.click(button);
    await vi.waitFor(() => {
      expect(button).toHaveTextContent("中文");
    });

    expect(getLocale()).toBe("zh-CN");
    expect(document.documentElement.lang).toBe("zh-CN");
    expect(window.localStorage.getItem("jackioh.locale")).toBe("zh-CN");

    // Click again to switch back to English:
    fireEvent.click(button);
    await vi.waitFor(() => {
      expect(button).toHaveTextContent("EN");
    });

    expect(getLocale()).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    expect(window.localStorage.getItem("jackioh.locale")).toBe("en");
  });

  it("R711000 every zh-CN.json dictionary entry has matching parameter tokens with its English key", () => {
    const TOKEN = /\{(\w+)\}/g;
    for (const [key, value] of Object.entries(zhCN)) {
      const keyTokens = Array.from(key.matchAll(TOKEN), (m) => m[1]).sort();
      const stringValues: string[] = typeof value === "string" ? [value] : Object.values(value as Record<string, string>);
      for (const str of stringValues) {
        const valTokens = Array.from(str.matchAll(TOKEN), (m) => m[1]).sort();
        expect(valTokens, `Token mismatch for key "${key}" and translation "${str}"`).toEqual(keyTokens);
      }
    }
  });
});
