// R3218000: BackLink's DOM is the top bar's one layout, in the order routes/top-bar.css draws it: Back, the
// line (the screen's title, then its extras), the end group (the Jlore pill, Friends, language, the gear). jsdom
// ignores the cascade, so top-bar.test.ts proves the sheet and e2e spec 46 the drawn geometry.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { expectOnePageTitle } from "../test/pageTitle.ts";
import { BackLink, navTestid } from "./nav.tsx";

beforeEach(() => {
  window.localStorage.clear();
  // The bar needs no server: an account read that never answers changes nothing.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R3218000 BackLink's slots", () => {
  it("R3218000 the nav holds Back, then the line (the title, then the extras), then the end group with the gear last", () => {
    render(
      <BackLink showWallet={false} title="Screen">
        <button type="button">Extra</button>
      </BackLink>,
    );

    const bar = screen.getByRole("navigation");
    expect(bar).toHaveClass("screen-nav");
    const slots = Array.from(bar.children);
    expect(slots).toHaveLength(3);
    expect(slots[0]).toBe(screen.getByTestId(navTestid.back));
    expect(slots[1]).toHaveClass("screen-nav__line");
    expect(slots[2]).toHaveClass("screen-nav__end");

    const line = Array.from((slots[1] as HTMLElement).children);
    expect(line.map((node) => node.tagName)).toEqual(["H1", "DIV"]);
    expect(line[1]).toHaveClass("screen-nav__extra");
    expect(line[1]).toContainElement(screen.getByRole("button", { name: "Extra" }));

    const end = Array.from((slots[2] as HTMLElement).querySelectorAll("[data-testid]"));
    expect(end.at(-1)).toBe(screen.getByTestId("settings-open-nav"));
    expect(end.map((node) => node.getAttribute("data-testid"))).toEqual(["language-nav", "settings-open-nav"]);
  });

  it("R3223000 the bar's title is the PageTitle, the only h1, first in the line", () => {
    render(<BackLink showWallet={false} title="Screen" />);

    const line = screen.getByRole("navigation").querySelector(".screen-nav__line");
    expect(line?.firstElementChild).toBe(expectOnePageTitle("Screen"));
  });

  it("R3218000 a bar with no title and no extras has no line, and the end group follows Back", () => {
    render(<BackLink showWallet={false} />);

    const bar = screen.getByRole("navigation");
    expect(Array.from(bar.children).map((node) => node.className.split(" ").pop())).toEqual([
      "top-bar-button--text",
      "screen-nav__end",
    ]);
  });

  it("R3218000 Back alone, a bar with no settings, is the one button", () => {
    render(<BackLink showSettings={false} />);

    expect(screen.getByRole("navigation").children).toHaveLength(1);
    expect(screen.getByTestId(navTestid.back)).toBeInTheDocument();
  });

  it("R3218000 a screen's own press and test id keep Back the bar's pill", () => {
    const onPress = vi.fn<() => void>();
    render(<BackLink onPress={onPress} testId="replay-back" showSettings={false} />);

    const back = screen.getByTestId("replay-back");
    expect(back).toHaveClass("top-bar-button", "top-bar-button--text");
    expect(back.closest("nav")).toHaveClass("screen-nav");
    fireEvent.click(back);
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  it("R3230000 Back is an arrow icon and the word Back", () => {
    const { unmount } = render(<BackLink showSettings={false} />);
    const back = screen.getByTestId(navTestid.back);
    expect(back.textContent).toBe("Back");
    expect(back.querySelector("svg.arrow-icon--back[aria-hidden='true']")).not.toBeNull();

    unmount();
    render(<BackLink showSettings={false} label="Back to Play together" />);
    const custom = screen.getByTestId(navTestid.back);
    expect(custom.textContent).toBe("Back to Play together");
    expect(custom.querySelector("svg.arrow-icon--back[aria-hidden='true']")).not.toBeNull();
  });
});
