// Tests and acceptance criteria for Icon component and Unicode glyph-as-icon replacement (#5016).

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { render } from "@testing-library/react";
import ts from "typescript";
import { describe, expect, it } from "vitest";

import { Icon, type IconName, type IconSize } from "./Icon.tsx";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = join(HERE, "..");

const ALL_NAMES: readonly IconName[] = ["close", "check", "spark", "star", "crown", "skull", "flag"];
const ALL_SIZES: readonly IconSize[] = ["3xs", "2xs", "xs", "sm", "md", "lg", "xl", "2xl", "3xl", "4xl"];

describe("Icon component", () => {
  it("renders all 7 inline SVGs with aria-hidden, focusable=false, and currentColor", () => {
    for (const name of ALL_NAMES) {
      const { container } = render(<Icon name={name} />);
      const svg = container.querySelector("svg");
      expect(svg, `SVG element for ${name}`).not.toBeNull();
      expect(svg?.getAttribute("aria-hidden")).toBe("true");
      expect(svg?.getAttribute("focusable")).toBe("false");
      expect(svg?.getAttribute("viewBox")).toBe("0 0 16 16");
      expect(svg?.getAttribute("stroke")).toBe("currentColor");
      expect(svg?.getAttribute("fill")).toBe("none");
      expect(svg?.classList.contains("icon")).toBe(true);
      expect(svg?.classList.contains(`icon--${name}`)).toBe(true);
    }
  });

  it("applies size classes from the type scale and merges custom className", () => {
    for (const size of ALL_SIZES) {
      const { container } = render(<Icon name="close" size={size} className="custom-test" />);
      const svg = container.querySelector("svg");
      expect(svg?.classList.contains(`icon--size-${size}`)).toBe(true);
      expect(svg?.classList.contains("custom-test")).toBe(true);
    }
  });
});

/** Collect all non-test tsx files under dir */
function nonTestTsxFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (name === "test" || name === "pkg" || name === "testkit") return [];
      return nonTestTsxFiles(full);
    }
    return name.endsWith(".tsx") && !name.includes(".test.") ? [full] : [];
  });
}

function i18nJsonFiles(dir: string): string[] {
  const i18nDir = join(dir, "i18n");
  return readdirSync(i18nDir)
    .filter((name) => name.endsWith(".json"))
    .map((name) => join(i18nDir, name));
}

const FORBIDDEN_GLYPHS = new Set(["✕", "✓", "✦", "★", "♛", "☠", "⚑"]);

/** Characters that represent legitimate text or mathematical punctuation rather than icons */
const ALLOWED_TEXT_SYMBOLS = new Set([
  "±", "×", "←", "↑", "→", "↓", "−", "∞", "⊘", "⋯", "⌘",
  "⏩", "⏪", "⏭", "⏮", "⏸", "▶", "⏳",
  "▲", "▼", "▸", "▾", "◆", "·", "“", "”", "⟳",
  "•", "–", "—", "…", "‹", "›", "。", "：", "¥", "§",
  "⌊", "⌋",
]);

function isEmojiOrSymbol(ch: string): boolean {
  const code = ch.codePointAt(0);
  if (code === undefined) return false;
  return (
    (code >= 0x1f300 && code <= 0x1f5ff) ||
    (code >= 0x1f600 && code <= 0x1f64f) ||
    (code >= 0x1f680 && code <= 0x1f6ff) ||
    (code >= 0x1f900 && code <= 0x1f9ff) ||
    (code >= 0x1fa70 && code <= 0x1faff) ||
    (code >= 0x2600 && code <= 0x26ff) ||
    (code >= 0x2700 && code <= 0x27bf)
  );
}

describe("Issue #5016 acceptance", () => {
  it("fails on any forbidden glyphs or unlisted emoji ranges in apps/web/src/**/*.tsx outside tests and in i18n strings", () => {
    const files = nonTestTsxFiles(SRC);
    const violations: { file: string; char: string; code: string; text: string }[] = [];

    for (const file of files) {
      const content = readFileSync(file, "utf8");
      const sf = ts.createSourceFile(file, content, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);

      function checkText(text: string) {
        for (const ch of text) {
          if (FORBIDDEN_GLYPHS.has(ch)) {
            violations.push({
              file: relative(SRC, file),
              char: ch,
              code: (ch.codePointAt(0) ?? 0).toString(16),
              text: text.trim(),
            });
          } else if (isEmojiOrSymbol(ch) && !ALLOWED_TEXT_SYMBOLS.has(ch)) {
            violations.push({
              file: relative(SRC, file),
              char: ch,
              code: (ch.codePointAt(0) ?? 0).toString(16),
              text: text.trim(),
            });
          }
        }
      }

      function walk(node: ts.Node) {
        if (
          ts.isStringLiteral(node) ||
          ts.isNoSubstitutionTemplateLiteral(node) ||
          ts.isTemplateHead(node) ||
          ts.isTemplateMiddle(node) ||
          ts.isTemplateTail(node) ||
          ts.isJsxText(node)
        ) {
          checkText(node.text);
        }
        ts.forEachChild(node, walk);
      }

      walk(sf);
    }

    // Also check i18n JSON files
    for (const jsonFile of i18nJsonFiles(SRC)) {
      const data = JSON.parse(readFileSync(jsonFile, "utf8")) as Record<string, unknown>;
      for (const [key, val] of Object.entries(data)) {
        const textToCheck = [key, typeof val === "string" ? val : ""].join(" ");
        for (const ch of textToCheck) {
          if (FORBIDDEN_GLYPHS.has(ch) || (isEmojiOrSymbol(ch) && !ALLOWED_TEXT_SYMBOLS.has(ch))) {
            violations.push({
              file: relative(SRC, jsonFile),
              char: ch,
              code: (ch.codePointAt(0) ?? 0).toString(16),
              text: `${key}: ${String(val)}`,
            });
          }
        }
      }
    }

    expect(violations).toEqual([]);
  });

  it("checks that every <Icon> inside a button whose only content is the icon has an accessible name", () => {
    const files = nonTestTsxFiles(SRC);
    const unlabelledButtons: { file: string; line: number }[] = [];

    for (const file of files) {
      const content = readFileSync(file, "utf8");
      const sf = ts.createSourceFile(file, content, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);

      function hasAccessibleName(attrs: ts.JsxAttributes): boolean {
        return attrs.properties.some((prop) => {
          if (!ts.isJsxAttribute(prop)) return false;
          const attrName = ts.isIdentifier(prop.name) ? prop.name.text : prop.name.name.text;
          return attrName === "aria-label" || attrName === "aria-labelledby" || attrName === "title";
        });
      }

      function isIconElement(node: ts.JsxChild): boolean {
        if (ts.isJsxSelfClosingElement(node)) {
          return ts.isIdentifier(node.tagName) && node.tagName.text === "Icon";
        }
        if (ts.isJsxElement(node)) {
          return ts.isIdentifier(node.openingElement.tagName) && node.openingElement.tagName.text === "Icon";
        }
        return false;
      }

      function walk(node: ts.Node) {
        if (ts.isJsxElement(node) && ts.isIdentifier(node.openingElement.tagName) && node.openingElement.tagName.text === "button") {
          // Check children ignoring whitespace-only JsxText
          const nonWhitespaceChildren = node.children.filter((child) => {
            if (ts.isJsxText(child) && child.text.trim() === "") return false;
            return true;
          });

          const isIconOnly = nonWhitespaceChildren.length === 1 && isIconElement(nonWhitespaceChildren[0]!);
          if (isIconOnly && !hasAccessibleName(node.openingElement.attributes)) {
            const { line } = sf.getLineAndCharacterOfPosition(node.getStart(sf));
            unlabelledButtons.push({ file: relative(SRC, file), line: line + 1 });
          }
        }
        ts.forEachChild(node, walk);
      }

      walk(sf);
    }

    expect(unlabelledButtons).toEqual([]);
  });
});
