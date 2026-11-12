// The pending choice as the board's prompt draws it (game/Prompt.tsx, fixtures `pendingFor`):
// the chooser's own Discover renders one picking button per option under its own chrome, and the
// seat only watching renders the waiting notice with no prompt kind (§10.8). Added for #3518: the
// fixtures' `pendingFor` had no new/changed test rendering one, only pre-existing suites.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { PlayerView } from "@jackioh/shared";

import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Prompt from "./Prompt.tsx";
import { baseView, card, emptySide, pendingFor, waitingPending } from "../test/fixtures.ts";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);

const DISCOVER = pendingFor("discover", [
  { key: "mode:core-043", label: "Flood", defId: "core-043" },
  { key: "mode:core-055", label: "Archivist", defId: "core-055" },
  { key: "mode:core-066", label: "Lava Golem", defId: "core-066" },
]);

function viewWithPending(): PlayerView {
  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-002" })] }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending: DISCOVER,
  });
}

function renderPrompt(view: PlayerView, onAction = vi.fn()): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt view={view} onAction={onAction} />
    </CatalogContext.Provider>,
  );
}

describe("pending choice", () => {
  it("the chooser's own Discover renders one picking button per option, and a click picks it", () => {
    const onAction = vi.fn();
    renderPrompt(viewWithPending(), onAction);

    const modal = screen.getByTestId("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "discover");
    expect(modal).not.toHaveAttribute("data-prompt-waiting");

    for (const defId of ["core-043", "core-055", "core-066"]) {
      const option = screen.getByTestId(`prompt-option-mode:${defId}`);
      expect(option.tagName).toBe("BUTTON");
      expect(option).toHaveAttribute("aria-pressed", "false");
    }

    fireEvent.click(screen.getByTestId("prompt-option-mode:core-055"));
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it("the seat only watching renders the waiting notice, with no prompt kind and no options", () => {
    const view = baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-002" })] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
      pending: waitingPending,
    });
    renderPrompt(view);

    const modal = screen.getByTestId("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-waiting", "waiting");
    expect(modal).not.toHaveAttribute("data-prompt-kind");
    expect(modal.querySelectorAll("[data-testid^='prompt-option-']")).toHaveLength(0);
    expect(screen.getByRole("status")).toHaveTextContent("Your opponent is choosing…");
  });
});
