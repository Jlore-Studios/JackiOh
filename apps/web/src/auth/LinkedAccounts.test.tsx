// R3025000: `/account`'s other ways to sign in, against a stubbed `fetch`.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES, startIdentityLink } from "../net/auth.ts";
import { skeletonTestid } from "../skeleton/Skeleton.tsx";
import { settle } from "../test/settle.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import LinkedAccounts, { linkedTestid } from "./LinkedAccounts.tsx";
import { holdLinkResult } from "./redirect.ts";
import { linkAfterSignIn, rememberLinkAfterSignIn } from "../net/session.ts";

vi.mock("../net/auth.ts", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../net/auth.ts")>()),
  startIdentityLink: vi.fn(() => Promise.resolve()),
}));

const TOKEN = "access-token";

type Call = { url: URL; method: string };

function serve(identities: { identity_id: string; provider: string; identity_data?: Record<string, unknown> }[], deleteStatus = 200): Call[] {
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

  it("R3228000 the list waits as a skeleton of rows whose sentence stays a status line, and the rows replace it", async () => {
    serve([{ identity_id: "id-email", provider: "email" }]);
    render(<LinkedAccounts token={TOKEN} />);
    expectSkeleton(screen.getByText("Loading…"), "rows");
    await settle();
    expect(screen.queryByText("Loading…")).toBeNull();
    expect(screen.queryByTestId(skeletonTestid.root)).toBeNull();
    expect(row("google")).toHaveAttribute("data-linked", "false");
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

describe("R3507000 the linked Roblox account's username and headshot", () => {
  const PICTURE = "https://tr.rbxcdn.com/30DAY-AvatarHeadshot-ABC-Png/150/150/AvatarHeadshot/Png/noFilter";
  const EMAIL = { identity_id: "id-email", provider: "email" };

  /** The Roblox row of a list holding the email identity and a Roblox one with `identity_data` `data`. */
  async function robloxRow(data: Record<string, unknown> | undefined): Promise<HTMLElement> {
    serve([EMAIL, { identity_id: "id-roblox", provider: "custom:roblox", identity_data: data }]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    return row("custom:roblox");
  }

  beforeEach(() => {
    vi.stubEnv("VITE_AUTH_OAUTH_PROVIDERS", "google,custom:roblox");
  });

  it("R3507000 shows the username and a lazy headshot with no alt text, at the one fixed size, beside the unchanged linked sentence", async () => {
    const roblox = await robloxRow({ preferred_username: "Builderman", name: "Builderman", picture: PICTURE });
    expect(roblox).toHaveTextContent("Roblox: linked. You can sign in with it.");
    expect(within(roblox).getByTestId(linkedTestid.identity)).toHaveTextContent(/^@Builderman$/u);
    const image = within(roblox).getByTestId(linkedTestid.avatar);
    expect(image.tagName).toBe("IMG");
    expect(image).toHaveAttribute("src", PICTURE);
    expect(image).toHaveAttribute("alt", "");
    expect(image).toHaveAttribute("loading", "lazy");
    expect(image).toHaveAttribute("width", "32");
    expect(image).toHaveAttribute("height", "32");
    // Google's row, unlinked, shows neither.
    expect(within(row("google")).queryByTestId(linkedTestid.identity)).toBeNull();
    expect(within(row("google")).queryByTestId(linkedTestid.avatar)).toBeNull();
  });

  it("R3507000 names the display name beside the username only when it differs", async () => {
    const roblox = await robloxRow({ preferred_username: "Builderman", name: "Builder Man", picture: PICTURE });
    expect(within(roblox).getByTestId(linkedTestid.identity)).toHaveTextContent(/^Builder Man \(@Builderman\)$/u);
  });

  it("R3507000 shows the username and no image when Roblox gave no usable headshot", async () => {
    for (const picture of [undefined, null, "", "https://example.com/headshot.png", "http://tr.rbxcdn.com/headshot.png"]) {
      const roblox = await robloxRow({ preferred_username: "Builderman", picture });
      expect(within(roblox).getByTestId(linkedTestid.identity), String(picture)).toHaveTextContent("@Builderman");
      expect(within(roblox).queryByTestId(linkedTestid.avatar), String(picture)).toBeNull();
      expect(roblox.querySelector("img"), String(picture)).toBeNull();
      cleanup();
    }
  });

  it("R3507000 keeps today's row for a Roblox identity with no username, a headshot alone included", async () => {
    for (const data of [undefined, {}, { name: "Builder Man" }, { picture: PICTURE }, { preferred_username: " " }]) {
      const roblox = await robloxRow(data);
      expect(roblox, JSON.stringify(data)).toHaveTextContent(/^Roblox: linked\. You can sign in with it\.Unlink Roblox$/u);
      expect(within(roblox).queryByTestId(linkedTestid.identity), JSON.stringify(data)).toBeNull();
      expect(roblox.querySelector("img"), JSON.stringify(data)).toBeNull();
      cleanup();
    }
  });

  it("R3507000 shows neither on a Roblox row that is not linked", async () => {
    serve([EMAIL]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(row("custom:roblox")).toHaveAttribute("data-linked", "false");
    expect(row("custom:roblox")).toHaveTextContent("Roblox: not linked.");
    expect(within(row("custom:roblox")).queryByTestId(linkedTestid.identity)).toBeNull();
    expect(within(row("custom:roblox")).queryByTestId(linkedTestid.avatar)).toBeNull();
  });

  it("R3507000 shows a linked Google identity as before: no username or image from its identity_data", async () => {
    serve([EMAIL, { identity_id: "id-google", provider: "google", identity_data: { preferred_username: "player", picture: PICTURE } }]);
    render(<LinkedAccounts token={TOKEN} />);
    await settle();
    expect(row("google")).toHaveAttribute("data-linked", "true");
    expect(within(row("google")).queryByTestId(linkedTestid.identity)).toBeNull();
    expect(row("google").querySelector("img")).toBeNull();
  });

  it("R3507000 removes a headshot that fails to load and keeps the username, leaving no broken image", async () => {
    const roblox = await robloxRow({ preferred_username: "Builderman", picture: PICTURE });
    fireEvent.error(within(roblox).getByTestId(linkedTestid.avatar));
    expect(within(roblox).queryByTestId(linkedTestid.avatar)).toBeNull();
    expect(roblox.querySelector("img")).toBeNull();
    expect(within(roblox).getByTestId(linkedTestid.identity)).toHaveTextContent(/^@Builderman$/u);
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
