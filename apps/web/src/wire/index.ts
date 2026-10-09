// `@jackioh/shared` for the web client (docs/v0.3.0/SURFACE.md §10.4): the wire types and the small
// helpers every client layer imports. apps/web/vite.config.ts, vitest.config.ts and tsconfig.json
// alias the specifier here, so the files that import it need no edit.
//
// THE TYPES ARE GENERATED. `cargo test -p jackioh-engine --features ts export_bindings` writes one
// file per Rust wire type into `./generated/` (ts-rs, SURFACE §5.1), so the client is typed against
// exactly the JSON the Rust engine and server produce; CI regenerates them and fails on a diff. Two
// names are not generated, because Rust spells them as aliases or as more than one type, and are
// kept here by hand: `CardDefs` (a type alias in Rust) and `BackrowView` (Rust's is the face-up or
// face-down zone; the client's also admits `null`, an empty zone, as TypeScript's always did).
//
// THE HELPERS ARE HAND-KEPT. The runtime values the client imports (`keywordKey`, `fillParams`,
// `readCodeInput`, `emoteGate`, `parseAim`, the stats formatters, …) are the TypeScript sources the
// server's Rust port was made from, kept here as the client's own (`./catalog.ts`, `./codes.ts`,
// `./emotes.ts`, `./aim.ts`, `./stats.ts`). The server's copies are `crates/engine/src/wire/*.rs`;
// both read `crates/engine/tests/fixtures/code-input-cases.json` (`./codes.test.ts`), so the reading
// of a typed code cannot drift between them.

import type { BackrowView as BackrowSlot } from "./generated/BackrowView.ts";
import type { CardDef } from "./generated/CardDef.ts";
import type { GameEventType } from "./generated/GameEventType.ts";

// ---------------------------------------------------------------------------------------------
// generated: catalog-types, actions, events, view
// ---------------------------------------------------------------------------------------------

export type { Action } from "./generated/Action.ts";
export type { ActionBody } from "./generated/ActionBody.ts";
export type { ActionInput } from "./generated/ActionInput.ts";
export type { ActionType } from "./generated/ActionType.ts";
export type { ActivationView } from "./generated/ActivationView.ts";
export type { CardCost } from "./generated/CardCost.ts";
export type { CardDef } from "./generated/CardDef.ts";
export type { CardFace } from "./generated/CardFace.ts";
export type { CardMark } from "./generated/CardMark.ts";
export type { CardType } from "./generated/CardType.ts";
export type { CardView } from "./generated/CardView.ts";
export type { CatalogQuery } from "./generated/CatalogQuery.ts";
export type { CopiedTextView } from "./generated/CopiedTextView.ts";
export type { CraftEffect } from "./generated/CraftEffect.ts";
export type { CraftHat } from "./generated/CraftHat.ts";
export type { CraftHatKind } from "./generated/CraftHatKind.ts";
export type { CraftHatPrice } from "./generated/CraftHatPrice.ts";
export type { CraftKeyword } from "./generated/CraftKeyword.ts";
export type { CraftKeywordPrice } from "./generated/CraftKeywordPrice.ts";
export type { CraftPreview } from "./generated/CraftPreview.ts";
export type { CraftRecipe } from "./generated/CraftRecipe.ts";
export type { CraftVerb } from "./generated/CraftVerb.ts";
export type { CraftVerbPrice } from "./generated/CraftVerbPrice.ts";
export type { Enchantment } from "./generated/Enchantment.ts";
export type { FusedIngredient } from "./generated/FusedIngredient.ts";
export type { GameEvent } from "./generated/GameEvent.ts";
export type { GameEventType } from "./generated/GameEventType.ts";
export type { GameOverReason } from "./generated/GameOverReason.ts";
export type { HeroPowerView } from "./generated/HeroPowerView.ts";
export type { HeroView } from "./generated/HeroView.ts";
export type { Keyword } from "./generated/Keyword.ts";
export type { KeywordKind } from "./generated/KeywordKind.ts";
export type { LibraryEntryView } from "./generated/LibraryEntryView.ts";
export type { LibraryOverflowOutcome } from "./generated/LibraryOverflowOutcome.ts";
export type { LibraryView } from "./generated/LibraryView.ts";
export type { ModeDecl } from "./generated/ModeDecl.ts";
export type { ModifierView } from "./generated/ModifierView.ts";
export type { MulliganView } from "./generated/MulliganView.ts";
export type { Param } from "./generated/Param.ts";
export type { PendingOption } from "./generated/PendingOption.ts";
export type { PendingPromptView } from "./generated/PendingPromptView.ts";
export type { PendingView } from "./generated/PendingView.ts";
export type { PlayerId } from "./generated/PlayerId.ts";
export type { PlayerView } from "./generated/PlayerView.ts";
export type { PreviewValue } from "./generated/PreviewValue.ts";
export type { PrintedRarity } from "./generated/PrintedRarity.ts";
export type { PromptKind } from "./generated/PromptKind.ts";
export type { QuestView } from "./generated/QuestView.ts";
export type { Rarity } from "./generated/Rarity.ts";
export type { Row } from "./generated/Row.ts";
export type { Selection } from "./generated/Selection.ts";
export type { SetName } from "./generated/SetName.ts";
export type { SideView } from "./generated/SideView.ts";
export type { Tag } from "./generated/Tag.ts";
export type { TargetDecl } from "./generated/TargetDecl.ts";
export type { TargetFilter } from "./generated/TargetFilter.ts";
export type { Tuning } from "./generated/Tuning.ts";
export type { TuningChange } from "./generated/TuningChange.ts";
export type { UnitView } from "./generated/UnitView.ts";
export type { Zone } from "./generated/Zone.ts";
export type { ZoneChoice } from "./generated/ZoneChoice.ts";
export type { ZoneRef } from "./generated/ZoneRef.ts";

// ---------------------------------------------------------------------------------------------
// by hand: the names Rust does not generate
// ---------------------------------------------------------------------------------------------

/** Every card definition by catalog id (Rust's `CardDefs`, an alias ts-rs does not export). */
export type CardDefs = Record<string, CardDef>;

/** One backrow zone as a view draws it: its card face up or face down, or `null` when it is empty. */
export type BackrowView = BackrowSlot | null;

/** `Omit` over each member of a union, so a discriminated union keeps its discriminant. */
export type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

/**
 * Every event type, for the animation and sound tables' tests (BUILD M5-T4), in the engine's order
 * (`GameEventType::ALL`).
 *
 * `satisfies readonly GameEventType[]` below is only a SUBSET check — it rejects a member that is
 * not a `GameEventType` and says nothing about one that is missing. `GameEventTypesAreExhaustive`
 * underneath the array closes that direction, so a new event in the Rust union fails `tsc` here.
 */
export const GAME_EVENT_TYPES = [
  "cardPlayed",
  "cardResolved",
  "summoned",
  "damage",
  "healthLost",
  "healed",
  "divineShieldLost",
  "destroyed",
  "enteredGraveyard",
  "exiled",
  "bounced",
  "burned",
  "fatigue",
  "libraryOverflow",
  "discarded",
  "drawn",
  "addedToHand",
  "shuffledIn",
  "buffed",
  "keywordGranted",
  "counterChanged",
  "costChanged",
  "modifierChanged",
  "radiantSet",
  "transformed",
  "fused",
  "positionSwitched",
  "controlChanged",
  "rotated",
  "swapped",
  "locked",
  "trapFired",
  "attackDeclared",
  "attackCancelled",
  "manaChanged",
  "turnStarted",
  "turnEnded",
  "turnAutoEnded",
  "promptOpened",
  "promptAnswered",
  "drawOffered",
  "drawAnswered",
  "gameOver",
  "cardAnnounced",
  "countered",
  "stolen",
  "unlocked",
  "activated",
  "animated",
  "deanimated",
  "crumbled",
  "degraded",
  "upgraded",
  "numberChanged",
  "redirected",
  "healthSet",
  "questProgressed",
  "questCompleted",
  "rolledBack",
  "chaosRolled",
  "flickered",
  "drawLimited",
  "turnCutShort",
  "marked",
  "glitched",
  "translated",
  "damageAbsorbed",
] as const satisfies readonly GameEventType[];

/**
 * The other half of the check: a `GameEventType` missing from `GAME_EVENT_TYPES` is a COMPILE ERROR
 * naming it — `Type '"newThing"' does not satisfy the constraint 'never'`. A type alias, so it costs
 * no runtime bytes.
 */
type NoneMissing<T extends never> = T;

export type GameEventTypesAreExhaustive = NoneMissing<Exclude<GameEventType, (typeof GAME_EVENT_TYPES)[number]>>;

// ---------------------------------------------------------------------------------------------
// hand-kept helpers
// ---------------------------------------------------------------------------------------------

// The catalog vocabulary's runtime half. Its types are the generated ones above; `./catalog.ts`
// declares its own copies only for its functions' signatures, which are the same shapes.
export {
  KEYWORD_KINDS,
  PARAM_PLACEHOLDER,
  PLAYER_IDS,
  SHIPPED_SETS,
  armorOf,
  fillParams,
  hasKeyword,
  keywordKey,
  newestShippedSet,
  opponentOf,
  paramPlaceholders,
  setShips,
} from "./catalog.ts";

export * from "./codes.ts";
export * from "./emotes.ts";
export * from "./aim.ts";
export * from "./stats.ts";
