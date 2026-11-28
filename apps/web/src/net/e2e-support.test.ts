// e2e/support's selectors hold no copy and no dead export (#5269): a test id one of the web's
// `testids.ts` files or `game/contract.ts`'s `testid` exports is imported from there, never written
// out again in e2e/support, and every export e2e/support makes is read by some file under e2e/.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");

/** The tracked files under `dir` (repository-relative) whose names match `pattern`, with their text. */
function tracked(dir: string, pattern: RegExp): Map<string, string> {
  const out = execFileSync("git", ["ls-files", dir], { cwd: ROOT, encoding: "utf8" });
  const files = out.split("\n").filter((file) => pattern.test(file));
  return new Map(files.map((file) => [file, readFileSync(join(ROOT, file), "utf8")]));
}

/** Every string a module's exports hold, at any depth of a plain object. */
function strings(value: unknown, into: Set<string>): Set<string> {
  if (typeof value === "string") into.add(value);
  else if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    for (const inner of Object.values(value)) strings(inner, into);
  }
  return into;
}

/** `export const NAME = "value";` declarations in `text` whose value `exported` holds. */
export function copiedTestids(text: string, exported: ReadonlySet<string>): string[] {
  return [...text.matchAll(/^export const (\w+) = "([^"]*)";$/gm)]
    .filter(([, , value]) => exported.has(value ?? ""))
    .map(([, name, value]) => `${name} = "${value}"`);
}

/** The `export const` and `export function` names of each `support` file that no file of `all` names but its declaration. */
export function unreadExports(support: ReadonlyMap<string, string>, all: ReadonlyMap<string, string>): string[] {
  const counts = new Map<string, number>();
  for (const text of all.values()) {
    for (const [word] of text.matchAll(/[A-Za-z_$][\w$]*/g)) counts.set(word, (counts.get(word) ?? 0) + 1);
  }
  const unread: string[] = [];
  for (const [file, text] of support) {
    for (const [, name] of text.matchAll(/^export (?:async )?(?:const|let|function) ([A-Za-z_$][\w$]*)/gm)) {
      if (name !== undefined && (counts.get(name) ?? 0) <= 1) unread.push(`${file}: ${name}`);
    }
  }
  return unread;
}

describe("#5269 e2e/support holds no copy of a web test id", () => {
  it("#5269 the scan finds a constant whose value the web exports, and only that one", () => {
    const text = 'export const LOBBY_LEAVE = "lobby-leave";\nexport const LOSS_POP = ".loss-pop";\n';
    expect(copiedTestids(text, new Set(["lobby-leave"]))).toEqual(['LOBBY_LEAVE = "lobby-leave"']);
  });

  it("#5269 no e2e/support constant restates a value the web's testids.ts files or contract.ts's testid export", async () => {
    const modules = tracked("apps/web/src", /\/testids\.ts$/);
    const exported = new Set<string>();
    for (const file of modules.keys()) {
      strings(await import(/* @vite-ignore */ join(ROOT, file)), exported);
    }
    const { testid } = await import("../game/contract.ts");
    strings(testid, exported);
    expect(exported.size).toBeGreaterThan(500);
    const copies = [...tracked("e2e/support", /\.ts$/)].flatMap(([file, text]) =>
      copiedTestids(text, exported).map((copy) => `${file}: ${copy}`),
    );
    expect(copies, "import these from the web's own testids.ts (or contract.ts's testid) instead").toEqual([]);
  });
});

describe("#5269 every e2e/support export is read", () => {
  it("#5269 the scan finds an export no file names but its declaration", () => {
    const support = new Map([["e2e/support/x.ts", 'export const READ = "a";\nexport function unread(): void {}\n']]);
    const all = new Map([...support, ["e2e/cypress/e2e/01.cy.ts", "cy.get(READ);\n"]]);
    expect(unreadExports(support, all)).toEqual(["e2e/support/x.ts: unread"]);
  });

  it("#5269 no export of e2e/support/*.ts goes unread under e2e/", () => {
    const all = tracked("e2e", /\.(ts|tsx|js|mjs)$/);
    const support = new Map([...all].filter(([file]) => /^e2e\/support\/[^/]+\.ts$/.test(file)));
    expect(support.size).toBeGreaterThan(10);
    expect(unreadExports(support, all), "delete these, or use them").toEqual([]);
  });
});
