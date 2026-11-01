// The emote UI (issue §2, §5): your portrait's picker of the seat's dealt hand only (R1343), drawn
// `inline` by `HeroInspect.tsx` too (R1449, R1330), the opponent's "Mute emotes" menu, and a hero's
// bubble/sticker, placed inside the hero so emotes.css owns the geometry. All cosmetic (R643): a pick
// reports the emote and the caller decides how it travels.

import { useEffect, useLayoutEffect, useRef, useState, type ReactElement, type RefObject } from "react";

import type { EmojiEmoteId, EmoteId, VoiceEmoteId } from "@jackioh/shared";
import { isVoiceEmote, type EmoteGate } from "@jackioh/shared";

import { DEFAULT_LOCALE, getLocale, t } from "../i18n/i18n.ts";
import { EmojiArt, EMOJI_LABEL } from "./EmojiArt.tsx";
import { EMOTE_MENU_EDGE_PX, EMOTE_MENU_TICK_MS } from "./config.ts";
import type { EmoteShow as EmoteShowState } from "./session.ts";
import "./emotes.css";

export const VOICE_LABEL: Record<VoiceEmoteId, string> = {
  greetings: "Greetings",
  wellPlayed: "Well Played",
  oops: "Oops",
  thanks: "Thanks",
  threaten: "Threaten",
};

/**
 * The arc the voice buttons sit on: an item `d` places from the middle dips `ARC_CURVE_PX × d²`
 * below the crown, as `--emote-arc-drop`, emotes.css's translateY (flat on an upright phone).
 */
const ARC_CURVE_PX = 3.5;

export function arcDrop(index: number, count: number): number {
  const fromMiddle = index - (count - 1) / 2;
  return Math.round(ARC_CURVE_PX * fromMiddle * fromMiddle);
}

/**
 * Closes on an outside press, Escape or a drag beginning elsewhere. A press on the portrait the
 * menu hangs off (`menu`'s parent) is not outside it (R1449): its click toggles the menu itself,
 * so closing on its pointerdown would have the click open it again.
 */
function useMenuLifetime(open: boolean, onClose: () => void, menu: RefObject<HTMLDivElement | null>): void {
  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent): void => {
      const target = event.target as Element | null;
      const inside = target?.closest?.("[data-emote-menu]");
      if (inside !== null && inside !== undefined) return;
      const portrait = menu.current?.parentElement;
      if (target instanceof Node && portrait?.contains(target) === true) return;
      onClose();
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open, onClose, menu]);
}

/**
 * How far to slide a menu centered on its portrait so it stays `margin` px inside a viewport
 * `viewportWidth` wide (#219). A menu too wide for both edges keeps its left edge on the screen.
 */
export function menuShift(rect: { left: number; right: number }, viewportWidth: number, margin: number): number {
  if (rect.left < margin) return margin - rect.left;
  if (rect.right > viewportWidth - margin) return Math.max(viewportWidth - margin - rect.right, margin - rect.left);
  return 0;
}

/**
 * Keeps an open menu on the screen (#219) by `menuShift` through `--emote-menu-shift`, which the CSS
 * adds to its centering transform; a layout effect, so it never paints off. The measure first puts the
 * variable back to 0: a StrictMode replay that read the applied shift would take the slid menu for
 * centered and write 0 back (the rect carries the transform).
 */
function useKeepOnScreen(ref: RefObject<HTMLDivElement | null>, enabled: boolean): void {
  useLayoutEffect(() => {
    if (!enabled) return;
    const menu = ref.current;
    if (menu === null) return;
    menu.style.setProperty("--emote-menu-shift", "0px");
    const rect = menu.getBoundingClientRect();
    const viewportWidth = document.documentElement.clientWidth;
    // No layout (jsdom, or a hidden board): nothing to measure, so the menu stays centered.
    if (rect.width === 0 || viewportWidth === 0) {
      menu.style.removeProperty("--emote-menu-shift");
      return;
    }
    menu.style.setProperty("--emote-menu-shift", `${menuShift(rect, viewportWidth, EMOTE_MENU_EDGE_PX)}px`);
  }, [ref, enabled]);
}

/**
 * Your portrait's menu: the hand's voice lines on their arc, its emoji below (R1343). `gate` is
 * the shared limiter's live reading — while limited the items grey and carry the wait in seconds,
 * and the press reports nothing (R643). Re-polled every EMOTE_MENU_TICK_MS so the wait counts down.
 */
export function EmoteMenu({
  side,
  hand,
  gate,
  onPick,
  onClose,
  inline = false,
}: {
  side: "you" | "opponent";
  /** The seat's dealt hand (R1341), in the pool's order: the menu shows these and nothing else. */
  hand: readonly EmoteId[];
  gate: () => EmoteGate;
  onPick: (emote: EmoteId) => void;
  onClose: () => void;
  /** Drawn inside the hero's inspect view (R1330): the view owns its lifetime and placement. */
  inline?: boolean;
}): ReactElement {
  const menuRef = useRef<HTMLDivElement>(null);
  useMenuLifetime(!inline, onClose, menuRef);
  useKeepOnScreen(menuRef, !inline);
  const [, forceTick] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(() => forceTick((tick) => tick + 1), EMOTE_MENU_TICK_MS);
    return () => window.clearInterval(timer);
  }, []);
  const state = gate();
  const waitS = state.ok === false ? Math.ceil(state.retryAfterMs / 1000) : 0;
  const limited = state.ok === false;
  const voices = hand.filter((emote): emote is VoiceEmoteId => isVoiceEmote(emote));
  const emoji = hand.filter((emote): emote is EmojiEmoteId => !isVoiceEmote(emote));

  const pick = (emote: EmoteId) => () => {
    if (gate().ok === false) return;
    onPick(emote);
    onClose();
  };

  return (
    <div
      ref={menuRef}
      className={`emote-menu emote-menu-${side}${inline ? " emote-menu--inline" : ""}`}
      data-emote-menu="true"
      data-testid="emote-menu"
      role="menu"
      aria-label={t("Emotes")}
      onPointerDown={(event) => event.stopPropagation()}
      // A click inside the menu must never reach .hero's own onClick: it would read the hero as a
      // non-legal target and reopen the very menu the pick just closed (issue §2).
      onClick={(event) => event.stopPropagation()}
    >
      <div className="emote-voice-arc" role="none">
        {voices.map((emote, index) => (
          <button
            key={emote}
            type="button"
            role="menuitem"
            className="emote-item emote-voice"
            data-testid={`emote-${emote}`}
            data-limited={limited ? "true" : undefined}
            disabled={limited}
            style={{ "--emote-arc-drop": `${arcDrop(index, voices.length)}px` } as React.CSSProperties}
            onClick={pick(emote)}
          >
            {t(VOICE_LABEL[emote])}
            {limited && waitS > 0 ? <span className="emote-wait">{waitS}</span> : null}
          </button>
        ))}
      </div>
      <div className="emote-emoji-row" role="none">
        {emoji.map((emote) => (
          <button
            key={emote}
            type="button"
            role="menuitem"
            className="emote-item emote-emoji-pick"
            data-testid={`emote-${emote}`}
            data-limited={limited ? "true" : undefined}
            disabled={limited}
            title={t(EMOJI_LABEL[emote])}
            aria-label={t(EMOJI_LABEL[emote])}
            onClick={pick(emote)}
          >
            <EmojiArt emoji={emote} />
            {limited && waitS > 0 ? <span className="emote-wait">{waitS}</span> : null}
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * The opponent portrait's one-item menu (issue §5): "Mute emotes" hides and silences everything
 * they send for the rest of the match. Already muted, the item reports it and does nothing.
 */
export function MuteMenu({
  muted,
  onMute,
  onClose,
  inline = false,
}: {
  muted: boolean;
  onMute: () => void;
  onClose: () => void;
  /** As `EmoteMenu`'s `inline` (R1330). */
  inline?: boolean;
}): ReactElement {
  const menuRef = useRef<HTMLDivElement>(null);
  useMenuLifetime(!inline, onClose, menuRef);
  useKeepOnScreen(menuRef, !inline);
  return (
    <div
      ref={menuRef}
      className={`emote-menu emote-menu-mute${inline ? " emote-menu--inline" : ""}`}
      data-emote-menu="true"
      data-testid="emote-mute-menu"
      role="menu"
      aria-label={t("Opponent emotes")}
      onPointerDown={(event) => event.stopPropagation()}
      // Same bubbling guard as the emote menu: muting closes the menu, and a click that reached
      // the hero would toggle it straight back open.
      onClick={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        role="menuitem"
        className="emote-item emote-mute"
        data-testid="emote-mute"
        disabled={muted}
        onClick={() => {
          onMute();
          onClose();
        }}
      >
        {muted ? t("Emotes muted") : t("Mute emotes")}
      </button>
    </div>
  );
}

/**
 * The bubble's words: the voice line as it was recorded, for an English reader; for a reader of any
 * other language the line's label in their words (#711), since the recording itself stays English.
 */
function bubbleText(show: EmoteShowState): string | null {
  if (show.text === null || getLocale() === DEFAULT_LOCALE || !isVoiceEmote(show.emote)) return show.text;
  return t(VOICE_LABEL[show.emote]);
}

/**
 * What a hero shows while its player emotes: a voice line's bubble or an emoji's sticker (issue §4).
 * The show's remaining span rides `--emote-hold` so the CSS animates as long as the session keeps it.
 */
export function EmoteShow({ show }: { show: EmoteShowState }): ReactElement {
  const isVoice = isVoiceEmote(show.emote);
  const holdMs = Math.max(0, show.until - Date.now());
  return (
    <span
      className={isVoice ? "emote-show emote-bubble" : "emote-show emote-sticker"}
      data-testid="emote-bubble"
      data-emoji={isVoice ? undefined : show.emote}
      style={{ "--emote-hold": `${holdMs}ms` } as React.CSSProperties}
      role={isVoice ? "status" : undefined}
    >
      {isVoice ? (
        bubbleText(show)
      ) : (
        <EmojiArt emoji={show.emote as EmojiEmoteId} />
      )}
    </span>
  );
}
