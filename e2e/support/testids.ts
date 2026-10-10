// E2E selector contract: BUILD M5-T1, BUILD M5-T2, BUILD M5-T3 and BUILD M5-T4 fix base testids and attributes; BUILD M8 exercises them. Assumptions are in e2e/README.md.

import type { EventType, Lane, PromptKind, Row, Side } from "./types.ts";

export function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

export function zoneId(side: Side, row: Row, lane: Lane): string {
  return `zone-${side}-${row}-${lane}`;
}

export function cardId(instanceId: string): string {
  return `card-${instanceId}`;
}

export function handCardId(instanceId: string): string {
  return `hand-card-${instanceId}`;
}

export function heroId(side: Side): string {
  return `hero-${side}`;
}

export const END_TURN = "end-turn";
export const OFFER_DRAW = "offer-draw";
export const POWER = "power";

export const PROMPT = "[data-prompt-kind]";

export function promptOf(kind: PromptKind): string {
  return `[data-prompt-kind="${kind}"]`;
}

export const ANIMATING = "[data-animating]";

export function animating(event: EventType): string {
  return `[data-animating="${event}"]`;
}

export const LOCKED = '[data-locked="true"]';

// A4–A5 selectors beyond the BUILD contract are listed in e2e/README.md.

/** A4: one option of an open prompt, keyed by `PendingOption.key` (SPEC §10.8). */
export function promptOptionId(key: string): string {
  return `prompt-option-${key}`;
}

export const PROMPT_SUBMIT = "prompt-submit";

export const PROMPT_X_INPUT = "prompt-x";

export const RESULT_OVERLAY = "result-overlay";

export const BANNER = "turn-banner";

export const SEAT_SWITCH = "seat-switch";

export function graveyardCountId(side: Side): string {
  return `graveyard-count-${side}`;
}

export function exileCountId(side: Side): string {
  return `exile-count-${side}`;
}

export function libraryCountId(side: Side): string {
  return `library-count-${side}`;
}

export function handCountId(side: Side): string {
  return `hand-count-${side}`;
}

export function manaId(side: Side): string {
  return `mana-${side}`;
}

export const MANA_CRYSTAL = ".mana-crystal";

export const DAMAGE_POP = ".damage-pop";
export const HEAL_POP = ".heal-pop";
export const LOSS_POP = ".loss-pop";
export const RADIANT = ".radiant";

export function switchPositionId(instanceId: string): string {
  return `switch-${instanceId}`;
}

export const LEGAL = '[data-legal="true"]';
export const ILLEGAL = '[data-legal="false"]';

// A5: read shown stats from attributes, not formatted text.

export function attackIs(attack: number): string {
  return `[data-attack="${String(attack)}"]`;
}

export function healthIs(health: number): string {
  return `[data-health="${String(health)}"]`;
}

export function maxHealthIs(maxHealth: number): string {
  return `[data-max-health="${String(maxHealth)}"]`;
}

export function armorIs(armor: number): string {
  return `[data-armor="${String(armor)}"]`;
}

/** §6.1 keyword badge. */
export function keywordIs(keyword: string): string {
  return `[data-keyword="${keyword}"]`;
}

/** §3.3: position. */
export function positionIs(position: "ATK" | "DEF"): string {
  return `[data-position="${position}"]`;
}

export const RADIANT_ATTR = '[data-radiant="true"]';

// A5: regions and animation targets mirror `apps/web/src/game/animations.ts` and `contract.ts`.

/** §10.8: an opponent hand has no card ids. */
export function handRegionId(side: Side): string {
  return `hand-${side}`;
}

export function libraryId(side: Side): string {
  return `library-${side}`;
}

export function graveyardId(side: Side): string {
  return `graveyard-${side}`;
}

export function exileId(side: Side): string {
  return `exile-${side}`;
}

/** R169: keep the empty modifier container so a leaving badge can animate. */
export function modifiersId(side: Side): string {
  return `modifiers-${side}`;
}

/** R169: repeated modifier badges use a class, not testids. */
export const MODIFIER_BADGE = ".modifier-badge";

/** R169: public modifier badges use `ModifierView.id`, never a face-down source id. */
export function modifierBadgeOf(modifierId: string): string {
  return `[data-modifier-id="${modifierId}"]`;
}

/** §10.8: a face-down `BackrowView` has no `instanceId`. */
export function backrowRegionId(side: Side): string {
  return `backrow-${side}`;
}

// A18: overflow notices (§2.4; R315, R316, R317, R318) mirror `Board.tsx` and `Hand.tsx`.

/** A18: `data-kind="fatigue|libraryFull"`; `data-playing` marks its entry animation. */
export function pileNoticeId(side: Side): string {
  return `pile-notice-${side}`;
}

export type PileNoticeKind = "fatigue" | "libraryFull";

/** A18: rejected-library card; R316 controls its public face and outcome. */
export function overflowCardId(side: Side): string {
  return `overflow-card-${side}`;
}

/** A18/R317: hand-full notice. */
export function burnNoticeId(side: Side): string {
  return `burn-notice-${side}`;
}

export function burnCardId(side: Side): string {
  return `burn-card-${side}`;
}

export const GAME = "game";
export const BOARD = "board";
export const CONCEDE = "concede";
export const LOG = "log";
/** §9.3: where a refused action's reason is shown, relayed and never restated. */
export const ACTION_ERROR = "action-error";
/** §2.5: the draw-offer toast. */
export const DRAW_TOAST = "draw-toast";
export const PROMPT_MODAL = "prompt-modal";
export const PROMPT_SCRIM = "prompt-scrim";

// A16: mulligan, concede and draw-offer selectors (§2.1, §2.5; R36, R265, R266, R269) mirror `apps/web/src/game/contract.ts`.

export const CONCEDE_DIALOG = "concede-dialog";
export const CONCEDE_CONFIRM = "concede-confirm";
export const CONCEDE_CANCEL = "concede-cancel";

export const DRAW_OFFER_STATUS = "draw-offer-status";
export const DRAW_OFFER = "draw-offer";
export const DRAW_ACCEPT = "draw-accept";
export const DRAW_DECLINE = "draw-decline";
export const DRAW_OUTCOME = "draw-outcome";

export const MULLIGAN_OPPONENT_STATUS = "mulligan-opponent-status";
export const MULLIGAN_OPPONENT_READY = "mulligan-opponent-ready";
/** `data-returning` waits after the viewer's answer; it has no `data-prompt-kind`. */
export const MULLIGAN_WAITING = "mulligan-waiting";

export function mulliganWaitingCardId(instanceId: string): string {
  return `mulligan-waiting-card-${instanceId}`;
}

export const CLOCK_YOU = "clock-you";
export const CLOCK_OPPONENT = "clock-opponent";
export const PROMPT_CLOCK = "prompt-clock";
export const TURN_CLOCK = "turn-clock";

// A11: workshop selectors mirror `apps/web/src/game/deckbuilder/testids.ts` (§9.4; R250, R251, R253, R255, R256).

export const WORKSHOP = "workshop";
/** R256: save status is always present. */
export const SYNC_STATUS = "sync-status";
export const WORKSHOP_BACK = "workshop-back";
export const WORKSHOP_EMPTY = "workshop-empty";

export const DECK_LIST = "deck-list";
export const DECK_CAP = "deck-cap";

export function deckRowId(deckId: string): string {
  return `deck-row-${deckId}`;
}

/** R250: deck creation is disabled at its cap. */
export const DECK_NEW = "deck-new";
export const DECK_CAP_REASON = "deck-cap-reason";

export const TRIO_LIST = "trio-list";
export const TRIO_CAP = "trio-cap";
export const TRIO_NEW = "trio-new";
export const TRIO_CAP_REASON = "trio-cap-reason";

/** R253: trio readiness. */
export function trioRowId(trioId: string): string {
  return `trio-row-${trioId}`;
}

/** R251: pool entries carry legality, membership and comparison state. */
export const CARD_POOL = "card-pool";

export function poolCardId(catalogCardId: string): string {
  return `${CARD_POOL}-${catalogCardId}`;
}

export const DECK_EDITOR = "deck-editor";
export const DECK_NAME_INPUT = "deck-name-input";
export const DECK_COUNT = "deck-count";
export const DECK_DROP = "deck-drop";
export const DECK_CARDS = "deck-cards";

/** Compared-deck conflicts are shown, never removed. */
export function deckCardId(catalogCardId: string): string {
  return `deck-card-${catalogCardId}`;
}

export const DECK_CURVE = "deck-curve";
export const DECK_FOLD = "deck-fold";
export const DECK_STATUS = "deck-status";
/** R256: save errors are verbatim. */
export const DECK_SAVE_ERROR = "deck-save-error";

/** R255: deck-code output. */
export const DECK_COPY_CODE = "deck-copy-code";
export const DECK_CODE_OUTPUT = "deck-code-output";

export const DECK_DELETE = "deck-delete";
export const DECK_DELETE_CONFIRM = "deck-delete-confirm";
export const DECK_DELETE_CANCEL = "deck-delete-cancel";

/** R251: comparison accepts a trio, deck or none. */
export const DECK_COMPARE_SELECT = "deck-compare-select";

export function deckCompareChipId(deckId: string): string {
  return `deck-compare-${deckId}`;
}

export const DECK_CONFLICTS = "deck-conflicts";

export const DECK_VERDICT = "deck-verdict";

/** L1–L6 verdict sentences. */
export const LOADOUT_ERRORS = "loadout-errors";

/** Validator failures share a testid, so their rule, source, deck and card stay in attributes. */
export function loadoutErrorId(rule: string): string {
  return `loadout-error-${rule}`;
}

export const DECKBUILDER_LOADING = "deckbuilder-loading";
export const DECKBUILDER_ERROR = "deckbuilder-error";

/** Pool drags carry a catalog id, not the board's click target. */
export const DECK_DRAG_MIME = "application/x-jackioh-card";

export const TRIO_EDITOR = "trio-editor";
export const TRIO_NAME_INPUT = "trio-name-input";

export function trioSlotId(slot: number): string {
  return `trio-slot-${String(slot)}`;
}

export function trioOpenDeckId(slot: number): string {
  return `trio-open-${String(slot)}`;
}

export const TRIO_VERDICT = "trio-verdict";
export const TRIO_COMPARE = "trio-compare";

/** Per-slot card conflict state. */
export function trioCardId(slot: number, catalogCardId: string): string {
  return `trio-card-${String(slot)}-${catalogCardId}`;
}

export const TRIO_DELETE = "trio-delete";
export const TRIO_DELETE_CONFIRM = "trio-delete-confirm";
export const TRIO_DELETE_CANCEL = "trio-delete-cancel";
/** R339: trio-code output. */
export const TRIO_COPY_CODE = "trio-copy-code";
export const TRIO_CODE_OUTPUT = "trio-code-output";

export const DECK_IMPORT_OPEN = "deck-import-open";
export const DECK_IMPORT = "deck-import";
export const DECK_IMPORT_INPUT = "deck-import-input";
export const DECK_IMPORT_PREVIEW = "deck-import-preview";
export const DECK_IMPORT_SUBMIT = "deck-import-submit";
export const DECK_IMPORT_CAP_REASON = "deck-import-cap-reason";
export const DECK_IMPORT_CANCEL = "deck-import-cancel";

/** R339, R340, R341: trio import. */
export const TRIO_IMPORT_OPEN = "trio-import-open";
export const TRIO_IMPORT = "trio-import";
export const TRIO_IMPORT_INPUT = "trio-import-input";
export const TRIO_IMPORT_PREVIEW = "trio-import-preview";
export function trioImportSlotId(slot: number): string {
  return `trio-import-slot-${String(slot)}`;
}
export const TRIO_IMPORT_SHARED = "trio-import-shared";
export const TRIO_IMPORT_SUBMIT = "trio-import-submit";
export const TRIO_IMPORT_CAP_REASON = "trio-import-cap-reason";
export const TRIO_IMPORT_CANCEL = "trio-import-cancel";
export const TRIO_IMPORT_ERROR = "trio-import-error";

// A13: invite and code-field selectors mirror `apps/web/src/auth/testids.ts` (BUILD M6-T1; §9.4).

/** §9.4; R191 rejects R104-excluded characters without dropping the value. */
export const INVITE_CODE_INPUT = "invite-code-input";
/** Submit only a complete, unpaused, non-rate-limited code. */
export const INVITE_SUBMIT = "invite-submit";
/** The server's refusal, rendered verbatim — §9.4's identical error is never paraphrased here. */
export const INVITE_ERROR = "invite-error";
export const INVITE_PAUSED = "invite-paused";
export const INVITE_NOT_NEEDED = "invite-not-needed";
export const INVITE_ATTEMPTS = "invite-attempts";
/** R192: rate-limit wait disables submit. */
export const INVITE_RATE_LIMITED = "invite-rate-limited";
export const INVITE_ACCOUNT_EMAIL = "invite-account-email";
export const INVITE_SIGN_OUT = "invite-sign-out";
export const INVITE_HELP = "invite-help";

export const CODE_FIELD = "code-field";
export const CODE_FIELD_PROGRESS = "code-field-progress";
export const CODE_FIELD_HINT = "code-field-hint";

export function codeFieldSegmentId(index: number): string {
  return `code-field-segment-${String(index)}`;
}

// Practice selectors mirror `apps/web/src/practice/testids.ts` (SPEC §9.9, R187; spec 13).

export const PRACTICE_SETUP = "practice-setup";

export function practiceDifficultyId(d: "easy" | "medium" | "hard"): string {
  return `practice-difficulty-${d}`;
}

export const PRACTICE_DECK = "practice-deck";
export const PRACTICE_DECK_HINT = "practice-deck-hint";
export const PRACTICE_DECK_PREVIEW = "practice-deck-preview";
/** R1373: Random's “More cards from the newest set” option. */
export const PRACTICE_LEAN_NEWEST = "practice-lean-newest";
export const PRACTICE_DECK_CURVE = "practice-deck-curve";
export function practiceDeckCardId(defId: string): string {
  return `practice-deck-card-${defId}`;
}
export const PRACTICE_START = "practice-start";
export const PRACTICE_LOADING = "practice-loading";
export const PRACTICE_ERROR = "practice-error";
export const PRACTICE_HUD = "practice-hud";
export const PRACTICE_THINKING = "practice-thinking";
export const PRACTICE_NEW_GAME = "practice-new-game";
export const PRACTICE_MENU = "practice-menu";
export const PRACTICE_LEAVE = "practice-leave";
/** R765: leave without saving. */
export const PRACTICE_LEAVE_CONFIRM = "practice-leave-confirm";
/** R765: save and leave for later resume. */
export const PRACTICE_LEAVE_SAVE = "practice-leave-save";
export const PRACTICE_LEAVE_STAY = "practice-leave-stay";
export const PRACTICE_RESUME_LOST = "practice-resume-lost";
export const PRACTICE_RESUME_BANNER = "practice-resume-banner";
export const PRACTICE_RESUME = "practice-resume";
/** R765: live online-game banner. */
export const LIVE_GAME_BANNER = "live-game-banner";
export const LIVE_GAME_REJOIN = "live-game-rejoin";
export const QUEUE_FOUND = "queue-found";
export const PRACTICE_RESULT = "practice-result";
export const PRACTICE_PLAY_AGAIN = "practice-play-again";
export const PRACTICE_CHANGE_SETUP = "practice-change-setup";
export const PRACTICE_VIEW_BOARD = "practice-view-board";
export const PRACTICE_OUTCOME = "practice-outcome";
/** R169: live-modifier count and panel. */
export const PRACTICE_MODIFIERS = "practice-modifiers";
export const PRACTICE_MODIFIERS_PANEL = "practice-modifiers-panel";

// Tutorial selectors mirror `apps/web/src/tutorial/testids.ts` (SPEC §9.10); lessons reuse the practice contract.

export const TUTORIAL_PATH = "tutorial-path";

export function tutorialLessonId(lessonId: string): string {
  return `tutorial-lesson-${lessonId}`;
}

export function tutorialStartId(lessonId: string): string {
  return `tutorial-start-${lessonId}`;
}

export const TUTORIAL_CONTINUE = "tutorial-continue";
export const TUTORIAL_PATH_TOGGLE = "tutorial-path-toggle";
/** R322: tutorial visibility controls. */
export const TUTORIAL_HIDE = "tutorial-hide";
export const TUTORIAL_PATH_HIDDEN = "tutorial-path-hidden";
export const TUTORIAL_SHOW = "tutorial-show";
export const TUTORIAL_HUD = "tutorial-hud";
export const TUTORIAL_STEP = "tutorial-step";
export const TUTORIAL_EXIT = "tutorial-exit";
export const TUTORIAL_OUTCOME = "tutorial-outcome";
/** Coach state, step and anchors. */
export const COACH = "coach";
export const COACH_ACK = "coach-ack";
export const COACH_RING = "coach-ring";
export const TUTORIAL_RESULT = "tutorial-result";
export const TUTORIAL_NEXT = "tutorial-next";
export const TUTORIAL_RETRY = "tutorial-retry";
export const TUTORIAL_BACK = "tutorial-back";
export const TUTORIAL_PLAY_PRACTICE = "tutorial-play-practice";
export const TUTORIAL_WHATS_NEXT = "tutorial-whats-next";
export const TUTORIAL_VIEW_BOARD = "tutorial-view-board";

// A14: selectors mirror `apps/web/src/cards/inspect/testids.ts` and deckbuilder `testids.ts` (SPEC §10.10; R277, R279, R280, R654, R660).

export const INSPECT_HOVER = "inspect-hover";
export const INSPECT_SHEET = "inspect-sheet";
export const INSPECT_DETAIL = "inspect-detail";
export const INSPECT_SCRIM = "inspect-scrim";
export const INSPECT_CLOSE = "inspect-close";
export const INSPECT_FACE = "inspect-face";
export const INSPECT_FACE_BASE = "inspect-face-base";
export const INSPECT_FACE_RADIANT = "inspect-face-radiant";
export const INSPECT_GLOSSARY = "inspect-glossary";
export const INSPECT_PRINTED = "inspect-printed";
export const INSPECT_REFS = "inspect-refs";
export const INSPECT_FLAVOUR = "inspect-flavour";
export const INSPECT_ARTIST = "inspect-artist";
export const INSPECT_STATS = "inspect-stats";
export const CARD_REF_TOOLTIP = "card-ref-tooltip";
export const INSPECT_REFS_FACE = ".inspect-refs-face";
export const RADIANT_MARK = ".cf-mark";
export const CARD_REF = ".cf-ref";
export const CARD_VALUE = ".cf-value";

export function slugOf(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export const DB_FILTERS = "db-filters";
export const DB_SEARCH = "db-search";

export function filterCostId(bucket: string): string {
  return `db-filter-cost-${bucket}`;
}

export function filterTypeId(type: string): string {
  return `db-filter-type-${slugOf(type)}`;
}

export function filterTagId(tag: string): string {
  return `db-filter-tag-${slugOf(tag)}`;
}

export function filterSetId(set: string): string {
  return `db-filter-set-${slugOf(set.replace(/\+/g, " plus"))}`;
}

export function filterRarityId(rarity: string): string {
  return `db-filter-rarity-${slugOf(rarity)}`;
}

export const DB_FILTER_OWNED = "db-filter-owned";
export const DB_FILTER_CLEAR = "db-filter-clear";
export const DB_FILTER_TOGGLE = "db-filter-toggle";
export const DB_SORT = "db-sort";
export const DB_SORT_DIR = "db-sort-dir";
export const DB_RESULT_COUNT = "db-result-count";
export const DB_EMPTY = "db-empty";

export function addPoolId(catalogCardId: string): string {
  return `db-add-${catalogCardId}`;
}

export const DB_DETAIL_ADD = "db-detail-add";
export const DB_SIDEBAR = "db-sidebar";

// A15: selectors mirror showcase constants, `Log.tsx` and inspect `testids.ts` (R97, R227, R502).

/** A15/R502: opponent play or cast-on-draw. */
export const SHOWCASE = "showcase";
export const SHOWCASE_CAPTION = "showcase-caption";
export const SHOWCASE_FACE = "showcase-face";
export const SHOWCASE_BACK = "showcase-back";
export const SHOWCASE_LIVE = "showcase-live";

export const LOG_CARD = "log-card";

export const BROWSABLE = '[data-browsable="true"]';
export const INSPECT_LIST_HOVER = "inspect-list-hover";
export const INSPECT_LIST_SHEET = "inspect-list-sheet";
export const INSPECT_LIST_COUNT = "inspect-list-count";
export const INSPECT_LIST_CARD = "inspect-list-card";
export const INSPECT_LIST_MORE = "inspect-list-more";
export const INSPECT_LIST_DETAIL = "inspect-list-detail";
export const INSPECT_LIST_BACK = "inspect-list-back";

// A16: selectors mirror play, series, picker and banner modules (SPEC §9.5; R257, R264, R330, R331, R333, R336, R338).

export type QueueMode = "bo1" | "bo3" | "random";

export const PLAY_QUEUE = "play-queue";
export const PLAY_LEAVE_QUEUE = "play-leave-queue";
export const PLAY_CREATE_ROOM = "play-create-room";
export const PLAY_ROOM_CODE = "play-room-code";
export const PLAY_ROOM_MODE = "play-room-mode";
export const PLAY_JOIN_INPUT = "play-join-code";
export const PLAY_JOIN_SUBMIT = "play-join-submit";
export const PLAY_STATUS = "play-status";
export const PLAY_ERROR = "play-error";
export const PLAY_DECK_SELECT = "play-deck-select";
export const PLAY_TRIO_SELECT = "play-trio-select";
export const PLAY_VERDICT = "play-choice-verdict";
/** R1372: All Random's “More cards from the newest set” option. */
export const PLAY_LEAN_NEWEST = "play-lean-newest";

export function playModeId(mode: QueueMode): string {
  return `play-mode-${mode}`;
}

export const SERIES_SCREEN = "series-screen";
export const SERIES_ERROR = "series-error";
export const SERIES_SCORE = "series-score";
/** R331: opponent-pick status never reveals the choice. */
export const SERIES_OPPONENT_STATUS = "series-opponent-status";
export const SERIES_PICK_CLOCK = "series-pick-clock";
export const SERIES_PICKER = "series-picker";
export const SERIES_LOCK_IN = "series-lock-in";
export const SERIES_OPEN_MATCH = "series-open-match";
export const SERIES_FORFEIT = "series-forfeit";
export const SERIES_FORFEIT_CONFIRM = "series-forfeit-confirm";
export const SERIES_RESULT = "series-result";

export function seriesDeckId(slot: number): string {
  return `series-deck-${String(slot)}`;
}

export function seriesPickId(slot: number): string {
  return `series-pick-${String(slot)}`;
}

export function seriesOpponentDeckId(slot: number): string {
  return `series-opponent-deck-${String(slot)}`;
}

export function seriesGameId(gameNo: number): string {
  return `series-game-${String(gameNo)}`;
}

export const SERIES_BANNER = "series-banner";
export const SERIES_BANNER_CONTINUE = "series-banner-continue";
export const SERIES_BANNER_RESULT = "series-banner-result";
export function seriesBannerYourDeckId(slot: number): string {
  return `series-banner-you-deck-${String(slot)}`;
}
export function seriesBannerOpponentDeckId(slot: number): string {
  return `series-banner-opponent-deck-${String(slot)}`;
}

// A19: face-down selectors mirror `apps/web/src/game/contract.ts` and inspect `testids.ts` (SPEC §10.10; R370, R371, R372).

export function unrevealedId(instanceId: string): string {
  return `unrevealed-${instanceId}`;
}

export const INSPECT_FACE_DOWN = "inspect-face-down";
export const INSPECT_FACE_DOWN_COST = "inspect-face-down-cost";
export const INSPECT_NOTE = "inspect-note";

// A20: activate and pile-play selectors mirror `apps/web/src/game/contract.ts` (R384, R510; B5 E11).

export function activateId(instanceId: string, ability?: string): string {
  return ability === undefined ? `activate-${instanceId}` : `activate-${instanceId}-${ability}`;
}

export function activateUsesId(instanceId: string, ability?: string): string {
  return ability === undefined ? `activate-uses-${instanceId}` : `activate-uses-${instanceId}-${ability}`;
}

export function powerOfId(instanceId: string): string {
  return `power-${instanceId}`;
}

export function pilePlayId(instanceId: string): string {
  return `pile-play-${instanceId}`;
}

// A21: almanac, footer and back selectors mirror their apps/web testid modules (R630).

export const ALMANAC = "almanac";
export const SITE_FOOTER = "site-footer";
export const SITE_FOOTER_ALMANAC = "site-footer-almanac";
export const SITE_FOOTER_PATCH_NOTES = "site-footer-patch-notes";
export const NAV_BACK = "nav-back";

// A22: public Statistics selectors mirror `apps/web/src/stats/testids.ts` (R654).

export const STATS_SCREEN = "stats-screen";
export const STATS_TAB_CARDS = "stats-tab-cards";
export const STATS_TAB_PLAYERS = "stats-tab-players";
export const STATS_SUMMARY_TILES = "stats-summary-tiles";
export const STATS_SUMMARY_TOTAL_GAMES = "stats-summary-total-games";
export const STATS_SUMMARY_BEST_CARD = "stats-summary-best-card";
export const STATS_SUMMARY_WORST_CARD = "stats-summary-worst-card";
export const STATS_PROVISIONAL_BANNER = "stats-provisional-banner";
export const STATS_FALLBACK_TOGGLE = "stats-fallback-toggle";
export const STATS_SEARCH_INPUT = "stats-search-input";
export const STATS_CARDS_TABLE = "stats-cards-table";
export const STATS_PLAYERS_TABLE = "stats-players-table";
export const STATS_DRILL_DOWN_MODAL = "stats-drill-down-modal";
export const STATS_DRILL_DOWN_CLOSE = "stats-drill-down-close";
export const SITE_FOOTER_STATS = "site-footer-stats";

