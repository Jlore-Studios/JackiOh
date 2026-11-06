// `auth/OAuthButtons.tsx`: the providers' buttons under `/login`'s forms, in one row, Google's and
// Roblox's drawn at one size (R3217000). Their layout is measured in `e2e/cypress/component/oauth-row.cy.tsx`.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { OAuthProvider } from "../net/auth.ts";
import OAuthButtons from "./OAuthButtons.tsx";
import { loginTestid } from "./testids.ts";

afterEach(() => {
  cleanup();
});

function draw(
  providers: readonly OAuthProvider[],
  { busy = false, onContinue = vi.fn() }: { busy?: boolean; onContinue?: (provider: OAuthProvider) => void } = {},
): void {
  render(
    <OAuthButtons
      providers={providers}
      googleClient={null}
      busy={busy}
      onContinue={onContinue}
      onGoogleCredential={vi.fn()}
    />,
  );
}

describe("R3217000 the providers' buttons", () => {
  it("R3217000 renders the providers in order inside one row", () => {
    draw(["google", "custom:roblox"]);
    const row = screen.getByTestId(loginTestid.oauthRow);
    const buttons = within(row).getAllByTestId(loginTestid.oauth);
    expect(buttons.map((button) => button.getAttribute("data-provider"))).toEqual(["google", "custom:roblox"]);
    expect(screen.getAllByTestId(loginTestid.oauth)).toHaveLength(2);
  });

  it("R3217000 one provider renders alone", () => {
    for (const provider of ["custom:roblox", "google"] as const) {
      draw([provider]);
      const row = screen.getByTestId(loginTestid.oauthRow);
      expect(row.children).toHaveLength(1);
      expect(within(row).getByTestId(loginTestid.oauth)).toHaveAttribute("data-provider", provider);
      cleanup();
    }
  });

  it("R3217000 the Roblox button's accessible name is Continue with Roblox", () => {
    draw(["google", "custom:roblox"]);
    const roblox = screen.getByRole("button", { name: "Continue with Roblox" });
    expect(roblox).toHaveAttribute("data-provider", "custom:roblox");
    expect(screen.getByRole("button", { name: "Continue with Google" })).toHaveAttribute("data-provider", "google");
  });

  it("R3217000 busy disables every button", () => {
    draw(["google", "custom:roblox", "github"], { busy: true });
    for (const button of screen.getAllByTestId(loginTestid.oauth)) expect(button).toBeDisabled();
    cleanup();
    draw(["google", "custom:roblox", "github"], { busy: false });
    for (const button of screen.getAllByTestId(loginTestid.oauth)) expect(button).toBeEnabled();
  });

  it("R3217000 a click names its provider", () => {
    const onContinue = vi.fn();
    draw(["google", "custom:roblox"], { onContinue });
    fireEvent.click(screen.getByRole("button", { name: "Continue with Roblox" }));
    expect(onContinue).toHaveBeenCalledTimes(1);
    expect(onContinue).toHaveBeenCalledWith("custom:roblox");
  });

  it("R3217000 the Roblox and Google buttons carry the same size classes", () => {
    draw(["google", "custom:roblox", "github"]);
    const [google, roblox, github] = screen.getAllByTestId(loginTestid.oauth);
    expect(google).toHaveClass("auth-oauth-branded", "auth-oauth-google");
    expect(roblox).toHaveClass("auth-oauth-branded", "auth-oauth-roblox");
    expect(github).not.toHaveClass("auth-oauth-branded");
    expect(github).toHaveClass("button-secondary");
  });

  it("R3217000 Google's button carries its own decorative mark", () => {
    draw(["google"]);
    const mark = screen.getByTestId(loginTestid.oauth).querySelector("svg");
    expect(mark).not.toBeNull();
    expect(mark).toHaveAttribute("aria-hidden", "true");
  });

  it("R3217000 Roblox's button carries Roblox's official tilt mark, white and decorative", () => {
    draw(["google", "custom:roblox"]);
    const roblox = screen.getByRole("button", { name: "Continue with Roblox" });
    const marks = roblox.querySelectorAll("svg.auth-oauth-mark");
    expect(marks).toHaveLength(1);
    expect(marks[0]).toHaveAttribute("aria-hidden", "true");
    const path = marks[0]?.querySelector("path");
    expect(path).toHaveAttribute(
      "d",
      "M31.6984 0L0 118.302L118.302 150L150 31.6984L31.6984 0ZM87.1031 95.9652L54.0454 87.1031L62.9075 54.0454L95.9784 62.9075L87.1031 95.9652Z",
    );
    expect(path).toHaveAttribute("fill", "currentColor");
  });
});
