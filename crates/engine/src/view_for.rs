//! What one player is allowed to see, and nothing else (SPEC §10.8). The client renders this view
//! and never holds rules or hidden information (CLAUDE.md rule 7, §9.1's trust model).
//!
//! The view is built by *copying* what the viewer is entitled to, never by deleting fields from a
//! state clone: a field added to `GameState` stays invisible until this file names it. Five rules do
//! all the work here.
//!   - §9.1 hidden list: library order, the opponent's library, the opponent's hand, face-down traps.
//!     Both libraries and the opponent's hand travel as counts; graveyards, exile and Field Spells are
//!     public. R310–R312: the viewer's own library also travels as a list without order of what the
//!     viewer was shown going in (`ownLibrary.ts`) — definitions, faces and counts, never an instance
//!     id — so R97's rule below still reads no library card, the owner's included.
//!   - R33: a face-down trap is readable by its *current controller* only, so a steal, a board swap
//!     or a rotation moves who may read it even though ownership never changed; a Field Trap that
//!     has fired (`faceUp`) is public to both.
//!   - R13, §3.2: the lower cards of a Stack pile are dormant and not on the field. The view shows
//!     the top card and a count of what is buried under it, never a buried card's identity.
//!   - R81: only the viewer's own prompt carries its options; the other player's prompt shows that
//!     it is open and whose it is, nothing more.
//!   - R97: the event stream is redacted, not truncated. An event that names a card the viewer may
//!     not read keeps its type and its animation fields and shows `HIDDEN_ID` for that card.
//!   - R764: a Glitch's reset or boards (R676, R678) takes cards out of the match unseen, so the events
//!     up to the end of its action read as the sentinel for each card the state no longer holds.
//!   - R227: a card set face-down took a fresh id, so the events that named its old id follow it to
//!     its zone through the `formerId` that set it, and `formerId` itself travels only with the card.
//!   - R177: more fields follow R97 — a prompt option offering a face-down card names it by id only,
//!     a `costChanged` on an unreadable card hides its cost and a `buffed` one its amounts, and a
//!     `transformed` whose new card is unreadable hides the card it replaced, as does one whose old
//!     card was unreadable where it ceased to exist (`hiddenFrom`), whatever became of its
//!     replacement since.
//!   - R169: the player modifiers (§10.1 `mods`) travel on both seats as `{ id, label }`, because
//!     every one of them is installed by a card played FACE-UP and `modifierChanged` is already
//!     public in both directions. Face-up, not "by a Cry": #35 and #78 are Spells and can never have
//!     one, #64 and #79 install theirs without one, and only #77 is a Cry. The caption is built from
//!     the modifier's own kind and numbers and never from its `sourceId`, so no card identity can
//!     leave through a badge.
//!   - R434: once the game is over, the opponent's hand travels in full, as the owner's does; the
//!     libraries, face-down traps and R97's event redaction stay as they were.
//!   - R437: a mark (an effect aimed at the card that still waits, #50's pending steal) rides every
//!     view of the card in both seats, a face-down card's back included (`marks.ts`).
//!   - R195, R280, R667: three things the engine works out for a card ride on its view.
//!     `conditionActive` (the yellow glow) on the viewer's own cards only; `counteredOnPlay` (Plague
//!     Chalice's warning) on the viewer's own hand cards only (`counterWarning.ts` owns that rule);
//!     `preview` (what a formula comes to now) on every card view the viewer may read — the viewer's
//!     hand, the top of a unit pile and a backrow card face-up to the viewer — and on no other
//!     (`preview.ts` owns that rule). #492, R81: a fourth, `embiggenCost` (what a play at the card's
//!     embiggen price costs now, `play_choices::embiggen_play_cost`), on the viewer's own hand cards
//!     only, beside the normal price `cost` already is.
//!
//! Stats are never read off an instance: `layers.unitView` recomputes every stat and keyword on read
//! (§10.4), so no stored total ever reaches the client.
//!
//! Port of `packages/engine/src/viewFor.ts` (SURFACE §4.1, §6.1). `VIEW_EVENT_LIMIT` lives in
//! `crate::config` (SURFACE §6.4); `syncFusedScripts` is gone (SURFACE §6.6). TS's spreads over an
//! event (`{ ...event, instanceId: HIDDEN_ID }`, `const { formerId, ...rest } = event`) are done on the
//! event's own JSON, key for key, so what a redacted event carries is exactly what TS's carried; the
//! match over `GameEventType` has no wildcard, so a new event type does not compile until someone
//! decides what it reveals (§10.3), as TS's exhaustive switch.

use indexmap::{IndexMap, IndexSet};
use serde_json::{Map, Value};

use crate::animated::is_animated;
use crate::announce::announced_face_down_to;
use crate::catalog::registered_catalog;
use crate::combat::{ExertionKind, has_exertion};
use crate::condition::condition_active;
use crate::config::VIEW_EVENT_LIMIT;
use crate::cost_rules::{cost_rule_text, enchant_next_spell_label};
use crate::counter_warning::countered_hand_cards;
use crate::damage::hero_armor_of;
use crate::echo::echo_grant_of;
use crate::faces::card_type_of;
use crate::instance_view::{hand_keywords_view, instance_data_view};
use crate::layers::{stats_with_buffs, unit_view as unit_layers};
use crate::mana::{NEXT_REFRESH_MODIFIER_ID, effective_cost, modifier_is_live};
use crate::marks::marks_on;
use crate::own_library::own_library_view;
use crate::params::params_view;
use crate::play_choices::embiggen_play_cost;
use crate::preview::{backrow_is_public, is_face_down, preview_of};
use crate::script::ConditionZone;
use crate::setup::{mulligan_prompt_for, returned_awaiting_shuffle};
use crate::state::{
    CardInstance, GameState, ModifierExpiry, ModifierKind, PendingChoice, PlayerModifier, PlayerState,
    PromptOption, find_instance,
};
use crate::subsystems::activate::activation_views_for;
use crate::subsystems::combo_index::grade_name;
use crate::subsystems::copied_text::{copied_text_of, text_face_of};
use crate::subsystems::hero_power::{power_of, power_title_of, used_this_turn};
use crate::subsystems::quests::quest_view_of;
use crate::turn::standing_draw_offer;
use crate::wire::{
    AnimatedView, BackrowCounters, BackrowView, CardDef, CardMark, CardType, CardView, CopiedTextView,
    DrawOfferView, FaceDownBackrowView, GameEvent, GameEventType, GameResult, GlitchOutcome, HandView,
    HeroPowerView, HeroView, ManaView, ModifierView, MulliganView, PLAYER_IDS, PendingElsewhereView,
    PendingOption, PendingPromptView, PendingView, PlayerId, PlayerView, PreviewValue, PublicBackrowView,
    Row, RowFlags, Selection, SideView, TuningChange, UnitView, Zone, ZoneName, opponent_of,
};
use crate::zones::{beneath_at, carried_at, carried_units_of, home_of, is_reserved, slot_of, slots_of};

/// R97's sentinel: the identity an event carries in place of a card the viewer may not read. Real
/// ids are `c<n>` for instances and catalog ids like `core-043` for definitions, so this collides
/// with neither and the client can test for it — a redacted event still animates, as a card back.
pub const HIDDEN_ID: &str = "hidden";

/// R97, §9.1: the slot a shuffled-in card landed in would give away library order, to either side.
const HIDDEN_POSITION: i32 = -1;

/// R177: the cost a `costChanged` event reports for a card the viewer may not read. A cost is a
/// property of the card as much as its definition is — the opponent's hand is a count (§10.8) — and
/// a sequence of costs over a library would spell out its order (§9.1).
const HIDDEN_COST: i32 = -1;

/// R385: the count a `counterChanged` "brittle" reports for a card the viewer may not read.
const HIDDEN_COUNT: i32 = -1;

/// R177: what a prompt option names when it offers a card the chooser may not read (§10.8, R33).
pub const HIDDEN_OPTION_LABEL: &str = "Face-down card";

// ---------------------------------------------------------------------------
// Visibility
// ---------------------------------------------------------------------------

/// §10.8, R33: a card in the backrow that this viewer sees only as a face-down card.
fn is_face_down_to(state: &GameState, card: &CardInstance, viewer: PlayerId) -> bool {
    matches!(
        card.zone,
        Zone::Field {
            row: Row::Backrow,
            ..
        }
    ) && !backrow_is_public(state, card, viewer)
}

/// R177: the card that took each vanished card's place, read off the events that replaced it — a
/// Replace (`transformed`, §6.3, R35) or a Fuse (`fused`, R77) — so a card that has ceased to exist
/// can still be judged by a zone: its replacement's. `state.applied` holds every event a view can
/// show, so every replacement that matters to one is in it.
#[derive(Clone, Debug, Default)]
struct Replacements {
    /// Each vanished card's replacement.
    replaced_by: IndexMap<String, String>,
    /// The players each replaced card was hidden from where it ceased to exist (`transformed.hiddenFrom`).
    hidden_from: IndexMap<String, Vec<PlayerId>>,
    /// R224: the cards a mulligan returned that wait in setup's owed item for their shuffle-back, in no
    /// pile (`setup::returned_awaiting_shuffle`). They are on their way to a library, so nobody reads them.
    to_library: Option<IndexSet<String>>,
    /// R764: set for the events up to the end of the action of a Glitch's reset or boards (R676, R678): a
    /// card they name that the state no longer holds ceased to exist out of that Glitch, unseen, and
    /// reads to nobody.
    voided: bool,
}

fn replacements_of(events: &[&GameEvent], state: Option<&GameState>) -> Replacements {
    let to_library: Option<IndexSet<String>> = state.map(|state| {
        returned_awaiting_shuffle(state)
            .into_iter()
            .map(|id| id.to_string())
            .collect()
    });
    let mut replaced_by: IndexMap<String, String> = IndexMap::new();
    let mut hidden_from: IndexMap<String, Vec<PlayerId>> = IndexMap::new();
    for event in events {
        match event {
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                hidden_from: hidden,
                ..
            } if new_instance_id != instance_id => {
                replaced_by.insert(instance_id.clone(), new_instance_id.clone());
                if let Some(hidden) = hidden {
                    hidden_from.insert(instance_id.clone(), hidden.clone());
                }
            }
            GameEvent::Fused {
                instance_ids,
                result_instance_id,
                ..
            } => {
                for id in instance_ids {
                    if id != result_instance_id {
                        replaced_by.insert(id.clone(), result_instance_id.clone());
                    }
                }
            }
            // R227: a card set face-down took a fresh id. It is the same card, so the events that named it
            // by its old id are judged by where it is now, exactly as they were before it moved: its draw
            // stays hidden while the trap is face-down and reads once the trap is public (R97). No
            // `hiddenFrom` is kept, since nothing ceased to exist.
            GameEvent::CardPlayed {
                instance_id,
                former_id: Some(former),
                ..
            }
            | GameEvent::Summoned {
                instance_id,
                former_id: Some(former),
                ..
            }
            | GameEvent::ControlChanged {
                instance_id,
                former_id: Some(former),
                ..
            } => {
                replaced_by.insert(former.clone(), instance_id.clone());
            }
            _ => {}
        }
    }
    Replacements {
        replaced_by,
        hidden_from,
        to_library: to_library.filter(|set| !set.is_empty()),
        voided: false,
    }
}

/// R97: whether this viewer may read the identity of the card an event names, judged by where the
/// card sits *now* and not where it was — so a card drawn last turn and played this turn reads
/// openly in both events, and a unit bounced into the opponent's hand stops reading the moment it
/// lands there (§9.1).
///
/// A card in the resolving zone is public: playing it was public, and R98 keeps it itself while it
/// sits there. A card the state no longer holds at all has ceased to exist, and R177 judges it by
/// the card that replaced it: a face-down trap #83 Transmogulate replaced keeps the secret its
/// replacement keeps, and a library #83 replaced still never reads, so its earlier `cardPlayed` or
/// `costChanged` cannot spell out what it was or in what order. With no replacement — a unit token
/// that left the field (R11), a card exiled out of existence (R86) — it was public when it went.
fn may_read(state: &GameState, viewer: PlayerId, instance_id: &str, replaced: &Replacements) -> bool {
    let mut id = instance_id.to_string();
    let mut card = find_instance(state, &id);
    let mut hops = 0;
    while card.is_none() && hops < replaced.replaced_by.len() {
        // R177: a card that ceased to exist where this viewer could not read it — a library card, an
        // enemy face-down trap — stays unread for good. Its replacement may reach a public pile later,
        // and judged by that pile alone the card it replaced would read, though it never was public.
        if replaced
            .hidden_from
            .get(&id)
            .is_some_and(|players| players.contains(&viewer))
        {
            return false;
        }
        let Some(next) = replaced.replaced_by.get(&id) else {
            break;
        };
        id = next.clone();
        card = find_instance(state, &id);
        hops += 1;
    }
    // R224: a card the mulligan returned waits for its shuffle-back in no pile, and is a library card.
    // R764: so is one a Glitch took off the board or out of the match before it was ever revealed.
    let Some(card) = card else {
        let to_library = replaced.to_library.as_ref().is_some_and(|ids| ids.contains(&id));
        return !to_library && !replaced.voided;
    };
    match &card.zone {
        Zone::Library { .. } => false,
        Zone::Hand { player } => *player == viewer,
        Zone::Field {
            row: Row::Backrow, ..
        } => backrow_is_public(state, card, viewer),
        // R448: a card waiting in the resolving zone to be set face-down is its player's alone (R33, R227).
        // Units, graveyard, exile, `resolving` (R98) and `gone` (R11, R86) are all public.
        _ => !announced_face_down_to(state, card, viewer),
    }
}

// ---------------------------------------------------------------------------
// Cards, units and the backrow
// ---------------------------------------------------------------------------

/// A card view with nothing set beyond what every view has.
fn bare_card_view(instance_id: String, def_id: String, radiant: bool, cost: i32) -> CardView {
    CardView {
        instance_id,
        def_id,
        radiant,
        chinese: None,
        cost,
        // filled in by `card_view` from the instance data; bare views carry none.
        tags: None,
        embiggen_cost: None,
        attack: None,
        health: None,
        power: None,
        condition_active: None,
        countered_on_play: None,
        preview: None,
        type_: None,
        brittle: None,
        params: None,
        tuning: None,
        enchantments: None,
        keywords: None,
        marks: None,
        activations: None,
        quest: None,
        copies: None,
    }
}

/// R65: the cost as it stands now. An X card has no chosen X outside a play, so it reads 0. Patch
/// v0.2.0's instance data rides on every card view (`instanceView.ts`): each is built only for a card
/// the viewer may read where it is.
fn card_view(state: &GameState, card: &CardInstance) -> CardView {
    // B5 E33, R404: a quest line on the field, public as the card is.
    let quest = quest_view_of(state, card);
    let data = instance_data_view(state, card);
    let mut view = bare_card_view(
        card.id.clone(),
        card.def_id.clone(),
        card.radiant,
        effective_cost(state, card, Default::default()),
    );
    view.type_ = data.type_;
    // MD-B15, R923: a granted tag is part of the card, so a view of it says so.
    view.tags = data.tags;
    view.brittle = data.brittle;
    view.params = data.params;
    view.tuning = data.tuning;
    view.enchantments = data.enchantments;
    view.marks = with_marks(state, &card.id);
    view.quest = quest;
    // ME-CN, R1301: public as the card is, so every view built for a card the viewer may read carries it.
    view.chinese = card.chinese;
    view
}

/// R437: the marks a card carries while an effect aimed at it waits (#50's pending steal), on every
/// view of it and in both players' views — or no key at all, so an unmarked card looks as it did.
fn with_marks(state: &GameState, instance_id: &str) -> Option<Vec<CardMark>> {
    let marks: Vec<CardMark> = marks_on(state, instance_id).into_iter().collect();
    if marks.is_empty() { None } else { Some(marks) }
}

/// R243, §10.8: a card in the viewer's own hand, in full — what it is made of beyond its printed
/// face as well. A Unit's stats are its face plus the permanent buffs it gained in hand (§10.4
/// layers 1, 3 and 4: #89 Corpse Eater's meals), since layer 2 and the auras are the field's; attack
/// floors at 0 as on the field. A #98 Heroic Power names the power it rolled as it arrived (R43,
/// R151), which its cost alone does not.
fn hand_card_view(state: &GameState, card: &CardInstance) -> CardView {
    let mut view = with_copies(card_view(state, card), state, card);
    let stats = if card_type_of(state, card) == CardType::Unit {
        Some(stats_with_buffs(state, card))
    } else {
        None
    };
    let power = power_of(card);
    // B5 E38: the keywords it gained in the hand or the deck, which it carries onto the field.
    let keywords = hand_keywords_view(state, card);
    if let Some(stats) = stats {
        view.attack = Some(stats.attack.max(0));
        view.health = Some(stats.max_health);
    }
    if let Some(power) = power {
        view.power = Some(power.name.to_string());
    }
    if let Some(keywords) = keywords {
        view.keywords = Some(keywords);
    }
    view
}

/// B5 E14, R399, R243: a copier's view carries the Spell text it has now (`CardView.copies`), with that
/// definition's declared numbers as they read on the card — or nothing, when it copies nothing. Asked
/// only where the viewer may read the card and R399 shows it: the owner's hand, and the resolving zone.
fn with_copies(mut view: CardView, state: &GameState, card: &CardInstance) -> CardView {
    let Some(copy) = copied_text_of(state, card) else {
        return view;
    };
    let face: CardInstance = text_face_of(state, card).clone();
    let params = params_view(state, &face);
    view.copies = Some(CopiedTextView {
        def_id: copy.def_id.clone(),
        radiant: copy.radiant,
        params,
    });
    view
}

/// R195, §10.8: the yellow glow rides on a card view as `conditionActive: true` or not at all — the
/// key is never `false`, so a card with no condition met looks exactly as it did before R195.
fn with_condition(mut view: CardView, active: bool) -> CardView {
    if active {
        view.condition_active = Some(true);
    }
    view
}

/// #492, R81: an embiggen card in the viewer's own hand carries what a play of it at its embiggen
/// price costs now (`embiggenCost`), read by the function step 1 prices that play with
/// (`play_choices::embiggen_play_cost`), beside the `cost` its normal price comes to. Absent on every
/// other card; this file asks it only of the viewer's own hand.
fn with_embiggen_cost(mut view: CardView, state: &GameState, card: &CardInstance) -> CardView {
    view.embiggen_cost = embiggen_play_cost(state, card);
    view
}

/// R667: the Plague Chalice warning, `true` or absent like the glow beside it.
fn with_counter_warning(mut view: CardView, countered: bool) -> CardView {
    if countered {
        view.countered_on_play = Some(true);
    }
    view
}

/// R280, §10.8: what a card's formula comes to now rides on its view as `preview`, or not at all —
/// never `[]`. `preview::preview_of` owns where it may: this file asks only about a card it shows the
/// viewer (the viewer's own hand, the top of a unit pile, a backrow card face-up to the viewer).
fn with_preview(mut view: CardView, values: Option<Vec<PreviewValue>>) -> CardView {
    if let Some(values) = values {
        view.preview = Some(values);
    }
    view
}

/// B3.2, R384: a card's Activate abilities ride its controller's own view of it on the field
/// (`activate::activation_views_for` owns where), or not at all — never `[]`.
fn with_activations(
    mut view: CardView,
    state: &GameState,
    card: &CardInstance,
    viewer: PlayerId,
) -> CardView {
    if let Some(activations) = activation_views_for(state, viewer, card) {
        view.activations = Some(activations);
    }
    view
}

/// The card view the field shows for `card`: the glow, the preview and the abilities on top of
/// `card_view`, in TS's nesting order (`withActivations(withPreview(withCondition(cardView(…)))))`).
fn field_card_view(state: &GameState, card: &CardInstance, viewer: PlayerId) -> CardView {
    with_activations(
        with_preview(
            with_condition(
                card_view(state, card),
                condition_active(state, card, viewer, ConditionZone::Field),
            ),
            preview_of(state, card, viewer, ConditionZone::Field),
        ),
        state,
        card,
        viewer,
    )
}

/// The card that acts in a unit zone: the top of the pile (§3.2). `buried` is how many dormant cards
/// sit under it (R13) — a count, so no buried identity reaches either player.
fn unit_view_of(state: &GameState, pile: &[CardInstance], viewer: PlayerId) -> Option<UnitView> {
    let top = pile.first()?;
    let layers = unit_layers(state, top);
    // B3.1, R383: a Field Spell, Trap or Field Trap standing here as a Unit, and the backrow lane an
    // "Animated on your turn" card will go back to (that zone is `reserved` meanwhile).
    let home: Option<i32> = if is_animated(state, top) {
        home_of(state, &top.id).map(|home| home.zone.lane)
    } else {
        None
    };
    let animated = if is_animated(state, top) {
        Some(AnimatedView { home })
    } else {
        None
    };
    let card = field_card_view(state, top, viewer);
    Some(UnitView {
        instance_id: card.instance_id,
        def_id: card.def_id,
        radiant: card.radiant,
        cost: card.cost,
        power: card.power,
        condition_active: card.condition_active,
        countered_on_play: card.countered_on_play,
        preview: card.preview,
        type_: card.type_,
        tags: card.tags,
        brittle: card.brittle,
        params: card.params,
        tuning: card.tuning,
        enchantments: card.enchantments,
        marks: card.marks,
        activations: card.activations,
        quest: card.quest,
        copies: card.copies,
        owner: top.owner,
        controller: top.controller,
        attack: layers.attack,
        max_health: layers.max_health,
        health: layers.health,
        keywords: layers.keywords.clone(),
        armor: layers.armor,
        position: layers.position,
        counters: top.counters,
        buried: pile.len() as i32 - 1,
        can_act: can_act(state, top),
        // R243, §6.3 Vanilla: the text is gone, which the definition the client reads does not say.
        vanilla: if top.vanilla { Some(true) } else { None },
        animated,
        // B5 E35: a status, public on the field like the unit itself.
        berserk: if top.berserk == Some(true) {
            Some(true)
        } else {
            None
        },
        // ME-CN, R1301: public on the field like the unit itself (the pile's top, R13).
        chinese: card.chinese,
    })
}

/// §4.1: "each unit has one exertion per turn: one attack or one position switch", so a unit can
/// still act while either exertion is unspent — a summoning-sick unit may still switch, and a unit
/// that cannot attack may still switch back. `combat::has_exertion` owns that rule, Deft's
/// two exertions included (R49); *which* of the two is legal is `combat::can_attack`'s answer and
/// `legal_actions`', never the view's.
fn can_act(state: &GameState, card: &CardInstance) -> bool {
    if state.result.is_some() || state.phase != crate::wire::Phase::Main {
        return false;
    }
    if state.pending.is_some() {
        return false;
    }
    if state.active != card.controller {
        return false;
    }
    has_exertion(state, card, ExertionKind::Attack) || has_exertion(state, card, ExertionKind::Switch)
}

/// The tokens a card carries now; an untouched card carries none (§6.3 Plague Counter). A private
/// copy of `plague::plague_on` (fullsend rule 5).
fn plague_tokens_on(card: &CardInstance) -> i32 {
    card.counters.plague.unwrap_or(0).max(0)
}

/// A public backrow card names its owner and its controller, since R33's readability and #87's board
/// swap both turn on the controller and a stolen card sits in a backrow that is not its own. A
/// face-down zone is `{ faceDown: true, cost }`: §10.8 grants the non-controller that a zone is
/// occupied and what the card in it costs (R351, a deliberate reveal), and nothing more, so not even
/// the controller's name travels with it. The cost is the one the controller's own view shows, so
/// both players read the same number.
fn backrow_view(state: &GameState, card: Option<&CardInstance>, viewer: PlayerId) -> Option<BackrowView> {
    let card = card?;
    // B5 E19, R471: Plague Counters are public wherever they sit, a face-down card's included; B5 E21: a
    // backrow pile shows how many cards lie under its top, as a unit pile does (R13, R447).
    let plague = plague_tokens_on(card);
    let under: i32 = match slot_of(state, card) {
        None => 0,
        Some(slot) => beneath_at(state, slot).len() as i32,
    };
    let buried = if under == 0 { None } else { Some(under) };
    let plague_shown = if plague == 0 { None } else { Some(plague) };
    // R437: a mark on a face-down card rides its back, which is all the other player sees of it (R33).
    if !backrow_is_public(state, card, viewer) {
        return Some(BackrowView::FaceDown(FaceDownBackrowView {
            face_down: true,
            cost: Some(effective_cost(state, card, Default::default())),
            plague: plague_shown,
            buried,
            marks: with_marks(state, &card.id),
        }));
    }
    let grade = card.counters.grade;
    let view = field_card_view(state, card, viewer);
    Some(BackrowView::Public(PublicBackrowView {
        instance_id: view.instance_id,
        def_id: view.def_id,
        radiant: view.radiant,
        chinese: view.chinese,
        tags: view.tags,
        cost: view.cost,
        attack: view.attack,
        health: view.health,
        power: view.power,
        condition_active: view.condition_active,
        countered_on_play: view.countered_on_play,
        preview: view.preview,
        brittle: view.brittle,
        params: view.params,
        tuning: view.tuning,
        enchantments: view.enchantments,
        keywords: view.keywords,
        marks: view.marks,
        activations: view.activations,
        quest: view.quest,
        copies: view.copies,
        face_down: false,
        type_: card_type_of(state, card),
        // R372: the engine names the grade's letter, so no client works out which letter 3 is.
        counters: BackrowCounters {
            grade,
            grade_letter: grade.map(|grade| grade_name(grade).to_string()),
            plague: plague_shown,
        },
        owner: card.owner,
        controller: card.controller,
        // R351, R371: the controller reads a face-down trap, and the view says the other player cannot.
        unrevealed: if is_face_down(state, card) {
            Some(true)
        } else {
            None
        },
        // ME-ALTPLAY (R1046, D3): the card's own reveal timing rides the controller's view only.
        reveal_at: if card.controller == viewer {
            card.set_as.as_ref().map(|set| set.reveal)
        } else {
            None
        },
        // R243, §6.3 Vanilla: a backrow card's text can be gone too, and the stamp renders off the
        // unit prop, so the flag travels here as it does on a unit view.
        vanilla: if card.vanilla { Some(true) } else { None },
        buried,
    }))
}

/// R446: the Units this side's carriers hold, by backrow lane, or nothing when none holds one.
fn carried_view(state: &GameState, player: PlayerId, viewer: PlayerId) -> Option<Vec<Option<UnitView>>> {
    if carried_units_of(state, player).is_empty() {
        return None;
    }
    Some(
        slots_of(player, Row::Backrow)
            .into_iter()
            .map(|zone_ref| {
                let unit: Option<CardInstance> = carried_at(state, zone_ref).cloned();
                unit.and_then(|unit| unit_view_of(state, &[unit], viewer))
            })
            .collect(),
    )
}

/// R43: a Heroic Power lives on its instance and it is a Field Spell (§8 #98), so it is public to
/// both players once it is on the field — the row §2's hero panel marks visible to both. The power,
/// its X and its use are `heroPower`'s to report, never re-derived here.
///
/// It follows control, not ownership: a stolen Heroic Power powers its new controller's hero. So a
/// player can hold more than one — their own plus one taken with #36 radiant or #49 — and each is
/// separately once-per-turn, which is why this is a list and every entry carries its `instanceId`
/// for `activatePower` (§10.2). Board order: p1's backrow lane 1 to 5, then p2's.
fn hero_powers_of(state: &GameState, player: PlayerId) -> Vec<HeroPowerView> {
    let mut powers: Vec<HeroPowerView> = Vec::new();
    for side in PLAYER_IDS {
        for card in state.players[side].backrow.iter().flatten() {
            if card.controller != player {
                continue;
            }
            let Some(power) = power_of(card) else {
                continue;
            };
            powers.push(HeroPowerView {
                instance_id: card.id.clone(),
                def_id: card.def_id.clone(),
                name: power.name.to_string(),
                title: power_title_of(power, card.radiant).to_string(),
                x: power.x,
                used_this_turn: used_this_turn(state, card),
            });
        }
    }
    powers
}

// ---------------------------------------------------------------------------
// Player modifiers (§10.1 `mods`, §10.3 `modifierChanged`)
// ---------------------------------------------------------------------------

/// The discount half of `modifier_label`, split out because `costDiscount` is the one kind whose
/// caption has to say *what* it applies to: R48's current-cost gate (#77), §8 #35's "next Spell",
/// and #78's flat "your cards" are three different sentences off one kind.
fn discount_label(
    amount: i32,
    only_type: Option<CardType>,
    min_current_cost: Option<i32>,
    once_per_turn: Option<bool>,
) -> String {
    let once = once_per_turn == Some(true);
    let less = format!("cost{} {amount} less", if once { "s" } else { "" });
    // R363, R432: #77's own words, "(4)+ Cost cards cost (1) less".
    if let Some(min) = min_current_cost {
        return format!("({min})+ Cost cards cost ({amount}) less");
    }
    if let Some(only) = only_type {
        return if once {
            format!("Next {only} {less}")
        } else {
            format!("{only}s {less}")
        };
    }
    if once {
        format!("Next card {less}")
    } else {
        format!("Your cards {less}")
    }
}

/// R449: a definition by id, the match-made ones first, as `catalog::find_def` reads it. A private
/// copy (fullsend rule 5).
fn find_def_in<'a>(state: &'a GameState, def_id: &str) -> Option<&'a CardDef> {
    state
        .transient_defs
        .get(def_id)
        .or_else(|| registered_catalog().get(def_id))
}

/// A badge caption for one modifier, built from the modifier alone. The match is exhaustive over
/// `ModifierKind` on purpose: with no wildcard, a new kind does not compile until someone decides
/// what the player is told about it.
///
/// `sourceId` (#79 Twinspell's instance) is deliberately not read here: it is a card id, and the view
/// must not hand either seat an identity through a badge. `echo` is the grant as it stands
/// (`echo::echo_grant_of`), a number read off the permanent's current face (R209, §5.2).
fn modifier_label(state: &GameState, modifier: &PlayerModifier, echo: i32) -> String {
    match &modifier.kind {
        ModifierKind::CostDiscount {
            amount,
            only_type,
            min_current_cost,
            once_per_turn,
        } => discount_label(*amount, *only_type, *min_current_cost, *once_per_turn),
        ModifierKind::EchoNextSpell { .. } => format!("Next Spell gains Echo +{echo}"),
        ModifierKind::RadiantFirstCheapCard { max_cost, .. } => {
            format!("First card costing {max_cost} or less becomes Radiant")
        }
        ModifierKind::ComboDraw { amount } => format!("Your cards gain \"Combo: draw {amount}\""),
        ModifierKind::QuickstrikerDamage => {
            "Your cards gain \"Combo X: X damage to the enemy hero\"".to_string()
        }
        // B5 E10, R456: how much of the turn is left.
        ModifierKind::TurnEnds { actions_left, .. } => {
            if *actions_left == 0 {
                "Your turn ends".to_string()
            } else {
                format!(
                    "Your turn ends after {actions_left} more action{}",
                    if *actions_left == 1 { "" } else { "s" }
                )
            }
        }
        // B5 E28, R458: the card's own words.
        ModifierKind::StartOfTurnEffect { label, .. } => label.clone(),
        // B5 E15, E39 (R455): the play pipeline's price rules and Forever&'s rider, worded by their owner:
        // `costRuleModifierLabel` is `costRuleText(rule, expiry.until === "used")`.
        ModifierKind::CostRule { rule } => {
            cost_rule_text(rule, matches!(modifier.expiry, ModifierExpiry::Used)).to_string()
        }
        ModifierKind::EnchantNextSpell { enchantment } => enchant_next_spell_label(enchantment).to_string(),
        // B5 E8: Classic+ #22 Blood Moon's base face, read off the modifier alone.
        ModifierKind::HealToDamage { .. } => {
            "Healing on your enemies deals Pierce damage instead".to_string()
        }
        // R449: Classic #23 Devil's Pact's replacement, named as the card every play becomes.
        // R757: #98's Armor Up, until the player's next turn.
        ModifierKind::HeroArmor { amount } => format!("Your hero has {amount} Armor until your next turn"),
        ModifierKind::ReplacePlays { def_id, radiant } => {
            let name = find_def_in(state, def_id).map_or_else(|| def_id.clone(), |def| def.name.clone());
            format!(
                "Each card you play becomes {}{name}",
                if *radiant { "a Radiant " } else { "a " }
            )
        }
    }
}

/// §10.8 does not list the player modifiers, so R169 decides them: both seats carry the list, since
/// every Core modifier is installed by a card played face-up and `modifierChanged` is already public
/// in both directions (see `redact_event`). Only the id and the caption travel.
///
/// "Face-up" is the load-bearing word and "Cry" would be wrong: #35 Lunar Eclipse and #78 /fullsend
/// are Spells, which never enter the field and so can never have a Cry; #64 and #79 install theirs
/// from other hooks. What all five share is that the play itself was public.
///
/// R48: a modifier that covers the controller's *next* turn is installed at once and bites later, so
/// the caption says so while `modifier_is_live` is still false — otherwise #77's badge would claim a
/// discount on the very turn the discount does nothing.
fn modifier_views(state: &GameState, player: PlayerId) -> Vec<ModifierView> {
    let mut views: Vec<ModifierView> = state.players[player]
        .mods
        .iter()
        .map(|modifier| {
            let label = modifier_label(state, modifier, echo_grant_of(state, player, modifier));
            ModifierView {
                id: modifier.id.clone(),
                label: if modifier_is_live(state, modifier) {
                    label
                } else {
                    format!("{label} (next turn)")
                },
            }
        })
        .collect();
    // §6.3 Mana: the next refresh's rider (#21 Hinder, #24 Efficiency Dividend) is a modifier too, one
    // badge under the id `modifierChanged` names for it, while it is not 0 (R169).
    let rider = state.players[player].mana.next_turn_mod;
    if rider != 0 {
        views.push(ModifierView {
            id: NEXT_REFRESH_MODIFIER_ID.to_string(),
            label: format!(
                "Next refresh {}{} mana",
                if rider > 0 { "+" } else { "−" },
                rider.abs()
            ),
        });
    }
    views
}

// ---------------------------------------------------------------------------
// One side of the board
// ---------------------------------------------------------------------------

/// R64: the zones this player is holding for a dying Reborn unit, as a mask per row — and B3.1 rule 6's
/// backrow zones held for an animated "Animated on your turn" card's return (`zones::is_reserved`).
fn reserved_mask(state: &GameState, player: PlayerId) -> RowFlags {
    let mask = |row: Row| -> Vec<bool> {
        slots_of(player, row)
            .into_iter()
            .map(|zone_ref| is_reserved(state, zone_ref))
            .collect()
    };
    RowFlags {
        units: mask(Row::Units),
        backrow: mask(Row::Backrow),
    }
}

/// R1141: how many cards of the opponent's hand carry a mark, while that hand is a count; `None` when
/// none does, so a seat with no marked hand card looks as it did.
fn hand_marked_view(state: &GameState, player: PlayerId, viewer: PlayerId) -> Option<i32> {
    if player == viewer || state.result.is_some() {
        return None;
    }
    let marked = state.players[player]
        .hand
        .iter()
        .filter(|card| !marks_on(state, &card.id).is_empty())
        .count() as i32;
    (marked > 0).then_some(marked)
}

fn side_view(state: &GameState, player: PlayerId, viewer: PlayerId) -> SideView {
    let side: &PlayerState = &state.players[player];
    let powers = hero_powers_of(state, player);
    // R667: asked only for the viewer's own hand, the one hand a warning may ride.
    let countered: IndexSet<String> = if player == viewer {
        countered_hand_cards(state, viewer)
            .into_iter()
            .map(|id| id.to_string())
            .collect()
    } else {
        IndexSet::new()
    };
    // §10.8: the viewer's own hand in full, the opponent's as a count — until the game is over, when
    // both hands are revealed (R434): the opponent's cards as they stand, as their owner saw them.
    let hand = if player == viewer {
        HandView::Cards(
            side.hand
                .iter()
                .map(|card| {
                    with_preview(
                        with_counter_warning(
                            with_condition(
                                with_embiggen_cost(hand_card_view(state, card), state, card),
                                condition_active(state, card, viewer, ConditionZone::Hand),
                            ),
                            countered.contains(&card.id),
                        ),
                        preview_of(state, card, viewer, ConditionZone::Hand),
                    )
                })
                .collect(),
        )
    } else if state.result.is_some() {
        HandView::Cards(side.hand.iter().map(|card| hand_card_view(state, card)).collect())
    } else {
        HandView::Count {
            count: side.hand.len() as i32,
        }
    };
    SideView {
        player,
        hero: HeroView {
            health: side.hero.health,
            // §10.8's "armor": the number §4.4 step 2 will actually subtract, so the hero panel is read
            // the same way a unit's is — never the stored field alone. `hero_armor_of` adds every backrow
            // grant (#84 Going Long) to it, summed per R124, and the grant disappears from the view the
            // moment the granting card leaves the backrow.
            armor: hero_armor_of(state, player),
            power: powers.first().cloned(),
            powers,
        },
        // R169: the badge list beside the hero, public on both seats.
        modifiers: modifier_views(state, player),
        mana: ManaView {
            current: side.mana.current,
            max: side.mana.max,
        },
        hand,
        // §9.1: a library's order ships to nobody, and the opponent's library is a count and nothing
        // else. R310–R312: the viewer's own is a list without order as well, of what they were shown
        // going in (`own_library.rs`), with no instance id or position in it.
        library_count: side.library.len() as i32,
        own_library: if player == viewer {
            Some(own_library_view(state, player))
        } else {
            None
        },
        graveyard: side.graveyard.iter().map(|card| card_view(state, card)).collect(),
        exile: side.exile.iter().map(|card| card_view(state, card)).collect(),
        // §10.5 step 4, R98: a Spell between its play and its graveyard. Playing it was public. R448: a
        // card announced to be set face-down waits here too, and the other player sees a card back.
        resolving: side
            .resolving
            .iter()
            .map(|card| {
                if announced_face_down_to(state, card, viewer) {
                    bare_card_view(HIDDEN_ID.to_string(), HIDDEN_ID.to_string(), false, HIDDEN_COST)
                } else {
                    with_copies(card_view(state, card), state, card)
                }
            })
            .collect(),
        units: side
            .units
            .iter()
            .map(|pile| pile.as_ref().and_then(|pile| unit_view_of(state, pile, viewer)))
            .collect(),
        backrow: side
            .backrow
            .iter()
            .map(|card| backrow_view(state, card.as_ref(), viewer))
            .collect(),
        carried: carried_view(state, player, viewer),
        hand_cap: side.hand_cap,
        hand_marked: hand_marked_view(state, player, viewer),
        locks: RowFlags {
            units: side.locks.units.clone(),
            backrow: side.locks.backrow.clone(),
        },
        reserved: reserved_mask(state, player),
        fatigue_count: side.fatigue_count,
    }
}

// ---------------------------------------------------------------------------
// The open prompt
// ---------------------------------------------------------------------------

/// A prompt option with nothing but its key and its label.
fn bare_option(key: String, label: String) -> PendingOption {
    PendingOption {
        key,
        label,
        instance_id: None,
        def_id: None,
        player: None,
        row: None,
        lane: None,
        cost: None,
        radiant: None,
        chinese: None,
    }
}

/// §10.8: "a card revealed out of a library is revealed only as an option of the prompt that reveals
/// it: the chooser sees it in full". These options only ever travel to the chooser (R81), so naming
/// the definition behind an option is exactly what the chooser is owed.
fn option_view(state: &GameState, viewer: PlayerId, option: &PromptOption) -> PendingOption {
    // B5 E18: a `pick` option's cost against the budget, and the face an option shows when it is Radiant.
    let mut base = bare_option(option.key.clone(), option.label.clone());
    base.cost = option.cost;
    base.radiant = if option.radiant == Some(true) {
        Some(true)
    } else {
        None
    };
    match &option.selection {
        Selection::Instance { instance_id } => {
            let card = find_instance(state, instance_id);
            // R177: a prompt may offer a card its chooser may not read — a target prompt reaching an
            // enemy face-down trap (#49, #50, an Echo repeat's fresh pick). The option is the zone's card
            // and nothing more: the id to answer with, never the definition, and neither the label nor
            // the key the engine built from its name. A card revealed out of a library is the opposite
            // case: the prompt IS its reveal, so the chooser sees it in full (above) — and so is B5 E17's
            // look at the opponent's hand (Classic #11), whose options only their chooser is sent (R81).
            if let Some(card) = card
                && is_face_down_to(state, card, viewer)
            {
                let mut hidden =
                    bare_option(format!("instance:{instance_id}"), HIDDEN_OPTION_LABEL.to_string());
                hidden.instance_id = Some(instance_id.clone());
                return hidden;
            }
            base.instance_id = Some(instance_id.clone());
            if let Some(card) = card {
                base.def_id = Some(card.def_id.clone());
                if card.radiant {
                    base.radiant = Some(true);
                }
                // ME-CN, R1301: the chooser reads this card, so its language travels with it.
                if card.chinese == Some(true) {
                    base.chinese = Some(true);
                }
            }
            base
        }
        Selection::Hero { player } => {
            base.player = Some(*player);
            base
        }
        Selection::Zone { player, row, lane } => {
            base.player = Some(*player);
            base.row = Some(*row);
            base.lane = Some(*lane);
            base
        }
        Selection::Mode { option } => {
            // §6.3 Discover offers definitions, as `mode` options whose string is a catalog def id.
            if let Some(def) = find_def_in(state, option) {
                base.def_id = Some(def.id.clone());
            }
            base
        }
        Selection::None => base,
    }
}

/// §10.6, R81: the other player learns that a prompt is open and whose it is, never its options.
fn pending_view(state: &GameState, viewer: PlayerId) -> Option<PendingView> {
    let Some(pending) = &state.pending else {
        return mulligan_pending_view(state, viewer);
    };
    if pending.player_id != viewer {
        return Some(PendingView::Elsewhere(PendingElsewhereView {
            for_you: false,
            pending_for: pending.player_id,
        }));
    }
    Some(prompt_view(state, viewer, pending))
}

/// The chooser's own prompt, copied field by field: `resume` never travels, so nothing a prompt keeps
/// for its answer — the owner a prompt the other player holds continues as (B5 E18), a multiple-choice
/// problem's key (R465) — can leave the engine through the view.
fn prompt_view(state: &GameState, viewer: PlayerId, pending: &PendingChoice) -> PendingView {
    PendingView::ForYou(PendingPromptView {
        for_you: true,
        choice_id: pending.id.clone(),
        kind: pending.kind,
        options: pending
            .options
            .iter()
            .map(|option| option_view(state, viewer, option))
            .collect(),
        min: pending.min,
        max: pending.max,
        prompt: pending.prompt.clone(),
        budget: pending.budget,
    })
}

/// R265, R266: while both mulligans are open, a seat that still owes one sees its own prompt, and a
/// seat that has answered sees only that the other seat still owes one — as it would a prompt the
/// other seat held — never what that seat is choosing from or has chosen.
fn mulligan_pending_view(state: &GameState, viewer: PlayerId) -> Option<PendingView> {
    if let Some(own) = mulligan_prompt_for(state, viewer) {
        return Some(prompt_view(state, viewer, own));
    }
    let other = opponent_of(viewer);
    if mulligan_prompt_for(state, other).is_none() {
        None
    } else {
        Some(PendingView::Elsewhere(PendingElsewhereView {
            for_you: false,
            pending_for: other,
        }))
    }
}

/// R265, R266: who has answered, and what the viewer kept; absent outside the mulligan window.
fn mulligan_view(state: &GameState, viewer: PlayerId) -> Option<MulliganView> {
    let open = state.mulligan.as_ref()?;
    let own = &open[viewer].keep;
    Some(MulliganView {
        you_ready: own.is_some(),
        opponent_ready: open[opponent_of(viewer)].keep.is_some(),
        kept: own.clone(),
    })
}

/// R269: the standing draw offer, public to both seats; absent when none.
fn draw_offer_view(state: &GameState) -> Option<DrawOfferView> {
    standing_draw_offer(state).map(|by| DrawOfferView { by })
}

// ---------------------------------------------------------------------------
// Events (§10.10)
// ---------------------------------------------------------------------------

/// An event's JSON object, `{ ...event }`, as `redact_event` reads and changes it. Until something is
/// changed it is not written out at all: a field is read off the event itself (`field_of`), and an
/// event nothing was changed on is the event, cloned. Most events a view carries are shown as they
/// are, and writing each one out as a `Map` and reading it back was most of a view's cost.
struct Shown<'e> {
    event: &'e GameEvent,
    /// The event's JSON, once a change needed it.
    map: Option<Map<String, Value>>,
}

impl<'e> Shown<'e> {
    fn new(event: &'e GameEvent) -> Shown<'e> {
        Shown { event, map: None }
    }

    /// The JSON, written out now if it was not yet.
    fn map(&mut self) -> &mut Map<String, Value> {
        let event = self.event;
        self.map.get_or_insert_with(|| match serde_json::to_value(event) {
            Ok(Value::Object(map)) => map,
            _ => panic!("an event serialises to an object"),
        })
    }

    /// The JSON value at `key` (absent: `None`).
    fn get(&self, key: &str) -> Option<Value> {
        match &self.map {
            Some(map) => map.get(key).cloned(),
            None => field_of(self.event, key),
        }
    }

    fn insert(&mut self, key: String, value: Value) {
        self.map().insert(key, value);
    }

    fn remove(&mut self, key: &str) -> Option<Value> {
        if self.map.is_none() && self.get(key).is_none() {
            return None;
        }
        self.map().remove(key)
    }
}

/// One top-level field of `event`'s JSON, as `serde_json::to_value(event)?.get(key)` reads it, without
/// writing out the rest.
fn field_of(event: &GameEvent, key: &str) -> Option<Value> {
    match serde::Serialize::serialize(event, field_capture::FieldOf { key }) {
        Ok(found) => found,
        Err(_) => serde_json::to_value(event)
            .ok()
            .and_then(|value| value.get(key).cloned()),
    }
}

/// `field_of`'s serializer: an object's fields are passed over unwritten, but for the one asked for,
/// which is written with `serde_json::to_value` (the last one, should a key come twice, as a `Map`
/// keeps it). Anything but an object is an error, and `field_of` then writes the event out.
mod field_capture {
    use serde::ser::{self, Impossible, Serialize};
    use serde_json::{Error, Value};

    pub struct FieldOf<'k> {
        pub key: &'k str,
    }

    pub struct Fields<'k> {
        key: &'k str,
        found: Option<Value>,
        /// A map's last key was the one asked for.
        matched: bool,
    }

    fn not_an_object<T>() -> Result<T, Error> {
        Err(ser::Error::custom("not an object"))
    }

    impl<'k> ser::Serializer for FieldOf<'k> {
        type Ok = Option<Value>;
        type Error = Error;
        type SerializeSeq = Impossible<Option<Value>, Error>;
        type SerializeTuple = Impossible<Option<Value>, Error>;
        type SerializeTupleStruct = Impossible<Option<Value>, Error>;
        type SerializeTupleVariant = Impossible<Option<Value>, Error>;
        type SerializeMap = Fields<'k>;
        type SerializeStruct = Fields<'k>;
        type SerializeStructVariant = Impossible<Option<Value>, Error>;

        fn serialize_map(self, _len: Option<usize>) -> Result<Fields<'k>, Error> {
            Ok(Fields {
                key: self.key,
                found: None,
                matched: false,
            })
        }
        fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Fields<'k>, Error> {
            Ok(Fields {
                key: self.key,
                found: None,
                matched: false,
            })
        }
        fn serialize_bool(self, _v: bool) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_i8(self, _v: i8) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_i16(self, _v: i16) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_i32(self, _v: i32) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_i64(self, _v: i64) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_u8(self, _v: u8) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_u16(self, _v: u16) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_u32(self, _v: u32) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_u64(self, _v: u64) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_f32(self, _v: f32) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_f64(self, _v: f64) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_char(self, _v: char) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_str(self, _v: &str) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_bytes(self, _v: &[u8]) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_none(self) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_some<T: Serialize + ?Sized>(self, _value: &T) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_unit(self) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_unit_struct(self, _name: &'static str) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
        ) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_newtype_struct<T: Serialize + ?Sized>(
            self,
            _name: &'static str,
            _value: &T,
        ) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_newtype_variant<T: Serialize + ?Sized>(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Option<Value>, Error> {
            not_an_object()
        }
        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
            not_an_object()
        }
        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
            not_an_object()
        }
        fn serialize_tuple_struct(
            self,
            _name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleStruct, Error> {
            not_an_object()
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Error> {
            not_an_object()
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Error> {
            not_an_object()
        }
    }

    impl ser::SerializeStruct for Fields<'_> {
        type Ok = Option<Value>;
        type Error = Error;
        fn serialize_field<T: Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Error> {
            if key == self.key {
                self.found = Some(serde_json::to_value(value)?);
            }
            Ok(())
        }
        fn end(self) -> Result<Option<Value>, Error> {
            Ok(self.found)
        }
    }

    impl ser::SerializeMap for Fields<'_> {
        type Ok = Option<Value>;
        type Error = Error;
        fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
            self.matched = serde_json::to_value(key)?.as_str() == Some(self.key);
            Ok(())
        }
        fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
            if self.matched {
                self.found = Some(serde_json::to_value(value)?);
            }
            Ok(())
        }
        fn end(self) -> Result<Option<Value>, Error> {
            Ok(self.found)
        }
    }
}

/// A string field of an event's JSON, or "" when it has none (every key read below is always set).
fn text_at(shown: &Shown, key: &str) -> String {
    shown
        .get(key)
        .as_ref()
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// A nullable string field of an event's JSON: `None` for `null` or absent.
fn nullable_at(shown: &Shown, key: &str) -> Option<String> {
    shown
        .get(key)
        .as_ref()
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// A list of strings on an event's JSON (`targets`, `instanceIds`, `hiddenFrom`).
fn texts_at(shown: &Shown, key: &str) -> Option<Vec<String>> {
    shown.get(key).as_ref().and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect()
    })
}

/// `{ ...event, key: HIDDEN_ID, … }` for each key.
fn hide(shown: &mut Shown, keys: &[&str]) {
    for key in keys {
        shown.insert((*key).to_string(), Value::String(HIDDEN_ID.to_string()));
    }
}

/// The redacted JSON, read back as the event it is; the event itself when nothing was changed.
fn rebuild(shown: Shown) -> GameEvent {
    let Some(map) = shown.map else {
        return shown.event.clone();
    };
    match serde_json::from_value(Value::Object(map)) {
        Ok(event) => event,
        Err(error) => panic!("a redacted event no longer reads as an event: {error}"),
    }
}

/// R97. Every event carries ids and several carry a `defId` as well — `drawn` names the card that
/// went into a hand, `bounced` the one that left the field for it, `costChanged` a card discounted
/// in hand — so the animation stream is filtered like every other zone. An event that names a card
/// this viewer may not read keeps its type and every field §10.10's animation table needs, with the
/// identity replaced by `HIDDEN_ID`: redacted, never dropped, so the cue still plays as a card back.
///
/// The match is exhaustive over every event type on purpose (§10.3): with no wildcard, adding an
/// event type does not compile until someone decides what it reveals.
fn redact_event(
    state: &GameState,
    viewer: PlayerId,
    event: &GameEvent,
    replaced: &Replacements,
) -> GameEvent {
    let hidden = |id: &str| -> bool { !may_read(state, viewer, id, replaced) };
    let mut shown = Shown::new(event);
    let instance = text_at(&shown, "instanceId");

    match event.event_type() {
        // A card named with its definition: both go, or neither.
        //
        // `cardResolved` (§10.5 step 7) is one of these rather than a public event: R97 judges a card
        // by where it sits *now*, and once resolution is over a Spell has reached the graveyard and a
        // permanent is on the field, both public — so it ordinarily reads openly, and `may_read` keeps
        // the sentinel for the card that ended up somewhere this viewer may not read (a Trap set
        // face-down, a card resolved back into a hand or a library). Its `player` and `permanent` are
        // not identity fields and never travel redacted; `permanent` is R61's "still in play" answer,
        // which #85 keys on. The face that resolved (`radiant`) is the card's, so it goes with the id.
        GameEventType::CardResolved => {
            // R119's `arrivedDuring` is the engine's own bookkeeping, and it names face-down traps (#95);
            // so is the exit mark the event happened at (R174, R212).
            shown.remove("arrivedDuring");
            shown.remove("exitsFrom");
            if hidden(&instance) {
                shown.remove("radiant");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R97, R177: `killerId` names a card as well, and the card that dealt the lethal hit — a unit,
        // or a Spell whose damage was lethal — may since have gone somewhere this viewer cannot read,
        // like the #31 KY's Math Equation that returns to its owner's hand at the end of the turn.
        GameEventType::Destroyed => {
            let killer_hidden = nullable_at(&shown, "killerId").is_some_and(|killer| hidden(&killer));
            if hidden(&instance) {
                shown.remove("radiant");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            if killer_hidden {
                hide(&mut shown, &["killerId"]);
            }
            rebuild(shown)
        }

        // R119's `arrivedDuring` and the exit mark on a play's step-4 pair are the engine's bookkeeping,
        // as on `cardResolved`. R227: `formerId` is the id a card set face-down had, and it goes with the
        // card's identity — shown to a viewer who may read the card, never to one who may not, or the old
        // id would name the face-down card after all (R177).
        GameEventType::CardPlayed | GameEventType::Summoned => {
            shown.remove("arrivedDuring");
            shown.remove("exitsFrom");
            if hidden(&instance) {
                // ME-ALTPLAY, R1046: a hidden set card shows neither its `x` nor its `embiggened`.
                shown.remove("formerId");
                shown.remove("x");
                shown.remove("embiggened");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R316: a card a full library refused is judged like the cards below: one that went to the
        // graveyard reads as long as it stays there. One never created is in no pile and never was
        // anywhere hidden, so it reads as the card it copies does (`copyOf`), and openly when it copies
        // none — #33's copy of a Trap set face-down names the trap no more than the trap does. `copyOf`
        // is the engine's bookkeeping and never travels.
        GameEventType::LibraryOverflow => {
            let copy_of = shown
                .remove("copyOf")
                .and_then(|value| value.as_str().map(str::to_string));
            let unread = hidden(&instance) || copy_of.is_some_and(|copied| hidden(&copied));
            if unread {
                // The face it would have had is the card's too, so it goes with the identity.
                shown.remove("radiant");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R317: a burned card lands in its owner's graveyard, or ceases to exist (R11), so both seats read
        // it — the hand it never entered is not where it is — until something takes it somewhere hidden.
        GameEventType::EnteredGraveyard
        | GameEventType::Exiled
        | GameEventType::Bounced
        | GameEventType::Burned
        | GameEventType::Discarded
        | GameEventType::Drawn
        | GameEventType::AddedToHand => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }

        // R177: a Make Radiant on a card in a library (#42's roll over every card, top down) is one nobody
        // could read where it happened (§3), and read openly once the card does, its place in the batch
        // would say where it lay. The event's `zone` is where it happened, so it stays unread for good.
        //
        // And a cue on the other player's card this viewer may not read says only whose it was: a random
        // pick over several hidden zones (#28's hand, library and field) picks among non-Radiant cards
        // only (R60), so a cue located in the hand, or at a face-down trap's lane, would tell this viewer
        // that the hand still held a base-face card, or that the trap was base-face (R33). The zone is
        // given as that player's hand, the region this viewer is shown the player's unread cards in.
        GameEventType::RadiantSet => {
            let zone = shown.get("zone").unwrap_or(Value::Null);
            let in_library = zone.get("z").and_then(Value::as_str) == Some("library");
            let unread = in_library || hidden(&instance);
            if !unread {
                return event.clone();
            }
            let owner = zone
                .get("player")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let shown_zone = if owner == viewer.as_str() {
                zone
            } else {
                serde_json::json!({ "z": "hand", "player": owner })
            };
            hide(&mut shown, &["instanceId", "defId"]);
            shown.insert("zone".to_string(), shown_zone);
            rebuild(shown)
        }

        // R154: the one identity R97's "judged by where the card sits now" cannot decide, so the row
        // names the seat instead — the controller reads `instanceId` and `defId`, the other player
        // reads the sentinel. Firing a Trap consumes it into its owner's graveyard (a public pile) or
        // leaves a Field Trap face-up, so `may_read` would call every fired trap public and hand the
        // opponent the card's identity on the event that announces the flip. The animation needs the
        // opposite: §10.10's `trapFired` row flips a card back in the right lane, and §10.8 gives a
        // face-down trap no instance id to hang that on. `row`, `lane` and `controller` are not
        // identity and always travel, which is the whole point of the row — the opponent animates the
        // flip in the right zone without being told which card it was.
        //
        // R763: to its controller it follows R97, so a fired trap since shuffled into a library, or
        // taken into the other player's hand, is the sentinel there too.
        GameEventType::TrapFired => {
            if text_at(&shown, "controller") == viewer.as_str() && !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }

        // §9.1: library order is hidden from both players, so the slot never travels either way.
        GameEventType::ShuffledIn => {
            if hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            shown.insert("position".to_string(), Value::from(HIDDEN_POSITION));
            rebuild(shown)
        }

        // R177: a Replace puts the new card where the old one was (§6.3), and the old one ceased to
        // exist there (R35), so no zone of its own is left to judge it by — it is judged by its
        // replacement's. A card replaced inside a library or a face-down backrow zone never reads.
        GameEventType::Transformed => {
            let new_hidden = hidden(&text_at(&shown, "newInstanceId"));
            shown.remove("hiddenFrom");
            if new_hidden || hidden(&instance) {
                hide(&mut shown, &["instanceId", "fromDefId"]);
            }
            if new_hidden {
                hide(&mut shown, &["newInstanceId", "toDefId"]);
            }
            rebuild(shown)
        }

        GameEventType::Fused => {
            let ids: Vec<Value> = texts_at(&shown, "instanceIds")
                .unwrap_or_default()
                .into_iter()
                .map(|id| Value::String(if hidden(&id) { HIDDEN_ID.to_string() } else { id }))
                .collect();
            shown.insert("instanceIds".to_string(), Value::Array(ids));
            if hidden(&text_at(&shown, "resultInstanceId")) {
                hide(&mut shown, &["resultInstanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R177: a buff's size is the card's too — #89 Corpse Eater gains double on its radiant face —
        // so a hidden card's buff keeps its type for the cue and says nothing of how much.
        GameEventType::Buffed => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId"]);
            shown.insert("attack".to_string(), Value::from(0));
            shown.insert("health".to_string(), Value::from(0));
            rebuild(shown)
        }

        // One instance, no definition: the id alone would still name a card in a hidden zone.
        GameEventType::DivineShieldLost | GameEventType::KeywordGranted | GameEventType::PositionSwitched => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId"]);
            rebuild(shown)
        }
        // R227: as on `summoned`, a fresh id's `formerId` goes with the card's identity (C+ #35, R419).
        GameEventType::ControlChanged => {
            if !hidden(&instance) {
                return event.clone();
            }
            shown.remove("formerId");
            hide(&mut shown, &["instanceId"]);
            rebuild(shown)
        }

        // R385: a Brittle count is its card's, read only where the card is (a face-down card's by its
        // controller alone), so on a card this viewer may not read the number goes with the id.
        GameEventType::CounterChanged => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId"]);
            if text_at(&shown, "counter") == "brittle" {
                shown.insert("value".to_string(), Value::from(HIDDEN_COUNT));
            }
            rebuild(shown)
        }

        // R177: the new cost is the card's too, and over a library it would give the order away — so a
        // change made in a library stays unread for good (`hiddenFrom`), whatever became of the card.
        GameEventType::CostChanged => {
            let hidden_from = texts_at(&shown, "hiddenFrom");
            shown.remove("hiddenFrom");
            if hidden_from.is_some_and(|players| players.iter().any(|p| p == viewer.as_str()))
                || hidden(&instance)
            {
                hide(&mut shown, &["instanceId"]);
                shown.insert("cost".to_string(), Value::from(HIDDEN_COST));
            }
            rebuild(shown)
        }

        GameEventType::Healed => {
            if !hidden(&text_at(&shown, "targetId")) {
                return event.clone();
            }
            hide(&mut shown, &["targetId"]);
            rebuild(shown)
        }

        // R1361: the report of a hit Armor took whole names what `damage` names, and hides it alike.
        GameEventType::Damage | GameEventType::DamageAbsorbed => {
            if nullable_at(&shown, "sourceId").is_some_and(|source| hidden(&source)) {
                hide(&mut shown, &["sourceId"]);
            }
            if hidden(&text_at(&shown, "targetId")) {
                hide(&mut shown, &["targetId"]);
            }
            rebuild(shown)
        }

        GameEventType::AttackDeclared => {
            if hidden(&text_at(&shown, "attackerId")) {
                hide(&mut shown, &["attackerId"]);
            }
            if hidden(&text_at(&shown, "targetId")) {
                hide(&mut shown, &["targetId"]);
            }
            rebuild(shown)
        }

        GameEventType::AttackCancelled => {
            for key in ["attackerId", "targetId", "byInstanceId"] {
                if hidden(&text_at(&shown, key)) {
                    hide(&mut shown, &[key]);
                }
            }
            rebuild(shown)
        }

        // Public through and through: these name a player, a zone or a number, never a card. A
        // `promptOpened` event says a prompt is open and whose, which is all §10.6 grants.
        // R315: a fatigue draw names a player and two numbers, and the fatigue count is public (§10.8).
        GameEventType::HealthLost
        | GameEventType::Fatigue
        | GameEventType::ModifierChanged
        | GameEventType::Rotated
        | GameEventType::Swapped
        | GameEventType::Locked
        | GameEventType::ManaChanged
        | GameEventType::TurnStarted
        | GameEventType::TurnEnded
        | GameEventType::TurnAutoEnded
        | GameEventType::PromptOpened
        | GameEventType::PromptAnswered
        | GameEventType::DrawOffered
        | GameEventType::DrawAnswered
        | GameEventType::GameOver => event.clone(),

        // ---- Patch v0.2.0 (docs/classic-sets.md B3, B5) ----

        // B5 E1: an announce shows what `cardPlayed` would. A card being set face-down is its zone only
        // to the other player (R97, R227): the identity and the targets it declared go, the zone stays.
        GameEventType::CardAnnounced => {
            let face_down = shown.get("faceDown").as_ref().and_then(Value::as_bool) == Some(true);
            let unread = (face_down && text_at(&shown, "player") != viewer.as_str()) || hidden(&instance);
            let targets = texts_at(&shown, "targets").unwrap_or_default();
            if !unread {
                let shown_targets: Vec<Value> = targets
                    .into_iter()
                    .map(|id| Value::String(if hidden(&id) { HIDDEN_ID.to_string() } else { id }))
                    .collect();
                shown.insert("targets".to_string(), Value::Array(shown_targets));
                return rebuild(shown);
            }
            // R448: whether a face-down card is a Trap or a Field Trap is the card's too (R33), so the
            // other player reads every one as a Trap, as its backrow will show it.
            if face_down {
                shown.insert(
                    "cardType".to_string(),
                    Value::String(CardType::Trap.as_str().to_string()),
                );
            }
            hide(&mut shown, &["instanceId", "defId"]);
            let hidden_targets: Vec<Value> = targets
                .iter()
                .map(|_| Value::String(HIDDEN_ID.to_string()))
                .collect();
            shown.insert("targets".to_string(), Value::Array(hidden_targets));
            rebuild(shown)
        }

        // B5 E1, E2, B3.3: judged by where the card is now (R97) — a countered card in a public pile reads,
        // one stolen into a hand reads to that hand's owner only, a crumbled card reads once it is in the
        // graveyard. `byInstanceId` is the countering card, a fired trap by then, judged the same way.
        GameEventType::Countered => {
            let by_hidden = nullable_at(&shown, "byInstanceId").is_some_and(|by| hidden(&by));
            if hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            if by_hidden {
                hide(&mut shown, &["byInstanceId"]);
            }
            rebuild(shown)
        }
        // B5 E2, E16, R466: a stolen card reads to whoever could read it where it was taken from — the
        // hand's holder, the controller of a face-down zone, everyone for a face-up card or a public pile
        // (`readableFrom`, written as it was taken) — and to whoever can read it where it is now (R97). A
        // card out of a library was nobody's to read, so its old owner never learns which card left.
        GameEventType::Stolen => {
            shown.remove("readableFrom");
            let readable = match event {
                GameEvent::Stolen {
                    zone,
                    from,
                    readable_from,
                    ..
                } => readable_where_stolen(*zone, *from, readable_from.as_deref(), viewer),
                _ => false,
            };
            if hidden(&instance) && !readable {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }
        GameEventType::Crumbled => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }

        // B3.4, R386, R177: a change is the card's, and over a library or a hidden hand it would say
        // which card changed and how — so it stays unread for good for whoever could not read the card
        // where it changed (`hiddenFrom`), and for whoever cannot read it now.
        GameEventType::Degraded | GameEventType::Upgraded => {
            let hidden_from = texts_at(&shown, "hiddenFrom");
            shown.remove("hiddenFrom");
            if hidden_from.is_some_and(|players| players.iter().any(|p| p == viewer.as_str()))
                || hidden(&instance)
            {
                hide(&mut shown, &["instanceId", "defId"]);
                let change = serde_json::to_value(hidden_tuning_change()).unwrap_or(Value::Null);
                shown.insert("change".to_string(), change);
            }
            rebuild(shown)
        }
        // Classic+ #41: a number set outright is the card's as well, so it follows `degraded`.
        GameEventType::NumberChanged => {
            let hidden_from = texts_at(&shown, "hiddenFrom");
            shown.remove("hiddenFrom");
            if hidden_from.is_some_and(|players| players.iter().any(|p| p == viewer.as_str()))
                || hidden(&instance)
            {
                hide(&mut shown, &["instanceId", "defId", "key"]);
                shown.insert("value".to_string(), Value::from(0));
            }
            rebuild(shown)
        }

        // B5 E9: a hit, an attack or a pick moves between cards on the field or heroes, all public; a
        // card that has since gone somewhere unreadable is the sentinel, as on `damage`.
        GameEventType::Redirected => {
            if hidden(&text_at(&shown, "fromId")) {
                hide(&mut shown, &["fromId"]);
            }
            if hidden(&text_at(&shown, "toId")) {
                hide(&mut shown, &["toId"]);
            }
            if nullable_at(&shown, "byInstanceId").is_some_and(|by| hidden(&by)) {
                hide(&mut shown, &["byInstanceId"]);
            }
            rebuild(shown)
        }

        // A card on the field acting face-up (an ability, an animation, a quest, a flicker, a mark): public
        // while it is readable, the sentinel once it has gone somewhere hidden (R97).
        GameEventType::Activated
        | GameEventType::Animated
        | GameEventType::Deanimated
        | GameEventType::Flickered => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }
        GameEventType::QuestProgressed
        | GameEventType::QuestCompleted
        | GameEventType::Marked
        | GameEventType::Translated => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId"]);
            rebuild(shown)
        }

        // R436: Call to Chaos names what it rolled to both players; the card itself follows R97.
        GameEventType::ChaosRolled => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }

        GameEventType::TurnCutShort => {
            if !nullable_at(&shown, "byInstanceId").is_some_and(|by| hidden(&by)) {
                return event.clone();
            }
            hide(&mut shown, &["byInstanceId"]);
            rebuild(shown)
        }

        // Public: a zone, a player, a number.
        GameEventType::Unlocked
        | GameEventType::HealthSet
        | GameEventType::RolledBack
        | GameEventType::Glitched
        | GameEventType::DrawLimited => {
            let source_hidden = event.event_type() == GameEventType::HealthSet
                && nullable_at(&shown, "sourceId").is_some_and(|source| hidden(&source));
            if !source_hidden {
                return event.clone();
            }
            hide(&mut shown, &["sourceId"]);
            rebuild(shown)
        }
    }
}

/// B5 E2, E16, R466: whether `viewer` could read the card a `stolen` event names where it was taken
/// from. `ownership::change_owner` writes who could (`readableFrom`) as it takes the card; an event
/// without the record is judged by its pile alone — a hand is its holder's (§9.1), a library nobody's,
/// a graveyard, an exile pile or the resolving zone everyone's (a play is public, R98), and a card off
/// the field nobody's, since whether it stood face-down there is not otherwise on the event.
fn readable_where_stolen(
    zone: ZoneName,
    from: PlayerId,
    readable_from: Option<&[PlayerId]>,
    viewer: PlayerId,
) -> bool {
    if let Some(readable_from) = readable_from {
        return readable_from.contains(&viewer);
    }
    match zone {
        ZoneName::Hand => from == viewer,
        ZoneName::Graveyard | ZoneName::Exile | ZoneName::Resolving => true,
        // A stolen card never comes out of "gone"; TS's switch had no case for it and answered nothing.
        ZoneName::Library | ZoneName::Field | ZoneName::Gone => false,
    }
}

/// R386, R177: what a hidden Degrade or Upgrade shows — that a card changed, never how.
fn hidden_tuning_change() -> TuningChange {
    TuningChange::Number {
        key: HIDDEN_ID.to_string(),
        delta: 0,
    }
}

/// §10.8: "the last N events for animation". `state.applied` is the only event history a state
/// carries (§9.3's nonce dedupe), oldest action first, so flattening it in order and taking the tail
/// is the stream — and it is bounded by `NONCE_HISTORY` already.
///
/// `VIEW_EVENT_LIMIT` is a FLOOR, not a cap: the window never ends inside the newest applied action.
/// BUILD M5-T4 gives every §10.3 event an animation and `PlayerView.events` is the client's only
/// channel for them, so a fixed length silently drops the FRONT of any single action that emits more
/// than it — the client then animates the tail of something whose beginning it was never told about.
/// One #96 My Pawn cancel plus the §10.7 AI turn it hands over is 38 events in one `reduce`, and the
/// three the cancel is made of (`attackDeclared`, `trapFired`, `attackCancelled`) were exactly the
/// ones lost, which made M5-T4's `attackCancelled` row unreachable. Measured: 38 emitted, 32
/// carried, 6 dropped from the front.
///
/// This is SPEC §11 R168, which states the floor and records the measurement above.
fn recent_events(state: &GameState, viewer: PlayerId) -> Vec<GameEvent> {
    let all: Vec<&GameEvent> = state
        .applied
        .iter()
        .flat_map(|entry| entry.events.iter())
        .collect();
    let newest = state.applied.last().map_or(0, |entry| entry.events.len());
    let window = VIEW_EVENT_LIMIT.max(newest);
    let replaced = replacements_of(&all, Some(state));
    // R764: a Glitch's reset or boards (R676, R678) took cards out of the match without a word, so what
    // the events up to the end of its action name of one the state no longer holds is not public, as a
    // token's was (R11). The reset's own setup events end that action and name only the cards it holds.
    let mut voided_until = 0;
    let mut end = 0;
    for entry in &state.applied {
        end += entry.events.len();
        let glitched = entry.events.iter().any(|event| {
            matches!(
                event,
                GameEvent::Glitched {
                    outcome: GlitchOutcome::Reset | GlitchOutcome::Boards,
                    ..
                }
            )
        });
        if glitched {
            voided_until = end;
        }
    }
    let earlier = Replacements {
        voided: true,
        ..replaced.clone()
    };
    let from = all.len().saturating_sub(window);
    all[from..]
        .iter()
        .enumerate()
        .map(|(at, event)| {
            redact_event(
                state,
                viewer,
                event,
                if from + at < voided_until {
                    &earlier
                } else {
                    &replaced
                },
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// §10.8
// ---------------------------------------------------------------------------

/// The one window a player has onto a match (SPEC §10.8). Pure: it reads the state and builds a
/// fresh object, sharing nothing mutable with it. No clock is running (`clockMs` is `null`); a host
/// that runs one asks `view_for_with_clock`.
pub fn view_for(state: &GameState, player_id: PlayerId) -> PlayerView {
    view_for_with_clock(state, player_id, None)
}

/// TS `viewFor(state, playerId, clockMs)`: `clock_ms` is the turn clock the server is running (R79).
/// The engine never reads a clock, so the caller passes the number in and it is `None` whenever
/// nobody is counting.
pub fn view_for_with_clock(state: &GameState, player_id: PlayerId, clock_ms: Option<i32>) -> PlayerView {
    let mut view = PlayerView {
        viewer: player_id,
        turn: state.turn,
        active: state.active,
        phase: state.phase,
        you: side_view(state, player_id, player_id),
        opponent: side_view(state, opponent_of(player_id), player_id),
        pending: pending_view(state, player_id),
        events: recent_events(state, player_id),
        result: state.result.as_ref().map(|result| GameResult {
            winner: result.winner,
            reason: result.reason,
        }),
        clock_ms,
        mulligan: mulligan_view(state, player_id),
        draw_offer: draw_offer_view(state),
        // R345: the viewer's own preference, and only when it is off, so every other view is unchanged.
        auto_end_turn: if state.players[player_id].auto_end_turn == Some(false) {
            Some(false)
        } else {
            None
        },
        defs: None,
    };
    let defs = match_defs_in(state, &view);
    if !defs.is_empty() {
        view.defs = Some(defs);
    }
    view
}

/// R243: the match-made definitions (`state.transient_defs`: a Fuse's, a crafted card's — R77, R102,
/// R179) the finished view names anywhere — a card in a zone, a unit, a prompt option, an event —
/// copied beside it, since no catalog a client holds has them. The view is read after it is built,
/// so only an id that survived redaction brings its definition: a card this viewer may not read is
/// the sentinel by then (R97), and its definition stays in the match.
fn match_defs_in(state: &GameState, view: &PlayerView) -> IndexMap<String, CardDef> {
    fn visit(value: &Value, state: &GameState, defs: &mut IndexMap<String, CardDef>) {
        match value {
            Value::String(text) => {
                if defs.contains_key(text) {
                    return;
                }
                if let Some(def) = state.transient_defs.get(text) {
                    defs.insert(text.clone(), def.clone());
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, state, defs);
                }
            }
            Value::Object(fields) => {
                for item in fields.values() {
                    visit(item, state, defs);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
    let mut defs: IndexMap<String, CardDef> = IndexMap::new();
    if state.transient_defs.is_empty() {
        return defs;
    }
    match serde_json::to_value(view) {
        Ok(value) => visit(&value, state, &mut defs),
        Err(error) => panic!("a view serialises: {error}"),
    }
    defs
}
