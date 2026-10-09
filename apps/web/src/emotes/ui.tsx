// The emote UI (issue §2, §5): the picker that opens on your own portrait, the one-item "Mute
// emotes" menu on the opponent's, and the bubble/sticker that pops out of a hero. Since R1330 the
// two menus are drawn inline in the hero's inspect view (`HeroInspect.tsx`); the hung-off-the-hero
// placement below is what each does when it is not `inline`.
//
// Placement is inside the hero element (`position: relative`), so it needs no board coordinates:
// the voice lines sit on an arc above the portrait, the emoji in a row under them, the bubble or
// sticker beside the portrait — `.emote-*` in emotes.css owns the geometry, and an open menu only
// measures itself once to slide back onto the screen (`useKeepOnScreen`, #219). Everything here is
// cosmetic (R643): a menu pick reports the emote and the caller decides how it travels.

import { useEffect, useLayoutEffect, useRef, useState, type ReactElement, type RefObject } from "react";

import type { EmojiEmoteId, EmoteId, VoiceEmoteId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, VOICE_EMOTE_IDS, isVoiceEmote, type EmoteGate } from "@jackioh/shared";

import { EmojiArt, EMOJI_LABEL } from "./EmojiArt.tsx";
import { EMOTE_MENU_EDGE_PX, EMOTE_MENU_TICK_MS } from "./config.ts";
import type { EmoteShow as EmoteShowState } from "./session.ts";
import "./emotes.css";

const VOICE_LABEL: Record<VoiceEmoteId, string> = {
  greetings: "Greetings",
  wellPlayed: "Well Played",
  oops: "Oops",
  thanks: "Thanks",
  threaten: "Threaten",
};

/**
 * The arc the five voice buttons sit on: how far each dips below the arc's crown, in px. Each item
 * carries its drop as `--emote-arc-drop`, which emotes.css turns into a translateY, and flattens on
 * a phone held upright, where the voice lines wrap onto two rows.
 */
const ARC_DROP: readonly number[] = [14, 4, 0, 4, 14];

/**
 * Closes on an outside press, Escape or a drag beginning elsewhere — the issue's menu lifetime —
 * by watching pointerdown/keydown on the window the menu's own clicks stopPropagation against.
 */
function useMenuLifetime(open: boolean, onClose: () => void): void {
  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent): void => {
      const inside = (event.target as Element | null)?.closest?.("[data-emote-menu]");
      if (inside === null || inside === undefined) onClose();
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
  }, [open, onClose]);
}

/**
 * How far to slide a menu centred on its portrait so it stays `margin` px inside a viewport
 * `viewportWidth` wide (#219): right by what pokes out on the left, left by what pokes out on the
 * right, nothing when it fits. A menu too wide for both keeps its left edge on the screen.
 */
export function menuShift(rect: { left: number; right: number }, viewportWidth: number, margin: number): number {
  if (rect.left < margin) return margin - rect.left;
  if (rect.right > viewportWidth - margin) return Math.max(viewportWidth - margin - rect.right, margin - rect.left);
  return 0;
}

/**
 * Keeps an open menu on the screen (#219). emotes.css centres the menu on the portrait, and a seat
 * puts the portrait anywhere from the screen's left edge (a phone) to its middle, so once laid out
 * the menu measures itself and slides sideways by `menuShift` through `--emote-menu-shift`, which
 * the CSS adds to its centring transform. A layout effect, so the menu never paints off the screen.
 * The measure first puts the variable back to 0: StrictMode replays this effect in dev (main.tsx
 * mounts the app under it), and a re-run that read the shift the first run applied would take the
 * slid menu for centred and write 0 back — the rect carries the transform.
 */
function useKeepOnScreen(ref: RefObject<HTMLDivElement | null>, enabled: boolean): void {
  useLayoutEffect(() => {
    if (!enabled) return;
    const menu = ref.current;
    if (menu === null) return;
    menu.style.setProperty("--emote-menu-shift", "0px");
    const rect = menu.getBoundingClientRect();
    const viewportWidth = document.documentElement.clientWidth;
    // No layout (jsdom, or a hidden board): nothing to measure, so the menu stays centred.
    if (rect.width === 0 || viewportWidth === 0) {
      menu.style.removeProperty("--emote-menu-shift");
      return;
    }
    menu.style.setProperty("--emote-menu-shift", `${menuShift(rect, viewportWidth, EMOTE_MENU_EDGE_PX)}px`);
  }, [ref, enabled]);
}

/**
 * Your portrait's menu: the five voice lines on their arc, the five emoji below. `gate` is the
 * shared limiter's live reading — while limited the items grey and carry the wait in seconds,
 * and the press reports nothing (R643). Re-polled every EMOTE_MENU_TICK_MS so the wait counts down.
 */
export function EmoteMenu({
  side,
  gate,
  onPick,
  onClose,
  inline = false,
}: {
  side: "you" | "opponent";
  gate: () => EmoteGate;
  onPick: (emote: EmoteId) => void;
  onClose: () => void;
  /** Drawn inside the hero's inspect view (R1330): the view owns its lifetime and placement. */
  inline?: boolean;
}): ReactElement {
  useMenuLifetime(!inline, onClose);
  const menuRef = useRef<HTMLDivElement>(null);
  useKeepOnScreen(menuRef, !inline);
  const [, forceTick] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(() => forceTick((tick) => tick + 1), EMOTE_MENU_TICK_MS);
    return () => window.clearInterval(timer);
  }, []);
  const state = gate();
  const waitS = state.ok === false ? Math.ceil(state.retryAfterMs / 1000) : 0;
  const limited = state.ok === false;

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
      aria-label="Emotes"
      onPointerDown={(event) => event.stopPropagation()}
      // A click inside the menu must never reach .hero's own onClick — it would read the hero as
      // a non-legal target and reopen the very menu the pick just closed (issue §2: closes on a
      // pick).
      onClick={(event) => event.stopPropagation()}
    >
      <div className="emote-voice-arc" role="none">
        {VOICE_EMOTE_IDS.map((emote, index) => (
          <button
            key={emote}
            type="button"
            role="menuitem"
            className="emote-item emote-voice"
            data-testid={`emote-${emote}`}
            data-limited={limited ? "true" : undefined}
            disabled={limited}
            style={{ "--emote-arc-drop": `${ARC_DROP[index] ?? 0}px` } as React.CSSProperties}
            onClick={pick(emote)}
          >
            {VOICE_LABEL[emote]}
            {limited && waitS > 0 ? <span className="emote-wait">{waitS}</span> : null}
          </button>
        ))}
      </div>
      <div className="emote-emoji-row" role="none">
        {EMOJI_EMOTE_IDS.map((emote) => (
          <button
            key={emote}
            type="button"
            role="menuitem"
            className="emote-item emote-emoji-pick"
            data-testid={`emote-${emote}`}
            data-limited={limited ? "true" : undefined}
            disabled={limited}
            title={EMOJI_LABEL[emote]}
            aria-label={EMOJI_LABEL[emote]}
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
  /** Drawn inside the hero's inspect view (R1330): the view owns its lifetime and placement. */
  inline?: boolean;
}): ReactElement {
  useMenuLifetime(!inline, onClose);
  const menuRef = useRef<HTMLDivElement>(null);
  useKeepOnScreen(menuRef, !inline);
  return (
    <div
      ref={menuRef}
      className={`emote-menu emote-menu-mute${inline ? " emote-menu--inline" : ""}`}
      data-emote-menu="true"
      data-testid="emote-mute-menu"
      role="menu"
      aria-label="Opponent emotes"
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
        {muted ? "Emotes muted" : "Mute emotes"}
      </button>
    </div>
  );
}

/**
 * What a hero shows while its player emotes: the speech bubble for a voice line (its text, for the
 * line's audible span, 2–4s), or the sticker that pops out and bounces for an emoji (issue §4).
 * The show's remaining span rides `--emote-hold` so the CSS animates exactly as long as the
 * session keeps it up.
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
        show.text
      ) : (
        <EmojiArt emoji={show.emote as EmojiEmoteId} />
      )}
    </span>
  );
}
