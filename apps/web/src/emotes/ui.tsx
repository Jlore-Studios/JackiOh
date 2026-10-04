// The emote UI (issue §2, §5): the picker that opens on your own portrait, the one-item "Mute
// emotes" menu on the opponent's, and the bubble/sticker that pops out of a hero.
//
// Placement is inside the hero element (`position: relative`), so it needs no coordinates: the
// voice lines sit on an arc above the portrait, the emoji in a row under them, the bubble or
// sticker beside the portrait — `.emote-*` in emotes.css owns the geometry. Everything here is
// cosmetic (R643): a menu pick reports the emote and the caller decides how it travels.

import { useEffect, useState, type ReactElement } from "react";

import type { EmojiEmoteId, EmoteId, VoiceEmoteId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, VOICE_EMOTE_IDS, isVoiceEmote, type EmoteGate } from "@jackioh/shared";

import { EmojiArt, EMOJI_LABEL } from "./EmojiArt.tsx";
import { EMOTE_MENU_TICK_MS } from "./config.ts";
import type { EmoteShow as EmoteShowState } from "./session.ts";
import "./emotes.css";

const VOICE_LABEL: Record<VoiceEmoteId, string> = {
  greetings: "Greetings",
  wellPlayed: "Well Played",
  oops: "Oops",
  thanks: "Thanks",
  threaten: "Threaten",
};

/** The arc the five voice buttons sit on: how far each dips below the arc's crown, in px. */
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
 * Your portrait's menu: the five voice lines on their arc, the five emoji below. `gate` is the
 * shared limiter's live reading — while limited the items grey and carry the wait in seconds,
 * and the press reports nothing (R643). Re-polled every EMOTE_MENU_TICK_MS so the wait counts down.
 */
export function EmoteMenu({
  side,
  gate,
  onPick,
  onClose,
}: {
  side: "you" | "opponent";
  gate: () => EmoteGate;
  onPick: (emote: EmoteId) => void;
  onClose: () => void;
}): ReactElement {
  useMenuLifetime(true, onClose);
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
      className={`emote-menu emote-menu-${side}`}
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
            style={{ transform: `translateY(${ARC_DROP[index] ?? 0}px)` }}
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
}: {
  muted: boolean;
  onMute: () => void;
  onClose: () => void;
}): ReactElement {
  useMenuLifetime(true, onClose);
  return (
    <div
      className="emote-menu emote-menu-mute"
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
