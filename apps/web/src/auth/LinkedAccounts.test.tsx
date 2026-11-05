// R3025000: `/account`'s other ways to sign in, against a stubbed `fetch`.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES, startIdentityLink } from "../net/auth.ts";
import { settle } from "../test/settle.ts";
import LinkedAccounts, { linkedTestid } from "./LinkedAccounts.tsx";
import { holdLinkResult } from "./redirect.ts";
import { linkAfterSignIn, rememberLinkAfterSignIn } from "../net/session.ts";

vi.mock("../net/auth.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../net/auth.ts")>()),
  startIdentityLink: vi.fn(() => Promise.resolve()),
}));

const TOKEN = "access-token";

type Call = { url: URL; method: string };

function serve(identities: { identity_id: string; provider: string }[], deleteStatus = 200): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = new URL(String(input));
      const method = (init?.method ?? "GET").toUpperCase();
      calls.push({ url, method });
      if (method === "DELETE") {
        const body = deleteStatus === 200 ? {} : { error_code: "single_identity_not_deletable" };
        return Promise.resolve(new Response(JSON.stringify(body), { status: deleteStatus }));
      }
      return Promise.resolve(new Response(JSON.stringify({ identities }), { status: 200 }));
    }),
  );
  return calls;
}

function row(provider: string): HTMLElement {
  const found = screen.getAllByTestId(linkedTestid.row).find((element) => element.getAttribute("data-provider") === provider);
  if (found === undefined) throw new Error(`no row for ${provider}`);
  return found;
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", "https://project.supabase.co");
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "sb_publishable_test");
  vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google");
  vi.mocked(startIdentityLink).mockClear();
  window.sessionStorage.clear();
  window.localStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("R3025000 linking on the account screen", () => {
  it("R3025000 shows nothing on a build that names no provider", () => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "");
    const calls = serve([]);
    render(<LinkedAccounts token={TOKEN} />);
    expect(screen.queryByTestId(linkedTestid.section)).toBeNull();
    expect(calls).toHaveLength(0);
  });

  it("R3025000 a provider not linked offers Link, which leaves for it with this session", async () => {
    serve([{ identity_id: "id-email", provider: "email" }]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(row("google")).toHaveAttribute("data-linked", "false");
    fireEvent.click(screen.getByTestId(linkedTestid.link));
    expect(startIdentityLink).toHaveBeenCalledWith("google", TOKEN);
  });

  it("R3025000 a linked provider offers Unlink, which removes it and reads the list again", async () => {
    const calls = serve([
      { identity_id: "id-email", provider: "email" },
      { identity_id: "id-google", provider: "google" },
    ]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(row("google")).toHaveAttribute("data-linked", "true");
    fireEvent.click(screen.getByTestId(linkedTestid.unlink));
    await waitFor(() => {
      expect(screen.getByTestId(linkedTestid.notice)).toBeInTheDocument();
    });
    expect(calls.find((call) => call.method === "DELETE")?.url.pathname).toBe("/auth/v1/user/identities/id-google");
    // The list is read again by the effect after the render that shows the notice, so it can come a tick later.
    await waitFor(() => {
      expect(calls.filter((call) => call.method === "GET")).toHaveLength(2);
    });
  });

  it("R3025000 the account's only way in is never offered for removal", async () => {
    serve([{ identity_id: "id-google", provider: "google" }]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(row("google")).toHaveAttribute("data-linked", "true");
    expect(screen.queryByTestId(linkedTestid.unlink)).toBeNull();
  });

  it("R3025000 says how a link that went to the provider ended, once", async () => {
    serve([{ identity_id: "id-email", provider: "email" }]);
    holdLinkResult("failed");
    render(<LinkedAccounts token={TOKEN} />);
    expect(screen.getByTestId(linkedTestid.error)).toHaveTextContent(AUTH_MESSAGES.linkRefused);
    cleanup();

    holdLinkResult("linked");
    render(<LinkedAccounts token={TOKEN} />);
    expect(screen.getByTestId(linkedTestid.notice)).toBeInTheDocument();
    await settle();
    cleanup();

    render(<LinkedAccounts token={TOKEN} />);
    expect(screen.queryByTestId(linkedTestid.notice)).toBeNull();
    expect(screen.queryByTestId(linkedTestid.error)).toBeNull();
  });
});

describe("R3147000 Roblox among the ways to sign in", () => {
  it("R3147000 lists Roblox, linked by its custom provider's name, and links it with this session", async () => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google,custom:roblox");
    serve([
      { identity_id: "id-email", provider: "email" },
      { identity_id: "id-roblox", provider: "custom:roblox" },
    ]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(row("custom:roblox")).toHaveAttribute("data-linked", "true");
    expect(row("custom:roblox")).toHaveTextContent("Roblox: linked. You can sign in with it.");
    expect(row("google")).toHaveAttribute("data-linked", "false");
    cleanup();

    serve([{ identity_id: "id-email", provider: "email" }]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    const link = screen.getAllByTestId(linkedTestid.link).find((button) => button.getAttribute("data-provider") === "custom:roblox");
    expect(link).toHaveTextContent("Link Roblox");
    fireEvent.click(link as HTMLElement);
    expect(startIdentityLink).toHaveBeenCalledWith("custom:roblox", TOKEN);
  });
});

describe("R3147002 linking the provider of a removed new account", () => {
  it("R3147002 says once which provider to link to finish, until it is linked", async () => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google,custom:roblox");
    rememberLinkAfterSignIn("custom:roblox");
    serve([{ identity_id: "id-email", provider: "email" }]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(screen.getByTestId(linkedTestid.moving)).toHaveTextContent("Link Roblox below to finish");
    expect(linkAfterSignIn()).toBeNull();
    cleanup();

    // Said once: a reload of `/account` does not say it again.
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(screen.queryByTestId(linkedTestid.moving)).toBeNull();
  });

  it("R3147002 says nothing once the provider is linked here already", async () => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google,custom:roblox");
    rememberLinkAfterSignIn("custom:roblox");
    serve([
      { identity_id: "id-email", provider: "email" },
      { identity_id: "id-roblox", provider: "custom:roblox" },
    ]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(screen.queryByTestId(linkedTestid.moving)).toBeNull();
  });
});
