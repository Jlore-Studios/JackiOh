// The deck builder's portrait picker (issue §8, R641): the deck's current portrait, and on tap a
// grid of all six. Each tile has a Preview button that lists the whole pool in that portrait's
// voice — its five voice lines and the nineteen emoji, any of which a game may deal (R1343) — and a
// press on one plays it exactly as in a match — the voice channel's line plus the speech bubble
// for a voice emote, the effects channel's synth plus the sticker for an emoji. Picking a tile
// reports the id; the caller saves it through the deck store's usual upsert (D5).
//
// The preview runs the same pieces the board does — `emoteShowInfo` for the bubble's text and
// span, `playEmote` for the sound, `EmoteShow` for what draws — so a preview can't drift from a
// match: same text, same file, same 2s sticker.

import { useCallback, useEffect, useId, useRef, useState, type ReactElement } from "react";

import type { EmoteId, PortraitId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, PORTRAIT_IDS, PORTRAITS, VOICE_EMOTE_IDS } from "@jackioh/shared";

import { getAudioEngine } from "../audio/index.ts";
import { readerName } from "../cards/chinese.ts";
import { t } from "../i18n/i18n.ts";
import { EmojiArt, EMOJI_LABEL } from "./EmojiArt.tsx";
import { emoteShowInfo, playEmote } from "./play.ts";
import { PortraitArt } from "./PortraitArt.tsx";
import { PORTRAIT_DEFS } from "./portraits.ts";
import type { EmoteShow as EmoteShowState } from "./session.ts";
import { EmoteShow, VOICE_LABEL } from "./ui.tsx";

/** One portrait's tile: its vivid oval art (R1332), name and flavour, and the pick/preview controls. */
function PortraitTile({
  portrait,
  selected,
  previewing,
  panelId,
  onPick,
  onPreview,
}: {
  portrait: PortraitId;
  selected: boolean;
  previewing: boolean;
  /** The id of the picker's one preview panel, which this tile's Preview shows while `previewing`. */
  panelId: string;
  onPick: () => void;
  onPreview: () => void;
}): ReactElement {
  const { def } = PORTRAIT_DEFS[portrait];
  const name = readerName(def);
  return (
    <div className="portrait-tile" data-portrait={portrait} data-selected={selected ? "true" : undefined}>
      <button
        type="button"
        className="portrait-pick"
        data-testid={`portrait-pick-${portrait}`}
        aria-pressed={selected}
        title={t("{name} — {flavour}", { name, flavour: t(PORTRAITS[portrait].flavour) })}
        onClick={onPick}
      >
        <PortraitArt portrait={portrait} />
        <span className="portrait-name">{name}</span>
      </button>
      <button
        type="button"
        className="portrait-preview-toggle"
        data-testid={`portrait-preview-${portrait}`}
        aria-expanded={previewing}
        aria-controls={previewing ? panelId : undefined}
        onClick={onPreview}
      >
        {t("Preview")}
      </button>
    </div>
  );
}

/**
 * The picker itself. `portrait` is the deck's current one (`vanilla` when the column is null);
 * `onPick` gets the chosen id, to be saved through the same upsert every deck edit takes.
 */
export function PortraitPicker({
  portrait,
  onPick,
}: {
  portrait: PortraitId;
  onPick: (portrait: PortraitId) => void;
}): ReactElement {
  const [open, setOpen] = useState(false);
  const [previewing, setPreviewing] = useState<PortraitId | null>(null);
  const [show, setShow] = useState<EmoteShowState | null>(null);
  const seq = useRef(0);
  const root = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  const menuId = useId();
  const panelId = useId();

  // Outside press and Escape close the picker, like the match's emote menu does (issue §2's
  // lifetime carried over — the picker's a menu of the same kind). Escape gives the focus back to the
  // portrait button unless the player has put it elsewhere on the page (#1569); an outside press leaves
  // it where the press put it.
  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent): void => {
      if (root.current !== null && !root.current.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== "Escape") return;
      setOpen(false);
      const focused = document.activeElement;
      if (focused === null || focused === document.body || root.current?.contains(focused) === true) {
        toggle.current?.focus({ preventScroll: true });
      }
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open]);

  // A preview's show expires on its own span, the same as the board's (session.expire's job
  // there; a one-shot here).
  useEffect(() => {
    if (show === null) return undefined;
    const key = show.key;
    const holdMs = Math.max(0, show.until - Date.now());
    const timer = window.setTimeout(() => {
      setShow((current) => (current?.key === key ? null : current));
    }, holdMs);
    return () => window.clearTimeout(timer);
  }, [show]);

  const preview = useCallback((emote: EmoteId, forPortrait: PortraitId) => {
    seq.current += 1;
    const info = emoteShowInfo(forPortrait, emote);
    setShow({ key: seq.current, emote, text: info.text, until: Date.now() + info.holdMs });
    playEmote(getAudioEngine(), forPortrait, emote);
  }, []);

  const { def } = PORTRAIT_DEFS[portrait];
  return (
    <div className="portrait-picker" ref={root} data-testid="portrait-picker">
      <button
        ref={toggle}
        type="button"
        className="portrait-current"
        data-testid="portrait-current"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-haspopup="dialog"
        title={t("Hero portrait: {name}", { name: readerName(def) })}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        <PortraitArt portrait={portrait} />
        <span className="portrait-current-label">{t("Portrait")}</span>
      </button>

      {open ? (
        <div className="portrait-menu" id={menuId} role="dialog" aria-label={t("Choose a hero portrait")}>
          <div className="portrait-grid" role="listbox" aria-label={t("Portraits")}>
            {PORTRAIT_IDS.map((id) => (
              <PortraitTile
                key={id}
                portrait={id}
                selected={id === portrait}
                previewing={previewing === id}
                panelId={panelId}
                onPick={() => {
                  onPick(id);
                  setOpen(false);
                  // The tile goes with the menu, so the focus goes back to the portrait button (#1569).
                  toggle.current?.focus({ preventScroll: true });
                }}
                onPreview={() => {
                  setPreviewing((was) => (was === id ? null : id));
                }}
              />
            ))}
          </div>

          {previewing === null ? null : (
            <div className="portrait-preview" id={panelId} data-testid="portrait-preview-panel">
              <span className="portrait-preview-show">
                {show === null ? null : <EmoteShow key={show.key} show={show} />}
              </span>
              <div className="portrait-preview-lines" role="group" aria-label={t("{name}'s emotes", { name: readerName(PORTRAIT_DEFS[previewing].def) })}>
                {VOICE_EMOTE_IDS.map((emote) => (
                  <button
                    key={emote}
                    type="button"
                    className="emote-item emote-voice"
                    data-testid={`emote-preview-${emote}`}
                    onClick={() => {
                      preview(emote, previewing);
                    }}
                  >
                    {t(VOICE_LABEL[emote])}
                  </button>
                ))}
              </div>
              <div className="portrait-preview-emojis" role="group" aria-label={t("Emoji")}>
                {EMOJI_EMOTE_IDS.map((emote) => (
                  <button
                    key={emote}
                    type="button"
                    className="emote-item emote-emoji-pick"
                    data-testid={`emote-preview-${emote}`}
                    title={t(EMOJI_LABEL[emote])}
                    aria-label={t(EMOJI_LABEL[emote])}
                    onClick={() => {
                      preview(emote, previewing);
                    }}
                  >
                    <EmojiArt emoji={emote} />
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
}
