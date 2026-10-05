// R663: the Account tab's email change, against a stubbed auth provider.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES, AUTH_NOTICES } from "../net/auth.ts";
import { writeSession } from "../net/session.ts";
import ChangeEmail, { changeEmailTestid as T, tokenEmail } from "./ChangeEmail.tsx";

const URL_ = "https://project.supabase.co";

function base64url(text: string): string {
  return btoa(text).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
}

/** An unsigned token with an `email` claim: the component only reads it for display. */
function tokenFor(email: string): string {
  return `${base64url(JSON.stringify({ alg: "none" }))}.${base64url(JSON.stringify({ sub: "u1", email }))}.sig`;
}

type Answer = { status: number; body: unknown };

function serve(...answers: Answer[]): { url: string; method: string; body: unknown; auth: string | null }[] {
  const calls: { url: string; method: string; body: unknown; auth: string | null }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const headers = new Headers(init?.headers);
      calls.push({
        url: String(input),
        method: init?.method ?? "GET",
        body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
        auth: headers.get("authorization"),
      });
      const answer = answers[Math.min(calls.length - 1, answers.length - 1)] ?? { status: 200, body: {} };
      return Promise.resolve(new Response(JSON.stringify(answer.body), { status: answer.status }));
    }),
  );
  return calls;
}

function submit(address: string): void {
  fireEvent.change(screen.getByTestId(T.input), { target: { value: address } });
  fireEvent.click(screen.getByTestId(T.submit));
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "pk");
  window.localStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  window.localStorage.clear();
});

describe("R663 changing the account's email from the settings", () => {
  it("R663 is not offered signed out, or on a build with no auth provider", () => {
    render(<ChangeEmail />);
    expect(screen.queryByTestId(T.root)).toBeNull();
    cleanup();

    writeSession({ accessToken: tokenFor("me@example.com") });
    vi.stubEnv("VITE_SUPABASE_URL", "");
    render(<ChangeEmail />);
    expect(screen.queryByTestId(T.root)).toBeNull();
  });

  it("R663 sends the new address to the provider with the session's token, and says to open the link", async () => {
    const token = tokenFor("me@example.com");
    writeSession({ accessToken: token });
    const calls = serve({ status: 200, body: { id: "u1" } });
    render(<ChangeEmail />);
    expect(screen.getByTestId(T.root)).toHaveTextContent("Signed in as me@example.com");

    submit("  new@example.com ");
    expect(await screen.findByTestId(T.sent)).toHaveTextContent(AUTH_NOTICES.emailChangeSent);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.method).toBe("PUT");
    expect(calls[0]?.url.startsWith(`${URL_}/auth/v1/user?redirect_to=`)).toBe(true);
    expect(calls[0]?.body).toMatchObject({ email: "new@example.com" });
    expect(calls[0]?.auth).toBe(`Bearer ${token}`);
    expect((screen.getByTestId(T.input) as HTMLInputElement).value).toBe("");
  });

  it("R663 checks the address before it sends anything, and refuses the account's own", async () => {
    writeSession({ accessToken: tokenFor("Me@Example.com") });
    const calls = serve({ status: 200, body: {} });
    render(<ChangeEmail />);

    submit("not-an-address");
    expect(await screen.findByTestId(T.error)).toHaveTextContent("Enter a valid email address");
    submit("me@example.com");
    expect(await screen.findByTestId(T.error)).toHaveTextContent(AUTH_MESSAGES.sameEmail);
    expect(calls).toHaveLength(0);
  });

  it("R663 R160 an address another account holds reads as any unusable address", async () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    serve({ status: 422, body: { error_code: "email_exists", msg: "A user with this email address has already been registered" } });
    render(<ChangeEmail />);
    submit("taken@example.com");
    const error = await screen.findByTestId(T.error);
    expect(error).toHaveTextContent(AUTH_MESSAGES.emailChangeRefused);
    expect(error).not.toHaveTextContent("registered");
  });

  it("R663 a token the provider no longer takes, with nothing to renew it, says the session ended", async () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    serve({ status: 401, body: { error_code: "bad_jwt" } });
    render(<ChangeEmail />);
    submit("new@example.com");
    await waitFor(() => {
      expect(screen.getByTestId(T.error)).toHaveTextContent(AUTH_MESSAGES.sessionEnded);
    });
  });

  it("R663 reads the token's email claim for display, and nothing from a token without one", () => {
    expect(tokenEmail(tokenFor("a@b.co"))).toBe("a@b.co");
    expect(tokenEmail("opaque")).toBeNull();
    expect(tokenEmail("a.%%%.c")).toBeNull();
  });
});
