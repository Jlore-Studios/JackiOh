// R3024000: a mailed link opens `/login` and waits. Only the player's press goes on to the provider, since a
// mail scanner opens links, and that alone must neither confirm an address nor spend the link.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { clearConsumedAuthRedirect } from "../auth/redirect.ts";
import { loginTestid } from "../auth/testids.ts";
import { AUTH_MESSAGES, AuthError, followEmailLink } from "../net/auth.ts";
import { settle } from "../test/settle.ts";
import LoginRoute from "./login.tsx";

vi.mock("../net/auth.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../net/auth.ts")>()),
  followEmailLink: vi.fn(),
}));

const TOKEN = "pkce_0123456789abcdef";

const fetchStub = vi.fn(() => Promise.resolve(new Response(null, { status: 404 })));

function open(type: string): void {
  window.history.replaceState(null, "", `/login?token_hash=${TOKEN}&type=${type}`);
  render(<LoginRoute />);
}

function button(): HTMLElement {
  return screen.getByTestId(loginTestid.pressLink);
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", "https://project.supabase.co");
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "sb_publishable_test");
  vi.stubGlobal("fetch", fetchStub);
  vi.mocked(followEmailLink).mockReset();
  fetchStub.mockClear();
  window.localStorage.clear();
  clearConsumedAuthRedirect();
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  clearConsumedAuthRedirect();
  window.history.replaceState(null, "", "/login");
});

describe("R3024000 a mailed link waits for a press", () => {
  it("R3024000 opening the link sends nothing, and the token leaves the address bar", async () => {
    open("signup");
    await settle();
    expect(button()).toBeEnabled();
    expect(followEmailLink).not.toHaveBeenCalled();
    expect(fetchStub).not.toHaveBeenCalled();
    expect(window.location.href).not.toContain(TOKEN);
    expect(screen.queryByTestId(loginTestid.form)).toBeNull();
  });

  it("R3024000 the press goes on to the provider once, with the link's own token and type", () => {
    open("recovery");
    fireEvent.click(button());
    fireEvent.click(button());
    expect(followEmailLink).toHaveBeenCalledTimes(1);
    expect(followEmailLink).toHaveBeenCalledWith(TOKEN, "recovery");
    expect(button()).toBeDisabled();
  });

  it.each([
    ["signup", "Confirm your email", "Confirm my email"],
    ["recovery", "Reset your password", "Continue"],
    ["magiclink", "Sign in", "Sign in"],
    ["email_change", "Confirm your new email", "Confirm my email"],
  ])("R3024000 a %s link says what it is for", (type, title, label) => {
    open(type);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(title);
    expect(button()).toHaveTextContent(label);
  });

  it("R3024000 a site with no sign-in configured says so, and the button can be pressed again", () => {
    vi.mocked(followEmailLink).mockImplementation(() => {
      throw new AuthError("unconfigured");
    });
    open("signup");
    fireEvent.click(button());
    expect(screen.getByTestId(loginTestid.error)).toHaveTextContent(AUTH_MESSAGES.unconfigured);
    expect(button()).toBeEnabled();
  });

  it("R3024000 there is a way out to the sign-in form", () => {
    open("signup");
    fireEvent.click(screen.getByTestId(loginTestid.backToSignIn));
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "signIn");
    expect(screen.queryByTestId(loginTestid.pressLink)).toBeNull();
  });

  it("R3024000 a link with an unknown type is no link, and nothing waits", async () => {
    open("invite");
    await settle();
    expect(screen.queryByTestId(loginTestid.pressLink)).toBeNull();
    expect(window.location.href).not.toContain(TOKEN);
    expect(screen.getByTestId(loginTestid.form)).toHaveAttribute("data-mode", "signIn");
  });
});
