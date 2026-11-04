// The sideline strip's test ids (R2990000, R2990004), mirrored in `e2e/support/testids.ts`.

export const sidelineTestid = {
  /** The strip: `data-phone` on a phone layout. */
  strip: "sideline-strip",
  /** One per show: `data-watcher` (empty for the sender's own), `data-emote`, `data-self`. */
  emote: "sideline-emote",
  /** A watcher's Send an emote, which opens `EmoteMenu` (`emote-menu`, `emote-<id>`). */
  menu: "sideline-menu",
  mute: "sideline-mute",
} as const;
