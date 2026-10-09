// `auth/UsernamePrompt.tsx` and the field inside it (`auth/UsernameField.tsx`): the prompt an active
// account meets once after activation (R1435). The server judges every name; what is asserted here is
// that the client asks after a pause in the typing, shows the answer as it stands, saves exactly the
// username it showed, and reads the account again after a pick or a skip.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ApiRequestError, previewUsername, saveUsername, skipUsernamePrompt, type UsernamePreview } from "../net/api.ts";
import { announceAccountChange } from "../net/gate.ts";
import { usernameTestid } from "./testids.ts";
import { nextChangeSentence, USERNAME_CHANGED, USERNAME_SAVE_FAILED } from "./UsernameField.tsx";
import UsernamePrompt, { USERNAME_SKIP_FAILED } from "./UsernamePrompt.tsx";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, previewUsername: vi.fn(), saveUsername: vi.fn(), skipUsernamePrompt: vi.fn() };
});

vi.mock("../net/gate.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/gate.ts")>();
  return { ...actual, announceAccountChange: vi.fn() };
});

const TOKEN = "token-1";

/** The preview's debounce plus a stubbed round trip, retried against the DOM, never slept. */
const PREVIEW = { timeout: 3_000 } as const;

const TAKEN: UsernamePreview = { ok: true, base: "Max", username: "Max#3", tagged: true };

const SAVED = { name: "Max#3", nextChangeAt: 1_900_000_000_000, promptOwed: false };

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

function prompt(): void {
  render(<UsernamePrompt token={TOKEN} name="Player#7" />);
}

async function typeAndWait(text: string, expected: string): Promise<void> {
  await userEvent.type(screen.getByTestId(usernameTestid.input), text);
  await waitFor(() => {
    expect(screen.getByTestId(usernameTestid.preview).textContent).toBe(expected);
  }, PREVIEW);
}

describe("R1435 the username prompt", () => {
  it("R1435 asks the player to choose, and names the username they hold for now", () => {
    prompt();
    const root = screen.getByTestId(usernameTestid.prompt);
    expect(root).toHaveTextContent("Choose your username");
    expect(root).toHaveTextContent("You’re Player#7 for now.");
    expect(screen.getByTestId(usernameTestid.promptSkip)).toHaveTextContent("Skip for now");
    expect(screen.getByTestId(usernameTestid.save), "nothing to save before a preview").toBeDisabled();
  });

  it("R1435 previews once the typing pauses, for the whole name, and shows the tag it would get", async () => {
    vi.mocked(previewUsername).mockResolvedValue(TAKEN);
    prompt();

    await typeAndWait("Max", "Max is taken, so you’d be Max#3");
    // Debounced: one question for "Max", none for "M" or "Ma".
    expect(previewUsername).toHaveBeenCalledTimes(1);
    expect(previewUsername).toHaveBeenCalledWith(TOKEN, "Max", expect.any(AbortSignal));
    expect(screen.getByTestId(usernameTestid.save)).toBeEnabled();
  });

  it("R1435 an empty box asks nothing and shows no preview", async () => {
    vi.mocked(previewUsername).mockResolvedValue(TAKEN);
    prompt();

    await typeAndWait("Max", "Max is taken, so you’d be Max#3");
    await userEvent.clear(screen.getByTestId(usernameTestid.input));
    expect(screen.queryByTestId(usernameTestid.preview)).toBeNull();
    expect(screen.getByTestId(usernameTestid.save)).toBeDisabled();
    expect(previewUsername).toHaveBeenCalledTimes(1);
  });

  it("R1435 a new keystroke aborts the question before it", async () => {
    vi.mocked(previewUsername).mockReturnValue(new Promise<UsernamePreview>(() => {}));
    prompt();
    const input = screen.getByTestId(usernameTestid.input);

    fireEvent.change(input, { target: { value: "Ma" } });
    await waitFor(() => {
      expect(previewUsername).toHaveBeenCalledTimes(1);
    }, PREVIEW);
    const first = vi.mocked(previewUsername).mock.calls[0]?.[2];
    expect(first?.aborted).toBe(false);

    fireEvent.change(input, { target: { value: "Max" } });
    expect(first?.aborted).toBe(true);
  });

  it("R1435 shows the server's refusal as it wrote it, and Save stays off", async () => {
    vi.mocked(previewUsername).mockResolvedValue({
      ok: false,
      reason: "characters",
      message: "Use only letters, digits and underscores.",
    });
    prompt();

    await typeAndWait("Max!", "Use only letters, digits and underscores.");
    expect(screen.getByTestId(usernameTestid.save)).toBeDisabled();
    expect(screen.getByTestId(usernameTestid.input)).toHaveAttribute("aria-invalid", "true");
  });

  it("R1435 Save sends exactly the username the preview showed, then the account is read again", async () => {
    vi.mocked(previewUsername).mockResolvedValue(TAKEN);
    vi.mocked(saveUsername).mockResolvedValue(SAVED);
    prompt();

    await typeAndWait("Max", "Max is taken, so you’d be Max#3");
    await userEvent.click(screen.getByTestId(usernameTestid.save));

    await waitFor(() => {
      expect(announceAccountChange).toHaveBeenCalledTimes(1);
    });
    expect(saveUsername).toHaveBeenCalledWith(TOKEN, "Max#3");
    expect(skipUsernamePrompt).not.toHaveBeenCalled();
  });

  it("R1435 a 409 with a fresh preview replaces the one shown, says so, and the next Save takes the new tag", async () => {
    vi.mocked(previewUsername).mockResolvedValue(TAKEN);
    vi.mocked(saveUsername)
      .mockRejectedValueOnce(
        new ApiRequestError(409, {
          code: "conflict",
          message: "That name changed since you looked.",
          details: { ok: true, base: "Max", username: "Max#4", tagged: true },
        }),
      )
      .mockResolvedValueOnce({ ...SAVED, name: "Max#4" });
    prompt();

    await typeAndWait("Max", "Max is taken, so you’d be Max#3");
    await userEvent.click(screen.getByTestId(usernameTestid.save));

    await waitFor(() => {
      expect(screen.getByTestId(usernameTestid.preview).textContent).toBe("Max is taken, so you’d be Max#4");
    });
    expect(screen.getByTestId(usernameTestid.changed)).toHaveTextContent(USERNAME_CHANGED);
    expect(announceAccountChange, "nothing was saved").not.toHaveBeenCalled();

    await userEvent.click(screen.getByTestId(usernameTestid.save));
    await waitFor(() => {
      expect(announceAccountChange).toHaveBeenCalledTimes(1);
    });
    expect(vi.mocked(saveUsername).mock.calls.map((call) => call[1])).toEqual(["Max#3", "Max#4"]);
  });

  it("R1435 a 409 whose fresh preview refuses (the cooldown) shows that refusal with its end time and Save goes off", async () => {
    vi.mocked(previewUsername).mockResolvedValue(TAKEN);
    vi.mocked(saveUsername).mockRejectedValue(
      new ApiRequestError(409, {
        code: "conflict",
        message: "conflict",
        details: { ok: false, reason: "cooldown", message: "You changed your username recently.", nextChangeAt: 1_900_000_000_000 },
      }),
    );
    prompt();

    await typeAndWait("Max", "Max is taken, so you’d be Max#3");
    await userEvent.click(screen.getByTestId(usernameTestid.save));

    await waitFor(() => {
      expect(screen.getByTestId(usernameTestid.preview).textContent).toBe(
        `You changed your username recently. ${nextChangeSentence(1_900_000_000_000)}`,
      );
    });
    expect(screen.queryByTestId(usernameTestid.changed)).toBeNull();
    expect(screen.getByTestId(usernameTestid.save)).toBeDisabled();
  });

  it("R1435 a preview refused for the cooldown says when the next change is allowed", async () => {
    vi.mocked(previewUsername).mockResolvedValue({
      ok: false,
      reason: "cooldown",
      message: "You can change your username once every 24 hours.",
      nextChangeAt: 1_900_000_000_000,
    });
    prompt();

    await typeAndWait(
      "Max",
      `You can change your username once every 24 hours. ${nextChangeSentence(1_900_000_000_000)}`,
    );
    expect(screen.getByTestId(usernameTestid.preview)).toHaveTextContent(new Date(1_900_000_000_000).toLocaleString());
    expect(screen.getByTestId(usernameTestid.save)).toBeDisabled();
  });

  it("R1435 a save that fails with no preview says so in the field's own words", async () => {
    vi.mocked(previewUsername).mockResolvedValue(TAKEN);
    vi.mocked(saveUsername).mockRejectedValue(new Error("socket hang up"));
    prompt();

    await typeAndWait("Max", "Max is taken, so you’d be Max#3");
    await userEvent.click(screen.getByTestId(usernameTestid.save));

    expect(await screen.findByTestId(usernameTestid.error)).toHaveTextContent(USERNAME_SAVE_FAILED);
    expect(screen.getByTestId(usernameTestid.save), "the same name may be saved again").toBeEnabled();
  });

  it("R1435 Skip for now answers the prompt at the server, then the account is read again", async () => {
    vi.mocked(skipUsernamePrompt).mockResolvedValue({ name: "Player#7", nextChangeAt: null, promptOwed: false });
    prompt();

    await userEvent.click(screen.getByTestId(usernameTestid.promptSkip));

    await waitFor(() => {
      expect(announceAccountChange).toHaveBeenCalledTimes(1);
    });
    expect(skipUsernamePrompt).toHaveBeenCalledWith(TOKEN);
    expect(saveUsername).not.toHaveBeenCalled();
  });

  it("R1435 a skip that fails says so, and Skip can be pressed again", async () => {
    vi.mocked(skipUsernamePrompt).mockRejectedValue(new Error("offline"));
    prompt();

    await userEvent.click(screen.getByTestId(usernameTestid.promptSkip));

    expect(await screen.findByTestId(usernameTestid.promptError)).toHaveTextContent(USERNAME_SKIP_FAILED);
    expect(screen.getByTestId(usernameTestid.promptSkip)).toBeEnabled();
    expect(announceAccountChange).not.toHaveBeenCalled();
  });
});
