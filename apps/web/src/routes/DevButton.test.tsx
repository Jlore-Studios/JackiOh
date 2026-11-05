// R3156000 (#3156): the front screen's Dev button, a dev account's door to the Dev tools on a computer.
// jsdom has no media queries, so each test answers `DEV_PANEL_QUERY` itself.

import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { MeResponse } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import { DevButton, devButtonTestid } from "./DevButton.tsx";
import { DEV_PANEL_MIN_WIDTH_PX, DEV_PANEL_QUERY } from "./devPanel.ts";
import { devToolsTestid } from "./DevTools.tsx";

const TOKEN = "token-1";

function me(isDev: boolean | undefined): MeResponse {
  return {
    profile: { id: "d0000000-0000-4000-8000-000000003156", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "dev@example.test",
    ...(isDev === undefined ? {} : { isDev }),
  };
}

function ready(isDev: boolean | undefined): Account {
  return { kind: "ready", token: TOKEN, me: me(isDev) };
}

/** Answers `DEV_PANEL_QUERY` with `onComputer`, every other query as the setup does. */
function screenIs(onComputer: boolean): void {
  const original = window.matchMedia;
  vi.stubGlobal("matchMedia", (query: string): MediaQueryList => {
    if (query !== DEV_PANEL_QUERY) return original(query);
    return {
      media: query,
      matches: onComputer,
      onchange: null,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => false,
    } as unknown as MediaQueryList;
  });
}

beforeEach(() => {
  // The Dev tools read the held quests when they open.
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve({
        ok: true,
        status: 200,
        text: () => Promise.resolve(JSON.stringify({ quests: [], nextDueAt: null })),
      } as unknown as Response),
    ),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R3156000 the front screen's Dev button", () => {
  it("R3156000 a computer's window: a wide one whose main pointer is fine", () => {
    expect(DEV_PANEL_QUERY).toBe(`(min-width: ${String(DEV_PANEL_MIN_WIDTH_PX)}px) and (pointer: fine)`);
  });

  it("R3156000 shows for a dev account on a computer and opens the Dev tools in a dialog", async () => {
    screenIs(true);
    render(<DevButton account={ready(true)} />);
    const button = screen.getByTestId(devButtonTestid.button);
    expect(button).toHaveTextContent("Dev");
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId(devButtonTestid.panel)).toBeNull();

    await userEvent.click(button);
    const panel = screen.getByTestId(devButtonTestid.panel);
    expect(panel).toHaveAttribute("role", "dialog");
    expect(panel).toHaveAttribute("aria-modal", "true");
    expect(button).toHaveAttribute("aria-expanded", "true");
    expect(button).toHaveAttribute("aria-controls", panel.id);
    expect(within(panel).getByRole("heading", { name: "Dev tools" })).toBeInTheDocument();
    expect(within(panel).getByTestId(devToolsTestid.panel)).toBeInTheDocument();
    expect(within(panel).getByTestId(devToolsTestid.unlockCosmetics)).toHaveTextContent("Unlock all cosmetics");
    expect(within(panel).getByTestId(devToolsTestid.fill)).toBeInTheDocument();
  });

  it("R3156000 Escape, the close button and the scrim each close it, and the focus goes back to the button", async () => {
    screenIs(true);
    render(<DevButton account={ready(true)} />);
    const button = screen.getByTestId(devButtonTestid.button);

    await userEvent.click(button);
    await waitFor(() => {
      expect(screen.getByTestId(devButtonTestid.close)).toHaveFocus();
    });
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByTestId(devButtonTestid.panel)).toBeNull();
    expect(button).toHaveFocus();

    await userEvent.click(button);
    await userEvent.click(screen.getByTestId(devButtonTestid.close));
    expect(screen.queryByTestId(devButtonTestid.panel)).toBeNull();
    expect(button).toHaveFocus();

    await userEvent.click(button);
    const scrim = screen.getByTestId(devButtonTestid.panel).parentElement;
    if (scrim === null) throw new Error("the dialog sits on its scrim");
    await userEvent.click(scrim);
    expect(screen.queryByTestId(devButtonTestid.panel)).toBeNull();
  });

  it("R3156000 shows for no account the server does not call a dev account", () => {
    screenIs(true);
    for (const isDev of [false, undefined]) {
      const view = render(<DevButton account={ready(isDev)} />);
      expect(screen.queryByTestId(devButtonTestid.button), `isDev ${String(isDev)}`).toBeNull();
      view.unmount();
    }
    for (const account of [{ kind: "loading" }, { kind: "anonymous" }, { kind: "error", message: "down" }] as const) {
      const view = render(<DevButton account={account} />);
      expect(screen.queryByTestId(devButtonTestid.button), account.kind).toBeNull();
      view.unmount();
    }
  });

  it("R3156000 shows on no phone or tablet, even for a dev account", () => {
    screenIs(false);
    render(<DevButton account={ready(true)} />);
    expect(screen.queryByTestId(devButtonTestid.button)).toBeNull();
  });
});
