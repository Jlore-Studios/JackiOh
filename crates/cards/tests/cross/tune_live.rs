//! Issue #3888's sweep over the real cards (R3888000, SPEC §6.3's table of where a Buff or a Nerf is
//! live): every catalog card and token, on each face, in each zone the table names, offers only the
//! changes the table calls live there (soundness), and every change its hand or deck copy offers that
//! the table calls live in another zone is offered there too (completeness). The table is restated
//! here, footnotes and all, so a change to the engine's table that is not a change to the spec fails.

use jackioh_engine::effects::{
    TuneDirection, TuneItem, TuneOffer, TuneRow, TuneZone, menu_offers, tune_zone_of,
};
use jackioh_engine::testkit::*;

use super::scenario;

const P1: PlayerId = PlayerId::P1;
const DIRECTIONS: [TuneDirection; 2] = [TuneDirection::Degrade, TuneDirection::Upgrade];

/// The table's rows, restated (SPEC §6.3, R3888000).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Acts {
    Price,
    InHand,
    HandOrDeck,
    OnDraw,
    AtResolve,
    AtPlay,
    EntryTurn,
    OnAwaken,
    OnEvolve,
    Lasting,
    FromGraveyard,
    Plague,
}

/// The table's keyword members, restated.
fn keyword_acts(kind: KeywordKind) -> Acts {
    match kind {
        KeywordKind::Cloaked | KeywordKind::Tradable | KeywordKind::Temporary => Acts::InHand,
        KeywordKind::Crater
        | KeywordKind::Equip
        | KeywordKind::Stack
        | KeywordKind::Giga
        | KeywordKind::Tipsy
        | KeywordKind::Flexible => Acts::AtPlay,
        KeywordKind::Rush | KeywordKind::Charge => Acts::EntryTurn,
        KeywordKind::Flashback => Acts::FromGraveyard,
        _ => Acts::Lasting,
    }
}

/// When the change an offer makes acts, by the table's members.
fn acts_of(state: &GameState, card: &CardInstance, offer: &TuneOffer) -> Acts {
    match (&offer.row, &offer.item) {
        (TuneRow::Cost, _) => Acts::Price,
        (TuneRow::Stats, _) => Acts::Lasting,
        (TuneRow::Plague, _) => Acts::Plague,
        (_, Some(TuneItem::Add(kind) | TuneItem::Remove(kind))) => keyword_acts(*kind),
        (_, Some(TuneItem::X(key))) => match key.as_str() {
            "X" => Acts::AtResolve,
            "Tribute" => Acts::Price,
            "Echo" => Acts::AtPlay,
            _ => Acts::Lasting,
        },
        (_, Some(TuneItem::Number(key))) => {
            let def = def_of(Some(state), &card.def_id);
            let param = def
                .params
                .iter()
                .flatten()
                .find(|param| param.key == *key)
                .unwrap_or_else(|| panic!("{}: no param {key}", card.def_id));
            let face = if card.radiant {
                FaceKind::Radiant
            } else {
                FaceKind::Base
            };
            let read = match param.read_on {
                Some(on) if on != face => None,
                _ => param.read,
            };
            match read {
                None => Acts::Lasting,
                Some(ParamRead::Cry | ParamRead::Combo) => Acts::AtPlay,
                Some(ParamRead::CastOnDraw) => {
                    if jackioh_engine::alt_play::priced_type_of(state, card) == CardType::Spell {
                        Acts::AtPlay
                    } else {
                        Acts::OnDraw
                    }
                }
                Some(ParamRead::Awaken) => Acts::OnAwaken,
                Some(ParamRead::Evolve) => Acts::OnEvolve,
                Some(ParamRead::HandOrDeck) => Acts::HandOrDeck,
                Some(ParamRead::Graveyard) => Acts::FromGraveyard,
            }
        }
        (row, item) => panic!("{}: an offer of the {row:?} row with {item:?}", card.def_id),
    }
}

/// The table's cell and its footnotes, restated.
fn live(state: &GameState, card: &CardInstance, zone: TuneZone, acts: Acts) -> bool {
    use TuneZone::{Deck, Dormant, Exile, FaceDown, Field, Graveyard, Hand, Played};
    let playable = jackioh_engine::graveyard_play::playable_from_graveyard(state, card);
    let sick_again =
        jackioh_engine::combat::is_sick(state, card) || unit_has(state, card, KeywordKind::Reborn);
    let awoken = card.awakened == Some(true);
    let evolved = card.evolved == Some(true);
    let token = jackioh_engine::zones::is_unit_token(state, card);
    let cast_now = jackioh_engine::cast_on_draw_now::is_cast_on_draw(state, card);
    match (acts, zone) {
        (Acts::Price | Acts::InHand | Acts::HandOrDeck, Deck | Hand) => true,
        (Acts::OnDraw, Deck) => true,
        (Acts::OnDraw, Played) => cast_now,
        (Acts::AtResolve, Deck | Hand | Played) => true,
        (Acts::AtPlay, Deck | Hand | Played | FaceDown) => true,
        (Acts::EntryTurn, Deck | Hand | Played | FaceDown | Dormant) => true,
        (Acts::EntryTurn, Field) => sick_again,
        (Acts::OnAwaken, Deck | Hand | Played | FaceDown | Dormant) => !awoken,
        (Acts::OnAwaken, Graveyard) => !awoken && playable,
        (Acts::AtResolve | Acts::AtPlay | Acts::EntryTurn | Acts::Lasting, Graveyard) => playable,
        (Acts::OnEvolve, Exile) => false,
        (Acts::OnEvolve, _) => !evolved,
        (Acts::Lasting, Exile) => false,
        (Acts::Lasting, _) => true,
        (Acts::FromGraveyard, Exile) => false,
        (Acts::FromGraveyard, _) => !token,
        (Acts::Plague, Field | Dormant) => true,
        _ => false,
    }
}

/// Where the sweep puts a copy of the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Deck,
    Hand,
    Graveyard,
    Exile,
    Field,
    FaceDown,
    Dormant,
}

/// A fresh p1 copy of `def` on `face`, put where `place` says on a clone of `base`, or `None` where
/// the card has no such place: on the field face-up only a Unit (in its unit row, entered last turn),
/// a Field Spell or a Field Trap; face-down a printed Trap or Field Trap, or any other card set by a
/// Cloaked (R783004); Dormant only a Unit or a Field Spell (R1839000, R3556000).
fn placed(base: &GameState, def: &CardDef, radiant: bool, place: Place) -> Option<(GameState, CardInstance)> {
    let mut state = base.clone();
    let printed = jackioh_engine::faces::card_type_of_face(&state, &def.id, radiant);
    let mut card = new_instance(&mut state, &def.id, P1, Zone::Hand { player: P1 });
    card.radiant = radiant;
    let pile = |state: &mut GameState, mut card: CardInstance, zone: Zone| {
        card.zone = zone.clone();
        let player = &mut state.players[P1];
        match zone {
            Zone::Library { .. } => player.library.push(card.clone()),
            Zone::Hand { .. } => player.hand.push(card.clone()),
            Zone::Graveyard { .. } => player.graveyard.push(card.clone()),
            _ => player.exile.push(card.clone()),
        }
        card
    };
    let turn = state.turn;
    let card = match place {
        Place::Deck => pile(&mut state, card, Zone::Library { player: P1 }),
        Place::Hand => pile(&mut state, card, Zone::Hand { player: P1 }),
        Place::Graveyard => pile(&mut state, card, Zone::Graveyard { player: P1 }),
        Place::Exile => pile(&mut state, card, Zone::Exile { player: P1 }),
        Place::Field | Place::FaceDown | Place::Dormant => {
            let face_down = place == Place::FaceDown;
            let row = match printed {
                CardType::Unit if !face_down => Row::Units,
                CardType::Unit | CardType::FieldSpell | CardType::FieldTrap | CardType::Trap => Row::Backrow,
                CardType::Spell if face_down => Row::Backrow,
                CardType::Spell => return None,
            };
            match place {
                Place::Field if printed == CardType::Trap => return None,
                Place::Dormant if !matches!(printed, CardType::Unit | CardType::FieldSpell) => return None,
                _ => {}
            }
            if face_down && !matches!(printed, CardType::Trap | CardType::FieldTrap) {
                card.set_as = Some(SetAs {
                    reveal: RevealAt::Cloaked,
                    set_turn: turn - 1,
                    echo: None,
                    revealing: None,
                });
            }
            if !face_down && row == Row::Backrow {
                card.face_up = Some(true);
            }
            card.summoned_turn = Some(turn - 1);
            if place == Place::Dormant {
                card.slumber = Some(Slumber { turns: Some(2) });
            }
            let slot = ZoneSlot {
                player: P1,
                row,
                lane: 1,
            };
            let landed = jackioh_engine::zones::place_on_field(
                &mut state,
                &mut card,
                slot,
                PlaceOnFieldOptions::default(),
            );
            assert!(landed, "{} ({place:?}): placed in {row:?} lane 1", def.id);
            // A card that prints Dormant enters it (R1839000): face-up on the field it has awoken.
            if place == Place::Field
                && let Some(live) = find_instance_mut(&mut state, &card.id)
                && live.slumber.is_some()
            {
                live.slumber = None;
                live.awakened = Some(true);
            }
            find_instance(&state, &card.id).cloned().expect("the placed card")
        }
    };
    Some((state, card))
}

/// The column a placement is read in.
fn zone_of(place: Place) -> TuneZone {
    match place {
        Place::Deck => TuneZone::Deck,
        Place::Hand => TuneZone::Hand,
        Place::Graveyard => TuneZone::Graveyard,
        Place::Exile => TuneZone::Exile,
        Place::Field => TuneZone::Field,
        Place::FaceDown => TuneZone::FaceDown,
        Place::Dormant => TuneZone::Dormant,
    }
}

/// Whether the copy in `zone` could take `offer` at all, the table aside: a keyword added only to a
/// card that lacks it there, one removed only from a card that has it of its own there, and a numbered
/// keyword only on a card that carries it there (a set card's sleeps with its text, R1487000), the rest
/// as the hand copy reads them. An offer that fails this is no completeness claim.
fn could_take(state: &GameState, card: &CardInstance, offer: &TuneOffer) -> bool {
    match &offer.item {
        Some(TuneItem::X(key)) if key != "X" => numbered_keywords_on(state, card)
            .iter()
            .any(|entry| entry.key.as_str() == key),
        Some(TuneItem::Add(kind)) => {
            let held = if jackioh_engine::zones::in_units_row_or_standing(state, card) {
                unit_view(state, card).keywords
            } else {
                card_keywords(state, card)
            };
            !has_keyword(&held, *kind)
        }
        Some(TuneItem::Remove(kind)) => has_keyword(&card_keywords(state, card), *kind),
        _ => true,
    }
}

/// One failure, naming the card, its face, the zone and the offer.
fn named(
    def: &CardDef,
    radiant: bool,
    zone: TuneZone,
    direction: TuneDirection,
    offer: &TuneOffer,
    what: &str,
) -> String {
    let face = if radiant { "Radiant" } else { "base" };
    format!(
        "{} {} ({face}) in {zone:?}, {direction:?}: {offer:?} {what}",
        def.id, def.name
    )
}

#[test]
fn r3888000_every_catalog_card_offers_on_each_face_in_each_zone_exactly_the_live_changes() {
    let s = scenario(json!({ "seed": "r3888000-sweep", "turn": 5 }));
    let base = s.state().clone();
    let mut failures: Vec<String> = Vec::new();
    let mut read = 0usize;
    for def in registered_catalog().values() {
        for radiant in [false, true] {
            let mut sources: Vec<(TuneDirection, TuneOffer)> = Vec::new();
            for place in [Place::Deck, Place::Hand] {
                let Some((state, card)) = placed(&base, def, radiant, place) else {
                    continue;
                };
                for direction in DIRECTIONS {
                    for offer in menu_offers(&state, &card, direction, false) {
                        if !sources.contains(&(direction, offer.clone())) {
                            sources.push((direction, offer));
                        }
                    }
                }
            }
            // Being played is the hand copy read at §10.5 step 3 (R1845001).
            let columns: Vec<(TuneZone, Option<(GameState, CardInstance)>)> = vec![
                (TuneZone::Deck, placed(&base, def, radiant, Place::Deck)),
                (TuneZone::Hand, placed(&base, def, radiant, Place::Hand)),
                (TuneZone::Played, placed(&base, def, radiant, Place::Hand)),
                (TuneZone::Field, placed(&base, def, radiant, Place::Field)),
                (TuneZone::FaceDown, placed(&base, def, radiant, Place::FaceDown)),
                (TuneZone::Dormant, placed(&base, def, radiant, Place::Dormant)),
                (TuneZone::Graveyard, placed(&base, def, radiant, Place::Graveyard)),
                (TuneZone::Exile, placed(&base, def, radiant, Place::Exile)),
            ];
            for (zone, copy) in columns {
                let Some((state, card)) = copy else {
                    continue;
                };
                let as_played = zone == TuneZone::Played;
                if !as_played {
                    let expected = match zone {
                        TuneZone::Deck => Place::Deck,
                        TuneZone::Hand => Place::Hand,
                        TuneZone::Field => Place::Field,
                        TuneZone::FaceDown => Place::FaceDown,
                        TuneZone::Dormant => Place::Dormant,
                        TuneZone::Graveyard => Place::Graveyard,
                        _ => Place::Exile,
                    };
                    assert_eq!(zone_of(expected), zone);
                    if tune_zone_of(&state, &card) != Some(zone) {
                        failures.push(format!(
                            "{} ({}) is read in {:?}, placed in {zone:?}",
                            def.id,
                            if radiant { "Radiant" } else { "base" },
                            tune_zone_of(&state, &card)
                        ));
                        continue;
                    }
                }
                for direction in DIRECTIONS {
                    let offers = menu_offers(&state, &card, direction, as_played);
                    read += 1;
                    for offer in &offers {
                        if !live(&state, &card, zone, acts_of(&state, &card, offer)) {
                            failures.push(named(
                                def,
                                radiant,
                                zone,
                                direction,
                                offer,
                                "is offered, and the table calls it dead",
                            ));
                        }
                    }
                    for (from, offer) in &sources {
                        if *from != direction
                            || offer.row == TuneRow::Plague
                            || offers.contains(offer)
                            || !could_take(&state, &card, offer)
                            || !live(&state, &card, zone, acts_of(&state, &card, offer))
                        {
                            continue;
                        }
                        failures.push(named(
                            def,
                            radiant,
                            zone,
                            direction,
                            offer,
                            "is live by the table, and not offered",
                        ));
                    }
                }
            }
        }
    }
    assert!(
        read > registered_catalog().len() * 2 * 4,
        "the sweep read {read} menus"
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
