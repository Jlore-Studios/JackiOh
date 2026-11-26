// Tests and acceptance criteria for Icon component and Unicode glyph-as-icon replacement (#5016).

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { render } from "@testing-library/react";
import ts from "typescript";
import { describe, expect, it } from "vitest";

import { Icon, type IconName, type IconSize } from "./Icon.tsx";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = join(HERE, "..");

const ALL_NAMES: readonly IconName[] = ["close", "check", "spark", "star", "crown", "skull", "flag"];
const ALL_SIZES: readonly IconSize[] = ["xs", "sm", "md", "lg", "xl", "2xl", "3xl", "4xl"];

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

/** Collect all non-test ts and tsx files under dir */
function nonTestSourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (name === "test" || name === "pkg" || name === "testkit" || name === "node_modules") return [];
      return nonTestSourceFiles(full);
    }
    return (name.endsWith(".tsx") || name.endsWith(".ts")) && !name.includes(".test.") && !name.endsWith(".d.ts")
      ? [full]
      : [];
  });
}

/** Collect all non-test tsx files under dir */
function nonTestTsxFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (name === "test" || name === "pkg" || name === "testkit" || name === "node_modules") return [];
      return nonTestTsxFiles(full);
    }
    return name.endsWith(".tsx") && !name.includes(".test.") ? [full] : [];
  });
}

const CANDIDATE_RE = /[\u2600-\u27bf\u{1f300}-\u{1faff}]|\\u/u;

const sourceFileCache = new Map<string, ts.SourceFile>();
function getCachedSourceFile(filePath: string): ts.SourceFile | null {
  if (sourceFileCache.has(filePath)) return sourceFileCache.get(filePath)!;
  if (!existsSync(filePath)) return null;
  const content = readFileSync(filePath, "utf8");
  const kind = filePath.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS;
  const sf = ts.createSourceFile(filePath, content, ts.ScriptTarget.Latest, false, kind);
  sourceFileCache.set(filePath, sf);
  return sf;
}

function getStringsFromInitializer(init: ts.Expression | undefined): string[] {
  if (!init) return [];
  if (ts.isStringLiteral(init) || ts.isNoSubstitutionTemplateLiteral(init)) {
    return [init.text];
  }
  if (ts.isObjectLiteralExpression(init)) {
    const strings: string[] = [];
    for (const prop of init.properties) {
      if (ts.isPropertyAssignment(prop) && (ts.isStringLiteral(prop.initializer) || ts.isNoSubstitutionTemplateLiteral(prop.initializer))) {
        strings.push(prop.initializer.text);
      }
    }
    return strings;
  }
  if (ts.isArrayLiteralExpression(init)) {
    const strings: string[] = [];
    for (const el of init.elements) {
      if (ts.isStringLiteral(el) || ts.isNoSubstitutionTemplateLiteral(el)) {
        strings.push(el.text);
      }
    }
    return strings;
  }
  return [];
}

const importedStringsCache = new Map<string, string[]>();
function resolveImportedStrings(importInfo: { spec: string; importedName: string }, fromFile: string): string[] {
  const cacheKey = `${fromFile}:${importInfo.spec}:${importInfo.importedName}`;
  const cached = importedStringsCache.get(cacheKey);
  if (cached) return cached;
  if (!importInfo.spec.startsWith(".")) {
    importedStringsCache.set(cacheKey, []);
    return [];
  }
  const dir = dirname(fromFile);
  let targetPath = resolve(dir, importInfo.spec);
  if (!existsSync(targetPath)) {
    if (existsSync(targetPath + ".ts")) targetPath += ".ts";
    else if (existsSync(targetPath + ".tsx")) targetPath += ".tsx";
  }
  const targetSf = getCachedSourceFile(targetPath);
  if (!targetSf) {
    importedStringsCache.set(cacheKey, []);
    return [];
  }
  const strings: string[] = [];
  for (const stmt of targetSf.statements) {
    if (ts.isVariableStatement(stmt)) {
      for (const decl of stmt.declarationList.declarations) {
        if (ts.isIdentifier(decl.name) && decl.name.text === importInfo.importedName) {
          strings.push(...getStringsFromInitializer(decl.initializer));
        }
      }
    }
  }
  importedStringsCache.set(cacheKey, strings);
  return strings;
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
  // Pings and markdown release notes in .ts files
  "👍", "👎", "🌀", "⚠️", "⚠", "🎯",
  "✨", "⚖️", "⚖", "🔄", "📝", "🩹", "⚙️", "⚙", "📸",
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
  it("fails on any forbidden glyphs or unlisted emoji ranges in apps/web/src/**/*.{ts,tsx} outside tests and in i18n strings", () => {
    const files = nonTestSourceFiles(SRC);
    const violations: { file: string; char: string; code: string; text: string }[] = [];

    for (const file of files) {
      const isTsx = file.endsWith(".tsx");
      const rawContent = readFileSync(file, "utf8");
      const hasCandidates = CANDIDATE_RE.test(rawContent);
      if (!isTsx && !hasCandidates) continue;

      const sf = getCachedSourceFile(file);
      if (!sf) continue;

      function checkText(text: string, context?: string) {
        for (const ch of text) {
          if (FORBIDDEN_GLYPHS.has(ch)) {
            violations.push({
              file: relative(SRC, file),
              char: ch,
              code: (ch.codePointAt(0) ?? 0).toString(16),
              text: context ?? text.trim(),
            });
          } else if (isEmojiOrSymbol(ch) && !ALLOWED_TEXT_SYMBOLS.has(ch)) {
            violations.push({
              file: relative(SRC, file),
              char: ch,
              code: (ch.codePointAt(0) ?? 0).toString(16),
              text: context ?? text.trim(),
            });
          }
        }
      }

      const imports = new Map<string, { spec: string; importedName: string }>();
      if (isTsx) {
        for (const stmt of sf.statements) {
          if (ts.isImportDeclaration(stmt) && stmt.importClause?.namedBindings && ts.isNamedImports(stmt.importClause.namedBindings)) {
            const spec = (stmt.moduleSpecifier as ts.StringLiteral).text;
            for (const elem of stmt.importClause.namedBindings.elements) {
              const localName = elem.name.text;
              const importedName = elem.propertyName ? elem.propertyName.text : localName;
              imports.set(localName, { spec, importedName });
            }
          }
        }
      }

      function walk(node: ts.Node) {
        if (
          hasCandidates &&
          (ts.isStringLiteral(node) ||
            ts.isNoSubstitutionTemplateLiteral(node) ||
            ts.isTemplateHead(node) ||
            ts.isTemplateMiddle(node) ||
            ts.isTemplateTail(node) ||
            ts.isJsxText(node))
        ) {
          checkText(node.text);
        }

        if (isTsx && ts.isJsxExpression(node) && node.expression) {
          let id: string | null = null;
          if (ts.isIdentifier(node.expression)) {
            id = node.expression.text;
          } else if (ts.isElementAccessExpression(node.expression) && ts.isIdentifier(node.expression.expression)) {
            id = node.expression.expression.text;
          } else if (ts.isPropertyAccessExpression(node.expression) && ts.isIdentifier(node.expression.expression)) {
            id = node.expression.expression.text;
          }
          if (id && imports.has(id)) {
            const strings = resolveImportedStrings(imports.get(id)!, file);
            for (const str of strings) {
              checkText(str, `imported {${id}} from "${imports.get(id)!.spec}"`);
            }
          }
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
      const sf = getCachedSourceFile(file);
      if (!sf || !sf.text.includes("button")) continue;
      const sourceFile = sf;

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
            const { line } = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
            unlabelledButtons.push({ file: relative(SRC, file), line: line + 1 });
          }
        }
        ts.forEachChild(node, walk);
      }

      walk(sourceFile);
    }

    expect(unlabelledButtons).toEqual([]);
  });
});
