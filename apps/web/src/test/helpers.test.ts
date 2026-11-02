// #2215: the web's tests take `settle`, `withCatalog`, `renderBoard` and `ensurePointerEvent` from
// src/test/, never from a copy of their own (as the Rust tests take their shared kit, #1777); a helper
// whose body differs takes another name. The planted copies below are strings, not definitions.

import { describe, expect, it } from "vitest";

import boardSource from "./board.tsx?raw";
import settleSource from "./settle.ts?raw";
import setupSource from "./setup.ts?raw";

const TESTS = import.meta.glob("../**/*.test.{ts,tsx}", { query: "?raw", import: "default", eager: true }) as Record<string, string>;

/** Fewer test files than this means the glob missed the tree (#2215 counted 401). */
const MIN_TEST_FILES = 300;

const SHARED: readonly { name: string; home: string; source: string }[] = [
  { name: "settle", home: "test/settle.ts", source: settleSource },
  { name: "withCatalog", home: "test/board.tsx", source: boardSource },
  { name: "renderBoard", home: "test/board.tsx", source: boardSource },
  { name: "ensurePointerEvent", home: "test/setup.ts", source: setupSource },
];

/** A line defining `name`: a function declaration, or a `const`/`let`/`var` bound to a function, at any indent. */
function definesAt(name: string): RegExp {
  return new RegExp(String.raw`^\s*(?:export\s+)?(?:(?:async\s+)?function\s+${name}\s*[(<]|(?:const|let|var)\s+${name}\s*=\s*(?:async\b|function\b|[(<]))`);
}

const PATTERNS = SHARED.map(({ name }) => ({ name, pattern: definesAt(name) }));

function copiesIn(source: string): string[] {
  return source.split("\n").flatMap((line, index) =>
    PATTERNS.filter(({ pattern }) => pattern.test(line)).map(({ name }) => `line ${String(index + 1)}: ${name}`),
  );
}

describe("#2215 the web's tests take their shared helpers from src/test", () => {
  it("#2215 the scan reads every test file under src", () => {
    const paths = Object.keys(TESTS);
    expect(paths.length).toBeGreaterThan(MIN_TEST_FILES);
    for (const expected of ["/routes/practice.test.tsx", "/together/polling.test.tsx", "/game/Card.test.tsx", "/cards/inspect/swipe.test.tsx"]) {
      expect(
        paths.some((path) => path.endsWith(expected)),
        `${expected} is scanned`,
      ).toBe(true);
    }
  });

  it("#2215 each shared helper is defined once, in its home", () => {
    for (const { name, home, source } of SHARED) {
      expect(
        copiesIn(source).filter((copy) => copy.endsWith(`: ${name}`)),
        home,
      ).toHaveLength(1);
    }
  });

  it("#2215 the scan finds a planted copy of each helper, and nothing else", () => {
    const planted = [
      "async function settle(): Promise<void> {",
      "}",
      "  function renderBoard(view: PlayerView): void {",
      "export function withCatalog(node: ReactElement): ReactElement {",
      "const ensurePointerEvent = (): void => {};",
      "let settle = async () => {};",
      "function settleReads(): void {}",
      "const withCatalog = makeEngine();",
      "// function settle(): void {}",
      "await settle({ ticks: 1 });",
      'import { renderBoard, withCatalog } from "../test/board.tsx";',
      'const text = "function settle() {}";',
    ].join("\n");
    expect(copiesIn(planted)).toEqual(["line 1: settle", "line 3: renderBoard", "line 4: withCatalog", "line 5: ensurePointerEvent", "line 6: settle"]);
  });

  it("#2215 no test file defines its own settle, withCatalog, renderBoard or ensurePointerEvent", () => {
    const copies = Object.entries(TESTS).flatMap(([path, source]) => copiesIn(source).map((copy) => `${path} ${copy}`));
    expect(copies, "import the one in test/settle.ts, test/board.tsx or test/setup.ts, or give a helper whose body differs another name").toEqual([]);
  });
});
