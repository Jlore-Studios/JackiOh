//! What one player is allowed to see, and nothing else (SPEC §10.8). The client renders this view and never
//! holds rules or hidden information (CLAUDE.md rule 7, §9.1's trust model).
//!
//! The view is built by *copying* what the viewer is entitled to, never by deleting fields from a state
//! clone: a field added to `GameState` stays invisible until this file names it. The rules:
//!   - §9.1: library order, the opponent's library and hand and face-down traps are hidden; libraries and the
//!     opponent's hand travel as counts. R310–R312: the viewer's own library also travels as an unordered
//!     list of what they were shown going in, never an instance id, so R97 still reads no library card.
//!   - R33: a face-down trap is readable by its *current controller* only; a fired Field Trap (`faceUp`) is
//!     public.
//!   - R13, §3.2: a Stack pile shows its top card and a count of the dormant cards buried under it.
//!   - R81: the other player's prompt (and its `promptOpened`, `promptAnswered`) shows only that it is open
//!     and whose.
//!   - R97: events are redacted, not truncated: one naming a card the viewer may not read keeps its type and
//!     animation fields and shows `HIDDEN_ID` for that card.
//!   - R2652000: a `rolled` batch a face-down trap made is dropped for the other player unless the trap fired
//!     in the same action (`withheld_rolls`).
//!   - R764: a Glitch's reset or boards (R676, R678) takes cards out unseen, so events up to the end of its
//!     action read as the sentinel for each card the state no longer holds.
//!   - R227: a card set face-down took a fresh id; events naming its old id follow it through `formerId`.
//!   - R1074000: a Duos pass (B20) takes a hand card out unseen, so in a duel a card in no pile reads to the
//!     player whose hand the events last show it in, else to all if a token (R11), else nobody.
//!   - R177: a prompt option offering a face-down card names it by id only (and its zone, R821000); an
//!     unreadable card's cost, buff amounts and Replace go with its identity.
//!   - R169: the player modifiers (§10.1 `mods`) travel on both seats as `{ id, label }`.
//!   - R434: once the game is over the opponent's hand travels in full; the rest stays hidden.
//!   - R437: a mark (#50's pending steal) rides every view of the card in both seats, a back included.
//!   - R195, R280, R667, #492, #992, R1844001, R2268000, R849000, R979000, R1571000: what the engine works
//!     out for a card rides the viewer's own cards only, except `preview` and `sick` (every unit).
//!
//! Stats are recomputed on read (`layers.unitView`, §10.4), so no stored total reaches the client. The
//! boundary is SURFACE §4.1, §6.1, §6.4 and §6.6; `VIEW_EVENT_LIMIT` lives in `crate::config`.

use indexmap::{IndexMap, IndexSet};
use serde_json::{Map, Value};

use crate::animated::is_animated;
use crate::announce::announced_face_down_to;
use crate::catalog::{find_def, registered_catalog};
use crate::combat::{ExertionKind, has_exertion_in, is_asleep, why_cannot_declare_in};
use crate::condition::condition_active;
use crate::config::{DRAW_OFFER_BLOCK_TURNS, VIEW_EVENT_LIMIT};
use crate::cost_rules::{cost_rule_text, enchant_next_spell_label};
use crate::counter_warning::countered_hand_cards;
use crate::damage::hero_armor_of;
use crate::echo::echo_grant_of;
use crate::faces::card_type_of;
use crate::instance_view::{hand_keywords_view, instance_data_view};
use crate::layers::{AuraTable, stats_with_buffs};
use crate::mana::{NEXT_REFRESH_MODIFIER_ID, effective_cost, modifier_is_live};
use crate::marks::marks_on;
use crate::own_library::own_library_view;
use crate::params::params_view;
use crate::plague::plague_on;
use crate::play_choices::{embiggen_play_cost, hand_play_choices_truncated, tribute_cost_of};
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
use crate::subsystems::pareto::pareto_hint_ids;
use crate::subsystems::quests::quest_view_of;
use crate::turn::standing_draw_offer;
use crate::wire::{
    AnimatedView, BackrowCounters, BackrowView, CardDef, CardMark, CardType, CardView, CopiedTextView,
    CreditView, DrawOfferView, FaceDownBackrowView, GameEvent, GameEventType, GameResult, GlitchOutcome,
    HandView, HeroPowerView, HeroView, Keyword, KeywordKind, LibraryOverflowOutcome, ManaView, ModifierView,
    MulliganView, PLAYER_IDS, PendingElsewhereView, PendingOption, PendingPromptView, PendingView, PlayerId,
    PlayerView, PreviewValue, PublicBackrowView, Row, RowFlags, Selection, SideView, TuningChange, UnitView,
    Zone, ZoneName, opponent_of,
};
use crate::zones::{
    beneath_at, carried_at, carried_units_of, home_of, is_reserved, is_standing_unit, slot_of, slots_of,
};

/// R97's sentinel for a card the viewer may not read. It collides with no instance (`c<n>`) or
/// catalog id, so the client can test for it; a redacted event still animates, as a card back.
pub const HIDDEN_ID: &str = "hidden";

/// R97, §9.1: the slot a shuffled-in card landed in would give away library order, to either side.
pub const HIDDEN_POSITION: i32 = -1;

/// R177: the cost a `costChanged` reports for a card the viewer may not read: a cost is the card's
/// as much as its definition is, and costs over a library would spell out its order (§9.1).
pub const HIDDEN_COST: i32 = -1;

/// R385: the count a `counterChanged` "brittle" reports for a card the viewer may not read.
const HIDDEN_COUNT: i32 = -1;

/// R177: what a prompt option names when it offers a card the chooser may not read (§10.8, R33).
pub const HIDDEN_OPTION_LABEL: &str = "Face-down card";

// Visibility

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

/// R177: each vanished card's replacement (`transformed`, §6.3, R35; `fused`, R77), whose zone
/// judges it. `state.applied` holds every event a view can show, so it holds every replacement
/// that matters.
#[derive(Clone, Debug, Default)]
struct Replacements {
    /// Each vanished card's replacement.
    replaced_by: IndexMap<String, String>,
    /// The players each replaced card was hidden from where it ceased to exist (`transformed.hiddenFrom`).
    hidden_from: IndexMap<String, Vec<PlayerId>>,
    /// R224: the cards a mulligan returned, in no pile until their shuffle-back: library cards, unread.
    to_library: Option<IndexSet<String>>,
    /// R764: set for the events up to the end of a Glitch's reset or boards (R676, R678): a card they
    /// name that the state no longer holds left unseen and reads to nobody.
    voided: bool,
    /// R1074000: `Some` in a duel, where a pass (B20) takes a hand card out of the state unseen.
    duel: Option<DuelGone>,
    /// R1020: the cards the events last put into a library (`in_library_of`). A Fuse eats a library
    /// card unseen, so one of these the state no longer holds ceased to exist there and reads to nobody.
    in_library: IndexSet<String>,
}

/// R1074000: what a duel's events say of the cards in no pile that took no successor.
#[derive(Clone, Debug, Default)]
struct DuelGone {
    /// Read openly, as in 1v1: a token no pass took (R11), by its last named definition, and a card
    /// a full library refused before it existed (R316).
    public: IndexSet<String>,
    /// The ones a pass took out of a hand, tokens among them, each with the player whose hand it left.
    passed: IndexMap<String, PlayerId>,
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
            // R227: a card set face-down took a fresh id but is the same card, so events naming its old id
            // are judged by where it is now (R97). No `hiddenFrom`: nothing ceased to exist.
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
        duel: state.and_then(|state| duel_gone(events, state)),
        in_library: in_library_of(events),
    }
}

/// R1020: the cards the events last put into a library (a `shuffledIn`, or a library card's
/// replacement hidden from every seat, R177) that no move, steal or announcement took out since.
/// An opening-deck card no event named needs nothing: no event names it either.
fn in_library_of(events: &[&GameEvent]) -> IndexSet<String> {
    let mut in_library: IndexSet<String> = IndexSet::new();
    for event in events {
        match event {
            GameEvent::ShuffledIn { instance_id, .. } => {
                in_library.insert(instance_id.clone());
            }
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                hidden_from: Some(hidden),
                ..
            } if PLAYER_IDS.iter().all(|player| hidden.contains(player)) => {
                in_library.shift_remove(instance_id);
                in_library.insert(new_instance_id.clone());
            }
            GameEvent::Stolen { instance_id, .. } | GameEvent::CardAnnounced { instance_id, .. } => {
                in_library.shift_remove(instance_id);
            }
            _ => {
                for id in crate::stays::moved_by(event) {
                    in_library.shift_remove(&id);
                }
            }
        }
    }
    in_library
}

/// R1074000: a pass (B20) takes a card out of its duel's hand with no event, so in a duel a card in no
/// pile with no successor is a token that ceased to exist (R11), a card a full library refused (R316),
/// or a passed one, whose last place the events show is a hand (a set's old id counts, R227). Only a
/// Duos seat passes, so a 1v1 state is no duel.
fn duel_gone(events: &[&GameEvent], state: &GameState) -> Option<DuelGone> {
    if PLAYER_IDS
        .into_iter()
        .all(|player| state.players[player].duos.is_none())
    {
        return None;
    }
    // `ceased`: unit tokens gone on the way to a hand; `addedToHand` may follow (a counter's steal).
    let mut in_hand: IndexMap<String, PlayerId> = IndexMap::new();
    let mut last_def: IndexMap<String, String> = IndexMap::new();
    let mut never_made: IndexSet<String> = IndexSet::new();
    let mut ceased: IndexSet<String> = IndexSet::new();
    let unit_token = |def_id: &str| {
        find_def(Some(state), def_id).is_some_and(|def| def.token && def.type_ == CardType::Unit)
    };
    for event in events {
        match event {
            GameEvent::Bounced {
                instance_id, def_id, ..
            }
            | GameEvent::Stolen {
                instance_id,
                def_id,
                zone: ZoneName::Field | ZoneName::Resolving,
                ..
            } if unit_token(def_id) => {
                last_def.insert(instance_id.clone(), def_id.clone());
                in_hand.shift_remove(instance_id);
                ceased.insert(instance_id.clone());
            }
            GameEvent::Drawn {
                player,
                instance_id,
                def_id,
                ..
            }
            | GameEvent::AddedToHand {
                player,
                instance_id,
                def_id,
                ..
            }
            | GameEvent::Bounced {
                owner: player,
                instance_id,
                def_id,
            }
            | GameEvent::Stolen {
                to: player,
                instance_id,
                def_id,
                ..
            } => {
                last_def.insert(instance_id.clone(), def_id.clone());
                if !ceased.contains(instance_id) {
                    in_hand.insert(instance_id.clone(), *player);
                }
            }
            GameEvent::Transformed {
                instance_id,
                new_instance_id,
                from_def_id,
                to_def_id,
                ..
            } => {
                last_def.insert(instance_id.clone(), from_def_id.clone());
                last_def.insert(new_instance_id.clone(), to_def_id.clone());
                if let Some(holder) = in_hand.shift_remove(instance_id) {
                    in_hand.insert(new_instance_id.clone(), holder);
                }
            }
            GameEvent::Fused {
                instance_ids,
                result_instance_id,
                def_id,
            } => {
                last_def.insert(result_instance_id.clone(), def_id.clone());
                let held: Vec<PlayerId> = instance_ids
                    .iter()
                    .filter_map(|id| in_hand.shift_remove(id))
                    .collect();
                if let Some(holder) = held.first() {
                    in_hand.insert(result_instance_id.clone(), *holder);
                }
            }
            GameEvent::CardPlayed {
                instance_id,
                def_id,
                former_id,
                ..
            }
            | GameEvent::Summoned {
                instance_id,
                def_id,
                former_id,
                ..
            } => {
                last_def.insert(instance_id.clone(), def_id.clone());
                in_hand.shift_remove(instance_id);
                if let Some(former) = former_id {
                    in_hand.shift_remove(former);
                }
            }
            GameEvent::ControlChanged {
                instance_id,
                former_id,
                ..
            } => {
                in_hand.shift_remove(instance_id);
                if let Some(former) = former_id {
                    in_hand.shift_remove(former);
                }
            }
            GameEvent::CardAnnounced {
                instance_id, def_id, ..
            }
            | GameEvent::Discarded {
                instance_id, def_id, ..
            }
            | GameEvent::Burned {
                instance_id, def_id, ..
            }
            | GameEvent::ShuffledIn {
                instance_id, def_id, ..
            }
            | GameEvent::Exiled {
                instance_id, def_id, ..
            }
            | GameEvent::EnteredGraveyard {
                instance_id, def_id, ..
            } => {
                last_def.insert(instance_id.clone(), def_id.clone());
                in_hand.shift_remove(instance_id);
            }
            GameEvent::Destroyed {
                instance_id, def_id, ..
            }
            | GameEvent::CardResolved {
                instance_id, def_id, ..
            } => {
                last_def.insert(instance_id.clone(), def_id.clone());
            }
            GameEvent::LibraryOverflow {
                instance_id,
                def_id,
                outcome,
                ..
            } => {
                last_def.insert(instance_id.clone(), def_id.clone());
                if *outcome == LibraryOverflowOutcome::NotCreated {
                    never_made.insert(instance_id.clone());
                }
            }
            _ => {}
        }
    }
    let token = |id: &str| {
        last_def
            .get(id)
            .and_then(|def_id| find_def(Some(state), def_id))
            .is_some_and(|def| def.token)
    };
    let gone = |id: &str| find_instance(state, id).is_none();
    let passed: IndexMap<String, PlayerId> = in_hand.into_iter().filter(|(id, _)| gone(id)).collect();
    let public: IndexSet<String> = last_def
        .keys()
        .filter(|id| gone(id) && token(id) && !passed.contains_key(*id))
        .chain(never_made.iter())
        .cloned()
        .collect();
    Some(DuelGone { public, passed })
}

/// R97: whether this viewer may read the card an event names, judged by where it sits *now*, not where it was
/// (§9.1). The resolving zone is public (R98). A card that ceased to exist is judged by its replacement
/// (R177), so what #83 Transmogulate replaced in a face-down trap or a library never reads; with none it went
/// publicly (a unit token, R11; a card exiled out of existence, R86), save in a duel (`duel_gone`, R1074000).
fn may_read(state: &GameState, viewer: PlayerId, instance_id: &str, replaced: &Replacements) -> bool {
    let mut id = instance_id.to_string();
    let mut card = find_instance(state, &id);
    let mut hops = 0;
    while card.is_none() && hops < replaced.replaced_by.len() {
        // R177: a card that ceased to exist where this viewer could not read it stays unread for good,
        // wherever its replacement goes later.
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
        // R1020: so is one a Fuse ate out of a library.
        if to_library || replaced.voided || replaced.in_library.contains(&id) {
            return false;
        }
        // R1074000: in a duel, a card a pass took out of a hand is as hidden as it was there, and one
        // that names no hand reads to nobody.
        let Some(duel) = &replaced.duel else {
            return true;
        };
        return duel.public.contains(&id) || duel.passed.get(&id) == Some(&viewer);
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

// Cards, units and the backrow

fn bare_card_view(instance_id: String, def_id: String, radiant: bool, cost: i32) -> CardView {
    CardView {
        instance_id,
        def_id,
        radiant,
        chinese: None,
        created: None,
        created_by: None,
        cost,
        // filled in by `card_view` from the instance data; bare views carry none.
        tags: None,
        embiggen_cost: None,
        tribute: None,
        attack: None,
        health: None,
        power: None,
        label: None,
        condition_active: None,
        countered_on_play: None,
        pareto_optimal: None,
        play_reason: None,
        trade_reason: None,
        tradable: None,
        choices_truncated: None,
        preview: None,
        type_: None,
        brittle: None,
        element: None,
        params: None,
        tuning: None,
        enchantments: None,
        keywords: None,
        marks: None,
        activations: None,
        quest: None,
        copies: None,
        fixed_x: None,
    }
}

/// R65: the cost as it stands now (an X card's X reads 0 outside a play). The instance data rides
/// every card view, each built only for a card the viewer may read where it is.
fn card_view(state: &GameState, card: &CardInstance, viewer: PlayerId) -> CardView {
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
    // MD-B15, R923: a granted tag is part of the card.
    view.tags = data.tags;
    view.brittle = data.brittle;
    // R980: the card's element, where the viewer may read the card and a Feng Shui acts.
    view.element = crate::subsystems::feng_shui::shown_element(state, card);
    view.params = data.params;
    view.tuning = data.tuning;
    view.enchantments = data.enchantments;
    view.marks = with_marks(state, &card.id);
    view.quest = quest;
    // ME-CN, R1301: public as the card is.
    view.chinese = card.chinese;
    // MD-B6, R943: so is Created.
    view.created = card.created;
    // R1956001: and the card that generated it, to its owner alone.
    view.created_by = crate::generators::generator_shown_to(state, card, viewer);
    // R1848003: a set X is the card's price, read where the card is.
    view.fixed_x = card.fixed_x.filter(|_| crate::mana::is_x_cost(state, card));
    view
}

/// R437: a card's marks (#50's pending steal), in both players' views, or no key when it has none.
fn with_marks(state: &GameState, instance_id: &str) -> Option<Vec<CardMark>> {
    let marks: Vec<CardMark> = marks_on(state, instance_id).into_iter().collect();
    if marks.is_empty() { None } else { Some(marks) }
}

/// R243, §10.8: a card in the viewer's own hand, in full. A Unit's stats are its face plus its
/// hand buffs (§10.4 layers 1, 3 and 4: #89 Corpse Eater's meals; layer 2 and auras are the
/// field's), attack floored at 0. A #98 Heroic Power names the power it rolled (R43, R151), and a
/// card its label (R2247000).
fn hand_card_view(state: &GameState, card: &CardInstance, viewer: PlayerId) -> CardView {
    let mut view = with_copies(card_view(state, card, viewer), state, card);
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
    // R2247000: the word it shows in place of its text, held in its memory.
    view.label = card
        .memory
        .get(crate::effects::CARD_LABEL_KEY)
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(keywords) = keywords {
        view.keywords = Some(keywords);
    }
    view
}

/// B5 E14, R399, R243: a copier's view carries the Spell text it has now (`CardView.copies`), with
/// that definition's numbers as they read on the card. Asked only where R399 shows it: the owner's
/// hand and the resolving zone.
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

/// R195, §10.8: the yellow glow, `conditionActive: true` or no key, never `false`.
fn with_condition(mut view: CardView, active: bool) -> CardView {
    if active {
        view.condition_active = Some(true);
    }
    view
}

/// #492, R81: what a play of an embiggen card at its embiggen price costs now (`embiggenCost`), by the
/// function step 1 prices that play with. Asked only of the viewer's own hand.
fn with_embiggen_cost(mut view: CardView, state: &GameState, card: &CardInstance) -> CardView {
    view.embiggen_cost = embiggen_play_cost(state, card);
    view
}

/// #2268, R2268000: how many Tributes a play of the card asks (`tribute`), by the function its play
/// is held to, so the client never works one out (CLAUDE.md rule 7). Absent at 0; asked only of the
/// viewer's own hand.
fn with_tribute(mut view: CardView, state: &GameState, card: &CardInstance) -> CardView {
    let tribute = tribute_cost_of(state, card);
    view.tribute = (tribute > 0).then_some(tribute);
    view
}

/// R667: the Plague Chalice warning, `true` or absent like the glow beside it.
fn with_counter_warning(mut view: CardView, countered: bool) -> CardView {
    if countered {
        view.countered_on_play = Some(true);
    }
    view
}

/// R1844001: the Pareto hint's mark, `true` or absent like the warning above.
fn with_pareto_hint(mut view: CardView, hinted: bool) -> CardView {
    if hinted {
        view.pareto_optimal = Some(true);
    }
    view
}

/// #992, R90: the bound's mark, `true` or absent like the warning above.
fn with_choices_truncated(mut view: CardView, truncated: bool) -> CardView {
    if truncated {
        view.choices_truncated = Some(true);
    }
    view
}

/// R979000: why the viewer's own hand card cannot be played now, or no key (never empty). Only while
/// the viewer may act (their turn, the main phase, no prompt open) and `play_actions_for` lists no
/// play of it; nothing else calls this, so other hands carry no key. R1571000: likewise for a card
/// in the viewer's own graveyard that a permission admits.
fn with_play_reason(
    mut view: CardView,
    state: &GameState,
    card: &CardInstance,
    viewer: PlayerId,
) -> CardView {
    if state.result.is_some() || state.phase != crate::wire::Phase::Main {
        return view;
    }
    if state.pending.is_some() {
        return view;
    }
    if state.active != viewer {
        return view;
    }
    let from_graveyard = matches!(card.zone, Zone::Graveyard { .. });
    let listed = if from_graveyard {
        crate::play_choices::graveyard_play_actions_for(state, viewer, card)
    } else {
        crate::play_choices::play_actions_for(state, viewer, card)
    };
    if !listed.is_empty() {
        return view;
    }
    let reason = if from_graveyard {
        crate::play_choices::why_graveyard_card_unplayable(state, viewer, card)
    } else {
        crate::play_choices::why_hand_card_unplayable(state, viewer, card)
    };
    if let Some(reason) = reason
        && !reason.is_empty()
    {
        view.play_reason = Some(reason);
    }
    view
}

/// R1838000: why the viewer's own Tradable hand card cannot be Traded now, or no key (never empty).
/// Only while the viewer may act, on a card `trade::why_trade_refused` refuses, exactly one
/// `legal_actions` lists no `trade` of; nothing else calls this, so other hands carry no key.
fn with_trade_reason(
    mut view: CardView,
    state: &GameState,
    card: &CardInstance,
    viewer: PlayerId,
) -> CardView {
    if state.result.is_some() || state.phase != crate::wire::Phase::Main {
        return view;
    }
    if state.pending.is_some() || state.active != viewer {
        return view;
    }
    if !crate::trade::is_tradable(state, card) {
        return view;
    }
    if let Err(refusal) = crate::trade::why_trade_refused(state, viewer, &card.id)
        && !refusal.message.is_empty()
    {
        view.trade_reason = Some(refusal.message);
    }
    view
}

/// R2574000: whether the viewer's own hand card may be Traded (`trade::is_tradable`),
/// whether by its printed face, a granted keyword or an aura over its controller's hand.
/// Present, and `true`, only on the viewer's own hand card in a live game; absent otherwise,
/// never `false`, never on the opponent's cards or post-game hands.
fn with_tradable(mut view: CardView, state: &GameState, card: &CardInstance) -> CardView {
    if state.result.is_none() && crate::trade::is_tradable(state, card) {
        view.tradable = Some(true);
    }
    view
}

/// R1838000: the viewer's own Trades while their hand holds a Tradable card in a live game, or nothing.
fn trades_view(state: &GameState, viewer: PlayerId) -> Option<crate::wire::TradesView> {
    let holds = state.result.is_none()
        && state.players[viewer]
            .hand
            .iter()
            .any(|card| crate::trade::is_tradable(state, card));
    holds.then(|| crate::wire::TradesView {
        cost: crate::config::TRADE_COST,
        left: crate::trade::trades_left(state, viewer),
    })
}

/// R280, §10.8: what a card's formula comes to now, as `preview` or no key, never `[]`;
/// `preview::preview_of` owns where.
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

/// The card view the field shows: the glow, the preview and the abilities on top of `card_view`.
fn field_card_view(state: &GameState, card: &CardInstance, viewer: PlayerId) -> CardView {
    with_activations(
        with_preview(
            with_condition(
                card_view(state, card, viewer),
                condition_active(state, card, viewer, ConditionZone::Field),
            ),
            preview_of(state, card, viewer, ConditionZone::Field),
        ),
        state,
        card,
        viewer,
    )
}

/// R940: a retired kind (`KeywordKind::RETIRED`) a shipped catalog's card still prints is no kind the
/// client's types name, so the view leaves it out.
fn shown_keywords(keywords: &[Keyword]) -> Vec<Keyword> {
    keywords
        .iter()
        .filter(|keyword| !KeywordKind::RETIRED.contains(&keyword.kind()))
        .cloned()
        .collect()
}

/// The card that acts in a unit zone: the top of the pile (§3.2). `buried` is how many dormant cards
/// sit under it (R13) — a count, so no buried identity reaches either player.
fn unit_view_of(
    state: &GameState,
    auras: &AuraTable<'_>,
    pile: &[CardInstance],
    viewer: PlayerId,
) -> Option<UnitView> {
    let top = pile.first()?;
    let layers = auras.unit_view(top);
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
        element: card.element,
        params: card.params,
        tuning: card.tuning,
        enchantments: card.enchantments,
        marks: card.marks,
        activations: card.activations,
        quest: card.quest,
        copies: card.copies,
        condition_targets: condition_targets_of(state, auras, top, viewer),
        owner: top.owner,
        controller: top.controller,
        attack: layers.attack,
        max_health: layers.max_health,
        health: layers.health,
        keywords: shown_keywords(&layers.keywords),
        armor: layers.armor,
        position: layers.position,
        counters: top.counters,
        buried: pile.len() as i32 - 1,
        can_act: can_act(state, auras, top),
        // R243, §6.3 Vanilla: the text is gone, which the definition the client reads does not say.
        vanilla: if top.vanilla { Some(true) } else { None },
        animated,
        // B5 E35: a status, public on the field like the unit itself.
        berserk: if top.berserk == Some(true) {
            Some(true)
        } else {
            None
        },
        // R1174000: likewise.
        frozen: top.frozen.map(|_| true),
        // R1839000: likewise, its countdown included.
        slumber: top.slumber,
        // R1848000: likewise, whether it never thaws.
        frozen_forever: crate::freeze::frozen_forever(top).then_some(true),
        // R1852005: a Starship's memory, likewise.
        starship: top.starship.clone(),
        // ME-CN, R1301: likewise (the pile's top, R13).
        chinese: card.chinese,
        // MD-B6, R943: likewise.
        created: card.created,
        created_by: card.created_by,
        // ME-GRANT (MD-D13): the granted Death abilities, as the Unit's lines read them.
        grants: crate::grants::grant_texts(state, top),
        // R849000: summoning sick with neither Rush nor Charge, from public facts on the field.
        sick: if is_asleep(state, top, &layers.keywords) {
            Some(true)
        } else {
            None
        },
        attack_reason: attack_reason_of(state, auras, top, viewer),
        // R2321000: the counts cards on the field the viewer may read keep on it.
        tallies: crate::preview::tallies_on(state, top, viewer),
    })
}

/// R849000: why the viewer's own unit that may act now cannot declare an attack, gated as
/// `condition_targets_of`. R1174000: a Frozen unit has no exertion, yet is told why while its
/// controller could otherwise act with it.
fn attack_reason_of(
    state: &GameState,
    auras: &AuraTable<'_>,
    top: &CardInstance,
    viewer: PlayerId,
) -> Option<String> {
    let frozen_in_its_turn = top.frozen.is_some() && turn_open_to(state, top);
    if top.controller != viewer || !(frozen_in_its_turn || can_act(state, auras, top)) {
        return None;
    }
    why_cannot_declare_in(auras, top).err().map(|error| error.message)
}

/// MD-D5, R1121: the yellow a dragged attack paints, the enemy Units an `attack_mods` entry would
/// apply to if this Unit attacked them now. Only on the viewer's own attackers that may act now (as
/// R195's flag). Tags are public (§10.8), so the field reveals nothing.
fn condition_targets_of(
    state: &GameState,
    auras: &AuraTable<'_>,
    top: &CardInstance,
    viewer: PlayerId,
) -> Option<Vec<String>> {
    if top.controller != viewer || !can_act(state, auras, top) {
        return None;
    }
    let targets: Vec<String> = crate::combat::attack_targets_in(auras, top)
        .into_iter()
        .filter_map(|target| match target {
            crate::damage::DamageTarget::Unit { instance } => {
                let bonus = crate::combat::attack_mod_for(state, top, &instance);
                if bonus.attack != 0 || bonus.poisonous {
                    Some(instance.id.clone())
                } else {
                    None
                }
            }
            crate::damage::DamageTarget::Hero { .. } => None,
        })
        .collect();
    if targets.is_empty() { None } else { Some(targets) }
}

/// §4.1: one exertion per turn, an attack or a position switch, so a unit can act while either is
/// unspent (`combat::has_exertion`, Deft's two included, R49). *Which* is legal is
/// `legal_actions`' answer, never the view's.
fn can_act(state: &GameState, auras: &AuraTable<'_>, card: &CardInstance) -> bool {
    turn_open_to(state, card)
        && (has_exertion_in(auras, card, ExertionKind::Attack)
            || has_exertion_in(auras, card, ExertionKind::Switch))
}

/// `can_act`'s first half: the game is on, in its controller's main phase, with no prompt open.
fn turn_open_to(state: &GameState, card: &CardInstance) -> bool {
    state.result.is_none()
        && state.phase == crate::wire::Phase::Main
        && state.pending.is_none()
        && state.active == card.controller
}

/// A public backrow card names its owner and controller, since R33's readability and #87's board swap
/// turn on the controller. A face-down zone is `{ faceDown: true, cost }` and no more (§10.8, R351),
/// the cost the controller's own view shows, so both players read the same number.
fn backrow_view(
    state: &GameState,
    auras: &AuraTable<'_>,
    card: Option<&CardInstance>,
    viewer: PlayerId,
) -> Option<BackrowView> {
    let card = card?;
    // B5 E19, R471: Plague Counters are public wherever they sit, a face-down card's included; B5 E21: a
    // backrow pile shows how many cards lie under its top, as a unit pile does (R13, R447).
    let plague = plague_on(card);
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
        created: view.created,
        created_by: view.created_by,
        tags: view.tags,
        cost: view.cost,
        attack: view.attack,
        health: view.health,
        power: view.power,
        condition_active: view.condition_active,
        countered_on_play: view.countered_on_play,
        preview: view.preview,
        brittle: view.brittle,
        element: view.element,
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
        // R1046, D3: how the card reveals rides the controller's view only.
        reveal_at: if card.controller == viewer {
            card.set_as.as_ref().map(|set| set.reveal)
        } else {
            None
        },
        // R243, §6.3 Vanilla: a backrow card's text can be gone too, and the stamp renders off the
        // unit prop, so the flag travels here as it does on a unit view.
        vanilla: if card.vanilla { Some(true) } else { None },
        buried,
        // R1081000: a Unit standing in the backrow rides as the minion it is — the same tops
        // `active_units_of` counts, so the two cannot disagree.
        unit: if is_standing_unit(state, card) {
            unit_view_of(state, auras, std::slice::from_ref(card), viewer)
        } else {
            None
        },
    }))
}

/// R446: the Units this side's carriers hold, by backrow lane, or nothing when none holds one.
fn carried_view(
    state: &GameState,
    auras: &AuraTable<'_>,
    player: PlayerId,
    viewer: PlayerId,
) -> Option<Vec<Option<UnitView>>> {
    if carried_units_of(state, player).is_empty() {
        return None;
    }
    Some(
        slots_of(player, Row::Backrow)
            .into_iter()
            .map(|zone_ref| {
                let unit: Option<CardInstance> = carried_at(state, zone_ref).cloned();
                unit.and_then(|unit| unit_view_of(state, auras, &[unit], viewer))
            })
            .collect(),
    )
}

/// R43: a Heroic Power is a Field Spell (§8 #98), public to both once on the field (§2's hero panel).
/// It follows control, so a player may hold several (#36 radiant, #49), each once per turn and named by
/// the `instanceId` `activatePower` takes (§10.2). R1447: a face-down card's power is no power, and
/// naming it would hand the other seat the hidden card.
fn hero_powers_of(state: &GameState, player: PlayerId) -> Vec<HeroPowerView> {
    let mut powers: Vec<HeroPowerView> = Vec::new();
    for side in PLAYER_IDS {
        for card in state.players[side].backrow.iter().flatten() {
            if card.controller != player || is_face_down(state, card) {
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

// Player modifiers (§10.1 `mods`, §10.3 `modifierChanged`)

/// The discount half of `modifier_label`: `costDiscount`'s caption must say *what* it applies to
/// (R48's current-cost gate, #77; §8 #35's "next Spell"; #78's "your cards").
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
        .or_else(|| state.custom_defs.get(def_id))
        .or_else(|| registered_catalog().get(def_id))
}

/// A badge caption built from the modifier alone; with no wildcard, a new `ModifierKind` does not
/// compile until someone decides what the player is told. `sourceId` (#79 Twinspell's) is never read:
/// no card identity may leave through a badge. `echo` is the grant as it stands (R209, §5.2).
fn modifier_label(state: &GameState, player: PlayerId, modifier: &PlayerModifier, echo: i32) -> String {
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
        // R831005: Meditative #53 Prestige.
        ModifierKind::RadiantNextPlay => "The next card you play becomes Radiant".to_string(),
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
        // B5 E15, E39 (R455): the price rules and Forever&'s rider, worded by their owner.
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
        // MD-C21: Tranquility's immunity badge.
        ModifierKind::HeroImmune => "Your hero is immune to damage until your next turn".to_string(),
        ModifierKind::ReplacePlays { def_id, radiant } => {
            let name = find_def_in(state, def_id).map_or_else(|| def_id.clone(), |def| def.name.clone());
            format!(
                "Each card you play becomes {}{name}",
                if *radiant { "a Radiant " } else { "a " }
            )
        }
        // R848: a held alternative win, shown with its progress.
        ModifierKind::AltWin { condition, threshold } => {
            crate::win_conditions::alt_win_label(state, player, *condition, *threshold)
        }
        // MD-B22, R946: the watcher's own words.
        ModifierKind::LaneWatch { label, .. } => label.clone(),
        // R1863000: Stinkyscape #44's watcher, in the card's own words.
        ModifierKind::PlayWatch { label, .. } => label.clone(),
        // R1882004: Stinkyscape #150's death watcher, in the card's own words.
        ModifierKind::DeathWatch { label, .. } => label.clone(),
        // R1180001: Meditative+ #11 Living Mana Potion's "Spell Damage +N this turn".
        ModifierKind::SpellDamage { amount } => format!("You have Spell Damage +{amount} this turn"),
        // R1180004: Meditative+ #6 Bloodbloom's health price, and the Echo the Radiant one adds.
        // R1852006: Stinkyscape #144 Prepare For Liftoff's price.
        ModifierKind::StarshipLaunchCost { cost } => format!("Your Starship launch costs ({cost})"),
        ModifierKind::HealthPrice { multiple, echo } => match echo {
            Some(echo) if *echo > 0 => format!(
                "Your next Spell costs health equal to {multiple} times its cost instead of mana, and has Echo {echo}"
            ),
            _ => format!("Your next Spell costs health equal to {multiple} times its cost instead of mana"),
        },
        // R1848002: Stinkyscape #13 Asphyxiants Cloud's "your next Spell this turn has Poisonous".
        ModifierKind::KeywordNextSpell { keyword } => format!(
            "Your next Spell this turn has {}",
            crate::wire::keyword_key(keyword)
        ),
        // R1859007: Stinkyscape's War Doctrines' "for the rest of the game, your Units have X". R1870004:
        // #4.4 Last Word's "your Units have Charge during it", the turn named until it comes.
        ModifierKind::UnitsHaveKeyword { keyword } => format!(
            "Your Units have {}{}",
            crate::wire::keyword_key(keyword),
            if crate::mana::modifier_is_live(state, modifier) {
                ""
            } else {
                " during your next turn"
            }
        ),
    }
}

/// §10.8 does not list the player modifiers, so R169 decides: both seats carry the id and caption,
/// since every one is installed by a card played face-up (not "by a Cry": #35 and #78 are Spells, #64
/// and #79 use other hooks) and `modifierChanged` is public. R48: one for the *next* turn says so, or
/// #77's badge would claim a discount on the very turn it does nothing.
fn modifier_views(state: &GameState, player: PlayerId) -> Vec<ModifierView> {
    let mut views: Vec<ModifierView> = state.players[player]
        .mods
        .iter()
        .map(|modifier| {
            let label = modifier_label(state, player, modifier, echo_grant_of(state, player, modifier));
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
    // R844: a lost refresh likewise, while a loss covers a future refresh.
    if state.players[player].lost_refresh_through.is_some() {
        let total = crate::mana::lost_refreshes_left(&state.players[player]);
        views.push(ModifierView {
            id: crate::mana::LOST_REFRESH_MODIFIER_ID.to_string(),
            label: if total == 1 {
                "Your next refresh gives 0 mana".to_string()
            } else {
                format!("Your next {total} refreshes give 0 mana")
            },
        });
    }
    // R846: owed extra turns likewise, while any is owed.
    if let Some(owed) = state.players[player].extra_turns {
        views.push(ModifierView {
            id: crate::turn::EXTRA_TURN_MODIFIER_ID.to_string(),
            label: format!("Extra turns owed: {owed}"),
        });
    }
    // R1181000: an effect made "in N turns" is a badge on its owner's side while it waits, since the
    // card that made it is not in play to carry a mark (R437).
    for entry in state.delayed.iter().filter(|entry| entry.owner == player) {
        if let (Some(due), Some(label)) = (entry.due_own_turn, &entry.label) {
            let left = due - state.players[entry.at.player].turns_started;
            views.push(ModifierView {
                id: entry.id.clone(),
                label: match left {
                    ..=0 => format!("{label} now"),
                    1 => format!("{label} in 1 turn"),
                    _ => format!("{label} in {left} turns"),
                },
            });
        }
    }
    views
}

// One side of the board

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

/// R1141: how many cards of the opponent's hand carry a mark, while that hand is a count.
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

/// R1223–R1225: this seat's credit line, `None` with no line acting, nothing owed and nothing locked.
/// Reads public state only: face-up lenders and the public schedule.
fn credit_view(state: &GameState, player: PlayerId) -> Option<CreditView> {
    let terms = crate::credit::credit_terms(state, player);
    let owed: Vec<i32> = state.players[player].owed_instalments.clone().unwrap_or_default();
    let locked = state.players[player].turn_log.mana_locked;
    if terms.is_none() && owed.iter().all(|owed| *owed == 0) && locked.is_none() {
        return None;
    }
    Some(CreditView {
        available: terms.map(|_| crate::credit::credit_available(state, player)),
        owed,
        locked,
        used: crate::credit::lapsing_face_acts(state, player)
            .then_some(crate::query::credit_used_this_turn(state, player)),
    })
}

fn side_view(state: &GameState, auras: &AuraTable<'_>, player: PlayerId, viewer: PlayerId) -> SideView {
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
    // R1844001: the Pareto hint, likewise only ever the viewer's own hand's.
    let hinted: IndexSet<String> = if player == viewer {
        pareto_hint_ids(state, viewer).into_iter().collect()
    } else {
        IndexSet::new()
    };
    // §10.8: the viewer's own hand in full, the opponent's as a count until the game is over (R434).
    let hand = if player == viewer {
        HandView::Cards(
            side.hand
                .iter()
                .map(|card| {
                    let view = with_trade_reason(
                        with_choices_truncated(
                            with_preview(
                                with_play_reason(
                                    with_counter_warning(
                                        with_condition(
                                            with_embiggen_cost(
                                                hand_card_view(state, card, viewer),
                                                state,
                                                card,
                                            ),
                                            condition_active(state, card, viewer, ConditionZone::Hand),
                                        ),
                                        countered.contains(&card.id),
                                    ),
                                    state,
                                    card,
                                    viewer,
                                ),
                                preview_of(state, card, viewer, ConditionZone::Hand),
                            ),
                            hand_play_choices_truncated(state, viewer, card),
                        ),
                        state,
                        card,
                        viewer,
                    );
                    with_tradable(
                        with_tribute(with_pareto_hint(view, hinted.contains(&card.id)), state, card),
                        state,
                        card,
                    )
                })
                .collect(),
        )
    } else if state.result.is_some() {
        HandView::Cards(
            side.hand
                .iter()
                .map(|card| hand_card_view(state, card, viewer))
                .collect(),
        )
    } else {
        HandView::Count {
            count: side.hand.len() as i32,
        }
    };
    SideView {
        player,
        hero: HeroView {
            health: side.hero.health,
            // §10.8's "armor": the number §4.4 step 2 will subtract, never the stored field alone, with
            // every backrow grant (#84 Going Long) summed per R124.
            armor: hero_armor_of(state, player),
            power: powers.first().cloned(),
            powers,
            // #936 S3: a Duos duel's pool and spent flag, public on both seats (absent in 1v1).
            duos: side.duos,
        },
        // R987: the side's Luck, read as the hero's armor is above — absent at 0 (D14).
        luck: match crate::query::luck_of(state, player) {
            0 => None,
            luck => Some(luck),
        },
        // R169: the badge list beside the hero, public on both seats.
        modifiers: modifier_views(state, player),
        mana: ManaView {
            current: side.mana.current,
            max: side.mana.max,
        },
        hand,
        // §9.1: a library's order ships to nobody, and the opponent's library is a count. R310–R312:
        // the viewer's own is also an unordered list of what they were shown going in, no id or place.
        library_count: side.library.len() as i32,
        own_library: if player == viewer {
            Some(own_library_view(state, player))
        } else {
            None
        },
        // R1571000: only the viewer's own graveyard says why a card cannot be played now.
        graveyard: side
            .graveyard
            .iter()
            .map(|card| {
                if player == viewer {
                    with_play_reason(card_view(state, card, viewer), state, card, viewer)
                } else {
                    card_view(state, card, viewer)
                }
            })
            .collect(),
        exile: side
            .exile
            .iter()
            .map(|card| card_view(state, card, viewer))
            .collect(),
        // §10.5 step 4, R98: a Spell between its play and its graveyard, public as its play was. R448:
        // one announced to be set face-down shows the other player a card back.
        resolving: side
            .resolving
            .iter()
            .map(|card| {
                if announced_face_down_to(state, card, viewer) {
                    bare_card_view(HIDDEN_ID.to_string(), HIDDEN_ID.to_string(), false, HIDDEN_COST)
                } else {
                    with_copies(card_view(state, card, viewer), state, card)
                }
            })
            .collect(),
        units: side
            .units
            .iter()
            .map(|pile| {
                pile.as_ref()
                    .and_then(|pile| unit_view_of(state, auras, pile, viewer))
            })
            .collect(),
        backrow: side
            .backrow
            .iter()
            .map(|card| backrow_view(state, auras, card.as_ref(), viewer))
            .collect(),
        carried: carried_view(state, auras, player, viewer),
        hand_cap: side.hand_cap,
        hand_marked: hand_marked_view(state, player, viewer),
        credit: credit_view(state, player),
        locks: RowFlags {
            units: side.locks.units.clone(),
            backrow: side.locks.backrow.clone(),
        },
        reserved: reserved_mask(state, player),
        fatigue_count: side.fatigue_count,
        // R961: public on both seats, and absent until it first rises (D14).
        jade: side.jade.filter(|value| *value > 0),
        // R1840005: public on both seats, and absent until first gained.
        market_credits: side.market_credits,
        // R846, R847: owed extra turns and the Rift flag, public on both seats.
        extra_turns: side.extra_turns,
        rift_extra_turn: side.rift_extra_turn,
        // R860: the side's secrets, the choice only where the viewer may read it.
        secrets: crate::secrets::secret_views(state, player, viewer),
        // R1847000: the seat's starscape map, public on both seats, absent until its first travel.
        starscape: crate::subsystems::starscape::starscape_view(state, player),
    }
}

// The open prompt

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
        recipe: None,
        created: None,
        code: None,
    }
}

/// §10.8: "a card revealed out of a library is revealed only as an option of the prompt that reveals
/// it: the chooser sees it in full". The options travel to the chooser alone (R81).
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
            // R177: a prompt may offer a card its chooser may not read (an enemy face-down trap: #49,
            // #50, an Echo repeat's pick): the id to answer with, never the definition, label or key
            // built from its name. B5 E17's look at the opponent's hand (Classic #11) is a reveal.
            if let Some(card) = card
                && is_face_down_to(state, card, viewer)
            {
                let mut hidden =
                    bare_option(format!("instance:{instance_id}"), HIDDEN_OPTION_LABEL.to_string());
                hidden.instance_id = Some(instance_id.clone());
                // R821000: where the back lies on the board is public (its zone draws it), so a client can
                // answer a prompt by a click on that zone while the card itself stays unnamed.
                if let Zone::Field { player, row, lane } = &card.zone {
                    hidden.player = Some(*player);
                    hidden.row = Some(*row);
                    hidden.lane = Some(*lane);
                }
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
                // MD-B6, R943: Created travels the same way.
                if card.created == Some(true) {
                    base.created = Some(true);
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
            // R1183005: a code Discover's option is its place in the offer, its label the code alone;
            // the card behind it stays in the prompt's resume data, which no view sends.
            if option.starts_with(crate::effects::choose::CODE_OPTION_PREFIX) {
                base.code = Some(true);
                return base;
            }
            // §6.3 Discover offers definitions, as `mode` options whose string is a catalog def id; a
            // night market's Radiant lot names its card with a suffix (R831002).
            let id = option
                .strip_suffix(crate::subsystems::MARKET_RADIANT_SUFFIX)
                .unwrap_or(option);
            if let Some(def) = find_def_in(state, id) {
                base.def_id = Some(def.id.clone());
            }
            base
        }
        // ME-CRAFT (R880): the recipe goes only to the prompt's holder, as this view does (§10.8).
        Selection::Craft { recipe } => {
            base.recipe = Some(recipe.clone());
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

/// R2646000: the card asking, its definition and face, when the chooser may read it where it now is
/// (R1088's test, `may_read`); none when no card opened the prompt, or the state no longer holds it, or
/// it has since changed definition, so the view says nothing the board does not.
fn prompt_source(state: &GameState, viewer: PlayerId, pending: &PendingChoice) -> Option<(String, bool)> {
    let id = pending.resume.instance_id.as_deref()?;
    let card = find_instance(state, id)?;
    // The state holds the card, so `may_read` never reaches for a replacement.
    if card.def_id != pending.resume.def_id || !may_read(state, viewer, id, &Replacements::default()) {
        return None;
    }
    Some((pending.resume.def_id.clone(), pending.resume.radiant))
}

/// The chooser's own prompt, field by field: `resume` never travels, so nothing a prompt keeps for its
/// answer (B5 E18's continuing owner, R465's problem key) leaves the engine through the view, but the
/// card asking, when the chooser may read it (R2646000).
fn prompt_view(state: &GameState, viewer: PlayerId, pending: &PendingChoice) -> PendingView {
    let source = prompt_source(state, viewer, pending);
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
        craft_mode: pending.craft_mode,
        craft_offer: pending.craft_offer.clone(),
        streak: pending.streak,
        source_def_id: source.as_ref().map(|(def_id, _)| def_id.clone()),
        source_radiant: source.map(|(_, radiant)| radiant),
    })
}

/// R265, R266: while both mulligans are open, a seat that still owes one sees its own prompt, and one
/// that has answered sees only that the other still owes one, as for any prompt the other holds.
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

/// R36: the viewer's own turns a declined offer still blocks, through the last before `blocked_until`;
/// the decline's own turn is not counted (its one offer is spent).
fn draw_offer_blocked_turns(state: &GameState, viewer: PlayerId) -> Option<i32> {
    let side = &state.players[viewer];
    let blocked_until = side.draw_offer.blocked_until?;
    let from = side.turns_started + i32::from(state.active != viewer);
    let left = (blocked_until - from).min(DRAW_OFFER_BLOCK_TURNS);
    (left > 0).then_some(left)
}

// Events (§10.10)

/// An event's JSON object as `redact_event` reads and changes it, written out only once something
/// changes (`field_of` reads it until then): writing each event out and back was most of a view's cost.
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
            _ => panic!("an event serializes to an object"),
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

/// `field_of`'s serializer: only the field asked for is written (the last, should a key come twice,
/// as a `Map` keeps it). Anything but an object is an error, and `field_of` then writes the event out.
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

/// R97: an event naming a card this viewer may not read keeps its type and every field §10.10's
/// animation table needs, the identity `HIDDEN_ID`, so the cue still plays as a card back. With no
/// wildcard (§10.3), a new event type does not compile until someone decides what it reveals.
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
        // R915004: a custom card's failure names the card as the card itself is named: a viewer who may
        // not read it learns that a script failed, never whose or why.
        GameEventType::ScriptFailed => {
            if !instance.is_empty() && hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
                shown.insert("message".to_string(), Value::String("a script failed".to_string()));
            }
            rebuild(shown)
        }

        // R1853003: a card bought at the Marketplace is named as on `addedToHand`, its face with it.
        GameEventType::MarketBought => {
            if hidden(&instance) {
                shown.remove("radiant");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // A card named with its definition: both go, or neither. `cardResolved` (§10.5 step 7) is
        // judged where the card is now (R97). `player` and `permanent` (R61's "still in play", which
        // #85 keys on) are not identity; `radiant` is the card's, so it goes with the id.
        GameEventType::CardResolved => {
            // R119's `arrivedDuring` is the engine's own bookkeeping, and it names face-down traps (#95);
            // so is the exit mark the event happened at (R174, R212).
            shown.remove("arrivedDuring");
            shown.remove("exitsFrom");
            // R1180005: the answer names a card in a hand or a deck, and is the engine's bookkeeping.
            shown.remove("answer");
            if hidden(&instance) {
                shown.remove("radiant");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R97, R177: `killerId` names a card as well, which may since have gone somewhere this viewer
        // cannot read (#31 KY's Math Equation returns to its owner's hand at the end of the turn).
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

        // R119's `arrivedDuring` and the exit mark are bookkeeping, as on `cardResolved`. R227:
        // `formerId` goes with the card's identity, or the old id would name the face-down card (R177).
        GameEventType::CardPlayed | GameEventType::Summoned => {
            shown.remove("arrivedDuring");
            shown.remove("exitsFrom");
            // R2089000: the game record's, as on `addedToHand`.
            shown.remove("creator");
            if hidden(&instance) {
                // R1046: a card set face-down by its Cloaked shows neither its `x` nor its
                // `embiggened`; any other hidden play keeps both (D14).
                if find_instance(state, &instance).is_some_and(|card| card.set_as.is_some()) {
                    shown.remove("x");
                    shown.remove("embiggened");
                }
                shown.remove("formerId");
                hide(&mut shown, &["instanceId", "defId"]);
            }
            // R1088: `sourceId`, the summoning card, may since be unreadable: hidden as `killerId` is.
            if nullable_at(&shown, "sourceId").is_some_and(|source| hidden(&source)) {
                hide(&mut shown, &["sourceId"]);
            }
            rebuild(shown)
        }

        // R316: a card a full library refused is judged like the cards below; one never created was
        // never anywhere hidden, so it reads as the card it copies (`copyOf`) does, openly when none
        // (#33's copy of a face-down Trap names it no more than the trap does). `copyOf` never travels.
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

        // R317: a burned card lands in its owner's graveyard, or ceases to exist (R11), so both seats
        // read it until something takes it somewhere hidden.
        GameEventType::EnteredGraveyard
        | GameEventType::Exiled
        | GameEventType::Bounced
        | GameEventType::Burned
        | GameEventType::Discarded
        | GameEventType::Drawn
        // R1843000: a vote names its card as the card itself is named, a face-down voter hidden.
        | GameEventType::VoteCast => {
            if !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }
        // As above; R2089000's `creator` is the game record's bookkeeping, which no view forwards.
        GameEventType::AddedToHand => {
            shown.remove("creator");
            if hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R1843000: the cards that agreed, each named as the card itself is.
        GameEventType::VotedOut => match event {
            GameEvent::VotedOut { player, ballots } => GameEvent::VotedOut {
                player: *player,
                ballots: ballots
                    .iter()
                    .map(|ballot| {
                        if hidden(&ballot.instance_id) {
                            crate::wire::Ballot {
                                instance_id: HIDDEN_ID.to_string(),
                                def_id: HIDDEN_ID.to_string(),
                            }
                        } else {
                            ballot.clone()
                        }
                    })
                    .collect(),
            },
            other => other.clone(),
        },

        // R177: a Make Radiant in a library (#42's roll, top down) stays unread for good (§3): its place
        // in the batch would say where the card lay. A cue on the other player's unread card is given
        // their hand as its zone: a random pick (#28) takes non-Radiant cards only (R60), so the real
        // zone would say a base-face card lay there (R33).
        GameEventType::RadiantSet | GameEventType::Deradianted => {
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

        // R154: the seat decides, not R97: a fired Trap is in a public graveyard or face-up, so
        // `may_read` would hand the opponent the card on the event that announces its flip. Only the
        // controller reads `instanceId` and `defId`; `row`, `lane` and `controller` always travel, so
        // the flip animates in the right zone (§10.10). R763: to its controller it follows R97 as well.
        GameEventType::TrapFired => {
            if text_at(&shown, "controller") == viewer.as_str() && !hidden(&instance) {
                return event.clone();
            }
            hide(&mut shown, &["instanceId", "defId"]);
            rebuild(shown)
        }

        // §9.1: library order is hidden from both players, so the slot never travels either way.
        GameEventType::ShuffledIn => {
            // R2089000: the game record's, as on `addedToHand`.
            shown.remove("creator");
            if hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            shown.insert("position".to_string(), Value::from(HIDDEN_POSITION));
            rebuild(shown)
        }

        // R177: the old card ceased to exist where the new one is (§6.3, R35), so it is judged by its
        // replacement's zone; one replaced inside a library or a face-down zone never reads.
        GameEventType::Transformed => {
            let new_hidden = hidden(&text_at(&shown, "newInstanceId"));
            shown.remove("hiddenFrom");
            // R2089000: the game record's, as on `addedToHand`.
            shown.remove("creator");
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

        // R177: a buff's size is the card's too (#89 Corpse Eater gains double on its radiant face), so
        // a hidden card's buff keeps only its type, for the cue.
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

        // R385: a Brittle count is its card's, so on an unreadable card it goes with the id.
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

        // R177: the new cost is the card's too, and over a library would give the order away, so a
        // change made there stays unread for good (`hiddenFrom`).
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
            // R1202: the Windfast a substitute attacks for, hidden when this viewer may not read it.
            if nullable_at(&shown, "insteadOf").is_some_and(|windfast| hidden(&windfast)) {
                hide(&mut shown, &["insteadOf"]);
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

        // R81, §10.6: the other seat learns only that a prompt is open and whose: its kind would
        // narrow the card that asked, and its id counts the match's prompts.
        GameEventType::PromptOpened | GameEventType::PromptAnswered => {
            if text_at(&shown, "player") == viewer.as_str() {
                return event.clone();
            }
            shown.remove("choiceId");
            shown.remove("kind");
            rebuild(shown)
        }

        // Public through and through: these name a player, a zone or a number, never a card.
        // R315: a fatigue draw names a player and two numbers, and the fatigue count is public (§10.8).
        GameEventType::HealthLost
        | GameEventType::Fatigue
        | GameEventType::ModifierChanged
        | GameEventType::Rotated
        | GameEventType::Swapped
        | GameEventType::Locked
        | GameEventType::ManaChanged
        | GameEventType::ManaSpent
        | GameEventType::TurnStarted
        | GameEventType::TurnEnded
        | GameEventType::TurnAutoEnded
        | GameEventType::DrawOffered
        | GameEventType::DrawAnswered
        // MD-D29, R1127: an emote names no card, so both seats read it as it is.
        | GameEventType::Emoted
        | GameEventType::GameOver
        // R961: the Jade Counter is public, a player and a number.
        | GameEventType::JadeChanged
        // R860, R864: a kept secret names only its player and id, and a reveal and a prediction
        // are public once they happen.
        | GameEventType::SecretChosen
        | GameEventType::SecretRevealed
        | GameEventType::Predicted
        // R831003: the curve is board positions only.
        | GameEventType::CurveDrawn
        // R1183002: the autopilot names a player and a turn.
        | GameEventType::AutopilotEngaged
        // R1838001: a Trade names its player alone; its `shuffledIn` and `drawn` are judged as any are.
        | GameEventType::Traded
        // R1840005: Credits are a player and a number.
        | GameEventType::MarketCreditsChanged
        // R1853003, R1861000: a card sold at the Marketplace ceased to exist where both seats see it
        // go, as a burned card is (R317).
        | GameEventType::MarketSold => event.clone(),
        // R1843002: a flight names a player and what it leaves the other.
        | GameEventType::Fled
        // R1847001: a starscape travel names a player, a node and its reward, all public.
        | GameEventType::StarscapeTravelled
        // R1855000: a board replacement names its player alone.
        | GameEventType::BoardReplaced => event.clone(),

        // R1852000, R97: a fusion names the dead piece (with its definition) and the Dormant Starship
        // it fused into, each judged by where it is now. The count is the Starship's.
        GameEventType::StarshipFused => {
            if hidden(&instance) {
                hide(&mut shown, &["instanceId"]);
            }
            if hidden(&text_at(&shown, "pieceId")) {
                hide(&mut shown, &["pieceId", "defId"]);
            }
            rebuild(shown)
        }
        // R1852003, R97: a launch names the Starship, judged as the card itself is; its name and the
        // pieces it held are the public memory of a public launch (R1852005).
        GameEventType::StarshipLaunched => {
            if hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            rebuild(shown)
        }

        // R1850003: only a face-up card is held, so both read it as it happens; then each is judged
        // by where it is now (R97), like `exiled`. A holder that never found a zone has no id.
        GameEventType::Held => {
            if hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
            if nullable_at(&shown, "holderId").is_some_and(|holder| !holder.is_empty() && hidden(&holder)) {
                hide(&mut shown, &["holderId"]);
            }
            rebuild(shown)
        }

        // docs/classic-sets.md B3, B5

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

        // B5 E1, E2, B3.3: judged by where the card is now (R97), and so is `byInstanceId`, the
        // countering card (a fired trap by then).
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
        // B5 E2, E16, R466: a stolen card reads to whoever could read it where it was taken from
        // (`readableFrom`) and to whoever can where it is now (R97).
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

        // B3.4, R386, R177: a change over a library or a hidden hand would say which card changed and
        // how, so it stays unread for whoever could not read the card there (`hiddenFrom`) or now.
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
        | GameEventType::Flickered
        // R1839002: an awakening names a card face-up on the field, public as the card is.
        | GameEventType::Awakened => {
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

        // R2652000: a batch of coins or dice a viewer reads at all (`withheld_rolls` drops the rest)
        // shows every face; the card itself follows R97, as Call to Chaos's does below. `faceDown` is
        // the engine's, so no view forwards it.
        GameEventType::Rolled => {
            shown.remove("faceDown");
            if !instance.is_empty() && hidden(&instance) {
                hide(&mut shown, &["instanceId", "defId"]);
            }
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

        // R983: only a face-up play is judged (R982); the card and its judge go by where each is now.
        GameEventType::FengShui => {
            let source_hidden = nullable_at(&shown, "sourceId").is_some_and(|source| hidden(&source));
            if !hidden(&instance) && !source_hidden {
                return event.clone();
            }
            if hidden(&instance) {
                hide(&mut shown, &["instanceId"]);
            }
            if source_hidden {
                hide(&mut shown, &["sourceId"]);
            }
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
        | GameEventType::DrawLimited
        | GameEventType::DiscardPrevented => {
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

/// Whether `viewer` could read the card a `stolen` event names where it was taken from
/// (`readableFrom`); without that, by its pile: a hand its holder's (§9.1), a public pile everyone's
/// (R98), a library or the field nobody's (whether it stood face-down there is not on the event).
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
        // A stolen card never comes out of "gone".
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

/// R2652000: the places, in `state.applied`'s events flattened oldest action first, of the `rolled`
/// batches `viewer` never reads: a face-down card's in its controller's backrow, to the other player,
/// unless that trap's own `trapFired` follows in the same action (Hearthstone's Secret). Judged inside
/// the action that rolled, so a batch never comes into a window or leaves it after the fact.
pub(crate) fn withheld_rolls(state: &GameState, viewer: PlayerId) -> IndexSet<usize> {
    let mut withheld = IndexSet::new();
    let mut base = 0;
    for entry in &state.applied {
        for (at, event) in entry.events.iter().enumerate() {
            let GameEvent::Rolled {
                player,
                instance_id,
                face_down: Some(true),
                ..
            } = event
            else {
                continue;
            };
            let fired = entry.events[at + 1..].iter().any(|later| {
                matches!(later, GameEvent::TrapFired { instance_id: trap, .. } if trap == instance_id)
            });
            if *player != viewer && !fired {
                withheld.insert(base + at);
            }
        }
        base += entry.events.len();
    }
    withheld
}

/// R2652000: how many of the newest `fresh` events of `state.applied` `viewer`'s view carries, all but
/// the face-down rolls `withheld_rolls` leaves out, so a host that slices the view's events by a step's
/// count (the server's bot personas, R645) still reads that step's events and no older one.
pub fn viewed_of_newest(state: &GameState, viewer: PlayerId, fresh: usize) -> usize {
    let total: usize = state.applied.iter().map(|entry| entry.events.len()).sum();
    let from = total.saturating_sub(fresh);
    let withheld = withheld_rolls(state, viewer)
        .iter()
        .filter(|at| **at >= from)
        .count();
    (total - from) - withheld
}

/// §10.8: "the last N events for animation": the tail of `state.applied`, the only event history a state
/// carries (§9.3's nonce dedupe). `VIEW_EVENT_LIMIT` is a FLOOR, not a cap (SPEC §11 R168): the window never
/// ends inside the newest action, since `PlayerView.events` is the client's only channel for every §10.3
/// animation (BUILD M5-T4) and one action can emit more (#96 My Pawn's cancel plus the §10.7 AI turn).
fn recent_events(state: &GameState, viewer: PlayerId) -> Vec<GameEvent> {
    let all: Vec<&GameEvent> = state
        .applied
        .iter()
        .flat_map(|entry| entry.events.iter())
        .collect();
    // R2652000: the window is taken from the events `viewer` reads, the face-down rolls withheld from
    // them already out, so it holds as many as it would had those cards never rolled: a window one
    // event short would tell them a hidden card rolled. The floor counts the newest action the same way.
    let withheld = withheld_rolls(state, viewer);
    let read: Vec<usize> = (0..all.len()).filter(|at| !withheld.contains(at)).collect();
    let newest_from = all.len() - state.applied.last().map_or(0, |entry| entry.events.len());
    let newest = read.iter().filter(|at| **at >= newest_from).count();
    let window = VIEW_EVENT_LIMIT.max(newest);
    let replaced = replacements_of(&all, Some(state));
    // R764: a Glitch's reset or boards (R676, R678) took cards out unseen, so a card the events up to the
    // end of its action name that the state no longer holds is not public, as a token's was (R11).
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
    read[read.len().saturating_sub(window)..]
        .iter()
        .map(|&at| {
            redact_event(
                state,
                viewer,
                all[at],
                if at < voided_until { &earlier } else { &replaced },
            )
        })
        .collect()
}

// §10.8

/// The one window a player has onto a match (SPEC §10.8), sharing nothing mutable with the state.
/// No clock is running (`clockMs` is `null`); a host that runs one asks `view_for_with_clock`.
pub fn view_for(state: &GameState, player_id: PlayerId) -> PlayerView {
    view_for_with_clock(state, player_id, None)
}

/// `clock_ms` is the turn clock the server runs (R79): the engine never reads a clock, so the caller
/// passes it in, `None` when nobody is counting.
pub fn view_for_with_clock(state: &GameState, player_id: PlayerId, clock_ms: Option<i32>) -> PlayerView {
    // #892: one aura table for the whole view.
    let auras = AuraTable::new(state);
    let mut view = PlayerView {
        viewer: player_id,
        turn: state.turn,
        active: state.active,
        phase: state.phase,
        you: side_view(state, &auras, player_id, player_id),
        opponent: side_view(state, &auras, opponent_of(player_id), player_id),
        pending: pending_view(state, player_id),
        events: recent_events(state, player_id),
        result: state.result.as_ref().map(|result| GameResult {
            winner: result.winner,
            reason: result.reason,
        }),
        clock_ms,
        mulligan: mulligan_view(state, player_id),
        draw_offer: draw_offer_view(state),
        // R36: the viewer's own block only, so the opponent's view never names it.
        draw_offer_blocked_turns: draw_offer_blocked_turns(state, player_id),
        // R345: the viewer's own preference, and only when it is off, so every other view is unchanged.
        auto_end_turn: if state.players[player_id].auto_end_turn == Some(false) {
            Some(false)
        } else {
            None
        },
        defs: None,
        // R1200: both seats read the same flag — a Mayor acts, or none does.
        random_targets: if crate::random_targets::targets_random(state) {
            Some(true)
        } else {
            None
        },
        // MD-D29, R1127: the client's emote layer sends `Emote` actions only while this is true.
        emotes_heard: if crate::query::emotes_heard(state, player_id) {
            Some(true)
        } else {
            None
        },
        // R1838000: the viewer's own Trades, while their hand holds a Tradable card.
        trades: trades_view(state, player_id),
        // R1853000, R1853003: the Marketplace as the viewer reads it, once it is open.
        marketplace: crate::marketplace::marketplace_view(state, player_id),
    };
    let defs = match_defs_in(state, &view);
    if !defs.is_empty() {
        view.defs = Some(defs);
    }
    view
}

/// R243: the match-made definitions (`state.transient_defs`: a Fuse's, a crafted card's; R77, R102,
/// R179) the finished view names anywhere, copied beside it since no client catalog has them. Read
/// after redaction, so an unreadable card is the sentinel by then (R97) and its definition stays put.
fn match_defs_in(state: &GameState, view: &PlayerView) -> IndexMap<String, CardDef> {
    fn visit(value: &Value, state: &GameState, defs: &mut IndexMap<String, CardDef>) {
        match value {
            Value::String(text) => {
                if defs.contains_key(text) {
                    return;
                }
                if let Some(def) = state.transient_defs.get(text) {
                    defs.insert(text.clone(), def.clone());
                } else if let Some(def) = state.custom_defs.get(text) {
                    // R915000: a custom card's definition, which no client catalog holds either.
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
    if state.transient_defs.is_empty() && state.custom_defs.is_empty() {
        return defs;
    }
    match serde_json::to_value(view) {
        Ok(value) => visit(&value, state, &mut defs),
        Err(error) => panic!("a view serializes: {error}"),
    }
    defs
}
