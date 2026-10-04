// The emote UI (ui.tsx): what a shown emote draws (R644) and what the two menus do (issue §2, §5,
// with R643's grey-out). The components are cosmetic — a pick reports the emote upward and the
// session decides whether anything shows — so the tests render them bare, against a hand-read
// `gate`, with no session and no clock but the bubble's `--emote-hold` span.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EMOJI_EMOTE_IDS, VOICE_EMOTE_IDS, type EmoteGate } from "@jackioh/shared";

import { EMOJI_LABEL } from "./EmojiArt.tsx";
import { EmoteMenu, EmoteShow, MuteMenu } from "./ui.tsx";
import type { EmoteShow as EmoteShowState } from "./session.ts";

const noop = (): void => undefined;

const OPEN: EmoteGate = { ok: true, sentAt: [] };
const LIMITED: EmoteGate = { ok: false, retryAfterMs: 2400, sentAt: [0] };

afterEach(() => {
  cleanup();
});

describe("R644 EmoteShow", () => {
  it("R644 a voice emote draws its line's text in a speech bubble", () => {
    const show: EmoteShowState = { key: 1, emote: "greetings", text: "Hey there, friend! Feeling lucky?", until: Date.now() + 3_000 };
    render(<EmoteShow show={show} />);

    const bubble = screen.getByTestId("emote-bubble");
    expect(bubble).toHaveClass("emote-bubble");
    expect(bubble).toHaveTextContent("Hey there, friend! Feeling lucky?");
    expect(bubble).toHaveAttribute("role", "status");
    // A voice show is a bubble, not a sticker: no emoji marker, and its span rides --emote-hold.
    expect(bubble).not.toHaveAttribute("data-emoji");
    expect(bubble.style.getPropertyValue("--emote-hold")).toMatch(/^\d+ms$/);
  });

  it("R644 an emoji emote draws its sticker — the SVG for that id, and no words", () => {
    const show: EmoteShowState = { key: 2, emote: "wahWah", text: null, until: Date.now() + 2_000 };
    render(<EmoteShow show={show} />);

    const sticker = screen.getByTestId("emote-bubble");
    expect(sticker).toHaveClass("emote-sticker");
    expect(sticker).toHaveAttribute("data-emoji", "wahWah");
    expect(sticker.querySelector("svg.emote-emoji-svg")).not.toBeNull();
    expect(sticker).not.toHaveTextContent(/\w/);
  });
});

describe("R643 the emote picker", () => {
  it("R643 offers the five voice lines on their arc and the five emoji below, ten items", () => {
    render(<EmoteMenu side="you" gate={() => OPEN} onPick={noop} onClose={noop} />);

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(VOICE_EMOTE_IDS.length + EMOJI_EMOTE_IDS.length);
    for (const emote of [...VOICE_EMOTE_IDS, ...EMOJI_EMOTE_IDS]) {
      expect(screen.getByTestId(`emote-${emote}`)).toBe(items.find((item) => item.dataset.testid === `emote-${emote}`));
    }
    // The voice items are labelled by name; the emoji carry their title/aria label and a sticker.
    for (const label of ["Greetings", "Well Played", "Oops", "Thanks", "Threaten"]) {
      expect(screen.getByRole("menuitem", { name: label })).toBeInTheDocument();
    }
    for (const emote of EMOJI_EMOTE_IDS) {
      const item = screen.getByRole("menuitem", { name: EMOJI_LABEL[emote] });
      expect(item.querySelector("svg.emote-emoji-svg")).not.toBeNull();
    }
  });

  it("R643 a press reports the emote and closes the menu", () => {
    const onPick = vi.fn();
    const onClose = vi.fn();
    render(<EmoteMenu side="you" gate={() => OPEN} onPick={onPick} onClose={onClose} />);

    fireEvent.click(screen.getByTestId("emote-thanks"));

    expect(onPick).toHaveBeenCalledTimes(1);
    expect(onPick).toHaveBeenCalledWith("thanks");
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId("emote-laugh"));
    expect(onPick).toHaveBeenLastCalledWith("laugh");
    expect(onPick).toHaveBeenCalledTimes(2);
  });

  it("R643 while the shared gate says limited, every item greys out with the wait and reports nothing", () => {
    const onPick = vi.fn();
    const onClose = vi.fn();
    render(<EmoteMenu side="you" gate={() => LIMITED} onPick={onPick} onClose={onClose} />);

    const items = screen.getAllByRole("menuitem");
    expect(items).toHaveLength(10);
    for (const item of items) {
      expect(item).toBeDisabled();
      expect(item).toHaveAttribute("data-limited", "true");
    }
    // Each item carries the wait in seconds — ceil(2400 / 1000) — the same reading the server drops on.
    const waits = document.querySelectorAll(".emote-wait");
    expect(waits).toHaveLength(10);
    expect(waits[0]).toHaveTextContent("3");

    // The press is still guarded: a limited menu reports nothing and stays open.
    fireEvent.click(screen.getByTestId("emote-oops"));
    expect(onPick).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("R643 the menu closes on a press outside it and on Escape, but not on a press inside", () => {
    const onClose = vi.fn();
    render(<EmoteMenu side="you" gate={() => OPEN} onPick={noop} onClose={onClose} />);

    fireEvent.pointerDown(screen.getByTestId("emote-menu"));
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.pointerDown(document.body);
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(2);
  });
});

describe("R643 the Mute emotes menu", () => {
  it("R643 offers the one item and reports it, then closes", () => {
    const onMute = vi.fn();
    const onClose = vi.fn();
    render(<MuteMenu muted={false} onMute={onMute} onClose={onClose} />);

    const item = screen.getByRole("menuitem", { name: "Mute emotes" });
    expect(item).not.toBeDisabled();

    fireEvent.click(item);

    expect(onMute).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R643 an already-muted opponent reads Emotes muted and does nothing", () => {
    const onMute = vi.fn();
    const onClose = vi.fn();
    render(<MuteMenu muted={true} onMute={onMute} onClose={onClose} />);

    const item = screen.getByRole("menuitem", { name: "Emotes muted" });
    expect(item).toBeDisabled();
    fireEvent.click(item);
    expect(onMute).not.toHaveBeenCalled();
  });
});
