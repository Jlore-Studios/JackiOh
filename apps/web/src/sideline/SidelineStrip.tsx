// The sideline strip (R2990000, R2990004): the watchers' emotes at the screen's inline end, along its
// top on a phone (`usePhoneBar`), each a sticker with its watcher's name, the sender's own reading You.
// A watcher's strip offers its dealt hand through the seat's own menu (`EmoteMenu`, R2990002 greying
// it from the gate); a player's appears once a watcher's emote has. Mute silences the other watchers
// for the session. Cosmetic: it never reaches the game, and decides nothing (CLAUDE.md rule 7).

import { useCallback, useEffect, useState, type ReactElement } from "react";

import { EMOJI_LABEL, EmojiArt } from "../emotes/EmojiArt.tsx";
import { EmoteMenu } from "../emotes/ui.tsx";
import { usePhoneBar } from "../game/phoneBar.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { sidelineTestid } from "./testids.ts";
import type { GameSideline } from "./useSideline.ts";
import "./sideline.css";

export function SidelineStrip({ sideline }: { sideline: GameSideline }): ReactElement | null {
  useLocale();
  const phone = usePhoneBar();
  const [open, setOpen] = useState(false);
  const close = useCallback((): void => {
    setOpen(false);
  }, []);
  const { shows, hand, gate, send, mute, muted, heard } = sideline;
  // A hand withdrawn (the watch socket dropped) closes the menu, so a reopened socket does not reopen it.
  const offered = hand.length > 0;
  useEffect(() => {
    if (!offered) setOpen(false);
  }, [offered]);
  if (!offered && !heard) return null;

  return (
    <aside
      className="sideline-strip"
      data-testid={sidelineTestid.strip}
      data-phone={phone ? "true" : undefined}
      aria-label={t("Spectator emotes")}
    >
      <div className="sideline-strip__shows" role="log" aria-live="polite">
        {shows.map((show) => (
          <div
            key={show.key}
            className="sideline-emote"
            data-testid={sidelineTestid.emote}
            data-watcher={show.watcher}
            data-emote={show.emote}
            data-self={show.self ? "true" : undefined}
          >
            <EmojiArt emoji={show.emote} />
            <span className="sideline-emote__name">{show.self ? t("You") : show.watcher}</span>
            <span className="sideline-sr-only">{t(EMOJI_LABEL[show.emote])}</span>
          </div>
        ))}
      </div>
      <div className="sideline-strip__controls">
        {offered ? (
          // The menu's parent is its anchor: a press here toggles it rather than closing it (ui.tsx).
          <div className="sideline-strip__send">
            <button
              type="button"
              className="sideline-strip__button"
              data-testid={sidelineTestid.menu}
              aria-expanded={open}
              onClick={() => {
                setOpen((was) => !was);
              }}
            >
              {t("Send an emote")}
            </button>
            {open ? <EmoteMenu side="you" hand={hand} gate={gate} onPick={send} onClose={close} /> : null}
          </div>
        ) : null}
        <button type="button" className="sideline-strip__button" data-testid={sidelineTestid.mute} disabled={muted} onClick={mute}>
          {muted ? t("Spectators muted") : t("Mute spectators")}
        </button>
      </div>
    </aside>
  );
}
