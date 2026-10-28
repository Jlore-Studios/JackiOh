// #2108, R2108000: the empty state. A brass tankard beside the words, hidden from a screen reader,
// and the next step where there is one; nothing moves.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { expectEmptyState } from "../test/empty.ts";
import { EmptyState, emptyStateTestid } from "./EmptyState.tsx";

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
});

describe("R2108000 the empty state", () => {
  it("R2108000 draws the tavern's mark beside the words, hidden from a screen reader, and no control unless given one", () => {
    render(<EmptyState text="Nothing here yet." testId="test-empty" />);
    const root = expectEmptyState(screen.getByTestId("test-empty"));
    expect(root.querySelector("svg")).not.toBeNull();
    expect(root.querySelector("a, button")).toBeNull();
  });

  it("R2108000 a next step with a path is a link that goes there inside the app", () => {
    render(<EmptyState text="Nothing here yet." testId="test-empty" action={{ label: "Play", to: "/play" }} />);
    const link = screen.getByTestId(emptyStateTestid.action);
    expect(link).toHaveAttribute("href", "/play");
    fireEvent.click(link);
    expect(window.location.pathname).toBe("/play");
  });

  it("R2108000 a next step with a handler is a button that calls it, under its own testid when given one", () => {
    const onPress = vi.fn();
    render(<EmptyState text="Nothing here yet." action={{ label: "Make one", onPress, testId: "test-action" }} />);
    const button = screen.getByTestId("test-action");
    expect(button.tagName).toBe("BUTTON");
    fireEvent.click(button);
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  it("R2108000 holds still and draws in the skeleton's brass", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "empty.css"), "utf8").replace(
      /\/\*[\s\S]*?\*\//g,
      "",
    );
    expect(css).not.toMatch(/\banimation\b|\btransition\b/);
    expect(css).toContain("var(--pal-brass-500)");
    const skeleton = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "../skeleton/skeleton.css"), "utf8");
    expect(skeleton).toContain("var(--pal-brass-500)");
  });
});
