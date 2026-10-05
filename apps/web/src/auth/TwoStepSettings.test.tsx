// `/account`'s two-step sign-in (R659): enrol an authenticator app, confirm it with a code (which
// raises this session to aal2 and stores it), and remove it. The provider is a stubbed `fetch`.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { AUTH_MESSAGES, AUTH_NOTICES } from "../net/auth.ts";
import { readSession, writeSession } from "../net/session.ts";
import TwoStepSettings, { twoStepTestid } from "./TwoStepSettings.tsx";

const URL_ = "https://project.supabase.co";
const TOKEN = "access-aal1";
const SVG = '<svg xmlns="http://www.w3.org/2000/svg"><rect width="1" height="1"/></svg>';

type Call = { method: string; path: string; body: unknown; auth: string | null };
type Answer = { status: number; body?: unknown };

function serve(answers: Record<string, Answer | ((call: Call) => Answer)>): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = new URL(String(input));
      const call: Call = {
        method: (init?.method ?? "GET").toUpperCase(),
        path: url.pathname,
        body: typeof init?.body === "string" ? (JSON.parse(init.body) as unknown) : undefined,
        auth: new Headers(init?.headers).get("authorization"),
      };
      calls.push(call);
      const entry = answers[`${call.method} ${call.path}`];
      const answer = typeof entry === "function" ? entry(call) : (entry ?? { status: 404, body: {} });
      return Promise.resolve(
        new Response(answer.body === undefined ? null : JSON.stringify(answer.body), {
          status: answer.status,
          headers: { "content-type": "application/json" },
        }),
      );
    }),
  );
  return calls;
}

function factors(list: { id: string; status: string }[]): Answer {
  return { status: 200, body: { id: "user-1", factors: list.map((factor) => ({ ...factor, factor_type: "totp" })) } };
}

const ENROLLED: Answer = {
  status: 200,
  body: { id: "f-new", type: "totp", totp: { qr_code: `data:image/svg+xml;utf-8,${SVG}`, secret: "JBSWY3DPEHPK3PXP", uri: "otpauth://totp/x" } },
};

beforeEach(() => {
  vi.stubEnv("VITE_SUPABASE_URL", URL_);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", "sb_publishable_test");
  window.sessionStorage.clear();
  window.localStorage.clear();
  writeSession({ accessToken: TOKEN, refreshToken: "refresh-1", expiresAt: Date.now() + 3_600_000 });
});

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("R659 turning two-step sign-in on and off", () => {
  it("R659 shows nothing on a build with no auth provider", () => {
    vi.stubEnv("VITE_SUPABASE_URL", "");
    const calls = serve({});
    render(<TwoStepSettings token={TOKEN} />);
    expect(screen.queryByTestId(twoStepTestid.section)).toBeNull();
    expect(calls).toHaveLength(0);
  });

  it("R659 enrols, and only a code from the app turns it on, storing the raised session", async () => {
    let listed = 0;
    const calls = serve({
      "GET /auth/v1/user": () => {
        listed += 1;
        return factors([]);
      },
      "POST /auth/v1/factors": ENROLLED,
      "POST /auth/v1/factors/f-new/challenge": { status: 200, body: { id: "challenge-1" } },
      "POST /auth/v1/factors/f-new/verify": { status: 200, body: { access_token: "access-aal2", refresh_token: "refresh-2", expires_in: 3600 } },
    });
    render(<TwoStepSettings token={TOKEN} />);
    expect((await screen.findByTestId(twoStepTestid.state)).getAttribute("data-state")).toBe("off");

    fireEvent.click(screen.getByTestId(twoStepTestid.start));
    const qr = await screen.findByTestId(twoStepTestid.qr);
    expect(qr.getAttribute("src")).toBe(`data:image/svg+xml;charset=utf-8,${encodeURIComponent(SVG)}`);
    expect(screen.getByTestId(twoStepTestid.secret).textContent).toBe("JBSWY3DPEHPK3PXP");
    // Not on yet: the factor is unverified until a code is typed.
    expect(readSession()?.accessToken).toBe(TOKEN);

    fireEvent.change(screen.getByTestId(twoStepTestid.code), { target: { value: "123456" } });
    fireEvent.click(screen.getByTestId(twoStepTestid.confirm));
    expect((await screen.findByTestId(twoStepTestid.notice)).textContent).toBe(AUTH_NOTICES.mfaEnrolled);
    expect(screen.getByTestId(twoStepTestid.state).getAttribute("data-state")).toBe("on");
    // The same session, raised: written over the stored one, never revoked.
    expect(readSession()).toMatchObject({ accessToken: "access-aal2", refreshToken: "refresh-2" });
    expect(calls.some((call) => call.path === "/auth/v1/logout")).toBe(false);
    expect(calls.find((call) => call.path === "/auth/v1/factors/f-new/verify")?.body).toEqual({
      challenge_id: "challenge-1",
      code: "123456",
    });
    expect(listed).toBeGreaterThan(0);
  });

  it("R659 a wrong code leaves it off and says so", async () => {
    serve({
      "GET /auth/v1/user": factors([]),
      "POST /auth/v1/factors": ENROLLED,
      "POST /auth/v1/factors/f-new/challenge": { status: 200, body: { id: "challenge-1" } },
      "POST /auth/v1/factors/f-new/verify": { status: 422, body: { error_code: "mfa_verification_failed" } },
    });
    render(<TwoStepSettings token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(twoStepTestid.start));
    fireEvent.change(await screen.findByTestId(twoStepTestid.code), { target: { value: "000000" } });
    fireEvent.click(screen.getByTestId(twoStepTestid.confirm));
    expect((await screen.findByTestId(twoStepTestid.error)).textContent).toBe(AUTH_MESSAGES.mfaCodeInvalid);
    expect(screen.getByTestId(twoStepTestid.code)).toBeTruthy();
    expect(readSession()?.accessToken).toBe(TOKEN);
  });

  it("R659 cancelling an enrolment removes the unverified factor", async () => {
    const calls = serve({
      "GET /auth/v1/user": factors([]),
      "POST /auth/v1/factors": ENROLLED,
      "DELETE /auth/v1/factors/f-new": { status: 200, body: { id: "f-new" } },
    });
    render(<TwoStepSettings token={TOKEN} />);
    fireEvent.click(await screen.findByTestId(twoStepTestid.start));
    fireEvent.click(await screen.findByTestId(twoStepTestid.cancel));
    expect(screen.getByTestId(twoStepTestid.state).getAttribute("data-state")).toBe("off");
    await waitFor(() => {
      expect(calls.some((call) => call.method === "DELETE" && call.path === "/auth/v1/factors/f-new")).toBe(true);
    });
  });

  it("R659 an account with a verified factor shows it on, and turning it off asks first", async () => {
    const calls = serve({
      "GET /auth/v1/user": factors([{ id: "f-app", status: "verified" }]),
      "DELETE /auth/v1/factors/f-app": { status: 200, body: { id: "f-app" } },
    });
    render(<TwoStepSettings token={TOKEN} />);
    expect((await screen.findByTestId(twoStepTestid.state)).getAttribute("data-state")).toBe("on");

    fireEvent.click(screen.getByTestId(twoStepTestid.remove));
    expect(calls.some((call) => call.method === "DELETE")).toBe(false);
    fireEvent.click(screen.getByTestId(twoStepTestid.removeConfirm));
    expect((await screen.findByTestId(twoStepTestid.notice)).textContent).toBe(AUTH_NOTICES.mfaRemoved);
    expect(screen.getByTestId(twoStepTestid.state).getAttribute("data-state")).toBe("off");
    expect(calls.find((call) => call.method === "DELETE")?.auth).toBe(`Bearer ${TOKEN}`);
  });

  it("R659 factors that cannot be read offer nothing rather than the wrong thing", async () => {
    serve({ "GET /auth/v1/user": { status: 500, body: {} } });
    render(<TwoStepSettings token={TOKEN} />);
    expect((await screen.findByTestId(twoStepTestid.state)).getAttribute("data-state")).toBe("unknown");
    expect(screen.queryByTestId(twoStepTestid.start)).toBeNull();
    expect(screen.queryByTestId(twoStepTestid.remove)).toBeNull();
  });
});
