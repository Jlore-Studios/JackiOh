// The deck builder's portrait picker (issue §8, R641): the deck's current portrait, and on tap a
// grid of all six. Each tile has a Preview button that lists that portrait's ten emotes, and a
// press on one plays it exactly as in a match — the voice channel's line plus the speech bubble
// for a voice emote, the effects channel's synth plus the sticker for an emoji. Picking a tile
// reports the id; the caller saves it through the deck store's usual upsert (D5).
//
// The preview runs the same pieces the board does — `emoteShowInfo` for the bubble's text and
// span, `playEmote` for the sound, `EmoteShow` for what draws — so a preview can't drift from a
// match: same text, same file, same 2s sticker.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import type { EmoteId, PortraitId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, PORTRAIT_IDS, PORTRAITS, VOICE_EMOTE_IDS } from "@jackioh/shared";

import { getAudioEngine } from "../audio/index.ts";
import { CardArt } from "../cards/art/CardArt.tsx";
import { EmojiArt, EMOJI_LABEL } from "./EmojiArt.tsx";
import { emoteShowInfo, playEmote } from "./play.ts";
import { PORTRAIT_DEFS } from "./portraits.ts";
import type { EmoteShow as EmoteShowState } from "./session.ts";
import { EmoteShow } from "./ui.tsx";

const VOICE_LABEL: Record<(typeof VOICE_EMOTE_IDS)[number], string> = {
  greetings: "Greetings",
  wellPlayed: "Well Played",
  oops: "Oops",
  thanks: "Thanks",
  threaten: "Threaten",
};

/** One portrait's tile: its oval art, name and flavour, and the pick/preview controls. */
function PortraitTile({
  portrait,
  selected,
  previewing,
  onPick,
  onPreview,
}: {
  portrait: PortraitId;
  selected: boolean;
  previewing: boolean;
  onPick: () => void;
  onPreview: () => void;
}): ReactElement {
  const { defId, def } = PORTRAIT_DEFS[portrait];
  return (
    <div className="portrait-tile" data-portrait={portrait} data-selected={selected ? "true" : undefined}>
      <button
        type="button"
        className="portrait-pick"
        data-testid={`portrait-pick-${portrait}`}
        aria-pressed={selected}
        title={`${def.name} — ${PORTRAITS[portrait].flavour}`}
        onClick={onPick}
      >
        <CardArt defId={defId} radiant={false} tags={def.tags} type={def.type} shape="oval" name={def.name} />
        <span className="portrait-name">{def.name}</span>
      </button>
      <button
        type="button"
        className="portrait-preview-toggle"
        data-testid={`portrait-preview-${portrait}`}
        aria-expanded={previewing}
        onClick={onPreview}
      >
        Preview
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

  // Outside press and Escape close the picker, like the match's emote menu does (issue §2's
  // lifetime carried over — the picker's a menu of the same kind).
  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent): void => {
      if (root.current !== null && !root.current.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") setOpen(false);
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

  const { defId, def } = PORTRAIT_DEFS[portrait];
  return (
    <div className="portrait-picker" ref={root} data-testid="portrait-picker">
      <button
        type="button"
        className="portrait-current"
        data-testid="portrait-current"
        aria-expanded={open}
        aria-haspopup="dialog"
        title={`Hero portrait: ${def.name}`}
        onClick={() => {
          setOpen((was) => !was);
        }}
      >
        <CardArt defId={defId} radiant={false} tags={def.tags} type={def.type} shape="oval" name={def.name} />
        <span className="portrait-current-label">Portrait</span>
      </button>

      {open ? (
        <div className="portrait-menu" role="dialog" aria-label="Choose a hero portrait">
          <div className="portrait-grid" role="listbox" aria-label="Portraits">
            {PORTRAIT_IDS.map((id) => (
              <PortraitTile
                key={id}
                portrait={id}
                selected={id === portrait}
                previewing={previewing === id}
                onPick={() => {
                  onPick(id);
                  setOpen(false);
                }}
                onPreview={() => {
                  setPreviewing((was) => (was === id ? null : id));
                }}
              />
            ))}
          </div>

          {previewing === null ? null : (
            <div className="portrait-preview" data-testid="portrait-preview-panel">
              <span className="portrait-preview-show">
                {show === null ? null : <EmoteShow key={show.key} show={show} />}
              </span>
              <div className="portrait-preview-lines" role="group" aria-label={`${PORTRAIT_DEFS[previewing].def.name}'s emotes`}>
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
                    {VOICE_LABEL[emote]}
                  </button>
                ))}
              </div>
              <div className="portrait-preview-emojis" role="group" aria-label="Emoji">
                {EMOJI_EMOTE_IDS.map((emote) => (
                  <button
                    key={emote}
                    type="button"
                    className="emote-item emote-emoji-pick"
                    data-testid={`emote-preview-${emote}`}
                    title={EMOJI_LABEL[emote]}
                    aria-label={EMOJI_LABEL[emote]}
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
