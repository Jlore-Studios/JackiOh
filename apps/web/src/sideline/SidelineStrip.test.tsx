// The sideline strip (SidelineStrip.tsx) over the hook that drives it (useSideline.ts): names and the
// sender's own You, the watcher's menu greyed from the gate at the server's numbers (R2990002), the hand
// checked before the gate (R2990001), a show's sound, Mute and the device setting, and the phone
// layout (R2990004).
// `useSideline` reads `Date.now`, so the faked one drives both the shows' expiry and the gate.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { useEffect, type ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { EmojiEmoteId } from "@jackioh/shared";
import { SPECTATOR_EMOTE_COOLDOWN_MS } from "@jackioh/server-config";

import type { SoundSink } from "../audio/types.ts";
import { EMOTE_EMOJI_MS, EMOTE_MENU_TICK_MS } from "../emotes/config.ts";
import { EMOJI_SFX } from "../emotes/play.ts";
import { stubPhoneBar } from "../test/phoneBar.ts";
import { SidelineStrip } from "./SidelineStrip.tsx";
import { sidelineTestid } from "./testids.ts";
import { useSideline, type SidelineApi } from "./useSideline.ts";

const HAND: readonly EmojiEmoteId[] = ["wave", "clap", "fire"];

type Options = Parameters<typeof useSideline>[0];

/** The strip over a live hook; `api` is the hook's latest return, for the relays a route would feed. */
function mount(options: Options = {}): { api: () => SidelineApi } {
  let latest: SidelineApi | null = null;
  function Harness({ onApi }: { onApi: (api: SidelineApi) => void }): ReactElement {
    const sideline = useSideline(options);
    useEffect(() => {
      onApi(sideline);
    });
    return <SidelineStrip sideline={sideline} />;
  }
  render(
    <Harness
      onApi={(api) => {
        latest = api;
      }}
    />,
  );
  return {
    api: () => {
      if (latest === null) throw new Error("the harness has not rendered");
      return latest;
    },
  };
}

function engineSpy(): SoundSink & { playSfx: ReturnType<typeof vi.fn> } {
  return { playSfx: vi.fn(() => true), playVoice: vi.fn(() => true), playEffect: vi.fn(() => true) };
}

function fakeClock(): void {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "setInterval", "clearInterval", "Date"] });
}

function shows(): { watcher: string | null; emote: string | null; self: boolean; text: string | null }[] {
  return screen.queryAllByTestId(sidelineTestid.emote).map((show) => ({
    watcher: show.getAttribute("data-watcher"),
    emote: show.getAttribute("data-emote"),
    self: show.getAttribute("data-self") === "true",
    text: show.querySelector(".sideline-emote__name")?.textContent ?? null,
  }));
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("R2990004 the sideline strip", () => {
  it("R2990004 a player's strip draws nothing until a watcher's emote lands, then names the watcher and sounds it", () => {
    fakeClock();
    const engine = engineSpy();
    const { api } = mount({ engine });
    expect(screen.queryByTestId(sidelineTestid.strip)).toBeNull();

    act(() => {
      api().receive("Wes#13", "sob");
    });

    const strip = screen.getByTestId(sidelineTestid.strip);
    expect(strip).toHaveAttribute("aria-label", "Spectator emotes");
    expect(shows()).toEqual([{ watcher: "Wes#13", emote: "sob", self: false, text: "Wes#13" }]);
    expect(engine.playSfx).toHaveBeenCalledWith(EMOJI_SFX.sob);
    // A player has no hand, so no menu; Mute is there from the first emote.
    expect(screen.queryByTestId(sidelineTestid.menu)).toBeNull();
    expect(screen.getByTestId(sidelineTestid.mute)).toHaveTextContent("Mute spectators");

    act(() => {
      vi.advanceTimersByTime(EMOTE_EMOJI_MS);
    });
    expect(shows()).toEqual([]);
    expect(screen.getByTestId(sidelineTestid.mute)).toBeEnabled();
  });

  it("R2990004 a watcher's own pick shows as You, beside the others' by name, newest last", () => {
    fakeClock();
    const emit = vi.fn();
    const { api } = mount({ hand: HAND, emit });

    act(() => {
      api().receive("Ann#2", "laugh");
    });
    fireEvent.click(screen.getByTestId(sidelineTestid.menu));
    fireEvent.click(screen.getByTestId("emote-clap"));

    expect(emit).toHaveBeenCalledTimes(1);
    expect(emit).toHaveBeenCalledWith("clap");
    expect(shows()).toEqual([
      { watcher: "Ann#2", emote: "laugh", self: false, text: "Ann#2" },
      { watcher: "", emote: "clap", self: true, text: "You" },
    ]);
    // The pick closes the menu.
    expect(screen.queryByTestId("emote-menu")).toBeNull();
  });

  it("R2990004 Mute takes the other watchers' emotes away for the session, keeps your own and says so", () => {
    fakeClock();
    const engine = engineSpy();
    const { api } = mount({ hand: HAND, engine });
    act(() => {
      api().receive("Wes#13", "sob");
    });
    fireEvent.click(screen.getByTestId(sidelineTestid.menu));
    fireEvent.click(screen.getByTestId("emote-wave"));
    expect(shows().map((show) => show.emote)).toEqual(["sob", "wave"]);

    fireEvent.click(screen.getByTestId(sidelineTestid.mute));

    expect(shows().map((show) => [show.emote, show.self])).toEqual([["wave", true]]);
    const mute = screen.getByTestId(sidelineTestid.mute);
    expect(mute).toBeDisabled();
    expect(mute).toHaveTextContent("Spectators muted");
    engine.playSfx.mockClear();
    let shown = true;
    act(() => {
      shown = api().receive("Ann#2", "laugh");
    });
    expect(shown).toBe(false);
    expect(shows().map((show) => show.emote)).toEqual(["wave"]);
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("R2990004 the device's muteSpectatorEmotes mutes the strip from the start: a player's never shows", () => {
    const engine = engineSpy();
    const { api } = mount({ engine, globalMute: true });
    let shown = true;
    act(() => {
      shown = api().receive("Wes#13", "sob");
    });
    expect(shown).toBe(false);
    expect(screen.queryByTestId(sidelineTestid.strip)).toBeNull();
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("R2990004 on a phone layout the strip says so, for its row along the top edge", () => {
    const restore = stubPhoneBar(true);
    try {
      mount({ hand: HAND });
      expect(screen.getByTestId(sidelineTestid.strip)).toHaveAttribute("data-phone", "true");
    } finally {
      restore();
    }
    cleanup();
    mount({ hand: HAND });
    expect(screen.getByTestId(sidelineTestid.strip)).not.toHaveAttribute("data-phone");
  });
});

describe("R2990002 the watcher's menu", () => {
  it("R2990002 the menu offers the dealt hand, and after a pick its items are data-limited until SPECTATOR_EMOTE_COOLDOWN_MS passes", () => {
    fakeClock();
    const emit = vi.fn();
    mount({ hand: HAND, emit });

    const button = screen.getByTestId(sidelineTestid.menu);
    expect(button).toHaveTextContent("Send an emote");
    fireEvent.click(button);
    const menu = screen.getByTestId("emote-menu");
    expect(within(menu).getAllByRole("menuitem").map((item) => item.getAttribute("data-testid"))).toEqual(
      HAND.map((emote) => `emote-${emote}`),
    );
    fireEvent.click(screen.getByTestId("emote-wave"));
    expect(emit).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId(sidelineTestid.menu));
    for (const item of within(screen.getByTestId("emote-menu")).getAllByRole("menuitem")) {
      expect(item).toHaveAttribute("data-limited", "true");
      expect(item).toBeDisabled();
    }
    fireEvent.click(screen.getByTestId("emote-fire"));
    expect(emit).toHaveBeenCalledTimes(1);

    act(() => {
      vi.advanceTimersByTime(SPECTATOR_EMOTE_COOLDOWN_MS - EMOTE_MENU_TICK_MS);
    });
    expect(screen.getByTestId("emote-fire")).toHaveAttribute("data-limited", "true");
    act(() => {
      vi.advanceTimersByTime(EMOTE_MENU_TICK_MS);
    });
    for (const item of within(screen.getByTestId("emote-menu")).getAllByRole("menuitem")) {
      expect(item).not.toHaveAttribute("data-limited");
    }
    fireEvent.click(screen.getByTestId("emote-fire"));
    expect(emit).toHaveBeenLastCalledWith("fire");
  });
});

describe("R2990001 the watcher's hand", () => {
  it("R2990001 a pick outside the hand is refused before the gate: nothing goes out and the limit is unspent", () => {
    fakeClock();
    const emit = vi.fn();
    const { api } = mount({ hand: HAND, emit });
    let sent = true;
    act(() => {
      sent = api().send("party");
    });
    expect(sent).toBe(false);
    expect(emit).not.toHaveBeenCalled();
    expect(api().gate().ok).toBe(true);
    expect(shows()).toEqual([]);
  });
});
