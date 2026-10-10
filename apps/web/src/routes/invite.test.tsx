// BUILD M8: §9.4/R145 refusal messages reach the DOM verbatim, preserving §9.8's oracle boundary.
// Format assertions read config rather than spelling values.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  CODE_ALPHABET,
  INVITE_CODE_GROUP_SIZE,
  INVITE_CODE_LENGTH,
  INVITE_CODE_SEPARATOR,
  REDEMPTION_IDENTICAL_ERROR,
} from "@jackioh/server-config";
import { codeFieldTestid, inviteTestid } from "../auth/testids.ts";
import { ApiRequestError, getCodeStatus, getMe, redeemCode } from "../net/api.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import InviteRoute, {
  INVITE_CODE_INPUT,
  INVITE_CODE_PLACEHOLDER,
  INVITE_ERROR,
  INVITE_NOT_NEEDED,
  INVITE_PAUSED,
  INVITE_SUBMIT,
  formatInviteCode,
} from "./invite.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return {
    ...actual,
    getMe: vi.fn(),
    getCodeStatus: vi.fn(),
    redeemCode: vi.fn(),
  };
});

const TOKEN = "e2e-token-pending";

/** A well-formed code over R104's alphabet, built rather than spelled. */
const GOOD_CODE = formatInviteCode(
  Array.from({ length: INVITE_CODE_LENGTH }, (_unused, index) =>
    CODE_ALPHABET[index % CODE_ALPHABET.length],
  ).join(""),
);

function meBody(status: "pending" | "active", needsInviteCode: boolean) {
  return {
    profile: { id: "p", status },
    needsInviteCode,
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
  };
}

beforeEach(() => {
  window.localStorage.clear();
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));
  window.history.replaceState(null, "", "/invite");
  vi.mocked(getMe).mockResolvedValue(meBody("pending", true));
  vi.mocked(getCodeStatus).mockResolvedValue({ redemptionEnabled: true, retryAfterMs: 0 });
  vi.mocked(redeemCode).mockReset();
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

async function mount(): Promise<void> {
  render(<InviteRoute />);
  await waitFor(() => {
    expect(getCodeStatus).toHaveBeenCalled();
  });
}

// §9.4 format from config

describe("the code format is §9.4's, read from crates/server/src/config.rs", () => {
  it("R104 normalises to upper case, and refuses an excluded character rather than dropping it", async () => {
    // R104 normalises lowercase `l` to `L`; R191 refuses ambiguous look-alikes rather than mapping them.
    // Refusal stops reading rather than dropping a character and shifting the submitted code.
    expect(formatInviteCode("abcd")).toBe("ABCD");
    expect(formatInviteCode("l")).toBe("L");
    for (const excluded of ["0", "O", "1", "I", "o", "i"]) {
      expect(
        CODE_ALPHABET.includes(excluded.toUpperCase()),
        `${excluded} is outside R104's alphabet`,
      ).toBe(false);
      expect(formatInviteCode(`AB${excluded}CD`), `${excluded} stops the reading`).toBe("AB");
    }
    expect(CODE_ALPHABET.includes("L"), "R104 keeps L; SPEC excludes only lowercase l").toBe(true);

    await mount();
    const input = screen.getByTestId(INVITE_CODE_INPUT);
    fireEvent.change(input, { target: { value: "ab" } });
    expect(input).toHaveValue("AB");
    expect(screen.queryByTestId(codeFieldTestid.hint)).toBeNull();

    fireEvent.change(input, { target: { value: "ab0" } });
    expect(input, "the excluded 0 is refused, not dropped and not kept").toHaveValue("AB");
    const hint = screen.getByTestId(codeFieldTestid.hint);
    expect(hint).toHaveAttribute("data-kind", "excluded");
    expect(hint.textContent).toContain("0");

    fireEvent.change(input, { target: { value: "abc" } });
    expect(input).toHaveValue("ABC");
    expect(screen.queryByTestId(codeFieldTestid.hint)).toBeNull();
  });

  it("groups the characters the way §9.4 formats them", () => {
    const bare = Array.from({ length: INVITE_CODE_LENGTH }, () => "A").join("");
    const formatted = formatInviteCode(bare);
    const groups = formatted.split(INVITE_CODE_SEPARATOR);
    expect(groups).toHaveLength(INVITE_CODE_LENGTH / INVITE_CODE_GROUP_SIZE);
    for (const group of groups) expect(group).toHaveLength(INVITE_CODE_GROUP_SIZE);
  });

  it("never accepts more than INVITE_CODE_LENGTH characters", () => {
    const tooLong = Array.from({ length: INVITE_CODE_LENGTH * 2 }, () => "A").join("");
    expect(formatInviteCode(tooLong).replaceAll(INVITE_CODE_SEPARATOR, "")).toHaveLength(
      INVITE_CODE_LENGTH,
    );
  });

  it("builds its placeholder from the constants rather than spelling the format", () => {
    expect(INVITE_CODE_PLACEHOLDER.split(INVITE_CODE_SEPARATOR)).toHaveLength(
      INVITE_CODE_LENGTH / INVITE_CODE_GROUP_SIZE,
    );
    expect(INVITE_CODE_PLACEHOLDER.replaceAll(INVITE_CODE_SEPARATOR, "")).toHaveLength(
      INVITE_CODE_LENGTH,
    );
  });
});

// Pending account, redemption open

describe("a pending account", () => {
  it("is shown the code screen", async () => {
    await mount();
    expect(screen.getByTestId(INVITE_CODE_INPUT)).toBeInTheDocument();
    expect(screen.getByTestId(INVITE_SUBMIT)).toBeInTheDocument();
    expect(screen.queryByTestId(INVITE_PAUSED)).toBeNull();
    expect(screen.queryByTestId(INVITE_NOT_NEEDED)).toBeNull();
    expect(window.location.pathname).toBe("/invite");
  });

  it("formats what is typed, and submits exactly what the box shows", async () => {
    vi.mocked(redeemCode).mockResolvedValue({ status: "active", needsInviteCode: false });
    await mount();

    const input = screen.getByTestId(INVITE_CODE_INPUT);
    fireEvent.change(input, { target: { value: GOOD_CODE.toLowerCase() } });
    expect(input).toHaveValue(GOOD_CODE);

    fireEvent.click(screen.getByTestId(INVITE_SUBMIT));
    await waitFor(() => {
      expect(redeemCode).toHaveBeenCalledWith(TOKEN, GOOD_CODE);
    });
  });

  it("says the code worked once it is redeemed, and offers the way on to the deckbuilder", async () => {
    vi.mocked(redeemCode).mockResolvedValue({ status: "active", needsInviteCode: false });
    await mount();
    fireEvent.change(screen.getByTestId(INVITE_CODE_INPUT), { target: { value: GOOD_CODE } });
    fireEvent.click(screen.getByTestId(INVITE_SUBMIT));

    // Not dropped silently onto an empty deckbuilder: the payoff is said, and takes focus.
    const done = await screen.findByTestId(inviteTestid.redeemed);
    expect(done.textContent).toMatch(/online play is open/);
    await waitFor(() => {
      expect(document.activeElement).toBe(done);
    });
    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe("You\u2019re in");
    expect(window.location.pathname).toBe("/invite");
    expect(screen.queryByTestId(INVITE_SUBMIT)).toBeNull();
    expect(screen.queryByTestId(inviteTestid.whereFrom)).toBeNull();

    const onward = screen.getByTestId(inviteTestid.goToDecks);
    expect(onward).toHaveClass("button-primary");
    fireEvent.click(onward);
    expect(window.location.pathname).toBe("/decks");
  });

  it("does not submit an empty box", async () => {
    await mount();
    expect(screen.getByTestId(INVITE_SUBMIT)).toBeDisabled();
    fireEvent.click(screen.getByTestId(INVITE_SUBMIT));
    expect(redeemCode).not.toHaveBeenCalled();
  });
});

// Verbatim refusal messages (§9.4, R145)

describe("a refusal is rendered exactly as the server wrote it", () => {
  async function submitAndFail(error: ApiRequestError): Promise<HTMLElement> {
    vi.mocked(redeemCode).mockRejectedValue(error);
    await mount();
    fireEvent.change(screen.getByTestId(INVITE_CODE_INPUT), { target: { value: GOOD_CODE } });
    fireEvent.click(screen.getByTestId(INVITE_SUBMIT));
    return waitFor(() => screen.getByTestId(INVITE_ERROR));
  }

  it("shows §9.4's one sentence for a code failure, character for character", async () => {
    const node = await submitAndFail(
      new ApiRequestError(400, { code: "invalid_code", message: REDEMPTION_IDENTICAL_ERROR }),
    );
    expect(node.textContent).toBe(REDEMPTION_IDENTICAL_ERROR);
  });

  it("shows an account-state refusal as its own sentence, not as the code one (R145)", async () => {
    // §9.4 code failures share one error; R145 requires a distinct already-active response.
    const distinct = "This account is already active.";
    const node = await submitAndFail(
      new ApiRequestError(409, { code: "already_active", message: distinct }),
    );
    expect(node.textContent).toBe(distinct);
    expect(node.textContent).not.toBe(REDEMPTION_IDENTICAL_ERROR);
  });

  it("shows a transport failure's own message rather than inventing one", async () => {
    const node = await submitAndFail(
      new ApiRequestError(503, { code: "unavailable", message: "service unavailable" }),
    );
    expect(node.textContent).toBe("service unavailable");
  });
});

// Redemption paused (§9.4 circuit breaker)

describe("when redemption is paused", () => {
  it("says so from GET /api/codes/status instead of guessing after a 503", async () => {
    vi.mocked(getCodeStatus).mockResolvedValue({ redemptionEnabled: false, retryAfterMs: 60_000 });
    await mount();
    await waitFor(() => {
      expect(screen.getByTestId(INVITE_PAUSED)).toBeInTheDocument();
    });
    expect(screen.getByTestId(INVITE_PAUSED)).toHaveAttribute("data-retry-after-ms", "60000");
    expect(screen.getByTestId(INVITE_SUBMIT)).toBeDisabled();
  });

  it("still takes a code when the status read itself failed", async () => {
    vi.mocked(getCodeStatus).mockRejectedValue(new Error("no status"));
    render(<InviteRoute />);
    await waitFor(() => {
      expect(screen.getByTestId(INVITE_CODE_INPUT)).toBeInTheDocument();
    });
    expect(screen.queryByTestId(INVITE_PAUSED)).toBeNull();
  });
});

// Active account

describe("an active account", () => {
  it("is told it needs no code, and the route stays reachable", async () => {
    vi.mocked(getMe).mockResolvedValue(meBody("active", false));
    await mount();
    await waitFor(() => {
      expect(screen.getByTestId(INVITE_NOT_NEEDED)).toBeInTheDocument();
    });
    expect(window.location.pathname).toBe("/invite");
  });
});

// No session

describe("with no session", () => {
  it("goes to the sign-in screen", async () => {
    window.localStorage.clear();
    render(<InviteRoute />);
    await waitFor(() => {
      expect(window.location.pathname).toBe("/login");
    });
    expect(getMe).not.toHaveBeenCalled();
  });
});
