// The tutorial's `data-testid` vocabulary (SPEC §9.10). `e2e/support/testids.ts` carries the same
// strings in its TUTORIAL block; keep the two files identical. Inside a lesson the board's own testids
// (game/contract.ts) and practice's (practice/testids.ts) still apply.

export const tutorialTestid = {
  path: "tutorial-path",
  /** One lesson on the path; `data-status="locked|unlocked|completed"`, `data-next="true"` on the one to play next. */
  lesson: (id: string): string => `tutorial-lesson-${id}`,
  /** A lesson's button: Start, Replay, or Locked (`aria-disabled="true"`). */
  lessonStart: (id: string): string => `tutorial-start-${id}`,
  /** "Continue: Lesson N", once a lesson is done and another is open. */
  continue: "tutorial-continue",
  /** Shows or hides the lessons once every one is done (`aria-expanded`). */
  pathToggle: "tutorial-path-toggle",
  /** "Hide tutorial", while a lesson is still to do (R322). */
  hide: "tutorial-hide",
  /** In the hidden path's place (R322): `data-complete` as the path's. */
  hidden: "tutorial-path-hidden",
  /** "Show tutorial", in the hidden path's place. */
  show: "tutorial-show",
  /** The lesson's HUD above the board; `data-lesson`, `data-human-seat`, `data-ai-seat`, `data-thinking`. */
  hud: "tutorial-hud",
  step: "tutorial-step",
  /** Back to the lessons; mid-game it opens practice's "Leave this game?" first. */
  exit: "tutorial-exit",
  /** The outcome once the lesson is over, which reopens the result dialog. */
  outcome: "tutorial-outcome",
  /** `data-coach-mode`, `data-coach-step` (absent while waiting), `data-coach-anchor` (board testids, space-separated). */
  coach: "coach",
  coachAck: "coach-ack",
  /** The ring round the anchor's elements; `pointer-events: none`; absent when none is on screen. */
  coachRing: "coach-ring",
  /** A moment's Hint; absent once the last is shown (R2205000). */
  coachHint: "coach-hint",
  /** The Field Guide's buttons in the HUD and the path's header (R2205002). */
  fieldGuideHud: "field-guide-hud",
  fieldGuidePath: "field-guide-path",
  fieldGuide: "field-guide",
  fieldGuideClose: "field-guide-close",
  /** One entry in the Field Guide; `data-found="true|false"`. */
  fieldGuideEntry: (id: string): string => `field-guide-entry-${id}`,
  /** The end-of-lesson dialog; `data-outcome="win|loss|draw"`, `data-lesson`. */
  result: "tutorial-result",
  next: "tutorial-next",
  /** After a loss or draw: the same lesson again, same seed. */
  retry: "tutorial-retry",
  back: "tutorial-back",
  /** After the last lesson: back to the lobby's practice setup. */
  playPractice: "tutorial-play-practice",
  /** After a win: the mechanics the game did not show found, each with its rule (R314). */
  stillToFind: "tutorial-still-to-find",
  /** After the last lesson: "What's next". */
  whatsNext: "tutorial-whats-next",
  viewBoard: "tutorial-view-board",
} as const;
