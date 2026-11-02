// The practice route's `data-testid` vocabulary. `e2e/support/testids.ts` carries the same strings
// in its PRACTICE block; keep the two files identical.

import type { Difficulty } from "@jackioh/engine/config";

export const practiceTestid = {
  setup: "practice-setup",
  /** An `<input type="radio">` per difficulty. */
  difficulty: (d: Difficulty): string => `practice-difficulty-${d}`,
  /** A `<select>`; its option values are `deckChoiceValue`'s. */
  deck: "practice-deck",
  /** Under the deck picker: why no saved deck is offered; absent when some are. */
  deckHint: "practice-deck-hint",
  deckPreview: "practice-deck-preview",
  /** R1373: beside the Random deck only, "More cards from the newest set", a checkbox. */
  leanNewest: "practice-lean-newest",
  /** In the preview: one bar per cost, `data-cost` and `data-count`. */
  deckCurve: "practice-deck-curve",
  deckCard: (defId: string): string => `practice-deck-card-${defId}`,
  /** R915005: under the preview, why a saved deck that names custom cards cannot start; Start is off while it shows. role="status". */
  customBlocked: "practice-custom-blocked",
  /** #1691: beside `customBlocked`, after the custom cards could not be read. */
  customRetry: "practice-custom-retry",
  /** #1691: beside the deck hint, after the saved decks could not be read. */
  savedRetry: "practice-saved-retry",
  /** #1691: in the preview, after the card list could not be loaded. */
  defsRetry: "practice-defs-retry",
  /** #921: `?try=<custom id>` while the account's custom cards are read, before the game starts. */
  tryLoading: "practice-try-loading",
  /** #921: on the setup, after `?try=` named no custom card of the account's; role="status". */
  tryMissing: "practice-try-missing",
  start: "practice-start",
  loading: "practice-loading",
  error: "practice-error",
  /** Carries data-difficulty, data-human-seat, data-ai-seat and data-thinking="true|false". */
  hud: "practice-hud",
  /** Rendered only while the AI is thinking; role="status", text "AI is thinking…". */
  thinking: "practice-thinking",
  /** Mid-game it opens `leave`; once the game is over, or on the failure screen, it leaves at once. */
  newGame: "practice-new-game",
  /** Out to the main menu: mid-game it opens `leave` first; once the game is over it leaves at once. */
  menu: "practice-menu",
  /** "Leave this game?": the confirmation `newGame` or `menu` opens. */
  leave: "practice-leave",
  /** In the confirmation: abandon the game ("Leave without saving" for a free game, R765; "Leave game" for a lesson). */
  leaveConfirm: "practice-leave-confirm",
  /** R765: in a free game's confirmation, "Save and leave": keep the game for the practice menu's banner. */
  leaveSave: "practice-leave-save",
  leaveStay: "practice-leave-stay",
  /** R765: on the setup, after the banner's Resume found no game to pick up; role="status". */
  resumeLost: "practice-resume-lost",
  /** The end-of-game dialog; data-outcome="win|loss|draw". */
  result: "practice-result",
  /**
   * R1303000: in the result dialog of a game on a server seed, what its claim earned:
   * data-earning="pending|paid|refused|unreachable|unearned", and data-reason when refused. role="status".
   */
  resultEarning: "practice-result-earning",
  /** R2379000: on the game screen of a free game the server would not seed, that it earns nothing and why. role="status". */
  unearned: "practice-unearned",
  /** In the result dialog: the same difficulty and deck again, with a fresh seed and seat. */
  playAgain: "practice-play-again",
  changeSetup: "practice-change-setup",
  viewBoard: "practice-view-board",
  /** R768000: in the result dialog of a kept game, its replay from step 0. */
  watchReplay: "practice-watch-replay",
  /** R768000: in the result dialog of a kept game, its last turn's GIF. */
  shareLastTurn: "practice-share-last-turn",
  /** In the HUD once the game is over: the outcome, which reopens the result dialog. */
  outcome: "practice-outcome",
  /** R923000: on the setup, a checkbox: the game is a sandbox with dev tools. */
  sandbox: "practice-sandbox",
  /** R923000: a sandbox game's dev tools panel, and the button that shows and hides it. */
  sandboxPanel: "practice-sandbox-panel",
  sandboxToggle: "practice-sandbox-toggle",
  /** A checkbox: hold the AI's next step while the board is arranged. */
  sandboxHold: "practice-sandbox-hold",
  /** The card search's `<input>`, its Radiant checkbox, and one button per card found. */
  sandboxSearch: "practice-sandbox-search",
  sandboxRadiant: "practice-sandbox-radiant",
  sandboxResult: (defId: string): string => `practice-sandbox-result-${defId}`,
  /** Spawn: whose (`you` or `opponent`), where (`hand`, `units:<lane>`, `backrow:<lane>`, `graveyard`, `libraryTop`), and the button. */
  sandboxSpawnSeat: "practice-sandbox-spawn-seat",
  sandboxSpawnWhere: "practice-sandbox-spawn-where",
  sandboxSpawn: "practice-sandbox-spawn",
  sandboxHeroSeat: "practice-sandbox-hero-seat",
  sandboxHealth: "practice-sandbox-health",
  sandboxArmor: "practice-sandbox-armor",
  sandboxSetHero: "practice-sandbox-set-hero",
  sandboxManaSeat: "practice-sandbox-mana-seat",
  sandboxManaCurrent: "practice-sandbox-mana-current",
  sandboxManaMax: "practice-sandbox-mana-max",
  sandboxSetMana: "practice-sandbox-set-mana",
  sandboxDrawSeat: "practice-sandbox-draw-seat",
  sandboxDrawCount: "practice-sandbox-draw-count",
  sandboxDraw: "practice-sandbox-draw",
  /** A card the human can see, and the two tools that take it. */
  sandboxTarget: "practice-sandbox-target",
  sandboxDestroy: "practice-sandbox-destroy",
  sandboxRemove: "practice-sandbox-remove",
} as const;
