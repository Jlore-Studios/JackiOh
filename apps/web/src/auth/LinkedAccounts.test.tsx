// R3025000: `/account`'s other ways to sign in, against a stubbed `fetch`.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES, startIdentityLink } from "../net/auth.ts";
import { settle } from "../test/settle.ts";
import LinkedAccounts, { linkedTestid } from "./LinkedAccounts.tsx";
import { holdLinkResult } from "./redirect.ts";

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
    expect(calls.filter((call) => call.method === "GET")).toHaveLength(2);
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
