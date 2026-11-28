// E2E selector contract: BUILD M5-T1 to M5-T4 fix base testids and attributes, M8 exercises them; assumptions in e2e/README.md.
// It holds only what the web exports nowhere a spec can import: a test id that one of `apps/web/src`'s
// `testids.ts` files or `game/contract.ts`'s `testid` exports is imported from there (#5269), and
// apps/web/src/net/e2e-support.test.ts fails on a copy here, and on an export no file reads.

import type { EventType, Lane, PromptKind, Row, Side } from "./types.ts";
import { CARD_POOL } from "../../apps/web/src/game/deckbuilder/testids.ts";

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

export const PROMPT = "[data-prompt-kind]";

export function promptOf(kind: PromptKind): string {
  return `[data-prompt-kind="${kind}"]`;
}

export const ANIMATING = "[data-animating]";

/** #3366: on the game while the opening deal runs, well after `data-animating` clears; only at full motion. */
export const DEALING = "[data-dealing]";

export function animating(event: EventType): string {
  return `[data-animating="${event}"]`;
}

// A4–A5 selectors beyond the BUILD contract are listed in e2e/README.md.

/** A4: one option of an open prompt, keyed by `PendingOption.key` (SPEC §10.8). */
export function promptOptionId(key: string): string {
  return `prompt-option-${key}`;
}

export const PROMPT_SUBMIT = "prompt-submit";

export const PROMPT_X_INPUT = "prompt-x";

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
export const RADIANT = ".radiant";
/** #991: a field card the hovered or selected hand card's preview counts (apps/web/src/game/glow.ts `countedAttr`). */
export const COUNTED = '[data-counted="true"]';

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

// A5: regions and animation targets mirror `apps/web/src/game/animations.ts` and `contract.ts`.

/** §10.8: an opponent hand has no card ids. */
export function handRegionId(side: Side): string {
  return `hand-${side}`;
}

export function libraryId(side: Side): string {
  return `library-${side}`;
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
/** §9.3: where a refused action's reason is shown, relayed and never restated. */
export const ACTION_ERROR = "action-error";
export const DRAW_TOAST = "draw-toast";

// A16: mulligan, concede and draw-offer selectors (§2.1, §2.5; R36, R265, R266, R269) mirror `apps/web/src/game/contract.ts`.

export function mulliganWaitingCardId(instanceId: string): string {
  return `mulligan-waiting-card-${instanceId}`;
}

export const CLOCK_YOU = "clock-you";
export const CLOCK_OPPONENT = "clock-opponent";
export const PROMPT_CLOCK = "prompt-clock";
/** §9.5: the opponent's disconnect grace while its socket is away (apps/web Clock.tsx `GraceLine`). */
export const GRACE_OPPONENT = "grace-opponent";

// A11: workshop selectors mirror `apps/web/src/game/deckbuilder/testids.ts` (§9.4; R250, R251, R253, R255, R256).

export function deckRowId(deckId: string): string {
  return `deck-row-${deckId}`;
}

/** R253: trio readiness. */
export function trioRowId(trioId: string): string {
  return `trio-row-${trioId}`;
}

export function poolCardId(catalogCardId: string): string {
  return `${CARD_POOL}-${catalogCardId}`;
}

/** Compared-deck conflicts are shown, never removed. */
export function deckCardId(catalogCardId: string): string {
  return `deck-card-${catalogCardId}`;
}

export function deckCompareChipId(deckId: string): string {
  return `deck-compare-${deckId}`;
}

/** Validator failures share a testid, so their rule, source, deck and card stay in attributes. */
export function loadoutErrorId(rule: string): string {
  return `loadout-error-${rule}`;
}

export function trioOpenDeckId(slot: number): string {
  return `trio-open-${String(slot)}`;
}

/** Per-slot card conflict state. */
export function trioCardId(slot: number, catalogCardId: string): string {
  return `trio-card-${String(slot)}-${catalogCardId}`;
}

export function trioImportSlotId(slot: number): string {
  return `trio-import-slot-${String(slot)}`;
}

// Practice selectors mirror `apps/web/src/practice/testids.ts` (SPEC §9.9, R187; spec 13).

export function practiceDifficultyId(d: "easy" | "medium" | "hard"): string {
  return `practice-difficulty-${d}`;
}

export const PRACTICE_RESUME_BANNER = "practice-resume-banner";
export const PRACTICE_RESUME = "practice-resume";
export const LIVE_GAME_BANNER = "live-game-banner";
export const LIVE_GAME_REJOIN = "live-game-rejoin";

// Tutorial selectors mirror `apps/web/src/tutorial/testids.ts` (SPEC §9.10); lessons reuse the practice contract.

export function tutorialLessonId(lessonId: string): string {
  return `tutorial-lesson-${lessonId}`;
}

export function tutorialStartId(lessonId: string): string {
  return `tutorial-start-${lessonId}`;
}

export function fieldGuideEntryId(entryId: string): string {
  return `field-guide-entry-${entryId}`;
}

// A14: selectors mirror `apps/web/src/cards/inspect/testids.ts` and deckbuilder `testids.ts` (SPEC §10.10; R277, R279, R280, R654, R660).

export const CARD_REF_TOOLTIP = "card-ref-tooltip";
export const INSPECT_REFS_FACE = ".inspect-refs-face";
export const RADIANT_MARK = ".cf-mark";
export const CARD_REF = ".cf-ref";
export const CARD_VALUE = ".cf-value";

export function filterCostId(bucket: string): string {
  return `db-filter-cost-${bucket}`;
}

export function filterOwnershipId(view: "all" | "owned" | "missing"): string {
  return `db-filter-ownership-${view}`;
}

export function addPoolId(catalogCardId: string): string {
  return `db-add-${catalogCardId}`;
}

// A15: selectors mirror showcase constants, `Log.tsx` and inspect `testids.ts` (R97, R227, R502).

/** A15/R502: opponent play or cast-on-draw. */
export const SHOWCASE = "showcase";
export const SHOWCASE_CAPTION = "showcase-caption";
export const SHOWCASE_FACE = "showcase-face";
export const SHOWCASE_BACK = "showcase-back";
export const SHOWCASE_LIVE = "showcase-live";

export const LOG_CARD = "log-card";

export const BROWSABLE = '[data-browsable="true"]';

// A16: selectors mirror play, series, picker and banner modules (SPEC §9.5; R257, R264, R330, R331, R333, R336, R338).

export type QueueMode = "bo1" | "bo3" | "random";

export const PLAY_QUEUE = "play-queue";
export const PLAY_STATUS = "play-status";
export const PLAY_DECK_SELECT = "play-deck-select";
export const PLAY_TRIO_SELECT = "play-trio-select";
export const PLAY_VERDICT = "play-choice-verdict";
/** R1578000: the Play screen and a visitor's sign-in panel. R3729000: the hub of four groups of cards, the panel an Online card opens, the Private lobby's dialog, and the links under the groups. */
export const PLAY_SCREEN = "play-screen";
export const PLAY_HUB = "play-hub";
export const PLAY_QUEUE_PANEL = "play-queue-panel";
export const PLAY_LOBBY = "play-lobby";
export const PLAY_LOBBY_DIALOG = "play-lobby-dialog";
/** R3729000: a group's section, `online`, `daily`, `friends` or `solo`. */
export function playGroupId(group: "online" | "daily" | "friends" | "solo"): string {
  return `play-group-${group}`;
}
/** R1578000: a visitor with no account is offered sign-in where the online controls would be. */
export const PLAY_SIGN_IN = "play-sign-in";

export function playModeId(mode: QueueMode): string {
  return `play-mode-${mode}`;
}

export const SERIES_SCREEN = "series-screen";
export const SERIES_SCORE = "series-score";
/** R331: opponent-pick status never reveals the choice. */
export const SERIES_OPPONENT_STATUS = "series-opponent-status";
export const SERIES_PICKER = "series-picker";
export const SERIES_LOCK_IN = "series-lock-in";
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

// A20: activate and pile-play selectors mirror `apps/web/src/game/contract.ts` (R384, R510; B5 E11).

export function activateId(instanceId: string, ability?: string): string {
  return ability === undefined ? `activate-${instanceId}` : `activate-${instanceId}-${ability}`;
}

export function activateUsesId(instanceId: string, ability?: string): string {
  return ability === undefined ? `activate-uses-${instanceId}` : `activate-uses-${instanceId}-${ability}`;
}

// A21: almanac, footer and back selectors mirror their apps/web testid modules (R630).

export const ALMANAC = "almanac";
export const SITE_FOOTER = "site-footer";
export const SITE_FOOTER_ALMANAC = "site-footer-almanac";
export const SITE_FOOTER_PATCH_NOTES = "site-footer-patch-notes";
export const NAV_BACK = "nav-back";

// A22: public Statistics selectors mirror `apps/web/src/stats/testids.ts` (R654).

export const SITE_FOOTER_STATS = "site-footer-stats";

// A23: rematch selectors mirror `rematchTestid` in `apps/web/src/routes/Rematch.tsx` (§9.5, R672).

export const REMATCH_OFFER = "rematch-offer";
export const REMATCH_DOUBLE = "rematch-double";
export const REMATCH_INCOMING = "rematch-incoming";
export const REMATCH_STATUS = "rematch-status";

// R612's banner mirrors matchTestid in apps/web/src/routes/match.tsx: whether the game moves the rating.
export const MATCH_RANKS = "match-ranks";

// R768000: the replay selectors mirror `replayTestid` and `gifTestid` in `apps/web/src/replay/`, the
// result panel's in `matchTestid` (`apps/web/src/routes/match.tsx`), and the recent games' in
// `replaysTestid` (`apps/web/src/routes/replays.tsx`).

export const MATCH_WATCH_REPLAY = "match-watch-replay";
export const MATCH_SHARE_LAST_TURN = "match-share-last-turn";
export const REPLAY_PLAYER = "replay-player";
export const REPLAY_FORWARD_STEP = "replay-forward-step";
export const REPLAY_BACK_STEP = "replay-back-step";
export const REPLAY_READOUT = "replay-readout";
export const REPLAY_BACK = "replay-back";
export const REPLAY_END = "replay-end";
export const REPLAY_SCRUBBER = "replay-scrubber";
export const REPLAY_GIF_SHARE = "replay-gif-share";
export const REPLAYS_ROOT = "replays";

export function replaysOnlineId(matchId: string): string {
  return `replays-online-${matchId}`;
}

// #936 (SPEC S8): Play together's selectors mirror `apps/web/src/together/testids.ts`.

/** R3732002: a tab of the Friends page, by section (`friends|requests|recent|blocked|shared`). */
export function togetherTabId(section: string): string {
  return `together-tab-${section}`;
}

export function togetherChallengeModeId(mode: QueueMode): string {
  return `together-challenge-mode-${mode}`;
}

export function togetherChallengeKindId(kind: "live" | "daily"): string {
  return `together-challenge-kind-${kind}`;
}

// #3727: daily games (`apps/web/src/daily/testids.ts`).
/** The Play screen's Daily games, and its badge of the games waiting on the player. */
export const PLAY_DAILY = "play-daily";
export const PLAY_DAILY_BADGE = "play-daily-badge";

export function dailyKindId(kind: "friend" | "queue"): string {
  return `daily-kind-${kind}`;
}

export function dailyModeId(mode: "bo1" | "random"): string {
  return `daily-mode-${mode}`;
}

/** `skeletonTestid.root` (apps/web/src/skeleton/Skeleton.tsx): a list still loading. */
export const SKELETON = "skeleton";

export const REPLAYS_SHARED_WITH_ME = "replays-shared-with-me";

// R738, R643: the opponent's aim and the emote layer, mirroring the literals in
// apps/web/src/game/aim/OpponentAim.tsx and apps/web/src/emotes/ui.tsx.
export const OPPONENT_AIM = "opponent-aim";
export const OPPONENT_AIM_ARROW = "opponent-aim-arrow";
export const EMOTE_BUBBLE = "emote-bubble";
export const EMOTE_MUTE_MENU = "emote-mute-menu";
export const EMOTE_MUTE = "emote-mute";
/** The emote picker (`EmoteMenu`), one `emote-<id>` item per emote of the hand it offers. */
export const EMOTE_MENU = "emote-menu";

export function emoteItemId(emote: string): string {
  return `emote-${emote}`;
}

// R2990000, R2990004: the sideline strip, mirroring `sidelineTestid` in apps/web/src/sideline/testids.ts.
// #936 S8: Duos co-op selectors mirror `duosTestid` in `apps/web/src/duos/testids.ts` (B60–B65).

export type DuosPingKind = "thumbsUp" | "thumbsDown" | "portal" | "careful" | "killThis";

export function duosPingId(kind: DuosPingKind): string {
  return `duos-ping-${kind}`;
}

// #1310 (SPEC §14): the economy's selectors, for spec 45. They mirror `accountTestid`, `devButtonTestid`,
// `devToolsTestid`, `shopTestid`, `packTestid`, `questsTestid` and `collectionTestid`, in `apps/web/src/`
// routes/account.tsx, routes/DevButton.tsx, routes/DevTools.tsx, shop/shop.ts, packs/packs.ts,
// routes/QuestsPanel.tsx and collection/collection.ts;
// this project can import none of them (each reaches `@jackioh/*`, which only the client's own build resolves).

/** R3156000: the front screen's Dev button, for a dev account on a computer, and the dialog it opens. */
export const LANDING_DEV = "landing-dev";
export const DEV_PANEL = "dev-panel";

/** §14.10: the Dev tools panel; its status line says "Done." or the server's refusal word for word. */
export const DEV_TOOLS = "dev-tools";
export const DEV_TOOLS_FILL_CARD_BAR = "dev-tools-fill-card-bar";
export const DEV_TOOLS_FILL_COSMETIC_BAR = "dev-tools-fill-cosmetic-bar";
export const DEV_TOOLS_GIVE_QUEST = "dev-tools-give-quest";
export const DEV_TOOLS_STATUS = "dev-tools-status";

/** §14.1, §14.3: the Jlore shop. `pack` is a shipped set's name, or `global`. */
export const SHOP_PANEL = "shop-panel";
export const SHOP_WALLET = "shop-wallet";

export function shopBuyId(pack: string): string {
  return `shop-buy-${pack}`;
}

/** §14.5: a card token held, spent from a locked card's preview in the Collection (R2730000). */
export const TOKEN_HELD_CARD = "token-held-card";

/** Issue #3734, R3734000: the Collection's Cards tab's grid and the Decks tab. */
export const COLLECTION_GRID = "collection-grid";
export const COLLECTION_DECKS = "collection-decks";

export function collectionCardId(cardId: string): string {
  return `collection-card-${cardId}`;
}

/** Every locked card in the Collection's grid, whatever its card; the card is the testid's suffix. */
export const COLLECTION_LOCKED_CARDS = '[data-testid^="collection-card-"][data-owned="false"]';

export const COLLECTION_PREVIEW = "collection-preview";

export function collectionTokenOfferId(id: string): string {
  return `collection-token-offer-${id}`;
}

export function collectionTokenOfferConfirmId(id: string): string {
  return `collection-token-offer-confirm-${id}`;
}

/** §14.3: the packs held, and the scene that opens one (`data-grant` on a slot is `card`, `cosmetic` or `bars`). */
export function packsOpenId(pack: string): string {
  return `packs-open-${pack}`;
}

export const PACK_SCENE = "pack-scene";
export const PACK_SLOTS = "pack-slots";
export const PACK_SKIP_ALL = "pack-skip-all";
export const PACK_CLOSE = "pack-close";

export function packSlotId(index: number): string {
  return `pack-slot-${String(index)}`;
}

export function packSlotOutcomeId(index: number): string {
  return `pack-slot-${String(index)}-outcome`;
}

/** §14.9: one held quest (`data-quest-id`), on `/collection`. */
export const QUESTS_QUEST = "quests-quest";

/** #1307: `/collection`'s wallet, and each shipped set's track (its level, and the XP bar's `<progress>`). */
export const COLLECTION_SCREEN = "collection-screen";
export const COLLECTION_JLORE = "collection-jlore";

export function collectionTrackId(set: string): string {
  return `collection-track-${set}`;
}

export function collectionTrackLevelId(set: string): string {
  return `collection-track-level-${set}`;
}

export function collectionTrackXpId(set: string): string {
  return `collection-track-xp-${set}`;
}

/** A card face's root (apps/web/src/cards/CardFace.tsx); `data-full-art="true"` when it prints full art (R1017001, R1313000). */
export const CARD_FACE = ".cf";
