// R663: the account page's email change, against a stubbed auth provider.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES, AUTH_NOTICES } from "../net/auth.ts";
import { writeSession } from "../net/session.ts";
import ChangeEmail, { changeEmailTestid as T, tokenEmail } from "./ChangeEmail.tsx";

const URL_ = "https://project.supabase.co";

function base64url(text: string): string {
  return btoa(text).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
}

/** An unsigned token carrying `claims`: the component only reads them for display. */
function tokenWith(claims: unknown): string {
  return `${base64url(JSON.stringify({ alg: "none" }))}.${base64url(JSON.stringify(claims))}.sig`;
}

/** An unsigned token with an `email` claim. */
function tokenFor(email: string): string {
  return tokenWith({ sub: "u1", email });
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

/** `serve`, with another tab renewing the session to `renewed` while the first request is out (R194). */
function serveWhileRenewed(renewed: string, ...answers: Answer[]): ReturnType<typeof serve> {
  const calls = serve(...answers);
  const served = globalThis.fetch;
  vi.stubGlobal("fetch", (input: RequestInfo | URL, init?: RequestInit) => {
    if (calls.length === 0) writeSession({ accessToken: renewed });
    return served(input, init);
  });
  return calls;
}

/** A provider that never answers: the request stays out for the whole test. */
function serveNothing(): unknown[] {
  const calls: unknown[] = [];
  vi.stubGlobal("fetch", (input: unknown) => {
    calls.push(input);
    return new Promise<Response>(() => undefined);
  });
  return calls;
}

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "pk");
  window.localStorage.clear();
  window.sessionStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  window.localStorage.clear();
  window.sessionStorage.clear();
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
    const calls = serve({ status: 401, body: { error_code: "bad_jwt" } });
    render(<ChangeEmail />);
    submit("new@example.com");
    await waitFor(() => {
      expect(screen.getByTestId(T.error)).toHaveTextContent(AUTH_MESSAGES.sessionEnded);
    });
    // R194: with no renewal there is nothing to send again.
    expect(calls).toHaveLength(1);
  });

  it("R663 reads the token's email claim for display, and nothing from a token without one", () => {
    expect(tokenEmail(tokenFor("a@b.co"))).toBe("a@b.co");
    expect(tokenEmail("opaque")).toBeNull();
    expect(tokenEmail("a.%%%.c")).toBeNull();
  });

  it("R663 reads a claim whose base64url holds both - and _, and no empty or non-string email", () => {
    // This payload's base64url has a "-" and a "_", the two characters base64 writes as "+" and "/".
    expect(tokenFor("x~~~~?@ex.co").split(".")[1]).toMatch(/-.*_/u);
    expect(tokenEmail(tokenFor("x~~~~?@ex.co"))).toBe("x~~~~?@ex.co");
    expect(tokenEmail(tokenWith({ sub: "u1", email: "" }))).toBeNull();
    expect(tokenEmail(tokenWith({ sub: "u1", email: ["a@b.co"] }))).toBeNull();
    expect(tokenEmail(tokenWith({ sub: "u1" }))).toBeNull();
  });
});

describe("R663 the email form's states", () => {
  it("R663 starts idle: an empty field, nothing marked invalid, no sent line and no error", () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    render(<ChangeEmail />);
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-phase", "idle");
    const input = screen.getByLabelText("Change email") as HTMLInputElement;
    expect(input).toBe(screen.getByTestId(T.input));
    expect(input.value).toBe("");
    expect(input).toHaveAttribute("placeholder", "New email address");
    expect(input).not.toHaveAttribute("aria-invalid");
    expect(input).not.toHaveAttribute("aria-describedby");
    const button = screen.getByTestId(T.submit);
    expect(button).toHaveTextContent("Send confirmation link");
    expect(button).not.toBeDisabled();
    expect(screen.queryByTestId(T.sent)).toBeNull();
    expect(screen.queryByTestId(T.error)).toBeNull();
  });

  it("R3147001 adding an address words the label and the placeholder as an addition", () => {
    writeSession({ accessToken: tokenWith({ sub: "u1" }) });
    render(<ChangeEmail adding />);
    expect(screen.getByLabelText("Add an email address")).toHaveAttribute("placeholder", "Email address");
  });

  it("R663 shows no signed-in line for a token with no email claim", () => {
    writeSession({ accessToken: tokenWith({ sub: "u1" }) });
    render(<ChangeEmail />);
    expect(screen.getByTestId(T.root)).not.toHaveTextContent("Signed in as");
  });

  it("R663 never lets the page navigate: its submit is cancelled", () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    render(<ChangeEmail />);
    expect(fireEvent.submit(screen.getByTestId(T.root))).toBe(false);
  });

  it("R663 while the link is on its way the button reads Sending… and is disabled, and a second submit sends nothing", async () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    const calls = serveNothing();
    render(<ChangeEmail />);
    submit("new@example.com");
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-phase", "sending");
    const button = screen.getByTestId(T.submit);
    expect(button).toHaveTextContent("Sending…");
    expect(button).toBeDisabled();
    fireEvent.submit(screen.getByTestId(T.root));
    await waitFor(() => {
      expect(calls).toHaveLength(1);
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    expect(calls).toHaveLength(1);
  });

  it("R663 an error marks the field invalid and describes it, and typing again clears it", async () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    render(<ChangeEmail />);
    submit("not-an-address");
    const error = await screen.findByTestId(T.error);
    const input = screen.getByTestId(T.input);
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAttribute("aria-describedby", T.error);
    expect(input).toHaveAccessibleDescription(error.textContent ?? "");
    fireEvent.change(input, { target: { value: "not-an-address!" } });
    expect(screen.queryByTestId(T.error)).toBeNull();
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-phase", "idle");
    expect(input).not.toHaveAttribute("aria-invalid");
  });

  it("R663 typing after the link is sent keeps the sent line", async () => {
    writeSession({ accessToken: tokenFor("me@example.com") });
    serve({ status: 200, body: { id: "u1" } });
    render(<ChangeEmail />);
    submit("new@example.com");
    await screen.findByTestId(T.sent);
    fireEvent.change(screen.getByTestId(T.input), { target: { value: "n" } });
    expect(screen.getByTestId(T.sent)).toHaveTextContent(AUTH_NOTICES.emailChangeSent);
    expect(screen.getByTestId(T.root)).toHaveAttribute("data-phase", "sent");
  });
});

describe("R194 an expired token is renewed once", () => {
  it("R194 a token refused as expired, renewed meanwhile by another tab, is sent again once with the renewed token", async () => {
    const token = tokenFor("me@example.com");
    const renewed = `${token}-renewed`;
    writeSession({ accessToken: token });
    const calls = serveWhileRenewed(
      renewed,
      { status: 401, body: { error_code: "bad_jwt" } },
      { status: 200, body: { id: "u1" } },
    );
    render(<ChangeEmail />);
    submit("new@example.com");
    expect(await screen.findByTestId(T.sent)).toHaveTextContent(AUTH_NOTICES.emailChangeSent);
    expect(calls.map((call) => call.auth)).toEqual([`Bearer ${token}`, `Bearer ${renewed}`]);
    expect(calls[1]?.body).toMatchObject({ email: "new@example.com" });
  });

  it("R194 a refusal that is not an expired token is never sent again, even with a renewed session waiting", async () => {
    const token = tokenFor("me@example.com");
    writeSession({ accessToken: token });
    const calls = serveWhileRenewed(
      `${token}-renewed`,
      { status: 422, body: { error_code: "email_exists" } },
      { status: 200, body: { id: "u1" } },
    );
    render(<ChangeEmail />);
    submit("taken@example.com");
    expect(await screen.findByTestId(T.error)).toHaveTextContent(AUTH_MESSAGES.emailChangeRefused);
    expect(calls).toHaveLength(1);
  });
});
