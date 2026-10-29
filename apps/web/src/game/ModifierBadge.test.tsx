// R169 on touch (#2321): a hero's modifier badge is cut to a few words beside the portrait (12ch on a
// phone), so Call to Chaos (Training-Grounds Edition)'s clause read "Add a rand…", and its `title` is a
// hover a touch screen never shows. A tap on the badge opens its whole label beside it, laid on the
// page so the hero row keeps its layout, and the tap never reaches the hero's own click.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ModifierView } from "@jackioh/shared";

import { withCatalog } from "../test/board.tsx";
import { baseView, emptySide } from "../test/fixtures.ts";
import Hero from "./Hero.tsx";
import { ModifierBadge, modifierFullTestid } from "./ModifierBadge.tsx";

afterEach(() => {
  cleanup();
});

const LONG: ModifierView = {
  id: "callToChaosTg",
  label: "Add a random Call to Chaos to your hand at the start of your every other turn",
};

describe("R169 a modifier badge read in full on touch", () => {
  it("R169 a tap opens the whole label on the page, and a second tap, Escape or a tap elsewhere closes it", () => {
    render(<ModifierBadge modifier={LONG} />);
    const badge = screen.getByRole("button", { name: LONG.label });
    expect(badge).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId(modifierFullTestid(LONG.id))).toBeNull();

    fireEvent.click(badge);
    const full = screen.getByTestId(modifierFullTestid(LONG.id));
    expect(full).toHaveTextContent(LONG.label);
    expect(full.parentElement).toBe(document.body);
    expect(badge).toHaveAttribute("aria-expanded", "true");
    expect(badge).toHaveAttribute("aria-controls", full.id);
    expect(badge).toHaveAttribute("data-open", "true");

    fireEvent.click(badge);
    expect(screen.queryByTestId(modifierFullTestid(LONG.id))).toBeNull();

    fireEvent.click(badge);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId(modifierFullTestid(LONG.id))).toBeNull();

    fireEvent.click(badge);
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId(modifierFullTestid(LONG.id))).toBeNull();
  });

  it("R169 the label stays inside a 16px gutter at a phone's width, under a badge in the top half and over one in the bottom half", () => {
    const width = window.innerWidth;
    const height = window.innerHeight;
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 390 });
    Object.defineProperty(window, "innerHeight", { configurable: true, value: 844 });
    try {
      render(<ModifierBadge modifier={LONG} />);
      const badge = screen.getByRole("button", { name: LONG.label });
      const rect = (top: number): DOMRect => ({ left: 300, right: 380, top, bottom: top + 20, width: 80, height: 20, x: 300, y: top, toJSON: () => ({}) });

      vi.spyOn(badge, "getBoundingClientRect").mockReturnValue(rect(100));
      fireEvent.click(badge);
      let full = screen.getByTestId(modifierFullTestid(LONG.id));
      expect(full.style.left).toBe("16px");
      expect(full.style.maxWidth).toBe("358px");
      expect(full.style.top).toBe("126px");
      fireEvent.click(badge);

      vi.spyOn(badge, "getBoundingClientRect").mockReturnValue(rect(700));
      fireEvent.click(badge);
      full = screen.getByTestId(modifierFullTestid(LONG.id));
      expect(full.style.top).toBe("");
      expect(full.style.bottom).toBe("150px");
    } finally {
      Object.defineProperty(window, "innerWidth", { configurable: true, value: width });
      Object.defineProperty(window, "innerHeight", { configurable: true, value: height });
    }
  });

  it("R169 the hero draws one such badge per view modifier, and a tap on one never reaches the hero's click", () => {
    const onClick = vi.fn();
    render(withCatalog(<Hero view={baseView({ you: emptySide("p1", { modifiers: [LONG] }) })} side="you" onClick={onClick} />));
    const badges = screen.getByTestId("modifiers-you").querySelectorAll(".modifier-badge");
    expect(badges).toHaveLength(1);
    expect(badges[0]).toHaveAttribute("data-modifier-id", LONG.id);
    expect(badges[0]).toHaveAttribute("title", LONG.label);
    fireEvent.click(badges[0] as Element);
    expect(screen.getByTestId(modifierFullTestid(LONG.id))).toHaveTextContent(LONG.label);
    expect(onClick).not.toHaveBeenCalled();
  });
});
