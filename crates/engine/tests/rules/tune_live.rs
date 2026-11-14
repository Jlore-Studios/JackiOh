//! Issue #3888: where a Buff or a Nerf is live (R3888000, SPEC §6.3's table). A change is drawn only
//! where it can still change what the card does, from the zone it is in or one it reaches in the
//! normal course: `tune_live.rs` holds the table, and every menu row reads it. R3888001: a declared
//! number's `read` mark says when it is read.
//!
//! This file's own `fx-` defs are indexed from 38880 (BUILD §0); Cloaked set and reveal follow
//! `balance_2727.rs`.

use jackioh_engine::effects::{
    TuneDirection, TuneItem, TuneOffer, TuneRow, TuneZone, applicable_changes, menu_offers, tune_zone_of,
};
use jackioh_engine::testkit::*;

use super::fixtures::catalog::token_def;
use super::fixtures::harness::{act, both, live, put, slot};
use super::fixtures::prompt_harness::board;

const P1: PlayerId = PlayerId::P1;
const DEGRADE: TuneDirection = TuneDirection::Degrade;
const UPGRADE: TuneDirection = TuneDirection::Upgrade;

/// How many seeds a "never" claim is tried over.
const SEEDS: u32 = 20;

/// A fixture card: its type, cost, printed keywords and text, and one declared number `n` (2, more is
/// better) with the `read` mark and `readOn` given.
fn def(index: i32, type_: &str, keywords: Value, read: Option<&str>, read_on: Option<&str>) -> CardDef {
    let mut param = json!({ "key": "n", "base": 2, "radiant": 2, "better": "up", "step": 1, "min": 1 });
    if let Some(read) = read {
        param["read"] = json!(read);
    }
    if let Some(read_on) = read_on {
        param["readOn"] = json!(read_on);
    }
    let face = |attack: i32, health: i32| {
        let mut face = json!({ "keywords": keywords, "text": "Draw {n}." });
        if type_ == "Unit" {
            face["attack"] = json!(attack);
            face["health"] = json!(health);
        }
        face
    };
    json_as(json!({
        "id": format!("fx-{index}"),
        "index": index.to_string(),
        "name": format!("Fixture {index}"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "params": [param],
        "base": face(2, 2),
        "radiant": face(4, 4),
    }))
}

fn cast_on_draw_unit() -> CardDef {
    def(38880, "Unit", json!([]), Some("castOnDraw"), None)
}
fn cast_on_draw_spell() -> CardDef {
    def(38881, "Spell", json!([]), Some("castOnDraw"), None)
}
fn combo_unit() -> CardDef {
    def(38882, "Unit", json!([]), Some("combo"), None)
}
fn awaken_unit() -> CardDef {
    def(38883, "Unit", json!([]), Some("awaken"), None)
}
fn evolve_unit() -> CardDef {
    def(38884, "Unit", json!([]), Some("evolve"), None)
}
fn hand_or_deck_unit() -> CardDef {
    def(38885, "Unit", json!([]), Some("handOrDeck"), None)
}
fn graveyard_unit() -> CardDef {
    def(38886, "Unit", json!([]), Some("graveyard"), None)
}
/// A Unit with Rush and Charge.
fn rusher() -> CardDef {
    def(
        38887,
        "Unit",
        json!([{ "kind": "Rush" }, { "kind": "Charge" }]),
        None,
        None,
    )
}
/// A Unit with Rush and Reborn: its Reborn body comes back summoning sick (R83).
fn reborn_rusher() -> CardDef {
    def(
        38888,
        "Unit",
        json!([{ "kind": "Rush" }, { "kind": "Reborn" }]),
        None,
        None,
    )
}
/// A Unit with nothing printed but its number.
fn plain() -> CardDef {
    def(38889, "Unit", json!([]), None, None)
}
/// A Field Spell with Echo 1 (a static flag).
fn echo_field() -> CardDef {
    def(38890, "Field Spell", json!([]), None, None)
}
/// A Spell with Echo 1, and one with Echo 1 and Flashback.
fn echo_spell() -> CardDef {
    def(38891, "Spell", json!([]), None, None)
}
fn echo_flashback_spell() -> CardDef {
    def(38892, "Spell", json!([{ "kind": "Flashback" }]), None, None)
}
/// A Unit with Cloaked, its number read only by its Cry.
fn cloaked_cry() -> CardDef {
    def(38893, "Unit", json!([{ "kind": "Cloaked" }]), Some("cry"), None)
}
/// A 0/1 Unit with Tribute 1 and nothing else a Nerf could move once it is played.
fn tributer() -> CardDef {
    let mut card: CardDef = def(38894, "Unit", json!([]), None, None);
    card.params = None;
    for face in [&mut card.base, &mut card.radiant] {
        face.attack = Some(0);
        face.health = Some(1);
        face.text = "Tribute 1".to_string();
    }
    card
}
/// A Trap with nothing printed but its number.
fn trap() -> CardDef {
    def(38895, "Trap", json!([]), None, None)
}
/// A Unit with Crater.
fn crater_unit() -> CardDef {
    def(38896, "Unit", json!([{ "kind": "Crater" }]), None, None)
}
/// A Unit whose number only its base face's Cry reads, its Radiant face's lasting text reading it too.
fn base_cry_unit() -> CardDef {
    def(38897, "Unit", json!([]), Some("cry"), Some("base"))
}
/// A Unit that casts itself on draw.
fn tempo_unit() -> CardDef {
    def(38898, "Unit", json!([]), None, None)
}
/// A Unit with Flashback.
fn flashback_unit() -> CardDef {
    def(38899, "Unit", json!([{ "kind": "Flashback" }]), None, None)
}

fn token() -> CardDef {
    token_def("tune-live", Vec::<Tag>::new())
}

/// p1's main phase on turn 3 with 4 mana a side, this file's cards registered.
fn playing(seed: &str) -> GameState {
    let state = board(seed);
    let flags = |flags: StaticFlags| {
        both(Script {
            static_flags: Some(flags),
            ..Script::default()
        })
    };
    let echo = || {
        flags(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        })
    };
    let defs: Vec<(CardDef, CardScripts)> = vec![
        (
            cast_on_draw_unit(),
            flags(StaticFlags {
                cast_on_draw: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (
            cast_on_draw_spell(),
            flags(StaticFlags {
                cast_on_draw: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (combo_unit(), both(Script::default())),
        (awaken_unit(), both(Script::default())),
        (evolve_unit(), both(Script::default())),
        (hand_or_deck_unit(), both(Script::default())),
        (graveyard_unit(), both(Script::default())),
        (rusher(), both(Script::default())),
        (reborn_rusher(), both(Script::default())),
        (plain(), both(Script::default())),
        (echo_field(), echo()),
        (echo_spell(), echo()),
        (echo_flashback_spell(), echo()),
        (cloaked_cry(), both(Script::default())),
        (
            tributer(),
            flags(StaticFlags {
                tribute: Some(1),
                ..StaticFlags::default()
            }),
        ),
        (trap(), both(Script::default())),
        (crater_unit(), both(Script::default())),
        (base_cry_unit(), both(Script::default())),
        (
            tempo_unit(),
            flags(StaticFlags {
                cast_on_draw: Some(true),
                ..StaticFlags::default()
            }),
        ),
        (flashback_unit(), both(Script::default())),
        (token(), both(Script::default())),
    ];
    let mut catalog = registered_catalog().clone();
    let mut scripts = registered_scripts().clone();
    for (def, script) in defs {
        catalog.insert(def.id.clone(), def.clone());
        scripts.insert(def.id.clone(), script);
    }
    register_catalog(catalog);
    register_scripts(scripts);
    state
}

/// Where a test puts a card.
#[derive(Clone, Copy, Debug)]
enum At {
    Deck,
    Hand,
    Graveyard,
    Exile,
    /// Face-up in p1's unit row (or backrow, for a card that is no Unit) at this lane, entered on an
    /// earlier turn.
    Field(i32),
}

/// A fresh p1 instance of `def` placed where `at` says.
fn place(state: &mut GameState, def: &CardDef, at: At) -> CardInstance {
    let pile = |state: &mut GameState, zone: Zone| {
        let card = new_instance(state, &def.id, P1, zone.clone());
        let player = &mut state.players[P1];
        match zone {
            Zone::Library { .. } => player.library.push(card.clone()),
            Zone::Hand { .. } => player.hand.push(card.clone()),
            Zone::Graveyard { .. } => player.graveyard.push(card.clone()),
            _ => player.exile.push(card.clone()),
        }
        card
    };
    match at {
        At::Deck => pile(state, Zone::Library { player: P1 }),
        At::Hand => pile(state, Zone::Hand { player: P1 }),
        At::Graveyard => pile(state, Zone::Graveyard { player: P1 }),
        At::Exile => pile(state, Zone::Exile { player: P1 }),
        At::Field(lane) => {
            let row = if def.type_ == CardType::Unit {
                Row::Units
            } else {
                Row::Backrow
            };
            let card = put(state, &def.id, slot(P1, row, lane), json!({}));
            let turn = state.turn;
            edit(state, &card.id, |card| card.summoned_turn = Some(turn - 1));
            live(state, card.id.as_str())
        }
    }
}

fn edit(state: &mut GameState, id: &str, change: impl FnOnce(&mut CardInstance)) {
    change(find_instance_mut(state, id).expect("the card is in the state"));
}

fn offers(state: &GameState, id: &str, direction: TuneDirection) -> Vec<TuneOffer> {
    menu_offers(state, &live(state, id), direction, false)
}

fn played_offers(state: &GameState, id: &str, direction: TuneDirection) -> Vec<TuneOffer> {
    menu_offers(state, &live(state, id), direction, true)
}

fn item(offers: &[TuneOffer], wanted: TuneItem) -> bool {
    offers.iter().any(|offer| offer.item.as_ref() == Some(&wanted))
}

fn number(offers: &[TuneOffer]) -> bool {
    item(offers, TuneItem::Number("n".to_string()))
}

fn row(offers: &[TuneOffer], wanted: TuneRow) -> bool {
    offers.iter().any(|offer| offer.row == wanted)
}

#[test]
fn r3888000_a_cast_on_draw_number_is_drawn_in_the_deck_only_on_a_unit_and_wherever_a_spell_resolves() {
    let mut state = playing("r3888000-cast-on-draw");
    // A Unit's "Cast on draw:" runs only as its draw casts it (`cast_on_draw_now`).
    let unit = cast_on_draw_unit();
    let deck = place(&mut state, &unit, At::Deck);
    let hand = place(&mut state, &unit, At::Hand);
    let field = place(&mut state, &unit, At::Field(1));
    assert!(number(&offers(&state, &deck.id, UPGRADE)));
    assert!(!number(&offers(&state, &hand.id, UPGRADE)));
    assert!(!number(&offers(&state, &field.id, DEGRADE)));
    assert!(
        !number(&played_offers(&state, &hand.id, DEGRADE)),
        "played from a hand"
    );
    state.held_draws = Some(vec![hand.id.clone()]);
    assert!(
        number(&played_offers(&state, &hand.id, DEGRADE)),
        "being cast by its draw"
    );

    // A Spell's is its resolution, run wherever it is cast.
    let spell = cast_on_draw_spell();
    let deck = place(&mut state, &spell, At::Deck);
    let hand = place(&mut state, &spell, At::Hand);
    let graveyard = place(&mut state, &spell, At::Graveyard);
    assert!(number(&offers(&state, &deck.id, UPGRADE)));
    assert!(number(&offers(&state, &hand.id, UPGRADE)));
    assert!(number(&played_offers(&state, &hand.id, DEGRADE)));
    assert!(
        !number(&offers(&state, &graveyard.id, UPGRADE)),
        "no play takes it from there"
    );
}

#[test]
fn r3888000_a_combo_number_is_not_drawn_on_the_field() {
    let mut state = playing("r3888000-combo");
    let deck = place(&mut state, &combo_unit(), At::Deck);
    let hand = place(&mut state, &combo_unit(), At::Hand);
    let field = place(&mut state, &combo_unit(), At::Field(1));
    assert!(number(&offers(&state, &deck.id, UPGRADE)));
    assert!(number(&offers(&state, &hand.id, UPGRADE)));
    assert!(number(&played_offers(&state, &hand.id, DEGRADE)));
    assert!(!number(&offers(&state, &field.id, UPGRADE)));
    assert!(!number(&offers(&state, &field.id, DEGRADE)));
    // Its stats still last.
    assert!(row(&offers(&state, &field.id, UPGRADE), TuneRow::Stats));
}

#[test]
fn r3888000_an_awaken_number_is_drawn_until_the_card_has_awoken() {
    let mut state = playing("r3888000-awaken");
    let hand = place(&mut state, &awaken_unit(), At::Hand);
    let dormant = place(&mut state, &awaken_unit(), At::Field(1));
    edit(&mut state, &dormant.id, |card| {
        card.slumber = Some(Slumber { turns: Some(2) })
    });
    let awake = place(&mut state, &awaken_unit(), At::Field(2));
    assert_eq!(
        tune_zone_of(&state, &live(&state, dormant.id.as_str())),
        Some(TuneZone::Dormant)
    );
    assert!(number(&offers(&state, &hand.id, UPGRADE)));
    assert!(number(&offers(&state, &dormant.id, UPGRADE)));
    assert!(!number(&offers(&state, &awake.id, UPGRADE)));
    // R3039000: an awakened card never sleeps again, so its awakening is spent in every zone.
    edit(&mut state, &hand.id, |card| card.awakened = Some(true));
    assert!(!number(&offers(&state, &hand.id, UPGRADE)));
}

#[test]
fn r3888000_an_evolve_number_is_drawn_on_the_field_until_it_evolves() {
    let mut state = playing("r3888000-evolve");
    let field = place(&mut state, &evolve_unit(), At::Field(1));
    let graveyard = place(&mut state, &evolve_unit(), At::Graveyard);
    let exile = place(&mut state, &evolve_unit(), At::Exile);
    assert!(number(&offers(&state, &field.id, UPGRADE)));
    // An Evolve reaches a graveyard card where it lies (`evolution::evolve_now`).
    assert!(number(&offers(&state, &graveyard.id, UPGRADE)));
    assert!(!number(&offers(&state, &exile.id, UPGRADE)));
    // R3536000: a card evolves once.
    edit(&mut state, &field.id, |card| card.evolved = Some(true));
    assert!(!number(&offers(&state, &field.id, UPGRADE)));
    assert!(row(&offers(&state, &field.id, UPGRADE), TuneRow::Stats));
}

#[test]
fn r3888000_a_hand_or_deck_number_is_not_drawn_on_the_field() {
    let mut state = playing("r3888000-hand-or-deck");
    let deck = place(&mut state, &hand_or_deck_unit(), At::Deck);
    let hand = place(&mut state, &hand_or_deck_unit(), At::Hand);
    let field = place(&mut state, &hand_or_deck_unit(), At::Field(1));
    assert!(number(&offers(&state, &deck.id, UPGRADE)));
    assert!(number(&offers(&state, &hand.id, DEGRADE)));
    assert!(!number(&played_offers(&state, &hand.id, DEGRADE)));
    assert!(!number(&offers(&state, &field.id, UPGRADE)));
    assert!(!number(&offers(&state, &field.id, DEGRADE)));
}

#[test]
fn r3888000_a_graveyard_number_is_drawn_on_the_field_and_in_the_graveyard_not_in_exile() {
    let mut state = playing("r3888000-graveyard-number");
    let hand = place(&mut state, &graveyard_unit(), At::Hand);
    let field = place(&mut state, &graveyard_unit(), At::Field(1));
    let graveyard = place(&mut state, &graveyard_unit(), At::Graveyard);
    let exile = place(&mut state, &graveyard_unit(), At::Exile);
    assert!(number(&offers(&state, &hand.id, UPGRADE)));
    assert!(number(&offers(&state, &field.id, UPGRADE)));
    assert!(number(&offers(&state, &graveyard.id, UPGRADE)));
    assert!(!number(&offers(&state, &exile.id, UPGRADE)));
}

#[test]
fn r3888000_rush_and_charge_are_drawn_on_the_field_only_the_turn_the_unit_entered() {
    let mut state = playing("r3888000-rush");
    let fast = place(&mut state, &rusher(), At::Field(1));
    let slow = place(&mut state, &plain(), At::Field(2));
    // On an earlier turn: neither comes off, and a Buff never gives Rush.
    let nerfs = offers(&state, &fast.id, DEGRADE);
    assert!(!item(&nerfs, TuneItem::Remove(KeywordKind::Rush)));
    assert!(!item(&nerfs, TuneItem::Remove(KeywordKind::Charge)));
    let buffs = offers(&state, &slow.id, UPGRADE);
    assert!(!item(&buffs, TuneItem::Add(KeywordKind::Rush)));
    assert!(
        item(&buffs, TuneItem::Add(KeywordKind::Taunt)),
        "the rest of R21's pool lasts"
    );
    // The turn it entered, summoning sick: both are live.
    let turn = state.turn;
    for id in [&fast.id, &slow.id] {
        edit(&mut state, id, |card| card.summoned_turn = Some(turn));
    }
    let nerfs = offers(&state, &fast.id, DEGRADE);
    assert!(item(&nerfs, TuneItem::Remove(KeywordKind::Rush)));
    assert!(item(&nerfs, TuneItem::Remove(KeywordKind::Charge)));
    assert!(item(
        &offers(&state, &slow.id, UPGRADE),
        TuneItem::Add(KeywordKind::Rush)
    ));
    // R83: a Unit whose Reborn would bring it back sick keeps its Rush live.
    let reborn = place(&mut state, &reborn_rusher(), At::Field(3));
    assert!(item(
        &offers(&state, &reborn.id, DEGRADE),
        TuneItem::Remove(KeywordKind::Rush)
    ));
    // In a hand it is still to enter.
    let hand = place(&mut state, &rusher(), At::Hand);
    assert!(item(
        &offers(&state, &hand.id, DEGRADE),
        TuneItem::Remove(KeywordKind::Charge)
    ));
}

#[test]
fn r3888000_echo_is_not_drawn_on_the_field() {
    let mut state = playing("r3888000-echo");
    let echo = TuneItem::X("Echo".to_string());
    let hand = place(&mut state, &echo_field(), At::Hand);
    let field = place(&mut state, &echo_field(), At::Field(1));
    assert_eq!(
        tune_zone_of(&state, &live(&state, field.id.as_str())),
        Some(TuneZone::Field)
    );
    assert!(item(&offers(&state, &hand.id, UPGRADE), echo.clone()));
    // Echo 1 is at its floor for a Nerf (`TUNED_FLOOR`), so the Buff reads it.
    assert!(item(&played_offers(&state, &hand.id, UPGRADE), echo.clone()));
    assert!(!item(&offers(&state, &field.id, UPGRADE), echo.clone()));
    assert!(!item(&offers(&state, &field.id, DEGRADE), echo.clone()));
    // A Spell's Echo in a graveyard counts only while a play may take it from there.
    let spent = place(&mut state, &echo_spell(), At::Graveyard);
    let again = place(&mut state, &echo_flashback_spell(), At::Graveyard);
    assert!(!item(&offers(&state, &spent.id, UPGRADE), echo.clone()));
    assert!(item(&offers(&state, &again.id, UPGRADE), echo));
}

#[test]
fn r3888000_cloaked_is_not_drawn_on_a_face_up_field_card_and_is_in_a_deck() {
    let mut state = playing("r3888000-cloaked");
    let cloaked = cloaked_cry();
    let field = place(&mut state, &cloaked, At::Field(1));
    let hand = place(&mut state, &cloaked, At::Hand);
    let deck = place(&mut state, &cloaked, At::Deck);
    let off = TuneItem::Remove(KeywordKind::Cloaked);
    assert!(!item(&offers(&state, &field.id, DEGRADE), off.clone()));
    assert!(item(&offers(&state, &hand.id, DEGRADE), off.clone()));
    assert!(item(&offers(&state, &deck.id, DEGRADE), off));
    let on = TuneItem::Add(KeywordKind::Cloaked);
    let plain_deck = place(&mut state, &plain(), At::Deck);
    let plain_field = place(&mut state, &plain(), At::Field(2));
    assert!(item(&offers(&state, &plain_deck.id, UPGRADE), on.clone()));
    assert!(!item(&offers(&state, &plain_field.id, UPGRADE), on));
}

#[test]
fn r3888000_a_hidden_nerf_never_draws_a_paid_tribute() {
    let tribute = TuneItem::X("Tribute".to_string());
    let mut state = playing("r3888000-tribute-menu");
    let held = place(&mut state, &tributer(), At::Hand);
    // In a hand a Nerf may raise the Tribute still to pay; as it is played, step 2 has paid it, and
    // the price with it. Crater, still to act as the card arrives at step 4, is what is left.
    let in_hand = offers(&state, &held.id, DEGRADE);
    assert!(item(&in_hand, tribute.clone()));
    assert!(row(&in_hand, TuneRow::Cost));
    assert_eq!(
        played_offers(&state, &held.id, DEGRADE),
        vec![TuneOffer {
            row: TuneRow::Keyword,
            item: Some(TuneItem::Add(KeywordKind::Crater))
        }]
    );
    for seed in 0..SEEDS {
        let mut state = playing(&format!("r3888000-tribute-{seed}"));
        place(&mut state, &plain(), At::Hand);
        let fodder = place(&mut state, &plain(), At::Field(1));
        let held = place(&mut state, &tributer(), At::Hand);
        edit(&mut state, &held.id, |card| card.hidden_nerfs = Some(1));
        let state = act(
            &state,
            json!({
                "type": "play",
                "instanceId": held.id,
                "zone": { "row": "units", "lane": 2 },
                "tributes": [fodder.id],
                "playerId": "p1",
            }),
        );
        let played = live(&state, held.id.as_str());
        assert_eq!(played.zone.z(), ZoneName::Field, "seed {seed}");
        assert_eq!(played.hidden_nerfs, None, "seed {seed}: the count is spent");
        assert_eq!(
            played
                .tuning
                .as_ref()
                .and_then(|tuning| tuning.x.as_ref())
                .and_then(|x| x.get("Tribute")),
            None,
            "seed {seed}: the paid Tribute is never moved"
        );
    }
}

#[test]
fn r3888000_a_face_down_cloaked_cards_cry_numbers_are_drawn_and_its_cost_is_not() {
    let mut state = playing("r3888000-face-down");
    place(&mut state, &plain(), At::Hand);
    let card = place(&mut state, &cloaked_cry(), At::Hand);
    assert!(row(&offers(&state, &card.id, UPGRADE), TuneRow::Cost));
    let state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": card.id,
            "zone": { "row": "backrow", "lane": 2 },
            "faceDown": "cloaked",
            "playerId": "p1",
        }),
    );
    let card = state.players[P1].backrow[1]
        .clone()
        .expect("the set card in its backrow zone");
    assert_eq!(tune_zone_of(&state, &card), Some(TuneZone::FaceDown));
    for direction in [UPGRADE, DEGRADE] {
        let set = offers(&state, &card.id, direction);
        assert!(number(&set), "its Cry runs as it reveals (§6.3 Cloaked)");
        assert!(!row(&set, TuneRow::Cost), "its price is paid");
        // A set card is read as the card it is (R783005): a Unit's stats and R21's pool.
        assert!(row(&set, TuneRow::Stats));
    }
    assert!(item(
        &offers(&state, &card.id, UPGRADE),
        TuneItem::Add(KeywordKind::Taunt)
    ));
    let state = act(
        &state,
        json!({ "type": "activate", "instanceId": card.id, "ability": "reveal", "playerId": "p1" }),
    );
    assert_eq!(
        tune_zone_of(&state, &live(&state, card.id.as_str())),
        Some(TuneZone::Field)
    );
    assert!(
        !number(&offers(&state, &card.id, UPGRADE)),
        "revealed, its Cry has run"
    );
}

#[test]
fn r3888000_in_a_graveyard_only_what_a_play_from_there_reads_is_drawn_and_in_exile_nothing() {
    let mut state = playing("r3888000-piles");
    let lying = place(&mut state, &plain(), At::Graveyard);
    // No play takes it from there: only Flashback, which would let one, is live.
    assert_eq!(
        offers(&state, &lying.id, UPGRADE),
        vec![TuneOffer {
            row: TuneRow::Keyword,
            item: Some(TuneItem::Add(KeywordKind::Flashback))
        }]
    );
    assert_eq!(
        applicable_changes(&state, &live(&state, lying.id.as_str()), DEGRADE),
        Vec::<TuneRow>::new()
    );
    // A card Flashback lets a play take is read as that play will read it, its price aside (R766).
    let again = place(&mut state, &flashback_unit(), At::Graveyard);
    for direction in [UPGRADE, DEGRADE] {
        let rows = applicable_changes(&state, &live(&state, again.id.as_str()), direction);
        assert!(rows.contains(&TuneRow::Stats), "{direction:?}");
        assert!(rows.contains(&TuneRow::Number), "{direction:?}");
        assert!(!rows.contains(&TuneRow::Cost), "{direction:?}");
    }
    // In an exile pile nothing it carries is read again.
    for def in [plain(), flashback_unit()] {
        let exiled = place(&mut state, &def, At::Exile);
        for direction in [UPGRADE, DEGRADE] {
            assert_eq!(
                offers(&state, &exiled.id, direction),
                Vec::<TuneOffer>::new(),
                "{}",
                def.id
            );
        }
    }
}

#[test]
fn r3888000_a_unit_token_is_never_given_flashback() {
    let mut state = playing("r3888000-token");
    let token = place(&mut state, &token(), At::Field(1));
    let unit = place(&mut state, &plain(), At::Field(2));
    let flashback = TuneItem::Add(KeywordKind::Flashback);
    assert!(!item(&offers(&state, &token.id, UPGRADE), flashback.clone()));
    assert!(item(&offers(&state, &unit.id, UPGRADE), flashback));
}

#[test]
fn r3888000_a_card_that_casts_on_draw_keeps_its_cost_row_in_its_deck_as_it_can_reach_a_hand_uncast() {
    // Past R58's cap, or with no unit zone open (R459), its draw adds it to the hand uncast, where it
    // is played at its price.
    let mut state = playing("r3888000-tempo-cost");
    let deck = place(&mut state, &tempo_unit(), At::Deck);
    assert!(row(&offers(&state, &deck.id, DEGRADE), TuneRow::Cost));
    assert!(row(&offers(&state, &deck.id, UPGRADE), TuneRow::Cost));
    let field = place(&mut state, &tempo_unit(), At::Field(1));
    assert!(!row(&offers(&state, &field.id, DEGRADE), TuneRow::Cost));
}

#[test]
fn r3888000_crater_is_drawn_in_a_deck_and_on_a_face_down_card_never_on_a_face_up_one() {
    let mut state = playing("r3888000-crater");
    let crater = TuneItem::Add(KeywordKind::Crater);
    let deck = place(&mut state, &plain(), At::Deck);
    let field = place(&mut state, &plain(), At::Field(1));
    let set = put(&mut state, &trap().id, slot(P1, Row::Backrow, 1), json!({}));
    assert_eq!(
        tune_zone_of(&state, &live(&state, set.id.as_str())),
        Some(TuneZone::FaceDown)
    );
    assert!(item(&offers(&state, &deck.id, DEGRADE), crater.clone()));
    assert!(
        item(&offers(&state, &set.id, DEGRADE), crater.clone()),
        "it Locks its zone as it fires"
    );
    assert!(!item(&offers(&state, &field.id, DEGRADE), crater));
    // A Buff takes Crater off where it has still to act.
    let off = TuneItem::Remove(KeywordKind::Crater);
    let deck = place(&mut state, &crater_unit(), At::Deck);
    let field = place(&mut state, &crater_unit(), At::Field(2));
    assert!(item(&offers(&state, &deck.id, UPGRADE), off.clone()));
    assert!(!item(&offers(&state, &field.id, UPGRADE), off));
}

#[test]
fn r3888001_a_number_marked_on_one_face_is_lasting_on_the_other() {
    let mut state = playing("r3888001-read-on");
    let base = place(&mut state, &base_cry_unit(), At::Field(1));
    let radiant = place(&mut state, &base_cry_unit(), At::Field(2));
    edit(&mut state, &radiant.id, |card| card.radiant = true);
    assert!(
        !number(&offers(&state, &base.id, UPGRADE)),
        "the base face's Cry has run"
    );
    assert!(
        number(&offers(&state, &radiant.id, UPGRADE)),
        "the Radiant face's lasting text reads it"
    );
}
