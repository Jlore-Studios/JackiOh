// R711000: client-wide internationalisation (#711).
// Covers:
// 1. t() with fallback, params, and plurals;
// 2. the card rule and the easter egg;
// 3. the language button switching live, persisting, and updating <html lang>;
// 4. every zh-CN.json dictionary entry's {params} matching its English key;
// 5. every literal sentence a non-test source wraps in t() having a zh-CN.json entry.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { createElement } from "react";
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ts from "typescript";

import { CATALOG } from "@jackioh/cards";
import { SERIES_WINS_NEEDED } from "@jackioh/server-config";
import { faceModel } from "../cards/model.ts";
import { drawnInChinese } from "../cards/chinese.ts";
import { CardDetail } from "../cards/inspect/CardDetail.tsx";
import { conquestText } from "../routes/conquestCopy.ts";
import { Flavour, showsEasterEgg } from "../cards/inspect/Flavour.tsx";
import { SETTINGS_SLOTS } from "../settings/slots.ts";
import {
  formatDateTime,
  formatDay,
  formatNumber,
  formatSeconds,
  getLocale,
  resetLocaleForTests,
  setLocale,
  t,
  words,
} from "./i18n.ts";
import LanguageButton from "./LanguageButton.tsx";
import zhCN from "./zh-CN.json";

/** The client's sources: `apps/web/src`, this folder's parent. */
const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Every non-test source under `dir`: no `*.test.ts(x)`, no `test/` folder, no `wire/generated/`. */
function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" || relative(SRC, path) === join("wire", "generated") ? [] : sources(path);
    return /\.(ts|tsx)$/u.test(name) && !/\.test\.tsx?$/u.test(name) ? [path] : [];
  });
}

/** The literal sentences an argument is: a string, a template with no substitutions, or each branch of a conditional of them. */
function literals(node: ts.Expression): (ts.StringLiteral | ts.NoSubstitutionTemplateLiteral)[] {
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return [node];
  if (ts.isParenthesizedExpression(node)) return literals(node.expression);
  if (ts.isConditionalExpression(node)) return [...literals(node.whenTrue), ...literals(node.whenFalse)];
  return [];
}

/** Each literal sentence a source wraps in `t()`, with the `file:line` it stands at. */
function sentencesIn(path: string, text: string): { at: string; key: string }[] {
  const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const out: { at: string; key: string }[] = [];
  const visit = (node: ts.Node): void => {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "t") {
      const first = node.arguments[0];
      for (const literal of first === undefined ? [] : literals(first)) {
        const line = file.getLineAndCharacterOfPosition(literal.getStart(file)).line + 1;
        out.push({ at: `${relative(SRC, path)}:${String(line)}`, key: literal.text });
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(file);
  return out;
}

/** The sentences `zh-CN.json` has no entry for, as `file:line: "sentence"`. */
function untranslated(sentences: readonly { at: string; key: string }[]): string[] {
  return sentences.filter(({ key }) => !Object.hasOwn(zhCN, key)).map(({ at, key }) => `${at}: ${JSON.stringify(key)}`);
}

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
    // A screen's copy held in a table of English with `{name}`s reads with its numbers filled
    expect(conquestText("winsNeeded")).toBe(
      `Win a game with each of your ${String(SERIES_WINS_NEEDED)} decks to take the series (first to ${String(SERIES_WINS_NEEDED)} wins).`,
    );

    expect(t("The English text governs.")).toBe("The English text governs.");
    expect(formatDay("2026-10-10")).toBe("2026-10-10");
    // English writes numbers and moments as the bare toLocaleString() they replace did: the browser's way.
    expect(formatNumber(1234)).toBe((1234).toLocaleString());
    // The clock reads as `${n}s` always did: never grouped (the match ceiling is 7200s).
    expect(formatSeconds(5)).toBe("5s");
    expect(formatSeconds(7200)).toBe("7200s");
    expect(formatDateTime(1900000000000)).toBe(new Date(1900000000000).toLocaleString());

    // 2. Switch to zh-CN
    await setLocale("zh-CN");
    expect(getLocale()).toBe("zh-CN");
    expect(t("Your turn")).toBe("你的回合");
    expect(t("Looking for a {mode} opponent…", { mode: "单局" })).toBe("正在寻找单局对手…");
    // Missing entries fall back to English
    expect(t("An untranslated engine sentence")).toBe("An untranslated engine sentence");
    expect(t("Hello {name}!", { name: "Player" })).toBe("Hello Player!");
    // Parameter formatting
    expect(t("{seat} drew a card", { seat: t("You") })).toBe("你抽了一张牌");
    expect(conquestText("winsNeeded")).toBe(
      `用你的 ${String(SERIES_WINS_NEEDED)} 套卡组各赢一局即可拿下系列赛（先拿到 ${String(SERIES_WINS_NEEDED)} 场胜利者获胜）。`,
    );
    // A finished sentence from the engine or the server matches the key its values were filled into
    expect(t("Your decks could not be loaded: offline")).toBe("无法加载卡组：offline");
    // ...and a token in parentheses matches only what stands inside them ("({cost}) Cost")
    expect(t("(3) Cost")).toBe("(3)费");
    // ...and the values it holds are looked up in turn, as the engine nests its sentences
    expect(t("Your cards cost 2 less (next turn)")).toBe("你的牌的法力值消耗减少2点（下回合）");
    // ...but a key with fewer than two words of its own ("{n} cards") never half-translates a sentence
    expect(t("Juggle 2 cards")).toBe("Juggle 2 cards");
    // Plurals: a `{ one, other }` entry picks its category by Intl.PluralRules over `count`
    const plural = { one: "{count} card", other: "{count} cards" };
    expect(words(plural, "en", { count: 1 })).toBe("{count} card");
    expect(words(plural, "en", { count: 2 })).toBe("{count} cards");
    expect(words(plural, "zh-CN", { count: 1 })).toBe("{count} cards");
    // Governing legal notice and date helpers
    expect(t("The English text governs.")).toBe("以英文版本为准。");
    expect(formatDay("2026-10-10")).toBe("2026年10月10日");
    expect(formatNumber(1234)).toBe("1,234");
    expect(formatSeconds(7200)).toBe("7200秒");
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

    // SETTINGS_SLOTS mounts language under gameplay; its reset forgets the choice, back to the browser's pick (en-US here):
    const langSlot = SETTINGS_SLOTS.find((slot) => slot.id === "language");
    expect(langSlot).toBeDefined();
    expect(langSlot?.section).toBe("gameplay");
    const panel = render(createElement(() => langSlot?.render()));
    fireEvent.change(panel.getByTestId("setting-language"), { target: { value: "zh-CN" } });
    await vi.waitFor(() => {
      expect(getLocale()).toBe("zh-CN");
    });
    expect(window.localStorage.getItem("jackioh.locale")).toBe("zh-CN");
    langSlot?.reset?.();
    await vi.waitFor(() => {
      expect(getLocale()).toBe("en");
    });
    expect(window.localStorage.getItem("jackioh.locale")).toBeNull();
    panel.unmount();

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

  it("R711000 every sentence the client wraps in t() has a zh-CN entry", () => {
    const files = sources(SRC);
    expect(files.length).toBeGreaterThan(100);
    const sentences = files.flatMap((path) => sentencesIn(path, readFileSync(path, "utf8")));
    expect(sentences.length).toBeGreaterThan(1000);
    expect(untranslated(sentences)).toEqual([]);
  });

  it("R711000 the t() guard names the file and line of a literal with no entry, in either branch of a conditional", () => {
    const probe = join(SRC, "probe.tsx");
    const text = [
      'const a = t("Your turn");',
      'const b = t("Some new sentence");',
      'const c = t(won ? "Victory" : `Another new sentence`, { n: 1 });',
      "const d = t(label);",
      'const e = other("Not wrapped in t");',
    ].join("\n");
    expect(sentencesIn(probe, text).map(({ key }) => key)).toEqual(["Your turn", "Some new sentence", "Victory", "Another new sentence"]);
    expect(untranslated(sentencesIn(probe, text))).toEqual(['probe.tsx:2: "Some new sentence"', 'probe.tsx:3: "Another new sentence"']);
    // Test files and ts-rs's generated wire types are not walked.
    const walked = sources(SRC).map((path) => relative(SRC, path));
    expect(walked).toContain(join("i18n", "i18n.ts"));
    expect(walked.filter((path) => /\.test\.tsx?$/u.test(path) || path.startsWith(join("wire", "generated")))).toEqual([]);
  });
});
