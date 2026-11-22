// The stylesheet parser the tests share (moved out of fx/css.test.ts for issue #4493's layer estimator, R4493000): a sheet's
// text, comments dropped, as its rules (native nesting flattened, each with the `@media` it sits under) and its
// `@keyframes` names and bodies. Vitest stubs CSS imports, so a test reads a sheet as text (`readSheet`) and parses it here.

export type Rule = { selectors: string[]; decls: Map<string, string>; media: string | null };
export type Sheet = { rules: Rule[]; keyframes: Set<string>; text: string };

export function closingBrace(src: string, open: number): number {
  let depth = 0;
  for (let i = open; i < src.length; i += 1) {
    const ch = src[i];
    if (ch === "{") depth += 1;
    else if (ch === "}") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return src.length;
}

/** Splits on `sep` at bracket depth 0, so `:is(a, b)` and `rgb(1, 2, 3)` stay whole. */
export function splitTop(text: string, sep: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === "(" || ch === "[") depth += 1;
    else if (ch === ")" || ch === "]") depth -= 1;
    else if (ch === sep && depth === 0) {
      out.push(text.slice(start, i));
      start = i + 1;
    }
  }
  out.push(text.slice(start));
  return out.map((part) => part.trim()).filter((part) => part !== "");
}

export function parseDecls(text: string): Map<string, string> {
  const decls = new Map<string, string>();
  for (const part of splitTop(text, ";")) {
    const colon = part.indexOf(":");
    if (colon <= 0) continue;
    decls.set(part.slice(0, colon).trim().toLowerCase(), part.slice(colon + 1).trim().replace(/\s+/g, " "));
  }
  return decls;
}

/** Native nesting: `&` stands for the parent, and a bare child selector is a descendant of it. */
function nest(parents: readonly string[] | null, selectors: readonly string[]): string[] {
  if (parents === null) return [...selectors];
  return parents.flatMap((parent) =>
    selectors.map((child) => (child.includes("&") ? child.replaceAll("&", parent) : `${parent} ${child}`)),
  );
}

function walk(src: string, media: string | null, parents: string[] | null, sheet: Sheet): string {
  let flat = "";
  let i = 0;
  while (i < src.length) {
    const open = src.indexOf("{", i);
    if (open < 0) {
      flat += src.slice(i);
      break;
    }
    const head = src.slice(i, open);
    const cut = Math.max(head.lastIndexOf(";"), head.lastIndexOf("}"));
    flat += head.slice(0, cut + 1);
    const prelude = head.slice(cut + 1).trim();
    const close = closingBrace(src, open);
    const body = src.slice(open + 1, close);
    if (/^@(-webkit-)?keyframes\s/.test(prelude)) {
      sheet.keyframes.add(prelude.replace(/^@(-webkit-)?keyframes\s+/, "").trim());
    } else if (prelude.startsWith("@media")) {
      walk(body, prelude, parents, sheet);
    } else if (prelude.startsWith("@")) {
      walk(body, media, parents, sheet);
    } else {
      const selectors = nest(parents, splitTop(prelude, ","));
      const own = walk(body, media, selectors, sheet);
      sheet.rules.push({ selectors, decls: parseDecls(own), media });
    }
    i = close + 1;
  }
  return flat;
}

export function parseSheet(css: string): Sheet {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const sheet: Sheet = { rules: [], keyframes: new Set(), text };
  walk(text, null, null, sheet);
  return sheet;
}

/** Every `@keyframes name { … }` body in a sheet's comment-free text. */
export function keyframeBodies(sheet: Sheet): Map<string, string> {
  const out = new Map<string, string>();
  const head = /@(?:-webkit-)?keyframes\s+([\w-]+)\s*\{/g;
  for (let match = head.exec(sheet.text); match !== null; match = head.exec(sheet.text)) {
    const open = match.index + match[0].length - 1;
    out.set(match[1]!, sheet.text.slice(open + 1, closingBrace(sheet.text, open)));
  }
  return out;
}
