// R2101000: the ranked win streak as a flame with its count, on the profile and a ranked
// match's line only.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { RANK_STREAK_FLAME_LENGTH } from "@jackioh/server-config";
import StreakFlame, { flameStreak, streakWords } from "./StreakFlame.tsx";
import { CREST_SIZE_PX } from "./RankCrest.tsx";

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

const N = RANK_STREAK_FLAME_LENGTH;

afterEach(cleanup);

describe("R2101000 the win-streak flame", () => {
  it("R2101000 shows a streak from RANK_STREAK_FLAME_LENGTH on and none below it", () => {
    expect(flameStreak(N)).toBe(N);
    expect(flameStreak(N + 5)).toBe(N + 5);
    expect(flameStreak(N - 1)).toBeNull();
    expect(flameStreak(0)).toBeNull();
  });

  it("R2101000 draws the flame with its count, named for a screen reader, at the crest's sizes", () => {
    for (const size of ["small", "large"] as const) {
      render(<StreakFlame streak={N} size={size} />);
      const flameElement = screen.getByRole("img", { name: streakWords(N) });
      expect(flameElement).toHaveAttribute("title", streakWords(N));
      expect(flameElement).toHaveAttribute("data-size", size);
      expect(flameElement).toHaveTextContent(String(N));
      expect(flameElement.querySelector("svg")).toHaveAttribute("width", String(CREST_SIZE_PX[size]));
      cleanup();
    }
    expect(streakWords(4)).toBe("4 ranked wins in a row");
  });

  it("R2101000 draws nothing below the threshold, and goes with the first loss's reset to 0", () => {
    const { container, rerender } = render(<StreakFlame streak={N - 1} size="small" />);
    expect(container).toBeEmptyDOMElement();
    rerender(<StreakFlame streak={N + 1} size="small" />);
    expect(screen.getByRole("img", { name: streakWords(N + 1) })).toBeInTheDocument();
    rerender(<StreakFlame streak={0} size="small" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("R2101000 only the profile and the match screen draw it: no practice, tutorial, co-op or lobby screen does", () => {
    const drawn = sources(SRC)
      .filter((path) => readFileSync(path, "utf8").includes('StreakFlame.tsx"'))
      .map((path) => relative(SRC, path))
      .sort();
    expect(drawn).toEqual([join("routes", "account.tsx"), join("routes", "match.tsx")]);
  });
});
