// What one player is allowed to see, and nothing else (SPEC §10.8). The client renders this view
// and never holds rules or hidden information (CLAUDE.md rule 7, §9.1's trust model).
//
// The view is built by *copying* what the viewer is entitled to, never by deleting fields from a
// state clone: a field added to `GameState` stays invisible until this file names it. Five rules do
// all the work here.
//   - §9.1 hidden list: library order, the opponent's library, the opponent's hand, face-down traps.
//     Both libraries and the opponent's hand travel as counts; graveyards, exile and Field Spells are
//     public. R310–R312: the viewer's own library also travels as a list without order of what the
//     viewer was shown going in (`ownLibrary.ts`) — definitions, faces and counts, never an instance
//     id — so R97's rule below still reads no library card, the owner's included.
//   - R33: a face-down trap is readable by its *current controller* only, so a steal, a board swap
//     or a rotation moves who may read it even though ownership never changed; a Field Trap that
//     has fired (`faceUp`) is public to both.
//   - R13, §3.2: the lower cards of a Stack pile are dormant and not on the field. The view shows
//     the top card and a count of what is buried under it, never a buried card's identity.
//   - R81: only the viewer's own prompt carries its options; the other player's prompt shows that
//     it is open and whose it is, nothing more.
//   - R97: the event stream is redacted, not truncated. An event that names a card the viewer may
//     not read keeps its type and its animation fields and shows `HIDDEN_ID` for that card.
//   - R227: a card set face-down took a fresh id, so the events that named its old id follow it to
//     its zone through the `formerId` that set it, and `formerId` itself travels only with the card.
//   - R177: more fields follow R97 — a prompt option offering a face-down card names it by id only,
//     a `costChanged` on an unreadable card hides its cost and a `buffed` one its amounts, and a
//     `transformed` whose new card is unreadable hides the card it replaced, as does one whose old
//     card was unreadable where it ceased to exist (`hiddenFrom`), whatever became of its
//     replacement since.
//   - R169: the player modifiers (§10.1 `mods`) travel on both seats as `{ id, label }`, because
//     every one of them is installed by a card played FACE-UP and `modifierChanged` is already
//     public in both directions. Face-up, not "by a Cry": #35 and #78 are Spells and can never have
//     one, #64 and #79 install theirs without one, and only #77 is a Cry. The caption is built from
//     the modifier's own kind and numbers and never from its `sourceId`, so no card identity can
//     leave through a badge.
//   - R434: once the game is over, the opponent's hand travels in full, as the owner's does; the
//     libraries, face-down traps and R97's event redaction stay as they were.
//   - R437: a mark (an effect aimed at the card that still waits, #50's pending steal) rides every
//     view of the card in both seats, a face-down card's back included (`marks.ts`).
//   - R195, R280: two things the engine works out for a card ride on its view. `conditionActive`
//     (the yellow glow) on the viewer's own cards only; `preview` (what a formula comes to now) on
//     every card view the viewer may read — the viewer's hand, the top of a unit pile and a backrow
//     card face-up to the viewer — and on no other (`preview.ts` owns that rule).
//
// Stats are never read off an instance: `layers.unitView` recomputes every stat and keyword on read
// (§10.4), so no stored total ever reaches the client.

import type {
  BackrowView,
  CardDef,
  CardMark,
  CardView,
  GameEvent,
  HeroPowerView,
  ModifierView,
  MulliganView,
  PendingOption,
  PendingView,
  PlayerId,
  PlayerView,
  PreviewValue,
  Row,
  SideView,
  TuningChange,
  UnitView,
  Zone,
} from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { announcedFaceDownTo } from "./announce";
import { findDef } from "./catalog";
import { hasExertion } from "./combat";
import { conditionActive } from "./condition";
import { heroArmorOf } from "./damage";
import { cardTypeOf } from "./faces";
import { handKeywordsView, instanceDataView } from "./instanceView";
import { echoGrantOf } from "./echo";
import { statsWithBuffs, unitView as unitLayers } from "./layers";
import { costRuleModifierLabel, enchantNextSpellLabel } from "./costRules";
import { NEXT_REFRESH_MODIFIER_ID, effectiveCost, modifierIsLive } from "./mana";
import {
  findInstance,
  type CardInstance,
  type GameState,
  type Pile,
  type PlayerModifier,
  type PendingChoice,
  type PlayerState,
  type PromptOption,
} from "./state";
import { gradeName } from "./subsystems/comboIndex";
import { copiedTextOf, textFaceOf } from "./subsystems/copiedText";
import { paramsView } from "./params";
import { activationViewsFor } from "./subsystems/activate";
import { syncFusedScripts } from "./subsystems/fuse";
import { powerCostOf, powerOf, usedThisTurn } from "./subsystems/heroPower";
import { questViewOf } from "./subsystems/quests";
import { marksOn } from "./marks";
import { ownLibraryView } from "./ownLibrary";
import { plagueOn } from "./plague";
import { backrowIsPublic, isFaceDown, previewOf } from "./preview";
import { mulliganPromptFor, returnedAwaitingShuffle } from "./setup";
import { standingDrawOffer } from "./turn";
import { beneathAt, carriedAt, carriedUnitsOf, homeOf, isReserved, slotOf, slotsOf } from "./zones";
import { isAnimated } from "./animated";

/** §10.8, §10.10: how many of the most recent events the view carries for animation. */
export const VIEW_EVENT_LIMIT = 32;

/**
 * R97's sentinel: the identity an event carries in place of a card the viewer may not read. Real
 * ids are `c<n>` for instances and catalog ids like `core-043` for definitions, so this collides
 * with neither and the client can test for it — a redacted event still animates, as a card back.
 */
export const HIDDEN_ID = "hidden";

/** R97, §9.1: the slot a shuffled-in card landed in would give away library order, to either side. */
const HIDDEN_POSITION = -1;

/**
 * R177: the cost a `costChanged` event reports for a card the viewer may not read. A cost is a
 * property of the card as much as its definition is — the opponent's hand is a count (§10.8) — and
 * a sequence of costs over a library would spell out its order (§9.1).
 */
const HIDDEN_COST = -1;

/** R385: the count a `counterChanged` "brittle" reports for a card the viewer may not read. */
const HIDDEN_COUNT = -1;

/** R177: what a prompt option names when it offers a card the chooser may not read (§10.8, R33). */
export const HIDDEN_OPTION_LABEL = "Face-down card";

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

/** §10.8, R33: a card in the backrow that this viewer sees only as a face-down card. */
function isFaceDownTo(state: GameState, card: CardInstance, viewer: PlayerId): boolean {
  const zone = card.zone;
  return zone.z === "field" && zone.row === "backrow" && !backrowIsPublic(state, card, viewer);
}

/**
 * R177: the card that took each vanished card's place, read off the events that replaced it — a
 * Replace (`transformed`, §6.3, R35) or a Fuse (`fused`, R77) — so a card that has ceased to exist
 * can still be judged by a zone: its replacement's. `state.applied` holds every event a view can
 * show, so every replacement that matters to one is in it.
 */
type Replacements = {
  /** Each vanished card's replacement. */
  replacedBy: ReadonlyMap<string, string>;
  /** The players each replaced card was hidden from where it ceased to exist (`transformed.hiddenFrom`). */
  hiddenFrom: ReadonlyMap<string, readonly PlayerId[]>;
  /**
   * R224: the cards a mulligan returned that wait in setup's owed item for their shuffle-back, in no
   * pile (`setup.returnedAwaitingShuffle`). They are on their way to a library, so nobody reads them.
   */
  toLibrary?: ReadonlySet<string>;
};

function replacementsOf(events: readonly GameEvent[], state?: GameState): Replacements {
  const toLibrary = state === undefined ? undefined : new Set(returnedAwaitingShuffle(state));
  const replacedBy = new Map<string, string>();
  const hiddenFrom = new Map<string, readonly PlayerId[]>();
  for (const event of events) {
    if (event.type === "transformed" && event.newInstanceId !== event.instanceId) {
      replacedBy.set(event.instanceId, event.newInstanceId);
      if (event.hiddenFrom !== undefined) hiddenFrom.set(event.instanceId, event.hiddenFrom);
    }
    if (event.type === "fused") {
      for (const id of event.instanceIds) {
        if (id !== event.resultInstanceId) replacedBy.set(id, event.resultInstanceId);
      }
    }
    // R227: a card set face-down took a fresh id. It is the same card, so the events that named it
    // by its old id are judged by where it is now, exactly as they were before it moved: its draw
    // stays hidden while the trap is face-down and reads once the trap is public (R97). No
    // `hiddenFrom` is kept, since nothing ceased to exist.
    if ((event.type === "cardPlayed" || event.type === "summoned" || event.type === "controlChanged") && event.formerId !== undefined) {
      replacedBy.set(event.formerId, event.instanceId);
    }
  }
  return { replacedBy, hiddenFrom, ...(toLibrary === undefined || toLibrary.size === 0 ? {} : { toLibrary }) };
}

/**
 * R97: whether this viewer may read the identity of the card an event names, judged by where the
 * card sits *now* and not where it was — so a card drawn last turn and played this turn reads
 * openly in both events, and a unit bounced into the opponent's hand stops reading the moment it
 * lands there (§9.1).
 *
 * A card in the resolving zone is public: playing it was public, and R98 keeps it itself while it
 * sits there. A card the state no longer holds at all has ceased to exist, and R177 judges it by
 * the card that replaced it: a face-down trap #83 Transmogulate replaced keeps the secret its
 * replacement keeps, and a library #83 replaced still never reads, so its earlier `cardPlayed` or
 * `costChanged` cannot spell out what it was or in what order. With no replacement — a unit token
 * that left the field (R11), a card exiled out of existence (R86) — it was public when it went.
 */
function mayRead(state: GameState, viewer: PlayerId, instanceId: string, replaced: Replacements): boolean {
  const { replacedBy, hiddenFrom } = replaced;
  let id = instanceId;
  let card = findInstance(state, id);
  for (let hops = 0; card === undefined && hops < replacedBy.size; hops += 1) {
    // R177: a card that ceased to exist where this viewer could not read it — a library card, an
    // enemy face-down trap — stays unread for good. Its replacement may reach a public pile later,
    // and judged by that pile alone the card it replaced would read, though it never was public.
    if (hiddenFrom.get(id)?.includes(viewer) === true) return false;
    const next = replacedBy.get(id);
    if (next === undefined) break;
    id = next;
    card = findInstance(state, id);
  }
  // R224: a card the mulligan returned waits for its shuffle-back in no pile, and is a library card.
  if (card === undefined) return replaced.toLibrary?.has(id) !== true;
  const zone = card.zone;
  if (zone.z === "library") return false;
  if (zone.z === "hand") return zone.player === viewer;
  if (zone.z === "field" && zone.row === "backrow") return backrowIsPublic(state, card, viewer);
  // R448: a card waiting in the resolving zone to be set face-down is its player's alone (R33, R227).
  if (announcedFaceDownTo(state, card, viewer)) return false;
  // Units, graveyard, exile, `resolving` (R98) and `gone` (R11, R86) are all public.
  return true;
}

// ---------------------------------------------------------------------------
// Cards, units and the backrow
// ---------------------------------------------------------------------------

/**
 * R65: the cost as it stands now. An X card has no chosen X outside a play, so it reads 0. Patch
 * v0.2.0's instance data rides on every card view (`instanceView.ts`): each is built only for a card
 * the viewer may read where it is.
 */
function cardView(state: GameState, card: CardInstance): CardView {
  // B5 E33, R404: a quest line on the field, public as the card is.
  const quest = questViewOf(state, card);
  return {
    instanceId: card.id,
    defId: card.defId,
    radiant: card.radiant,
    cost: effectiveCost(state, card),
    ...instanceDataView(state, card),
    ...withMarks(state, card.id),
    ...(quest === null ? {} : { quest }),
  };
}

/**
 * R437: the marks a card carries while an effect aimed at it waits (#50's pending steal), on every
 * view of it and in both players' views — or no key at all, so an unmarked card looks as it did.
 */
function withMarks(state: GameState, instanceId: string): { marks?: CardMark[] } {
  const marks = marksOn(state, instanceId);
  return marks.length === 0 ? {} : { marks };
}

/**
 * R243, §10.8: a card in the viewer's own hand, in full — what it is made of beyond its printed
 * face as well. A Unit's stats are its face plus the permanent buffs it gained in hand (§10.4
 * layers 1, 3 and 4: #89 Corpse Eater's meals), since layer 2 and the auras are the field's; attack
 * floors at 0 as on the field. A #98 Heroic Power names the power it rolled as it arrived (R43,
 * R151), which its cost alone does not.
 */
function handCardView(state: GameState, card: CardInstance): CardView {
  const view = withCopies(cardView(state, card), state, card);
  const stats = cardTypeOf(state, card) === "Unit" ? statsWithBuffs(state, card) : null;
  const power = powerOf(card);
  // B5 E38: the keywords it gained in the hand or the deck, which it carries onto the field.
  const keywords = handKeywordsView(state, card);
  return {
    ...view,
    ...(stats === null ? {} : { attack: Math.max(0, stats.attack), health: stats.maxHealth }),
    ...(power === null ? {} : { power: power.name }),
    ...(keywords === null ? {} : { keywords }),
  };
}

/**
 * B5 E14, R399, R243: a copier's view carries the Spell text it has now (`CardView.copies`), with that
 * definition's declared numbers as they read on the card — or nothing, when it copies nothing. Asked
 * only where the viewer may read the card and R399 shows it: the owner's hand, and the resolving zone.
 */
function withCopies<T extends CardView>(view: T, state: GameState, card: CardInstance): T {
  const copy = copiedTextOf(state, card);
  if (copy === null) return view;
  const params = paramsView(state, textFaceOf(state, card));
  return { ...view, copies: { defId: copy.defId, radiant: copy.radiant, ...(params === null ? {} : { params }) } };
}

/**
 * R195, §10.8: the yellow glow rides on a card view as `conditionActive: true` or not at all — the
 * key is never `false`, so a card with no condition met looks exactly as it did before R195.
 */
function withCondition<T extends CardView>(view: T, active: boolean): T {
  return active ? { ...view, conditionActive: true } : view;
}

/**
 * R280, §10.8: what a card's formula comes to now rides on its view as `preview`, or not at all —
 * never `[]`. `preview.previewOf` owns where it may: this file asks only about a card it shows the
 * viewer (the viewer's own hand, the top of a unit pile, a backrow card face-up to the viewer).
 */
function withPreview<T extends CardView>(view: T, values: PreviewValue[] | null): T {
  return values === null ? view : { ...view, preview: values };
}

/**
 * B3.2, R384: a card's Activate abilities ride its controller's own view of it on the field
 * (`activate.activationViewsFor` owns where), or not at all — never `[]`.
 */
function withActivations<T extends CardView>(view: T, state: GameState, card: CardInstance, viewer: PlayerId): T {
  const activations = activationViewsFor(state, viewer, card);
  return activations === null ? view : { ...view, activations };
}

/**
 * The card that acts in a unit zone: the top of the pile (§3.2). `buried` is how many dormant cards
 * sit under it (R13) — a count, so no buried identity reaches either player.
 */
function unitViewOf(state: GameState, pile: Pile, viewer: PlayerId): UnitView | null {
  const top = pile[0];
  if (top === undefined) return null;
  const layers = unitLayers(state, top);
  // B3.1, R383: a Field Spell, Trap or Field Trap standing here as a Unit, and the backrow lane an
  // "Animated on your turn" card will go back to (that zone is `reserved` meanwhile).
  const home = isAnimated(state, top) ? homeOf(state, top.id) : undefined;
  const animated = isAnimated(state, top) ? { animated: home === undefined ? {} : { home: home.zone.lane } } : {};
  return {
    ...withActivations(
      withPreview(
        withCondition(cardView(state, top), conditionActive(state, top, viewer, "field")),
        previewOf(state, top, viewer, "field"),
      ),
      state,
      top,
      viewer,
    ),
    owner: top.owner,
    controller: top.controller,
    attack: layers.attack,
    maxHealth: layers.maxHealth,
    health: layers.health,
    keywords: layers.keywords,
    armor: layers.armor,
    position: layers.position,
    counters: { ...top.counters },
    buried: pile.length - 1,
    canAct: canAct(state, top),
    // R243, §6.3 Vanilla: the text is gone, which the definition the client reads does not say.
    ...(top.vanilla === true ? { vanilla: true as const } : {}),
    ...animated,
    // B5 E35: a status, public on the field like the unit itself.
    ...(top.berserk === true ? { berserk: true as const } : {}),
  };
}

/**
 * §4.1: "each unit has one exertion per turn: one attack or one position switch", so a unit can
 * still act while either exertion is unspent — a summoning-sick unit may still switch, and a unit
 * that cannot attack may still switch back. `combat.hasExertion` owns that rule, Deft Duelist's
 * two exertions included (#45); *which* of the two is legal is `combat.canAttack`'s answer and
 * `legalActions`', never the view's.
 */
function canAct(state: GameState, card: CardInstance): boolean {
  if (state.result !== null || state.phase !== "main") return false;
  if (state.pending !== null) return false;
  if (state.active !== card.controller) return false;
  return hasExertion(state, card, "attack") || hasExertion(state, card, "switch");
}

/**
 * A public backrow card names its owner and its controller, since R33's readability and #87's board
 * swap both turn on the controller and a stolen card sits in a backrow that is not its own. A
 * face-down zone is `{ faceDown: true, cost }`: §10.8 grants the non-controller that a zone is
 * occupied and what the card in it costs (R351, a deliberate reveal), and nothing more, so not even
 * the controller's name travels with it. The cost is the one the controller's own view shows, so
 * both players read the same number.
 */
function backrowView(state: GameState, card: CardInstance | null, viewer: PlayerId): BackrowView {
  if (card === null) return null;
  // B5 E19, R471: Plague Tokens are public wherever they sit, a face-down card's included; B5 E21: a
  // backrow pile shows how many cards lie under its top, as a unit pile does (R13, R447).
  const plague = plagueOn(card);
  const slot = slotOf(state, card);
  const under = slot === null ? 0 : beneathAt(state, slot).length;
  const buried = under === 0 ? {} : { buried: under };
  // R437: a mark on a face-down card rides its back, which is all the other player sees of it (R33).
  if (!backrowIsPublic(state, card, viewer)) {
    return {
      faceDown: true,
      cost: effectiveCost(state, card),
      ...(plague === 0 ? {} : { plague }),
      ...buried,
      ...withMarks(state, card.id),
    };
  }
  const grade = card.counters.grade;
  return {
    ...withActivations(
      withPreview(
        withCondition(cardView(state, card), conditionActive(state, card, viewer, "field")),
        previewOf(state, card, viewer, "field"),
      ),
      state,
      card,
      viewer,
    ),
    faceDown: false,
    type: cardTypeOf(state, card),
    // R372: the engine names the grade's letter, so no client works out which letter 3 is.
    counters: {
      ...(grade === undefined ? {} : { grade, gradeLetter: gradeName(grade) }),
      ...(plague === 0 ? {} : { plague }),
    },
    owner: card.owner,
    controller: card.controller,
    // R351, R371: the controller reads a face-down trap, and the view says the other player cannot.
    ...(isFaceDown(state, card) ? { unrevealed: true as const } : {}),
    ...buried,
  };
}

/** R446: the Units this side's carriers hold, by backrow lane, or nothing when none holds one. */
function carriedView(state: GameState, player: PlayerId, viewer: PlayerId): { carried?: (UnitView | null)[] } {
  if (carriedUnitsOf(state, player).length === 0) return {};
  return {
    carried: slotsOf(player, "backrow").map((ref) => {
      const unit = carriedAt(state, ref);
      return unit === null ? null : unitViewOf(state, [unit], viewer);
    }),
  };
}

/**
 * R43: a Heroic Power lives on its instance and it is a Field Spell (§8 #98), so it is public to
 * both players once it is on the field — the row §2's hero panel marks visible to both. The power,
 * its X and its use are `heroPower`'s to report, never re-derived here.
 *
 * It follows control, not ownership: a stolen Heroic Power powers its new controller's hero. So a
 * player can hold more than one — their own plus one taken with #36 radiant or #49 — and each is
 * separately once-per-turn, which is why this is a list and every entry carries its `instanceId`
 * for `activatePower` (§10.2). Board order: p1's backrow lane 1 to 5, then p2's.
 */
function heroPowersOf(state: GameState, player: PlayerId): HeroPowerView[] {
  const powers: HeroPowerView[] = [];
  for (const side of PLAYER_IDS) {
    for (const card of state.players[side].backrow) {
      if (card === null || card.controller !== player) continue;
      const power = powerOf(card);
      if (power === null) continue;
      powers.push({
        instanceId: card.id,
        defId: card.defId,
        name: power.name,
        x: powerCostOf(card),
        usedThisTurn: usedThisTurn(state, card),
      });
    }
  }
  return powers;
}

// ---------------------------------------------------------------------------
// Player modifiers (§10.1 `mods`, §10.3 `modifierChanged`)
// ---------------------------------------------------------------------------

/**
 * The discount half of `modifierLabel`, split out because `costDiscount` is the one kind whose
 * caption has to say *what* it applies to: R48's current-cost gate (#77), §8 #35's "next Spell",
 * and #78's flat "your cards" are three different sentences off one kind.
 */
function discountLabel(mod: Extract<PlayerModifier, { kind: "costDiscount" }>): string {
  const less = `cost${mod.oncePerTurn === true ? "s" : ""} ${mod.amount} less`;
  // R363, R432: #77's own words, "(4)+ Cost cards cost (1) less".
  if (mod.minCurrentCost !== undefined) return `(${mod.minCurrentCost})+ Cost cards cost (${mod.amount}) less`;
  if (mod.onlyType !== undefined) {
    return mod.oncePerTurn === true ? `Next ${mod.onlyType} ${less}` : `${mod.onlyType}s ${less}`;
  }
  return mod.oncePerTurn === true ? `Next card ${less}` : `Your cards ${less}`;
}

/**
 * A badge caption for one modifier, built from the modifier alone. The switch is exhaustive over
 * `PlayerModifier["kind"]` on purpose: with no `default`, a new kind does not compile until someone
 * decides what the player is told about it.
 *
 * `sourceId` (#79 Twinspell's instance) is deliberately not read here: it is a card id, and the view
 * must not hand either seat an identity through a badge. `echo` is the grant as it stands
 * (`echo.echoGrantOf`), a number read off the permanent's current face (R209, §5.2).
 */
function modifierLabel(state: GameState, mod: PlayerModifier, echo: number): string {
  switch (mod.kind) {
    case "costDiscount":
      return discountLabel(mod);
    case "echoNextSpell":
      return `Next Spell gains Echo +${echo}`;
    case "radiantFirstCheapCard":
      return `First card costing ${mod.maxCost} or less becomes Radiant`;
    case "comboDraw":
      return `Your cards gain "Combo: draw ${mod.amount}"`;
    case "quickstrikerDamage":
      return `Your cards gain "Combo X: X damage to the enemy hero"`;
    // B5 E10, R456: how much of the turn is left.
    case "turnEnds":
      return mod.actionsLeft === 0
        ? "Your turn ends"
        : `Your turn ends after ${mod.actionsLeft} more action${mod.actionsLeft === 1 ? "" : "s"}`;
    // B5 E28, R458: the card's own words.
    case "startOfTurnEffect":
      return mod.label;
    // B5 E15, E39 (R455): the play pipeline's price rules and Forever&'s rider, worded by their owner.
    case "costRule":
      return costRuleModifierLabel(mod);
    case "enchantNextSpell":
      return enchantNextSpellLabel(mod.enchantment);
    // B5 E8: Classic+ #22 Blood Moon's base face, read off the modifier alone.
    case "healToDamage":
      return "Healing on your enemies deals Pierce damage instead";
    // R449: Classic #23 Devil's Pact's replacement, named as the card every play becomes.
    case "replacePlays":
      return `Each card you play becomes ${mod.radiant ? "a Radiant " : "a "}${findDef(state, mod.defId)?.name ?? mod.defId}`;
  }
}

/**
 * §10.8 does not list the player modifiers, so R169 decides them: both seats carry the list, since
 * every Core modifier is installed by a card played face-up and `modifierChanged` is already public
 * in both directions (see `redactEvent`). Only the id and the caption travel.
 *
 * "Face-up" is the load-bearing word and "Cry" would be wrong: #35 Lunar Eclipse and #78 /fullsend
 * are Spells, which never enter the field and so can never have a Cry; #64 and #79 install theirs
 * from other hooks. What all five share is that the play itself was public.
 *
 * R48: a modifier that covers the controller's *next* turn is installed at once and bites later, so
 * the caption says so while `modifierIsLive` is still false — otherwise #77's badge would claim a
 * discount on the very turn the discount does nothing.
 */
function modifierViews(state: GameState, player: PlayerId): ModifierView[] {
  const views = state.players[player].mods.map((mod) => {
    const label = modifierLabel(state, mod, echoGrantOf(state, player, mod));
    return { id: mod.id, label: modifierIsLive(state, mod) ? label : `${label} (next turn)` };
  });
  // §6.3 Mana: the next refresh's rider (#21 Hinder, #24 Efficiency Dividend) is a modifier too, one
  // badge under the id `modifierChanged` names for it, while it is not 0 (R169).
  const rider = state.players[player].mana.nextTurnMod;
  if (rider !== 0) {
    views.push({ id: NEXT_REFRESH_MODIFIER_ID, label: `Next refresh ${rider > 0 ? "+" : "−"}${Math.abs(rider)} mana` });
  }
  return views;
}

// ---------------------------------------------------------------------------
// One side of the board
// ---------------------------------------------------------------------------

/**
 * R64: the zones this player is holding for a dying Reborn unit, as a mask per row — and B3.1 rule 6's
 * backrow zones held for an animated "Animated on your turn" card's return (`zones.isReserved`).
 */
function reservedMask(state: GameState, player: PlayerId): { units: boolean[]; backrow: boolean[] } {
  const mask = (row: Row): boolean[] => slotsOf(player, row).map((ref) => isReserved(state, ref));
  return { units: mask("units"), backrow: mask("backrow") };
}

function sideView(state: GameState, player: PlayerId, viewer: PlayerId): SideView {
  const side: PlayerState = state.players[player];
  const powers = heroPowersOf(state, player);
  return {
    player,
    hero: {
      health: side.hero.health,
      // §10.8's "armor": the number §4.4 step 2 will actually subtract, so the hero panel is read
      // the same way a unit's is — never the stored field alone. `heroArmorOf` adds every backrow
      // grant (#84 Going Long) to it, summed per R124, and the grant disappears from the view the
      // moment the granting card leaves the backrow.
      armor: heroArmorOf(state, player),
      powers,
      power: powers[0] ?? null,
    },
    // R169: the badge list beside the hero, public on both seats.
    modifiers: modifierViews(state, player),
    mana: { current: side.mana.current, max: side.mana.max },
    // §10.8: the viewer's own hand in full, the opponent's as a count — until the game is over, when
    // both hands are revealed (R434): the opponent's cards as they stand, as their owner saw them.
    hand:
      player === viewer
        ? side.hand.map((card) =>
            withPreview(
              withCondition(handCardView(state, card), conditionActive(state, card, viewer, "hand")),
              previewOf(state, card, viewer, "hand"),
            ),
          )
        : state.result !== null
          ? side.hand.map((card) => handCardView(state, card))
          : { count: side.hand.length },
    // §9.1: a library's order ships to nobody, and the opponent's library is a count and nothing
    // else. R310–R312: the viewer's own is a list without order as well, of what they were shown
    // going in (`ownLibrary.ts`), with no instance id or position in it.
    libraryCount: side.library.length,
    ...(player === viewer ? { ownLibrary: ownLibraryView(state, player) } : {}),
    graveyard: side.graveyard.map((card) => cardView(state, card)),
    exile: side.exile.map((card) => cardView(state, card)),
    // §10.5 step 4, R98: a Spell between its play and its graveyard. Playing it was public. R448: a
    // card announced to be set face-down waits here too, and the other player sees a card back.
    resolving: side.resolving.map((card) =>
      announcedFaceDownTo(state, card, viewer)
        ? { instanceId: HIDDEN_ID, defId: HIDDEN_ID, radiant: false, cost: HIDDEN_COST }
        : withCopies(cardView(state, card), state, card),
    ),
    units: side.units.map((pile) => (pile === null ? null : unitViewOf(state, pile, viewer))),
    backrow: side.backrow.map((card) => backrowView(state, card, viewer)),
    ...carriedView(state, player, viewer),
    locks: { units: [...side.locks.units], backrow: [...side.locks.backrow] },
    reserved: reservedMask(state, player),
    fatigueCount: side.fatigueCount,
  };
}

// ---------------------------------------------------------------------------
// The open prompt
// ---------------------------------------------------------------------------

/**
 * §10.8: "a card revealed out of a library is revealed only as an option of the prompt that reveals
 * it: the chooser sees it in full". These options only ever travel to the chooser (R81), so naming
 * the definition behind an option is exactly what the chooser is owed.
 */
function optionView(state: GameState, viewer: PlayerId, option: PromptOption): PendingOption {
  // B5 E18: a `pick` option's cost against the budget, and the face an option shows when it is Radiant.
  const base = {
    key: option.key,
    label: option.label,
    ...(option.cost === undefined ? {} : { cost: option.cost }),
    ...(option.radiant === true ? { radiant: true as const } : {}),
  };
  const selection = option.selection;
  switch (selection.pick) {
    case "instance": {
      const card = findInstance(state, selection.instanceId);
      // R177: a prompt may offer a card its chooser may not read — a target prompt reaching an
      // enemy face-down trap (#49, #50, an Echo repeat's fresh pick). The option is the zone's card
      // and nothing more: the id to answer with, never the definition, and neither the label nor
      // the key the engine built from its name. A card revealed out of a library is the opposite
      // case: the prompt IS its reveal, so the chooser sees it in full (above) — and so is B5 E17's
      // look at the opponent's hand (Classic #11), whose options only their chooser is sent (R81).
      if (card !== undefined && isFaceDownTo(state, card, viewer)) {
        return { key: `instance:${selection.instanceId}`, label: HIDDEN_OPTION_LABEL, instanceId: selection.instanceId };
      }
      return card === undefined
        ? { ...base, instanceId: selection.instanceId }
        : {
            ...base,
            instanceId: selection.instanceId,
            defId: card.defId,
            ...(card.radiant ? { radiant: true as const } : {}),
          };
    }
    case "hero":
      return { ...base, player: selection.player };
    case "zone":
      return { ...base, player: selection.player, row: selection.row, lane: selection.lane };
    case "mode": {
      // §6.3 Discover offers definitions, as `mode` options whose string is a catalog def id.
      const def = findDef(state, selection.option);
      return def === undefined ? base : { ...base, defId: def.id };
    }
    case "none":
      return base;
  }
}

/** §10.6, R81: the other player learns that a prompt is open and whose it is, never its options. */
function pendingView(state: GameState, viewer: PlayerId): PendingView | null {
  const pending = state.pending;
  if (pending === null) return mulliganPendingView(state, viewer);
  if (pending.playerId !== viewer) return { forYou: false, pendingFor: pending.playerId };
  return promptView(state, viewer, pending);
}

/**
 * The chooser's own prompt, copied field by field: `resume` never travels, so nothing a prompt keeps
 * for its answer — the owner a prompt the other player holds continues as (B5 E18), a multiple-choice
 * problem's key (R465) — can leave the engine through the view.
 */
function promptView(state: GameState, viewer: PlayerId, pending: PendingChoice): PendingView {
  return {
    forYou: true,
    choiceId: pending.id,
    kind: pending.kind,
    options: pending.options.map((option) => optionView(state, viewer, option)),
    min: pending.min,
    max: pending.max,
    prompt: pending.prompt,
    ...(pending.budget === undefined ? {} : { budget: pending.budget }),
  };
}

/**
 * R265, R266: while both mulligans are open, a seat that still owes one sees its own prompt, and a
 * seat that has answered sees only that the other seat still owes one — as it would a prompt the
 * other seat held — never what that seat is choosing from or has chosen.
 */
function mulliganPendingView(state: GameState, viewer: PlayerId): PendingView | null {
  const own = mulliganPromptFor(state, viewer);
  if (own !== null) return promptView(state, viewer, own);
  const other = opponentOf(viewer);
  return mulliganPromptFor(state, other) === null ? null : { forYou: false, pendingFor: other };
}

/** R265, R266: who has answered, and what the viewer kept; absent outside the mulligan window. */
function mulliganView(state: GameState, viewer: PlayerId): { mulligan?: MulliganView } {
  const open = state.mulligan;
  if (open === undefined) return {};
  const own = open[viewer].keep;
  return {
    mulligan: {
      youReady: own !== null,
      opponentReady: open[opponentOf(viewer)].keep !== null,
      ...(own === null ? {} : { kept: [...own] }),
    },
  };
}

/** R269: the standing draw offer, public to both seats; absent when none. */
function drawOfferView(state: GameState): { drawOffer?: { by: PlayerId } } {
  const by = standingDrawOffer(state);
  return by === null ? {} : { drawOffer: { by } };
}

// ---------------------------------------------------------------------------
// Events (§10.10)
// ---------------------------------------------------------------------------

/**
 * R97. Every event carries ids and several carry a `defId` as well — `drawn` names the card that
 * went into a hand, `bounced` the one that left the field for it, `costChanged` a card discounted
 * in hand — so the animation stream is filtered like every other zone. An event that names a card
 * this viewer may not read keeps its type and every field §10.10's animation table needs, with the
 * identity replaced by `HIDDEN_ID`: redacted, never dropped, so the cue still plays as a card back.
 *
 * The switch is exhaustive over all 43 event types on purpose (§10.3): with no `default`, adding an
 * event type does not compile until someone decides what it reveals.
 */
function redactEvent(state: GameState, viewer: PlayerId, event: GameEvent, replaced: Replacements): GameEvent {
  const hidden = (id: string): boolean => !mayRead(state, viewer, id, replaced);

  switch (event.type) {
    // A card named with its definition: both go, or neither.
    //
    // `cardResolved` (§10.5 step 7) is one of these rather than a public event: R97 judges a card
    // by where it sits *now*, and once resolution is over a Spell has reached the graveyard and a
    // permanent is on the field, both public — so it ordinarily reads openly, and `mayRead` keeps
    // the sentinel for the card that ended up somewhere this viewer may not read (a Trap set
    // face-down, a card resolved back into a hand or a library). Its `player` and `permanent` are
    // not identity fields and never travel redacted; `permanent` is R61's "still in play" answer,
    // which #85 keys on. The face that resolved (`radiant`) is the card's, so it goes with the id.
    case "cardResolved": {
      // R119's `arrivedDuring` is the engine's own bookkeeping, and it names face-down traps (#95);
      // so is the exit mark the event happened at (R174, R212).
      const { arrivedDuring: _arrivals, exitsFrom: _mark, ...shown } = event;
      if (!hidden(event.instanceId)) return shown;
      const { radiant: _face, ...rest } = shown;
      return { ...rest, instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    }

    // R97, R177: `killerId` names a card as well, and the card that dealt the lethal hit — a unit,
    // or a Spell whose damage was lethal — may since have gone somewhere this viewer cannot read,
    // like the #31 KY's Math Equation that returns to its owner's hand at the end of the turn.
    case "destroyed": {
      const killerHidden = event.killerId !== null && hidden(event.killerId);
      const { radiant: _face, ...faceless } = event;
      const redacted = hidden(event.instanceId) ? { ...faceless, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event;
      return killerHidden ? { ...redacted, killerId: HIDDEN_ID } : redacted;
    }

    // R119's `arrivedDuring` and the exit mark on a play's step-4 pair are the engine's bookkeeping,
    // as on `cardResolved`. R227: `formerId` is the id a card set face-down had, and it goes with the
    // card's identity — shown to a viewer who may read the card, never to one who may not, or the old
    // id would name the face-down card after all (R177).
    case "cardPlayed":
    case "summoned": {
      const { arrivedDuring: _arrivals, exitsFrom: _mark, ...shown } = event;
      if (!hidden(event.instanceId)) return shown;
      const { formerId: _former, ...rest } = shown;
      return { ...rest, instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    }

    // R316: a card a full library refused is judged like the cards below: one that went to the
    // graveyard reads as long as it stays there. One never created is in no pile and never was
    // anywhere hidden, so it reads as the card it copies does (`copyOf`), and openly when it copies
    // none — #33's copy of a Trap set face-down names the trap no more than the trap does. `copyOf`
    // is the engine's bookkeeping and never travels.
    case "libraryOverflow": {
      const { copyOf, ...shown } = event;
      const unread = hidden(event.instanceId) || (copyOf !== undefined && hidden(copyOf));
      if (!unread) return shown;
      // The face it would have had is the card's too, so it goes with the identity.
      const { radiant: _face, ...rest } = shown;
      return { ...rest, instanceId: HIDDEN_ID, defId: HIDDEN_ID };
    }

    // R317: a burned card lands in its owner's graveyard, or ceases to exist (R11), so both seats read
    // it — the hand it never entered is not where it is — until something takes it somewhere hidden.
    case "enteredGraveyard":
    case "exiled":
    case "bounced":
    case "burned":
    case "discarded":
    case "drawn":
    case "addedToHand":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event;

    // R177: a Make Radiant on a card in a library (#42's roll over every card, top down) is one nobody
    // could read where it happened (§3), and read openly once the card does, its place in the batch
    // would say where it lay. The event's `zone` is where it happened, so it stays unread for good.
    //
    // And a cue on the other player's card this viewer may not read says only whose it was: a random
    // pick over several hidden zones (#28's hand, library and field) picks among non-Radiant cards
    // only (R60), so a cue located in the hand, or at a face-down trap's lane, would tell this viewer
    // that the hand still held a base-face card, or that the trap was base-face (R33). The zone is
    // given as that player's hand, the region this viewer is shown the player's unread cards in.
    case "radiantSet": {
      const unread = event.zone.z === "library" || hidden(event.instanceId);
      if (!unread) return event;
      const owner = event.zone.player;
      const zone: Zone = owner === viewer ? event.zone : { z: "hand", player: owner };
      return { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID, zone };
    }

    /**
     * R154: the one identity R97's "judged by where the card sits now" cannot decide, so the row
     * names the seat instead — the controller reads `instanceId` and `defId`, the other player
     * reads the sentinel. Firing a Trap consumes it into its owner's graveyard (a public pile) or
     * leaves a Field Trap face-up, so `mayRead` would call every fired trap public and hand the
     * opponent the card's identity on the event that announces the flip. The animation needs the
     * opposite: §10.10's `trapFired` row flips a card back in the right lane, and §10.8 gives a
     * face-down trap no instance id to hang that on. `row`, `lane` and `controller` are not
     * identity and always travel, which is the whole point of the row — the opponent animates the
     * flip in the right zone without being told which card it was.
     */
    case "trapFired":
      return event.controller === viewer
        ? event
        : { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID };

    // §9.1: library order is hidden from both players, so the slot never travels either way.
    case "shuffledIn":
      return hidden(event.instanceId)
        ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID, position: HIDDEN_POSITION }
        : { ...event, position: HIDDEN_POSITION };

    // R177: a Replace puts the new card where the old one was (§6.3), and the old one ceased to
    // exist there (R35), so no zone of its own is left to judge it by — it is judged by its
    // replacement's. A card replaced inside a library or a face-down backrow zone never reads.
    case "transformed": {
      const newHidden = hidden(event.newInstanceId);
      const { hiddenFrom: _record, ...shown } = event;
      return {
        ...shown,
        ...(newHidden || hidden(event.instanceId) ? { instanceId: HIDDEN_ID, fromDefId: HIDDEN_ID } : {}),
        ...(newHidden ? { newInstanceId: HIDDEN_ID, toDefId: HIDDEN_ID } : {}),
      };
    }

    case "fused":
      return {
        ...event,
        instanceIds: event.instanceIds.map((id) => (hidden(id) ? HIDDEN_ID : id)),
        ...(hidden(event.resultInstanceId) ? { resultInstanceId: HIDDEN_ID, defId: HIDDEN_ID } : {}),
      };

    // R177: a buff's size is the card's too — #89 Corpse Eater gains double on its radiant face —
    // so a hidden card's buff keeps its type for the cue and says nothing of how much.
    case "buffed":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID, attack: 0, health: 0 } : event;

    // One instance, no definition: the id alone would still name a card in a hidden zone.
    case "divineShieldLost":
    case "keywordGranted":
    case "positionSwitched":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID } : event;
    // R227: as on `summoned`, a fresh id's `formerId` goes with the card's identity (C+ #35, R419).
    case "controlChanged": {
      if (!hidden(event.instanceId)) return event;
      const { formerId: _former, ...rest } = event;
      return { ...rest, instanceId: HIDDEN_ID };
    }

    // R385: a Brittle count is its card's, read only where the card is (a face-down card's by its
    // controller alone), so on a card this viewer may not read the number goes with the id.
    case "counterChanged":
      if (!hidden(event.instanceId)) return event;
      return event.counter === "brittle"
        ? { ...event, instanceId: HIDDEN_ID, value: HIDDEN_COUNT }
        : { ...event, instanceId: HIDDEN_ID };

    // R177: the new cost is the card's too, and over a library it would give the order away — so a
    // change made in a library stays unread for good (`hiddenFrom`), whatever became of the card.
    case "costChanged": {
      const { hiddenFrom, ...shown } = event;
      return hiddenFrom?.includes(viewer) === true || hidden(event.instanceId)
        ? { ...shown, instanceId: HIDDEN_ID, cost: HIDDEN_COST }
        : shown;
    }

    case "healed":
      return hidden(event.targetId) ? { ...event, targetId: HIDDEN_ID } : event;

    case "damage":
      return {
        ...event,
        sourceId: event.sourceId !== null && hidden(event.sourceId) ? HIDDEN_ID : event.sourceId,
        targetId: hidden(event.targetId) ? HIDDEN_ID : event.targetId,
      };

    case "attackDeclared":
      return {
        ...event,
        attackerId: hidden(event.attackerId) ? HIDDEN_ID : event.attackerId,
        targetId: hidden(event.targetId) ? HIDDEN_ID : event.targetId,
      };

    case "attackCancelled":
      return {
        ...event,
        attackerId: hidden(event.attackerId) ? HIDDEN_ID : event.attackerId,
        targetId: hidden(event.targetId) ? HIDDEN_ID : event.targetId,
        byInstanceId: hidden(event.byInstanceId) ? HIDDEN_ID : event.byInstanceId,
      };

    // Public through and through: these name a player, a zone or a number, never a card. A
    // `promptOpened` event says a prompt is open and whose, which is all §10.6 grants.
    // R315: a fatigue draw names a player and two numbers, and the fatigue count is public (§10.8).
    case "healthLost":
    case "fatigue":
    case "modifierChanged":
    case "rotated":
    case "swapped":
    case "locked":
    case "manaChanged":
    case "turnStarted":
    case "turnEnded":
    case "turnAutoEnded":
    case "promptOpened":
    case "promptAnswered":
    case "drawOffered":
    case "drawAnswered":
    case "gameOver":
      return event;

    // ---- Patch v0.2.0 (docs/classic-sets.md B3, B5) ----

    // B5 E1: an announce shows what `cardPlayed` would. A card being set face-down is its zone only
    // to the other player (R97, R227): the identity and the targets it declared go, the zone stays.
    case "cardAnnounced": {
      const unread = (event.faceDown === true && event.player !== viewer) || hidden(event.instanceId);
      if (!unread) return { ...event, targets: event.targets.map((id) => (hidden(id) ? HIDDEN_ID : id)) };
      // R448: whether a face-down card is a Trap or a Field Trap is the card's too (R33), so the
      // other player reads every one as a Trap, as its backrow will show it.
      const cardType = event.faceDown === true ? "Trap" : event.cardType;
      return { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID, cardType, targets: event.targets.map(() => HIDDEN_ID) };
    }

    // B5 E1, E2, B3.3: judged by where the card is now (R97) — a countered card in a public pile reads,
    // one stolen into a hand reads to that hand's owner only, a crumbled card reads once it is in the
    // graveyard. `byInstanceId` is the countering card, a fired trap by then, judged the same way.
    case "countered":
      return {
        ...event,
        ...(hidden(event.instanceId) ? { instanceId: HIDDEN_ID, defId: HIDDEN_ID } : {}),
        ...(event.byInstanceId !== null && hidden(event.byInstanceId) ? { byInstanceId: HIDDEN_ID } : {}),
      };
    // B5 E2, E16, R466: a stolen card reads to whoever could read it where it was taken from — the
    // hand's holder, the controller of a face-down zone, everyone for a face-up card or a public pile
    // (`readableFrom`, written as it was taken) — and to whoever can read it where it is now (R97). A
    // card out of a library was nobody's to read, so its old owner never learns which card left.
    case "stolen": {
      const { readableFrom, ...shown } = event;
      return hidden(event.instanceId) && !readableWhereStolen(event, readableFrom, viewer)
        ? { ...shown, instanceId: HIDDEN_ID, defId: HIDDEN_ID }
        : shown;
    }
    case "crumbled":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event;

    // B3.4, R386, R177: a change is the card's, and over a library or a hidden hand it would say
    // which card changed and how — so it stays unread for good for whoever could not read the card
    // where it changed (`hiddenFrom`), and for whoever cannot read it now.
    case "degraded":
    case "upgraded": {
      const { hiddenFrom, ...shown } = event;
      return hiddenFrom?.includes(viewer) === true || hidden(event.instanceId)
        ? { ...shown, instanceId: HIDDEN_ID, defId: HIDDEN_ID, change: HIDDEN_TUNING_CHANGE }
        : shown;
    }
    // Classic+ #41: a number set outright is the card's as well, so it follows `degraded`.
    case "numberChanged": {
      const { hiddenFrom, ...shown } = event;
      return hiddenFrom?.includes(viewer) === true || hidden(event.instanceId)
        ? { ...shown, instanceId: HIDDEN_ID, defId: HIDDEN_ID, key: HIDDEN_ID, value: 0 }
        : shown;
    }

    // B5 E9: a hit, an attack or a pick moves between cards on the field or heroes, all public; a
    // card that has since gone somewhere unreadable is the sentinel, as on `damage`.
    case "redirected":
      return {
        ...event,
        fromId: hidden(event.fromId) ? HIDDEN_ID : event.fromId,
        toId: hidden(event.toId) ? HIDDEN_ID : event.toId,
        byInstanceId: event.byInstanceId !== null && hidden(event.byInstanceId) ? HIDDEN_ID : event.byInstanceId,
      };

    // A card on the field acting face-up (an ability, an animation, a quest, a flicker, a mark): public
    // while it is readable, the sentinel once it has gone somewhere hidden (R97).
    case "activated":
    case "animated":
    case "deanimated":
    case "flickered":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event;
    case "questProgressed":
    case "questCompleted":
    case "marked":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID } : event;

    // R436: Call to Chaos names what it rolled to both players; the card itself follows R97.
    case "chaosRolled":
      return hidden(event.instanceId) ? { ...event, instanceId: HIDDEN_ID, defId: HIDDEN_ID } : event;

    case "turnCutShort":
      return event.byInstanceId !== null && hidden(event.byInstanceId) ? { ...event, byInstanceId: HIDDEN_ID } : event;

    // Public: a zone, a player, a number.
    case "unlocked":
    case "healthSet":
    case "rolledBack":
    case "drawLimited":
      return event.type === "healthSet" && event.sourceId !== null && hidden(event.sourceId)
        ? { ...event, sourceId: HIDDEN_ID }
        : event;
  }
}

/**
 * B5 E2, E16, R466: whether `viewer` could read the card a `stolen` event names where it was taken
 * from. `ownership.changeOwner` writes who could (`readableFrom`) as it takes the card; an event
 * without the record is judged by its pile alone — a hand is its holder's (§9.1), a library nobody's,
 * a graveyard, an exile pile or the resolving zone everyone's (a play is public, R98), and a card off
 * the field nobody's, since whether it stood face-down there is not otherwise on the event.
 */
function readableWhereStolen(
  event: Extract<GameEvent, { type: "stolen" }>,
  readableFrom: readonly PlayerId[] | undefined,
  viewer: PlayerId,
): boolean {
  if (readableFrom !== undefined) return readableFrom.includes(viewer);
  switch (event.zone) {
    case "hand":
      return event.from === viewer;
    case "graveyard":
    case "exile":
    case "resolving":
      return true;
    case "library":
    case "field":
      return false;
  }
}

/** R386, R177: what a hidden Degrade or Upgrade shows — that a card changed, never how. */
const HIDDEN_TUNING_CHANGE: TuningChange = { kind: "number", key: HIDDEN_ID, delta: 0 };

/**
 * §10.8: "the last N events for animation". `state.applied` is the only event history a state
 * carries (§9.3's nonce dedupe), oldest action first, so flattening it in order and taking the tail
 * is the stream — and it is bounded by `NONCE_HISTORY` already.
 *
 * `VIEW_EVENT_LIMIT` is a FLOOR, not a cap: the window never ends inside the newest applied action.
 * BUILD M5-T4 gives every §10.3 event an animation and `PlayerView.events` is the client's only
 * channel for them, so a fixed length silently drops the FRONT of any single action that emits more
 * than it — the client then animates the tail of something whose beginning it was never told about.
 * One #96 My Pawn cancel plus the §10.7 AI turn it hands over is 38 events in one `reduce`, and the
 * three the cancel is made of (`attackDeclared`, `trapFired`, `attackCancelled`) were exactly the
 * ones lost, which made M5-T4's `attackCancelled` row unreachable. Measured: 38 emitted, 32
 * carried, 6 dropped from the front.
 *
 * This is SPEC §11 R168, which states the floor and records the measurement above.
 */
function recentEvents(state: GameState, viewer: PlayerId): GameEvent[] {
  const all = state.applied.flatMap((entry) => entry.events);
  const newest = state.applied[state.applied.length - 1]?.events.length ?? 0;
  const window = Math.max(VIEW_EVENT_LIMIT, newest);
  const replaced = replacementsOf(all, state);
  return all
    .slice(Math.max(0, all.length - window))
    .map((event) => redactEvent(state, viewer, event, replaced));
}

// ---------------------------------------------------------------------------
// §10.8
// ---------------------------------------------------------------------------

/**
 * The one window a player has onto a match (SPEC §10.8). Pure: it reads the state and builds a
 * fresh object, sharing nothing mutable with it.
 *
 * `clockMs` is the turn clock the server is running (R79). The engine never reads a clock, so the
 * caller passes the number in and it is `null` whenever nobody is counting.
 */
export function viewFor(state: GameState, playerId: PlayerId, clockMs: number | null = null): PlayerView {
  syncFusedScripts(state);
  const view: PlayerView = {
    viewer: playerId,
    turn: state.turn,
    active: state.active,
    phase: state.phase,
    you: sideView(state, playerId, playerId),
    opponent: sideView(state, opponentOf(playerId), playerId),
    pending: pendingView(state, playerId),
    events: recentEvents(state, playerId),
    result: state.result === null ? null : { winner: state.result.winner, reason: state.result.reason },
    clockMs,
    ...mulliganView(state, playerId),
    ...drawOfferView(state),
    // R345: the viewer's own preference, and only when it is off, so every other view is unchanged.
    ...(state.players[playerId].autoEndTurn === false ? { autoEndTurn: false as const } : {}),
  };
  const defs = matchDefsIn(state, view);
  return Object.keys(defs).length === 0 ? view : { ...view, defs };
}

/**
 * R243: the match-made definitions (`state.transientDefs`: a Fuse's, a crafted card's — R77, R102,
 * R179) the finished view names anywhere — a card in a zone, a unit, a prompt option, an event —
 * copied beside it, since no catalog a client holds has them. The view is read after it is built,
 * so only an id that survived redaction brings its definition: a card this viewer may not read is
 * the sentinel by then (R97), and its definition stays in the match.
 */
function matchDefsIn(state: GameState, view: PlayerView): Record<string, CardDef> {
  const defs: Record<string, CardDef> = {};
  const visit = (value: unknown): void => {
    if (typeof value === "string") {
      // Own keys only: a label that happens to read "constructor" names no definition.
      if (!Object.prototype.hasOwnProperty.call(state.transientDefs, value) || value in defs) return;
      const def = state.transientDefs[value];
      if (def !== undefined) defs[value] = JSON.parse(JSON.stringify(def)) as CardDef;
      return;
    }
    if (Array.isArray(value)) {
      for (const item of value) visit(item);
      return;
    }
    if (value !== null && typeof value === "object") {
      for (const item of Object.values(value)) visit(item);
    }
  };
  visit(view);
  return defs;
}
