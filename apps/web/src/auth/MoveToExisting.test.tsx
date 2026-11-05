// R3147002: "Already have a JackiOh account?" removes the new account a provider made, so the provider can
// be linked to the account the player had. The component alone, over a stubbed `fetch`; the code screen
// and the consent screen that show it are `invite-feedback.test.tsx`'s and `ConsentPrompt.test.tsx`'s.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { paths } from "../net/navigate.ts";
import { SESSION_STORAGE_KEY, linkAfterSignIn, readSession } from "../net/session.ts";
import MoveToExisting, { moveTestid, signedUpWith } from "./MoveToExisting.tsx";

function base64url(text: string): string {
  return btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** A token whose account signed up with `provider`. */
function tokenFor(provider: string): string {
  const claims = { sub: "user-1", app_metadata: { provider, providers: [provider] }, user_metadata: {} };
  return `${base64url(JSON.stringify({ alg: "none" }))}.${base64url(JSON.stringify(claims))}.sig`;
}

const ROBLOX = tokenFor("custom:roblox");

function answer(status: number, body?: unknown) {
  return vi.fn((_url: string, _init?: RequestInit) =>
    Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      text: () => Promise.resolve(body === undefined ? "" : JSON.stringify(body)),
    } as unknown as Response),
  );
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", "https://project.supabase.co");
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "sb_publishable_test");
  vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google,custom:roblox");
  window.localStorage.clear();
  window.sessionStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("R3147002 a new account a provider made, removed to link the provider to an older one", () => {
  it("R3147002 is offered only to an account that signed up with a provider this build names", () => {
    expect(signedUpWith(ROBLOX)).toBe("custom:roblox");
    expect(signedUpWith(tokenFor("google"))).toBe("google");
    expect(signedUpWith(tokenFor("email"))).toBeNull();
    expect(signedUpWith(tokenFor("github"))).toBeNull();
    expect(signedUpWith("not-a-token")).toBeNull();

    render(<MoveToExisting token={tokenFor("email")} />);
    expect(screen.queryByTestId(moveTestid.root)).toBeNull();
  });

  it("R3147002 asks first, then deletes at the server, signs this device out and goes to sign in", async () => {
    const fetchMock = answer(204);
    vi.stubGlobal("fetch", fetchMock);
    const assign = vi.fn();
    vi.stubGlobal("location", { ...window.location, assign });
    window.localStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify({ accessToken: ROBLOX }));

    render(<MoveToExisting token={ROBLOX} />);
    fireEvent.click(screen.getByTestId(moveTestid.open));
    expect(screen.getByTestId(moveTestid.root)).toHaveTextContent("Signing in with Roblox made this new account.");
    expect(fetchMock, "asking sends nothing").not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(moveTestid.confirm));
    await waitFor(() => {
      expect(assign).toHaveBeenCalledWith(paths.login);
    });
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(String(url)).toMatch(/\/api\/account$/);
    expect(init?.method).toBe("DELETE");
    expect((init?.headers as Record<string, string>).authorization).toBe(`Bearer ${ROBLOX}`);
    expect(readSession(), "this device forgot the session").toBeNull();
    expect(linkAfterSignIn()).toBe("custom:roblox");
  });

  it("R3147002 Keep it sends nothing, and a refusal keeps the account and says why", async () => {
    const fetchMock = answer(409, { error: { code: "conflict", message: "Finish your series first." } });
    vi.stubGlobal("fetch", fetchMock);
    const assign = vi.fn();
    vi.stubGlobal("location", { ...window.location, assign });

    render(<MoveToExisting token={ROBLOX} />);
    fireEvent.click(screen.getByTestId(moveTestid.open));
    fireEvent.click(screen.getByTestId(moveTestid.keep));
    expect(screen.queryByTestId(moveTestid.confirm)).toBeNull();
    expect(fetchMock).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId(moveTestid.open));
    fireEvent.click(screen.getByTestId(moveTestid.confirm));
    expect(await screen.findByTestId(moveTestid.error)).toHaveTextContent("Finish your series first.");
    expect(assign).not.toHaveBeenCalled();
    expect(linkAfterSignIn()).toBeNull();
  });
});
