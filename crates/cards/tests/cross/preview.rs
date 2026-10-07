//! Port of `packages/cards/test/preview.test.ts` (v0.3.0 part 27.5).
//!
//! R280 (SPEC §10.8, §10.9, §10.10): the number a Core card's formula comes to now, as `viewFor`
//! carries it (`preview`), for the six Core cards whose text computes one from the board by more than
//! a plain X. Each test reads the value off `s.view(...)` and then lets the card resolve, so the value
//! is proved against what the card's own resolution does, not against a second copy of the formula.
//!
//! What each formula may read (R280), stated here and proved by the "reads nothing hidden" test at
//! the end, which makes every library and every hand throw on access:
//!
//!   #18 Bread and Butter        the active player's current mana (public, §10.8)
//!   #31 KY's Math Equation      its own count of plays (the card's own instance, R429)
//!   #38 Quickstriker            its controller's count of plays this turn (public)
//!   #40 Echoes of the Forgotten its controller's exile count (public, §3)
//!   #70 Spiteful Stab           its controller's hero health and exile count (public)
//!   #91 Fed Fauci               its own Plague Counters (its counters travel on its view, §10.8)
//!   C #19 Lizard's Breath       its controller's deck, graveyard and exile SIZES (public, §10.8), never
//!                               their contents — so it has its own proof below rather than the fence
//!                               at the end, which walls a library off whole
//!
//! None reads a library's contents or order, or a hand's contents: the value shows to every viewer
//! who may read the card — the other seat too, for a card on the field — so it must say nothing more
//! than the card and the public board do. And the view shows it only where the viewer may read the
//! card: a #31 in the opponent's hand, or the opponent's face-down #18, carries none (§10.8).
//!
//! Not previewed, on purpose (R280): #92's stats, #100's cost and #89's hand stats are on the face
//! already, and #24's and #74's X is chosen at play. The set test pins the six.
//!
//! Rust port note (recorded in `78f131c^:.fullsend/notes/spec-gaps-part-27-5.md`): TS proved "a pure read" by
//! handing the hook a deep-frozen `structuredClone` whose libraries, hands and `state.active` throw on
//! access (`Proxy`, `Object.defineProperty`). Rust has neither. A preview hook takes `&GameState`, so
//! it cannot write; the reads are fenced by perturbation instead (`fenced` below): the hook is handed
//! a copy whose fenced piles are emptied (or, where TS let the length through, rewritten card by card
//! at the same length) and whose `active` is the other seat, and it must still answer exactly what the
//! view of the real state shows.

use jackioh_cards::{card_def, scripts_of};
use jackioh_engine::subsystems::FuseArgs;
use jackioh_engine::testkit::*;
use serde::Serialize;

const BREAD_AND_BUTTER: &str = "core-018";
const MATH_EQUATION: &str = "core-031";
const QUICKSTRIKER: &str = "core-038";
const ECHOES: &str = "core-040";
const SPITEFUL_STAB: &str = "core-070";
const FED_FAUCI: &str = "core-091";
const COMBO_INDEX: &str = "core-093";

/// R583: C+ #44 Simplicity Audit and #45 Complexity Audit, whose Radiant face previews a set of cards.
const AUDITS: [&str; 2] = ["classicplus-044", "classicplus-045"];

/// R280's six, in index order.
const PREVIEWED: [&str; 6] = [
    BREAD_AND_BUTTER,
    MATH_EQUATION,
    QUICKSTRIKER,
    ECHOES,
    SPITEFUL_STAB,
    FED_FAUCI,
];

/// Patch v0.2.0's Classic cards that declare one (R280), each proved in its own block below.
const CURSE: &str = "classic-001"; // C #1 Curse of the Forgotten Classic
const PLAGUE_NUKE: &str = "classic-043"; // C #43 Plague Nuke
const LIZARDS_BREATH: &str = "classic-019"; // C #19 Lizard's Breath
const PLAGUE_DOCTOR: &str = "classic-059"; // C #59 Plague Doctor
const SIPHON_SQUAD: &str = "classic-088"; // C #88 Siphon Squad
const DIVINE_FAVOR: &str = "classic-046"; // C #46 Divine Favor
const CLASSIC_PREVIEWED: [&str; 6] = [
    CURSE,
    LIZARDS_BREATH,
    PLAGUE_NUKE,
    DIVINE_FAVOR,
    PLAGUE_DOCTOR,
    SIPHON_SQUAD,
];
/// Patch v0.2.0's cards that declare preview, each proved in its own block below (R280).
const DATACENTER_FIRE: &str = "classicplus-t-ai-06";
const TWICE_FORWARD: &str = "classicplus-074";
const NEW_SET_PREVIEWED: [&str; 2] = [TWICE_FORWARD, DATACENTER_FIRE];

/// Patch v0.2.0's Classic+ cards #1–#39 that declare one (R280), each proved in its own test file.
const SNAKE: &str = "classicplus-003"; // C+ #3's hits: test/classic-plus/003-second-amendment-snake.test.ts
const FROZEN_WASTES: &str = "classicplus-012-6"; // C+ #12.6's exiles: test/classic-plus/012-6-frozen-wastes.test.ts
const BOOK_WORM: &str = "classicplus-039"; // C+ #39 Book Worm's N: test/classic-plus/039-book-worm.test.ts
const CLASSIC_PLUS_C_PREVIEWED: [&str; 3] = [SNAKE, FROZEN_WASTES, BOOK_WORM];

const RAPID_REPLENISH: &str = "core-010"; // 0-cost Spell; Combo 3, so nothing at one play — a free anchor
const TEMPO_TIMMY: &str = "core-011"; // 1-cost Unit
const BIG_D_FENDER: &str = "core-001"; // 2-cost Unit
const MENACE: &str = "core-019"; // library filler, and a 9/9 target that survives
const STOCKPILE: &str = "core-005";

/// TS `AT_ENEMY_HERO`'s one selection: `{ pick: "hero", player: "p2" }`.
fn at_enemy_hero() -> Value {
    json!({ "pick": "hero", "player": "p2" })
}

/// TS `type Face = "base" | "radiant"` is the wire's `FaceKind`.
const FACES: [FaceKind; 2] = [FaceKind::Base, FaceKind::Radiant];

fn radiant_of(face: FaceKind) -> bool {
    face == FaceKind::Radiant
}

fn face_of(radiant: bool) -> FaceKind {
    if radiant {
        FaceKind::Radiant
    } else {
        FaceKind::Base
    }
}

/// `cardDef(id)[face].text`.
fn text_of(id: &str, face: FaceKind) -> String {
    card_def(id).face(face).text.clone()
}

/// TS `CARDS[id]?.[face].preview`.
fn preview_hook(id: &str, face: FaceKind) -> Option<PreviewHook> {
    let scripts = scripts_of();
    let card = scripts.get(id)?;
    match face {
        FaceKind::Base => card.base.preview.clone(),
        FaceKind::Radiant => card.radiant.preview.clone(),
    }
}

/// A view part as the JSON the client receives (`null` for an empty place).
fn json_of<T: Serialize>(part: &T) -> Value {
    serde_json::to_value(part).expect("a view serialises")
}

fn own_hand(view: &PlayerView) -> Vec<CardView> {
    match &view.you.hand {
        HandView::Cards(cards) => cards.clone(),
        HandView::Count { .. } => panic!("the viewer's own hand must travel in full (§10.8)"),
    }
}

fn hand_card(view: &PlayerView, instance_id: &str) -> CardView {
    match own_hand(view)
        .into_iter()
        .find(|card| card.instance_id == instance_id)
    {
        Some(card) => card,
        None => panic!("{instance_id} is not in the viewer's hand"),
    }
}

/// The list the view carries, or `None` when the key is absent; a key holding `[]` fails here.
fn shown(card: &Value) -> Option<Vec<PreviewValue>> {
    assert!(!card.is_null(), "no card at that place in the view");
    let list = card.get("preview")?;
    assert!(
        list.as_array().is_some_and(|entries| !entries.is_empty()),
        "preview is absent rather than empty"
    );
    Some(serde_json::from_value(list.clone()).expect("a preview list"))
}

fn shown_of<T: Serialize>(card: &T) -> Option<Vec<PreviewValue>> {
    shown(&json_of(card))
}

/// The one value a card's view carries, failing unless there is exactly one.
fn value_of<T: Serialize>(card: &T) -> i32 {
    let list = shown_of(card);
    assert!(list.is_some(), "the card carries a preview");
    let list = list.unwrap_or_default();
    assert_eq!(list.len(), 1);
    list.first().map(|entry| entry.value).unwrap_or(i32::MIN)
}

/// The amounts of every `damage` event on a hero so far, in order: one entry per hit.
fn hits_on(s: &Scenario, player: PlayerId) -> Vec<i32> {
    let hero = format!("hero-{player}");
    s.events()
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } if *target_id == hero => Some(*amount),
            _ => None,
        })
        .collect()
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("missing: {what}"),
    }
}

/// The first card of `player`'s hand with `def_id`, as it stands.
fn held(s: &Scenario, player: PlayerId, def_id: &str) -> Option<CardInstance> {
    s.hand(player).iter().find(|card| card.def_id == def_id).cloned()
}

/// TS `subsystems.fuse({ state: s.state, events: [], rng }, args)`: a fresh sink over the live state,
/// its rng cursor not written back (as TS did not).
fn fuse_on(s: &mut Scenario, args: FuseArgs) -> Option<CardInstance> {
    let mut events: Vec<GameEvent> = vec![];
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
    subsystems::fuse(&mut sink, args)
}

// =============================================================================================
// A read of what the controller may read, and nothing else
// =============================================================================================

/// How `fenced` walls off a pile a preview must not read (TS: a getter that throws, or a `Proxy` that
/// answers `length` and throws on anything else).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fence {
    /// TS threw on any read of the pile: the copy's pile is empty.
    Whole,
    /// TS let `length` through and threw on the cards: the copy's pile keeps its length, but every
    /// card in it is another card, in the reverse order.
    SizeOnly,
}

/// The card a sealed pile is filled with: what a size-only pile's cards are rewritten to.
const SEALED: &str = "core-008";

fn seal(pile: &mut Vec<CardInstance>, fence: Fence, player: PlayerId, what: &str) {
    match fence {
        Fence::Whole => pile.clear(),
        Fence::SizeOnly => {
            pile.reverse();
            for (at, card) in pile.iter_mut().enumerate() {
                card.id = format!("sealed-{player}-{what}-{at}");
                card.def_id = SEALED.to_string();
            }
        }
    }
}

/// A copy of `state` for a preview hook to answer on, with every library and every hand fenced as
/// given and, when `flip_active`, the other seat active (a hook reads `yourTurn` instead, README §1).
/// `self` is found before its zone is fenced off, so a hand card's hook still has itself. The hook
/// takes `&GameState`, so TS's `deepFreeze` (no write) holds by the type.
fn fenced(
    state: &GameState,
    self_id: &str,
    libraries: Fence,
    hands: Fence,
    flip_active: bool,
) -> (GameState, CardInstance) {
    let mut copy = state.clone();
    let found = PLAYER_IDS
        .iter()
        .flat_map(|&player| {
            let side = &copy.players[player];
            side.hand
                .iter()
                .chain(side.units.iter().flatten().flatten())
                .chain(side.backrow.iter().flatten())
        })
        .find(|card| card.id == self_id)
        .cloned();
    let self_ = must(found, &format!("{self_id} in the copy"));
    for player in PLAYER_IDS {
        seal(&mut copy.players[player].library, libraries, player, "library");
        seal(&mut copy.players[player].hand, hands, player, "hand");
    }
    if flip_active {
        copy.active = opponent_of(copy.active);
    }
    (copy, self_)
}

/// TS `guarded`: no library, no hand and no `state.active` may be read.
fn guarded(state: &GameState, self_id: &str) -> (GameState, CardInstance) {
    fenced(state, self_id, Fence::Whole, Fence::Whole, true)
}

// =============================================================================================
// The set, and the labels
// =============================================================================================

mod r280_the_core_cards_that_declare_preview {
    use super::*;

    // R372 added #93 Combo-Index, whose grade is a counter on the card in play, so its hook answers
    // on the field only; the hand-based tests below keep to the six, and 093-combo-index.test.ts
    // proves its values.
    #[test]
    fn r280_r372_r583_are_exactly_18_31_38_40_70_91_and_93_and_the_new_sets_listed_ones_on_both_faces_and_c_44_and_45_on_the_radiant_face()
     {
        jackioh_cards::register_all();
        let cards = scripts_of();
        let mut hooked: Vec<String> = cards
            .iter()
            .filter(|(_, card)| card.base.preview.is_some() || card.radiant.preview.is_some())
            .map(|(id, _)| id.clone())
            .collect();
        hooked.sort();
        let mut expected: Vec<String> = PREVIEWED
            .iter()
            .chain([COMBO_INDEX].iter())
            .chain(CLASSIC_PREVIEWED.iter())
            .chain(CLASSIC_PLUS_C_PREVIEWED.iter())
            .chain(NEW_SET_PREVIEWED.iter())
            .chain(AUDITS.iter())
            .map(|id| (*id).to_string())
            .collect();
        expected.sort();
        assert_eq!(hooked, expected);
        for id in &hooked {
            let card = &cards[id.as_str()];
            // R583: an Audit's base face marks nothing; its Radiant face "highlights targets".
            if AUDITS.contains(&id.as_str()) {
                assert!(card.base.preview.is_none(), "{id} base");
            } else {
                assert!(card.base.preview.is_some(), "{id} base");
            }
            assert!(card.radiant.preview.is_some(), "{id} radiant");
        }
    }

    #[test]
    fn r280_each_label_is_an_exact_substring_of_the_running_faces_catalog_text_on_both_faces() {
        jackioh_cards::register_all();
        for id in PREVIEWED {
            for face in FACES {
                let s = scenario(
                    json!({ "p1": { "hand": [{ "def": id, "radiant": radiant_of(face) }, RAPID_REPLENISH] } }),
                );
                let list = must(
                    shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(id).id)),
                    &format!("{id} {face}'s preview in hand"),
                );
                let text = text_of(id, face);
                for entry in &list {
                    assert!(text.contains(&entry.label), "{id} {face}: \"{}\"", entry.label);
                }
            }
        }
    }

    #[test]
    fn r280_the_labels_are_the_formulas_each_face_prints() {
        jackioh_cards::register_all();
        let labels = |id: &str, face: FaceKind| -> Vec<String> {
            let s = scenario(
                json!({ "p1": { "hand": [{ "def": id, "radiant": radiant_of(face) }, RAPID_REPLENISH] } }),
            );
            shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(id).id))
                .unwrap_or_default()
                .into_iter()
                .map(|entry| entry.label)
                .collect()
        };
        assert_eq!(
            labels(BREAD_AND_BUTTER, FaceKind::Base),
            vec!["X = that player's unspent mana"]
        );
        assert_eq!(
            labels(BREAD_AND_BUTTER, FaceKind::Radiant),
            vec!["X = 3 × that player's unspent mana"]
        );
        assert_eq!(
            labels(MATH_EQUATION, FaceKind::Base),
            vec!["Fib(times played + 1)"]
        );
        assert_eq!(
            labels(MATH_EQUATION, FaceKind::Radiant),
            vec!["Fib(times played + 3)"]
        );
        assert_eq!(
            labels(QUICKSTRIKER, FaceKind::Base),
            vec!["X = cards you played earlier this turn"]
        );
        assert_eq!(
            labels(QUICKSTRIKER, FaceKind::Radiant),
            vec!["X = cards you played earlier this turn"]
        );
        assert_eq!(labels(ECHOES, FaceKind::Base), vec!["the cards in your exile"]);
        assert_eq!(
            labels(ECHOES, FaceKind::Radiant),
            vec!["twice the cards in your exile"]
        );
        assert_eq!(
            labels(SPITEFUL_STAB, FaceKind::Base),
            vec![text_of(SPITEFUL_STAB, FaceKind::Base)]
        );
        assert_eq!(
            labels(SPITEFUL_STAB, FaceKind::Radiant),
            vec![text_of(SPITEFUL_STAB, FaceKind::Radiant)]
        );
        assert_eq!(
            labels(FED_FAUCI, FaceKind::Base),
            vec!["+1 mana per Plague Counter"]
        );
        assert_eq!(
            labels(FED_FAUCI, FaceKind::Radiant),
            vec!["+2 mana per Plague Counter"]
        );
    }
}

// =============================================================================================
// #18 Bread and Butter: the active player's current mana
// =============================================================================================

mod c18_bread_and_butter_previews_the_bread_tokens_x_r280 {
    use super::*;

    fn trap(
        face: FaceKind,
        active: PlayerId,
        p1_mana: Option<i32>,
        p2_mana: Option<i32>,
        face_up: bool,
    ) -> Scenario {
        let mut entry = json!({ "def": BREAD_AND_BUTTER, "radiant": radiant_of(face) });
        if face_up {
            entry["faceUp"] = json!(true);
        }
        let mut p1 = json!({ "backrow": [entry], "field": ["core-012"], "library": [RAPID_REPLENISH] });
        if let Some(mana) = p1_mana {
            p1["mana"] = json!(mana);
        }
        let mut p2 = json!({ "field": ["core-012"], "library": [RAPID_REPLENISH] });
        if let Some(mana) = p2_mana {
            p2["mana"] = json!(mana);
        }
        scenario(json!({ "active": active, "p1": p1, "p2": p2 }))
    }

    #[test]
    fn r280_base_3_unspent_mana_on_its_controllers_turn_previews_3_and_the_turns_end_makes_a_3_3() {
        jackioh_cards::register_all();
        let mut s = trap(FaceKind::Base, PlayerId::P1, Some(3), None, false);
        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 3);

        s.end_turn();

        let token = must(s.unit(PlayerId::P1, 2), "the Bread Token");
        assert_eq!(token.def_id, "core-t-bread");
        s.expect_stats(token.id.as_str(), json!({ "attack": 3, "health": 3 }));
    }

    #[test]
    fn r280_radiant_2_unspent_mana_previews_3_2_6_and_the_turns_end_makes_a_6_6() {
        jackioh_cards::register_all();
        let mut s = trap(FaceKind::Radiant, PlayerId::P1, Some(2), None, false);
        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 6);

        s.end_turn();

        let token = must(
            s.unit(PlayerId::P1, 2).map(|card| card.id.clone()),
            "the Bread Token",
        );
        s.expect_stats(token.as_str(), json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r280_on_the_opponents_turn_it_reads_the_active_players_mana_not_its_controllers() {
        jackioh_cards::register_all();
        let mut s = trap(FaceKind::Base, PlayerId::P2, Some(1), Some(4), false);
        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 4);

        s.end_turn();

        // R52: p2 ended with 4, and the 4/4 is the trap's controller's.
        let token = must(
            s.unit(PlayerId::P1, 2).map(|card| card.id.clone()),
            "the Bread Token",
        );
        s.expect_stats(token.as_str(), json!({ "attack": 4, "health": 4 }));
    }

    #[test]
    fn r280_face_down_only_its_controller_sees_it_the_other_seats_view_carries_nothing_of_it() {
        jackioh_cards::register_all();
        let s = trap(FaceKind::Base, PlayerId::P1, Some(3), None, false);

        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 3);
        let theirs = s.view(PlayerId::P2);
        assert_eq!(
            json_of(&theirs.opponent.backrow[0]),
            json!({ "faceDown": true, "cost": 1 })
        );
        assert!(!json_of(&theirs).to_string().contains("unspent mana"));
    }

    #[test]
    fn r280_face_up_fired_it_is_public_and_both_seats_see_the_same_number() {
        jackioh_cards::register_all();
        let s = trap(FaceKind::Radiant, PlayerId::P1, Some(1), None, true);

        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 3);
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.backrow[0]), 3);
    }
}

// =============================================================================================
// #31 KY's Math Equation: its own count of plays (R429)
// =============================================================================================

mod c31_kys_math_equation_previews_its_damage_r280 {
    use super::*;

    fn equation(face: FaceKind, times_played: i32, cost_mod: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": MATH_EQUATION, "radiant": radiant_of(face), "costMod": cost_mod }, RAPID_REPLENISH],
                "mana": 10,
            },
            "p2": { "field": [MENACE], "hand": [MATH_EQUATION] },
        }));
        // R429: the plays it has had before, which the harness cannot seed.
        let card = must(held(&s, PlayerId::P1, MATH_EQUATION), "p1's Equation").id;
        must(find_instance_mut(s.state_mut(), &card), "p1's Equation").times_played = Some(times_played);
        s
    }

    /// TS's `CASES` loop: one `it` per `[face, timesPlayed, costMod, damage]`.
    fn previews_and_deals(face: FaceKind, times_played: i32, cost_mod: i32, damage: i32) {
        jackioh_cards::register_all();
        let mut s = equation(face, times_played, cost_mod);
        let card = must(held(&s, PlayerId::P1, MATH_EQUATION), "p1's Equation").id;
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), damage);

        s.play(card.as_str(), json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![damage]);
    }

    #[test]
    fn r280_r429_base_played_0_times_before_costmod_0_previews_1_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Base, 0, 0, 1); // its 1st play: Fib(1 + 1)
    }

    #[test]
    fn r280_r429_base_played_2_times_before_costmod_0_previews_3_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Base, 2, 0, 3); // its 3rd play: Fib(3 + 1)
    }

    #[test]
    fn r280_r429_base_played_0_times_before_costmod_3_previews_1_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Base, 0, 3, 1); // R67: a (4) Equation's 1st play still deals Fib(1 + 1)
    }

    #[test]
    fn r280_r429_radiant_played_0_times_before_costmod_0_previews_3_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Radiant, 0, 0, 3); // Fib(1 + 3)
    }

    #[test]
    fn r280_r429_radiant_played_1_times_before_costmod_0_previews_5_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Radiant, 1, 0, 5); // Fib(2 + 3)
    }

    #[test]
    fn r280_r67_a_player_discount_changes_the_price_not_the_preview() {
        jackioh_cards::register_all();
        let mut s = equation(FaceKind::Base, 0, 0);
        let card = must(held(&s, PlayerId::P1, MATH_EQUATION), "p1's Equation").id;
        s.state_mut()
            .players
            .p1
            .mods
            .push(json_as::<PlayerModifier>(json!({
                "id": "test-spell-discount",
                "kind": "costDiscount",
                "amount": 1,
                "onlyType": "Spell",
                "expiry": { "until": "never" },
            })));

        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), 1);
    }

    #[test]
    fn r280_in_the_opponents_hand_it_is_hidden_p1s_view_carries_no_preview_at_all_p2s_own_does() {
        jackioh_cards::register_all();
        let s = equation(FaceKind::Base, 2, 0);

        let p1_view = s.view(PlayerId::P1);
        assert_eq!(p1_view.opponent.hand, HandView::Count { count: 1 });
        // p1's own Equation carries one; nothing else in p1's view does, least of all p2's hand card.
        assert_eq!(json_of(&p1_view).to_string().matches("\"preview\"").count(), 1);

        let theirs = must(
            s.hand(PlayerId::P2).first().map(|card| card.id.clone()),
            "p2's Equation",
        );
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P2), &theirs)), 1);
    }
}

// =============================================================================================
// #38 Quickstriker: its controller's plays this turn
// =============================================================================================

mod c38_quickstriker_previews_x_the_count_the_next_play_reads_r280 {
    use super::*;

    /// TS's `for (const face of FACES)` loop: one `it` per face.
    fn x_follows_the_turn(face: FaceKind) {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": {
                "backrow": [{ "def": QUICKSTRIKER, "radiant": radiant_of(face) }],
                "hand": [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER, STOCKPILE],
                "library": [MENACE, MENACE, MENACE, MENACE],
                "mana": 10,
            },
            "p2": { "hand": [STOCKPILE], "library": [MENACE] },
        }));
        let multiple = if face == FaceKind::Radiant { 2 } else { 1 };

        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 0);
        s.play(RAPID_REPLENISH, json!({}));
        let x1 = value_of(&s.view(PlayerId::P1).you.backrow[0]);
        assert_eq!(x1, 1);
        s.play(TEMPO_TIMMY, json!({}));
        let x2 = value_of(&s.view(PlayerId::P1).you.backrow[0]);
        assert_eq!(x2, 2);
        s.play(BIG_D_FENDER, json!({}));

        // The same X on both faces; the Radiant face deals it twice over, as one hit (R281).
        assert_eq!(hits_on(&s, PlayerId::P2), vec![x1 * multiple, x2 * multiple]);
        // It is public: the other seat sees the same X.
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.backrow[0]), 3);
    }

    #[test]
    fn r280_base_x_is_0_then_1_then_2_as_the_turn_goes_and_the_next_plays_combo_reads_it() {
        x_follows_the_turn(FaceKind::Base);
    }

    #[test]
    fn r280_radiant_x_is_0_then_1_then_2_as_the_turn_goes_and_the_next_plays_combo_reads_it() {
        x_follows_the_turn(FaceKind::Radiant);
    }

    #[test]
    fn r280_in_hand_it_previews_the_x_the_plays_so_far_give() {
        jackioh_cards::register_all();
        let mut s = scenario(
            json!({ "p1": { "hand": [RAPID_REPLENISH, QUICKSTRIKER, TEMPO_TIMMY], "library": [MENACE, MENACE] } }),
        );
        let card = s.card(QUICKSTRIKER).id.clone();
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), 0);
        s.play(RAPID_REPLENISH, json!({}));
        s.play(TEMPO_TIMMY, json!({}));
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), 2);
    }
}

// =============================================================================================
// #40 Echoes of the Forgotten: its controller's exile count
// =============================================================================================

mod c40_echoes_of_the_forgotten_previews_its_start_of_turn_damage_r280 {
    use super::*;

    fn exile_count_deals(face: FaceKind, expected: i32) {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": {
                "backrow": [{ "def": ECHOES, "radiant": radiant_of(face) }],
                "exile": [STOCKPILE, TEMPO_TIMMY, MENACE],
                "library": [MENACE, TEMPO_TIMMY],
            },
            "p2": { "exile": [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE] },
        }));

        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), expected);
        // A public Field Spell: the other seat sees the same number, and the opponent's pile of 5 is
        // not counted (R72).
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.backrow[0]), expected);

        s.start_turn();

        assert_eq!(hits_on(&s, PlayerId::P2), vec![expected]);
    }

    #[test]
    fn r280_base_three_cards_in_your_exile_preview_3_and_the_start_of_your_turn_deals_it() {
        exile_count_deals(FaceKind::Base, 3);
    }

    #[test]
    fn r280_radiant_three_cards_in_your_exile_preview_6_and_the_start_of_your_turn_deals_it() {
        exile_count_deals(FaceKind::Radiant, 6);
    }
}

// =============================================================================================
// #70 Spiteful Stab: its controller's hero health and exile count
// =============================================================================================

mod c70_spiteful_stab_previews_its_damage_r280 {
    use super::*;

    fn stab(face: FaceKind, mut opts: Value) -> Scenario {
        opts["hand"] = json!([{ "def": SPITEFUL_STAB, "radiant": radiant_of(face) }, RAPID_REPLENISH]);
        scenario(json!({
            "p1": opts,
            "p2": { "field": [MENACE], "exile": [STOCKPILE, STOCKPILE, STOCKPILE] },
        }))
    }

    /// TS's `CASES` loop: one `it` per `[face, health, exiled, damage]`.
    fn previews_and_deals(face: FaceKind, health: i32, exiled: usize, damage: i32) {
        jackioh_cards::register_all();
        let mut s = stab(
            face,
            json!({ "health": health, "exile": vec![STOCKPILE; exiled] }),
        );
        let card = s.card(SPITEFUL_STAB).id.clone();
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), damage);

        s.play(SPITEFUL_STAB, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![damage]);
    }

    #[test]
    fn r280_base_at_30_health_with_0_exiled_previews_2_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Base, 30, 0, 2);
    }

    #[test]
    fn r280_base_at_23_health_with_2_exiled_previews_5_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Base, 23, 2, 5); // 2 + floor(7/5) + 2
    }

    #[test]
    fn r280_base_at_35_health_with_1_exiled_previews_3_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Base, 35, 1, 3); // R72: missing counts from 30, floored at 0
    }

    #[test]
    fn r280_radiant_at_30_health_with_0_exiled_previews_4_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Radiant, 30, 0, 4);
    }

    #[test]
    fn r280_radiant_at_23_health_with_2_exiled_previews_10_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Radiant, 23, 2, 10); // 4 + floor(7/3) + 2 × 2
    }

    #[test]
    fn r280_radiant_at_21_health_with_1_exiled_previews_9_and_deals_exactly_that() {
        previews_and_deals(FaceKind::Radiant, 21, 1, 9); // 4 + 3 + 2
    }
}

// =============================================================================================
// #91 Fed Fauci: its own Plague Counters
// =============================================================================================

mod c91_fed_fauci_previews_the_mana_its_next_start_of_turn_gives_r280 {
    use super::*;

    fn counters_give_mana(face: FaceKind, expected: i32) {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": {
                "field": [{ "def": FED_FAUCI, "radiant": radiant_of(face), "counters": { "plague": 2 } }],
                "hand": [STOCKPILE],
                "library": [MENACE, MENACE],
            },
            "p2": { "hand": [STOCKPILE], "library": [MENACE, MENACE] },
        }));

        assert_eq!(value_of(&s.view(PlayerId::P1).you.units[0]), expected);
        // A unit is public: the other seat sees the same number.
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.units[0]), expected);

        s.end_turn(); // p2 ends; p1's turn starts: the refresh, then the hook (R62)

        s.expect_mana(PlayerId::P1, 4 + expected);
    }

    #[test]
    fn r280_base_two_plague_counters_preview_2_and_its_controllers_next_turn_starts_with_that_much_more() {
        counters_give_mana(FaceKind::Base, 2);
    }

    #[test]
    fn r280_radiant_two_plague_counters_preview_4_and_its_controllers_next_turn_starts_with_that_much_more() {
        counters_give_mana(FaceKind::Radiant, 4);
    }

    #[test]
    fn r280_in_hand_it_holds_no_tokens_r78_so_it_previews_0() {
        jackioh_cards::register_all();
        let s = scenario(json!({ "p1": { "hand": [FED_FAUCI, RAPID_REPLENISH] } }));
        assert_eq!(
            value_of(&hand_card(&s.view(PlayerId::P1), &s.card(FED_FAUCI).id)),
            0
        );
    }
}

// =============================================================================================
// T-AI-6 Datacenter Fire: the Field Spells its sweep dooms, times its face's number
// =============================================================================================

mod t_ai_6_datacenter_fire_previews_the_damage_each_hero_would_take_r280 {
    use super::*;

    const TWINSPELL: &str = "core-079"; // (2) Field Spell
    const FARM: &str = "core-058"; // (2) Field Spell
    const HEROIC_POWER: &str = "core-098"; // Field Spell, Indestructible

    fn up(def_id: &str, lane: i32) -> Value {
        json!({ "def": def_id, "faceUp": true, "lane": lane })
    }

    /// TS `LABELS[face]`.
    fn label(face: FaceKind) -> &'static str {
        match face {
            FaceKind::Base => "Deal 1 damage to each hero for each one destroyed",
            FaceKind::Radiant => "Deal 2 damage to the enemy hero for each one destroyed",
        }
    }

    fn board(face: FaceKind) -> Scenario {
        scenario(json!({
            // Not a Twinspell of p1's: it would spend itself on this Spell as it is played, before the sweep.
            "p1": { "hand": [{ "def": DATACENTER_FIRE, "radiant": radiant_of(face) }, RAPID_REPLENISH], "backrow": [up(FARM, 1)] },
            "p2": { "backrow": [up(TWINSPELL, 1), up(FARM, 2), up(HEROIC_POWER, 3)] },
        }))
    }

    /// TS's `for (const face of FACES)` loop: one `it` per face.
    fn previews_each_hit(face: FaceKind) {
        jackioh_cards::register_all();
        let value = if face == FaceKind::Base { 3 } else { 4 };
        let mut s = board(face);
        let list = shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(DATACENTER_FIRE).id));
        assert_eq!(json_of(&list), json!([{ "label": label(face), "value": value }]));
        assert!(text_of(DATACENTER_FIRE, face).contains(label(face)));

        s.play(DATACENTER_FIRE, json!({}));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![value]);
        assert_eq!(
            hits_on(&s, PlayerId::P1),
            if face == FaceKind::Base {
                vec![value]
            } else {
                vec![]
            }
        );
    }

    #[test]
    fn r280_base_deal_1_damage_to_each_hero_for_each_one_destroyed_previews_3_and_each_hit_then_deals_exactly_that()
     {
        previews_each_hit(FaceKind::Base);
    }

    #[test]
    fn r280_radiant_deal_2_damage_to_the_enemy_hero_for_each_one_destroyed_previews_4_and_each_hit_then_deals_exactly_that()
     {
        previews_each_hit(FaceKind::Radiant);
    }

    #[test]
    fn r280_r46_with_no_field_spell_its_sweep_would_destroy_it_previews_0_and_deals_nothing() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [DATACENTER_FIRE, RAPID_REPLENISH] },
            "p2": { "backrow": [up(HEROIC_POWER, 1)] },
        }));
        assert_eq!(
            value_of(&hand_card(&s.view(PlayerId::P1), &s.card(DATACENTER_FIRE).id)),
            0
        );
        s.play(DATACENTER_FIRE, json!({}));
        assert_eq!(hits_on(&s, PlayerId::P2), Vec::<i32>::new());
    }

    #[test]
    fn r280_its_hook_is_a_pure_read_of_public_facts_no_write_and_no_library_hand_or_state_active() {
        jackioh_cards::register_all();
        for face in FACES {
            for active in PLAYER_IDS {
                let s = scenario(json!({
                    "active": active,
                    "p1": {
                        "hand": [{ "def": DATACENTER_FIRE, "radiant": radiant_of(face) }],
                        "library": [MENACE],
                        "backrow": [up(TWINSPELL, 1)],
                    },
                    "p2": { "hand": [STOCKPILE], "library": [MENACE], "backrow": [up(FARM, 1)] },
                }));
                let card = must(held(&s, PlayerId::P1, DATACENTER_FIRE), "p1's Datacenter Fire");
                let hook = must(preview_hook(DATACENTER_FIRE, face), "the hook");
                let (state, self_) = guarded(s.state(), &card.id);
                let answer = hook(ConditionContext {
                    state: &state,
                    self_: &self_,
                    controller: PlayerId::P1,
                    radiant: card.radiant,
                    zone: ConditionZone::Hand,
                    your_turn: active == PlayerId::P1,
                });
                assert_eq!(
                    shown_of(&hand_card(&s.view(PlayerId::P1), &card.id)),
                    Some(answer)
                );
            }
        }
    }
}

// =============================================================================================
// A fused card's list (R102)
// =============================================================================================

mod r280_a_fused_core_card_lists_its_ingredients_previews_in_order {
    use super::*;

    #[test]
    fn r280_a_crafted_equation_stab_previews_both_numbers_and_its_cry_deals_their_sum() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [MATH_EQUATION, SPITEFUL_STAB, RAPID_REPLENISH], "health": 20, "exile": [STOCKPILE], "mana": 10 },
            "p2": { "field": [MENACE] },
        }));
        let ingredients = vec![s.card(MATH_EQUATION).clone(), s.card(SPITEFUL_STAB).clone()];
        let fused = must(
            fuse_on(
                &mut s,
                FuseArgs {
                    ingredients,
                    to_hand: Some(PlayerId::P1),
                    ..FuseArgs::default()
                },
            ),
            "the crafted card",
        );

        let list = must(
            shown_of(&hand_card(&s.view(PlayerId::P1), &fused.id)),
            "the fused card's preview",
        );
        // #31's half reads the fused card's own count of plays, none yet (R429): its 1st play, Fib(0 + 1 +
        // 1) = 1. #70's half reads 20 health (missing 10: +2) and one exiled card: 2 + 2 + 1 = 5.
        assert_eq!(
            json_of(&list),
            json!([
                { "label": "Fib(times played + 1)", "value": 1 },
                { "label": text_of(SPITEFUL_STAB, FaceKind::Base), "value": 5 },
            ])
        );
        let text = must(
            s.state()
                .transient_defs
                .get(&fused.def_id)
                .map(|def| def.base.text.clone()),
            "the fused def",
        );
        for entry in &list {
            assert!(text.contains(&entry.label));
        }

        s.play(
            fused.id.as_str(),
            json!({ "targets": [at_enemy_hero(), at_enemy_hero()] }),
        );

        assert_eq!(
            hits_on(&s, PlayerId::P2),
            list.iter().map(|entry| entry.value).collect::<Vec<i32>>()
        );
    }
}

// =============================================================================================
// Classic #19 Lizard's Breath: the pile or piles that count now (R280)
// =============================================================================================

mod c_19_lizards_breath_previews_the_pile_or_piles_that_would_count_now_r280 {
    use super::*;

    const X: &str = "core-008";

    fn breath(face: FaceKind, deck: usize, graveyard: usize, exile: usize) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": LIZARDS_BREATH, "radiant": radiant_of(face) }, RAPID_REPLENISH],
                "library": vec![X; deck],
                "graveyard": vec![STOCKPILE; graveyard],
                "exile": vec![STOCKPILE; exile],
            },
            "p2": { "hand": [STOCKPILE] },
        }))
    }

    fn preview(s: &Scenario) -> Option<Vec<PreviewValue>> {
        shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(LIZARDS_BREATH).id))
    }

    fn draws(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { .. }))
            .count()
    }

    fn displays(list: Option<Vec<PreviewValue>>) -> Vec<Option<String>> {
        list.unwrap_or_default()
            .into_iter()
            .map(|entry| entry.display)
            .collect()
    }

    #[test]
    fn r280_base_the_deck_largest_names_the_deck_with_its_size_and_the_play_then_draws_1_after_a_hit_of_2() {
        jackioh_cards::register_all();
        let mut s = breath(FaceKind::Base, 5, 2, 1);
        assert_eq!(
            json_of(&preview(&s)),
            json!([{ "label": "Your largest pile", "value": 5, "display": "Deck" }])
        );

        s.play(LIZARDS_BREATH, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![2]);
        assert_eq!(draws(&s), 1);
    }

    #[test]
    fn r280_base_the_exile_largest_names_the_exile_and_the_play_then_hits_for_6() {
        jackioh_cards::register_all();
        let mut s = breath(FaceKind::Base, 1, 2, 3);
        assert_eq!(
            json_of(&preview(&s)),
            json!([{ "label": "Your largest pile", "value": 3, "display": "Exile" }])
        );

        s.play(LIZARDS_BREATH, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![6]);
    }

    #[test]
    fn r280_base_a_graveyard_exile_tie_names_the_graveyard_listed_first_and_the_play_gives_2_mana() {
        jackioh_cards::register_all();
        let mut s = breath(FaceKind::Base, 0, 2, 2);
        assert_eq!(
            json_of(&preview(&s)),
            json!([{ "label": "Your largest pile", "value": 2, "display": "Graveyard" }])
        );

        s.play(LIZARDS_BREATH, json!({ "targets": [at_enemy_hero()] }));

        s.expect_mana(PlayerId::P1, 5);
    }

    #[test]
    fn r280_base_three_empty_piles_name_the_deck_at_0() {
        jackioh_cards::register_all();
        assert_eq!(
            json_of(&preview(&breath(FaceKind::Base, 0, 0, 0))),
            json!([{ "label": "Your largest pile", "value": 0, "display": "Deck" }])
        );
    }

    #[test]
    fn r280_radiant_names_the_two_largest_in_rank_order_and_the_play_then_does_both() {
        jackioh_cards::register_all();
        let mut s = breath(FaceKind::Radiant, 1, 3, 5);
        assert_eq!(
            json_of(&preview(&s)),
            json!([
                { "label": "Your two largest piles", "value": 5, "display": "Exile" },
                { "label": "Your two largest piles", "value": 3, "display": "Graveyard" },
            ])
        );

        s.play(LIZARDS_BREATH, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![8]);
        s.expect_mana(PlayerId::P1, 5);
        assert_eq!(draws(&s), 0);
    }

    #[test]
    fn r280_radiant_three_equal_piles_name_the_deck_and_the_graveyard() {
        jackioh_cards::register_all();
        assert_eq!(
            displays(preview(&breath(FaceKind::Radiant, 2, 2, 2))),
            vec![Some("Deck".to_string()), Some("Graveyard".to_string())]
        );
    }

    #[test]
    fn r280_each_label_is_an_exact_substring_of_its_faces_text_with_no_placeholder_in_it() {
        jackioh_cards::register_all();
        for face in FACES {
            let text = text_of(LIZARDS_BREATH, face);
            for entry in preview(&breath(face, 3, 2, 1)).unwrap_or_default() {
                assert!(text.contains(&entry.label));
                assert!(!entry.label.contains(['{', '}']));
            }
        }
    }

    #[test]
    fn r280_it_follows_the_piles_as_they_change_a_pile_growing_past_another_moves_the_preview() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [LIZARDS_BREATH, RAPID_REPLENISH, STOCKPILE], "library": [X, X, X], "graveyard": [STOCKPILE, STOCKPILE] },
            "p2": { "hand": [STOCKPILE] },
        }));
        assert_eq!(
            preview(&s).and_then(|list| list.first().and_then(|entry| entry.display.clone())),
            Some("Deck".to_string())
        );

        // Stockpile draws 2 (deck 3 → 1) and lands in the graveyard (2 → 3).
        s.play(STOCKPILE, json!({}));

        assert_eq!(
            json_of(&preview(&s)),
            json!([{ "label": "Your largest pile", "value": 3, "display": "Graveyard" }])
        );
    }

    #[test]
    fn r280_in_the_opponents_hand_it_is_hidden_p2s_view_carries_no_preview_of_it() {
        jackioh_cards::register_all();
        let s = breath(FaceKind::Base, 3, 1, 1);

        assert!(
            !json_of(&s.view(PlayerId::P2))
                .to_string()
                .contains("Your largest pile")
        );
    }

    #[test]
    fn r280_it_reads_only_pile_sizes_two_games_whose_decks_and_hands_differ_but_whose_sizes_match_show_the_same_preview()
     {
        jackioh_cards::register_all();
        let one = scenario(json!({
            "p1": { "hand": [LIZARDS_BREATH, STOCKPILE], "library": [X, X, X], "graveyard": [STOCKPILE], "exile": [STOCKPILE] },
            "p2": { "hand": [STOCKPILE], "library": [X] },
        }));
        let other = scenario(json!({
            "p1": {
                "hand": [LIZARDS_BREATH, MENACE],
                "library": [TEMPO_TIMMY, BIG_D_FENDER, RAPID_REPLENISH],
                "graveyard": [STOCKPILE],
                "exile": [STOCKPILE],
            },
            "p2": { "hand": [MATH_EQUATION], "library": [BIG_D_FENDER] },
        }));

        assert_eq!(preview(&other), preview(&one));
    }

    #[test]
    fn r280_the_hook_is_a_pure_read_no_write_no_state_active_and_of_a_library_or_a_hand_only_its_size() {
        jackioh_cards::register_all();
        for face in FACES {
            let s = breath(face, 3, 2, 1);
            let card = must(held(&s, PlayerId::P1, LIZARDS_BREATH), "p1's Breath");
            let hook = must(
                preview_hook(LIZARDS_BREATH, face_of(card.radiant)),
                "the Breath's hook",
            );
            // A pile that answers its length and nothing else: the sizes are public, the cards are not.
            let (copy, self_) = fenced(s.state(), &card.id, Fence::SizeOnly, Fence::SizeOnly, true);

            let answer = hook(ConditionContext {
                state: &copy,
                self_: &self_,
                controller: PlayerId::P1,
                radiant: card.radiant,
                zone: ConditionZone::Hand,
                your_turn: true,
            });

            assert_eq!(
                shown_of(&hand_card(&s.view(PlayerId::P1), &card.id)),
                Some(answer.clone())
            );
            let expected: Vec<Option<String>> = if face == FaceKind::Base {
                vec![Some("Deck".to_string())]
            } else {
                vec![Some("Deck".to_string()), Some("Graveyard".to_string())]
            };
            assert_eq!(displays(Some(answer)), expected);
        }
    }
}

mod r280_each_core_hook_is_a_pure_read_of_public_facts {
    use super::*;

    struct Placement {
        name: String,
        opts: Value,
        id: &'static str,
        zone: ConditionZone,
    }

    /// Every previewed card, both faces, in hand and — for a permanent — on the field, on either turn.
    fn placements() -> Vec<Placement> {
        let mut out = vec![];
        for id in PREVIEWED {
            for radiant in [false, true] {
                for active in PLAYER_IDS {
                    let common_p2 = json!({ "hand": [MATH_EQUATION, STOCKPILE], "library": [MENACE, TEMPO_TIMMY], "exile": [STOCKPILE] });
                    let face = if radiant { "radiant" } else { "base" };
                    out.push(Placement {
                        name: format!("{id} {face} in hand, {active} active"),
                        opts: json!({
                            "active": active,
                            "p2": common_p2,
                            "p1": { "hand": [{ "def": id, "radiant": radiant }], "library": [MENACE], "exile": [STOCKPILE], "health": 17 },
                        }),
                        id,
                        zone: ConditionZone::Hand,
                    });
                    if id == MATH_EQUATION || id == SPITEFUL_STAB {
                        continue;
                    }
                    let mut p1 = if id == FED_FAUCI {
                        json!({ "field": [{ "def": id, "radiant": radiant, "counters": { "plague": 3 } }] })
                    } else {
                        json!({ "backrow": [{ "def": id, "radiant": radiant }] })
                    };
                    p1["library"] = json!([MENACE]);
                    p1["exile"] = json!([STOCKPILE]);
                    p1["health"] = json!(17);
                    out.push(Placement {
                        name: format!("{id} {face} on the field, {active} active"),
                        opts: json!({ "active": active, "p2": common_p2, "p1": p1 }),
                        id,
                        zone: ConditionZone::Field,
                    });
                }
            }
        }
        out
    }

    #[test]
    fn r280_no_hook_writes_or_reads_a_library_a_hand_or_state_active_and_each_answers_as_the_view_shows() {
        jackioh_cards::register_all();
        for Placement { name, opts, id, zone } in placements() {
            let s = scenario(opts);
            // p1's copy: p2 holds an Equation too, and `s.card` would look at the active side first.
            let card = must(
                if zone == ConditionZone::Hand {
                    held(&s, PlayerId::P1, id)
                } else {
                    s.backrow(PlayerId::P1, 1).or(s.unit(PlayerId::P1, 1))
                },
                &format!("p1's {id}"),
            );
            let hook = must(preview_hook(id, face_of(card.radiant)), &format!("{id}'s hook"));
            let (state, self_) = guarded(s.state(), &card.id);
            let ctx = ConditionContext {
                state: &state,
                self_: &self_,
                controller: card.controller,
                radiant: card.radiant,
                zone,
                your_turn: s.state().active == card.controller,
            };

            let answer = hook(ctx);

            let view = s.view(PlayerId::P1);
            let place = if zone == ConditionZone::Hand {
                json_of(&hand_card(&view, &card.id))
            } else {
                let backrow = json_of(&view.you.backrow[0]);
                if backrow.is_null() {
                    json_of(&view.you.units[0])
                } else {
                    backrow
                }
            };
            assert_eq!(shown(&place), Some(answer), "{name}");
        }
    }

    #[test]
    fn r280_two_games_that_differ_only_in_cards_a_viewer_may_not_read_show_that_viewer_the_same_previews() {
        jackioh_cards::register_all();
        fn board(library: &[&str], p1_hand: Vec<Value>, p2_hand: Vec<Value>) -> Scenario {
            let mut p1_hand = p1_hand;
            p1_hand.push(json!(SPITEFUL_STAB));
            p1_hand.push(json!(FED_FAUCI));
            let mut p2_hand = p2_hand;
            p2_hand.push(json!(MATH_EQUATION));
            scenario(json!({
                "p1": {
                    "hand": p1_hand,
                    "backrow": [BREAD_AND_BUTTER, QUICKSTRIKER, { "def": ECHOES, "radiant": true }],
                    "field": [{ "def": FED_FAUCI, "counters": { "plague": 2 } }],
                    "library": library,
                    "exile": [STOCKPILE, STOCKPILE],
                    "health": 18,
                    "mana": 3,
                },
                "p2": {
                    "hand": p2_hand,
                    "library": library,
                    "backrow": [{ "def": BREAD_AND_BUTTER, "radiant": true }, QUICKSTRIKER],
                    "exile": [STOCKPILE],
                },
            }))
        }
        fn previews(s: &Scenario, viewer: PlayerId) -> Vec<Option<Vec<PreviewValue>>> {
            let view = s.view(viewer);
            let mut out: Vec<Option<Vec<PreviewValue>>> = own_hand(&view).iter().map(shown_of).collect();
            for side in [&view.you, &view.opponent] {
                for unit in &side.units {
                    out.push(if unit.is_none() { None } else { shown_of(unit) });
                }
                for card in &side.backrow {
                    out.push(if card.is_none() { None } else { shown_of(card) });
                }
            }
            out
        }

        let one = board(
            &[MENACE, TEMPO_TIMMY, STOCKPILE],
            vec![json!({ "def": MATH_EQUATION, "costMod": 1 })],
            vec![json!(STOCKPILE)],
        );
        // p1 may not read p2's hand or either library: change all three, and p1's view is the same.
        let for_p1 = board(
            &[BIG_D_FENDER, BIG_D_FENDER, RAPID_REPLENISH],
            vec![json!({ "def": MATH_EQUATION, "costMod": 1 })],
            vec![json!(SPITEFUL_STAB)],
        );
        assert_eq!(previews(&for_p1, PlayerId::P1), previews(&one, PlayerId::P1));
        // p2 may not read p1's hand or either library: change all three, and p2's view is the same —
        // however much p1's hidden Equation or Stab would deal.
        let for_p2 = board(
            &[BIG_D_FENDER, RAPID_REPLENISH, RAPID_REPLENISH],
            vec![json!({ "def": MATH_EQUATION, "costMod": 4, "radiant": true })],
            vec![json!(STOCKPILE)],
        );
        assert_eq!(previews(&for_p2, PlayerId::P2), previews(&one, PlayerId::P2));

        // Not vacuous: each seat sees previews, and p1's own changed: its hidden Equation is its own.
        assert!(
            previews(&one, PlayerId::P1)
                .iter()
                .filter(|list| list.is_some())
                .count()
                >= 7
        );
        assert!(
            previews(&one, PlayerId::P2)
                .iter()
                .filter(|list| list.is_some())
                .count()
                >= 5
        );
        assert_ne!(previews(&for_p2, PlayerId::P1), previews(&one, PlayerId::P1));
    }
}

// =============================================================================================
// C #1 Curse of the Forgotten Classic: the damage per card times the opponent's exile size
// =============================================================================================

mod c_1_curse_of_the_forgotten_classic_previews_n_its_one_hit_r280 {
    use super::*;

    fn curse(face: FaceKind, their_exile: usize) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": CURSE, "radiant": radiant_of(face) }, RAPID_REPLENISH], "library": [MENACE, TEMPO_TIMMY], "exile": [STOCKPILE] },
            "p2": { "hand": [STOCKPILE], "exile": vec![STOCKPILE; their_exile], "library": [MENACE] },
        }))
    }

    /// TS's first `it` inside the `for (const face of FACES)` loop.
    fn label_is_the_formula(face: FaceKind) {
        jackioh_cards::register_all();
        let s = curse(face, 2);
        let list = must(
            shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(CURSE).id)),
            "the Curse's preview",
        );
        assert_eq!(
            list.iter()
                .map(|entry| entry.label.clone())
                .collect::<Vec<String>>(),
            vec!["for each card in their exile"]
        );
        for entry in &list {
            assert!(text_of(CURSE, face).contains(&entry.label));
        }
    }

    /// TS's `for (const exiled of [0, 3])` `it`s inside the face loop.
    fn exile_previews_and_deals(face: FaceKind, exiled: usize) {
        jackioh_cards::register_all();
        let mut s = curse(face, exiled);
        let n = exiled as i32;
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &s.card(CURSE).id)), n);

        s.play(CURSE, json!({}));

        // R63: 0 is no hit at all. Your own exile is not counted.
        assert_eq!(
            hits_on(&s, PlayerId::P2),
            if exiled == 0 { vec![] } else { vec![n] }
        );
    }

    #[test]
    fn r280_base_the_label_is_the_formula_the_face_prints_an_exact_substring_of_its_text() {
        label_is_the_formula(FaceKind::Base);
    }

    #[test]
    fn r280_base_0_cards_in_their_exile_preview_0_and_the_spell_deals_exactly_that() {
        exile_previews_and_deals(FaceKind::Base, 0);
    }

    #[test]
    fn r280_base_3_cards_in_their_exile_preview_3_and_the_spell_deals_exactly_that() {
        exile_previews_and_deals(FaceKind::Base, 3);
    }

    #[test]
    fn r280_radiant_the_label_is_the_formula_the_face_prints_an_exact_substring_of_its_text() {
        label_is_the_formula(FaceKind::Radiant);
    }

    #[test]
    fn r280_radiant_0_cards_in_their_exile_preview_0_and_the_spell_deals_exactly_that() {
        exile_previews_and_deals(FaceKind::Radiant, 0);
    }

    #[test]
    fn r280_radiant_3_cards_in_their_exile_preview_3_and_the_spell_deals_exactly_that() {
        exile_previews_and_deals(FaceKind::Radiant, 3);
    }

    #[test]
    fn r280_r386_an_upgrade_of_damage_per_card_doubles_the_preview_and_the_hit() {
        jackioh_cards::register_all();
        let mut s = curse(FaceKind::Base, 3);
        let card = s.card(CURSE).id.clone();
        step_param(s.card_mut(&card), "damage", 1);
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), 6);
        s.play(CURSE, json!({}));
        assert_eq!(hits_on(&s, PlayerId::P2), vec![6]);
    }

    #[test]
    fn r280_in_the_opponents_hand_it_is_hidden_p2s_view_carries_no_preview_of_it() {
        jackioh_cards::register_all();
        let s = curse(FaceKind::Base, 3);
        assert_eq!(s.view(PlayerId::P2).opponent.hand, HandView::Count { count: 2 });
    }

    #[test]
    fn r280_the_hook_is_a_pure_read_of_public_facts_no_library_no_hand_no_state_active() {
        jackioh_cards::register_all();
        for face in FACES {
            let s = curse(face, 4);
            let card = must(held(&s, PlayerId::P1, CURSE), "p1's Curse");
            let hook = must(preview_hook(CURSE, face_of(card.radiant)), "the Curse's hook");
            let (state, self_) = guarded(s.state(), &card.id);
            let answer = hook(ConditionContext {
                state: &state,
                self_: &self_,
                controller: PlayerId::P1,
                radiant: card.radiant,
                zone: ConditionZone::Hand,
                your_turn: true,
            });
            assert_eq!(
                shown_of(&hand_card(&s.view(PlayerId::P1), &card.id)),
                Some(answer.clone())
            );
            assert_eq!(
                json_of(&answer),
                json!([{ "label": "for each card in their exile", "value": 4 }])
            );
        }
    }
}

// =============================================================================================
// C #43 Plague Nuke: the mana it would give, the Plague Counters on the Units on the field
// =============================================================================================

mod c_43_plague_nuke_previews_the_mana_it_would_give_now_r280 {
    use super::*;

    const LABEL: &str = "for each Plague Counter that was on them";
    const VANILLA: &str = "core-008";

    fn nuke(face: FaceKind, mine: i32, theirs: i32) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": PLAGUE_NUKE, "radiant": radiant_of(face) }, RAPID_REPLENISH],
                "field": [{ "def": VANILLA, "counters": { "plague": mine } }],
            },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": MENACE, "counters": { "plague": theirs } }, TEMPO_TIMMY] },
        }))
    }

    fn gained(s: &Scenario) -> i32 {
        // The Spell paid (4) of 4; read off p1's own view (§10.8).
        s.view(PlayerId::P1).you.mana.current
    }

    /// TS's first `it` inside the `for (const face of FACES)` loop.
    fn label_is_the_formulas_words(face: FaceKind) {
        jackioh_cards::register_all();
        let s = nuke(face, 1, 1);
        let list = must(
            shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(PLAGUE_NUKE).id)),
            "the Nuke's preview",
        );
        assert_eq!(
            list.iter()
                .map(|entry| entry.label.clone())
                .collect::<Vec<String>>(),
            vec![LABEL]
        );
        assert!(text_of(PLAGUE_NUKE, face).contains(LABEL));
        assert!(!LABEL.contains(['{', '}']));
    }

    /// TS's second `it` inside the face loop.
    fn tokens_preview_and_give(face: FaceKind) {
        jackioh_cards::register_all();
        let mut s = nuke(face, 2, 3);
        assert_eq!(
            value_of(&hand_card(&s.view(PlayerId::P1), &s.card(PLAGUE_NUKE).id)),
            5
        );

        s.play(PLAGUE_NUKE, json!({}));

        assert_eq!(gained(&s), 5);
    }

    #[test]
    fn r280_base_the_label_is_the_formulas_words_an_exact_substring_of_its_text_with_no_placeholder() {
        label_is_the_formulas_words(FaceKind::Base);
    }

    #[test]
    fn r280_base_the_tokens_on_every_unit_on_both_sides_preview_5_and_the_spell_then_gives_exactly_5() {
        tokens_preview_and_give(FaceKind::Base);
    }

    #[test]
    fn r280_radiant_the_label_is_the_formulas_words_an_exact_substring_of_its_text_with_no_placeholder() {
        label_is_the_formulas_words(FaceKind::Radiant);
    }

    #[test]
    fn r280_radiant_the_tokens_on_every_unit_on_both_sides_preview_5_and_the_spell_then_gives_exactly_5() {
        tokens_preview_and_give(FaceKind::Radiant);
    }

    #[test]
    fn r280_no_tokens_on_the_board_preview_0_and_the_spell_gives_nothing() {
        jackioh_cards::register_all();
        let mut s = nuke(FaceKind::Base, 0, 0);
        assert_eq!(
            value_of(&hand_card(&s.view(PlayerId::P1), &s.card(PLAGUE_NUKE).id)),
            0
        );

        s.play(PLAGUE_NUKE, json!({}));

        assert_eq!(gained(&s), 0);
    }

    #[test]
    fn r280_r386_an_upgrade_of_mana_per_token_doubles_the_preview_and_the_gain() {
        jackioh_cards::register_all();
        let mut s = nuke(FaceKind::Base, 1, 2);
        let card = s.card(PLAGUE_NUKE).id.clone();
        step_param(s.card_mut(&card), "mana", 1);
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), 6);

        s.play(PLAGUE_NUKE, json!({}));

        assert_eq!(gained(&s), 6);
    }

    #[test]
    fn r280_in_the_opponents_hand_it_is_hidden_p2s_view_carries_no_preview_of_it() {
        jackioh_cards::register_all();
        let s = nuke(FaceKind::Base, 1, 1);
        assert!(!json_of(&s.view(PlayerId::P2)).to_string().contains(LABEL));
    }

    #[test]
    fn r280_the_hook_is_a_pure_read_of_public_facts_no_library_no_hand_no_state_active() {
        jackioh_cards::register_all();
        for face in FACES {
            let s = nuke(face, 2, 2);
            let card = must(held(&s, PlayerId::P1, PLAGUE_NUKE), "p1's Nuke");
            let hook = must(
                preview_hook(PLAGUE_NUKE, face_of(card.radiant)),
                "the Nuke's hook",
            );
            let (state, self_) = guarded(s.state(), &card.id);
            let answer = hook(ConditionContext {
                state: &state,
                self_: &self_,
                controller: PlayerId::P1,
                radiant: card.radiant,
                zone: ConditionZone::Hand,
                your_turn: true,
            });
            assert_eq!(
                shown_of(&hand_card(&s.view(PlayerId::P1), &card.id)),
                Some(answer.clone())
            );
            assert_eq!(json_of(&answer), json!([{ "label": LABEL, "value": 4 }]));
        }
    }
}

mod r280_r583_c_44_and_45_preview_the_permanents_they_would_exile {
    use super::*;

    /// p1's Radiant Audit in hand over a board of both sides, an enemy face-down card among it.
    fn board(audit: &str, enemy_trap: &str, active: PlayerId) -> Scenario {
        scenario(json!({
            "active": active,
            "p1": { "hand": [{ "def": audit, "radiant": true }, STOCKPILE], "field": ["core-008", "core-022"], "library": [MENACE] },
            "p2": {
                "hand": [STOCKPILE],
                "field": ["core-008", "core-022"],
                "backrow": [{ "def": enemy_trap, "faceUp": false }],
                "library": [MENACE],
            },
        }))
    }

    #[test]
    fn r280_r583_the_hook_reads_no_library_hand_or_state_active_and_answers_as_the_view_shows_on_either_turn()
    {
        jackioh_cards::register_all();
        for audit in AUDITS {
            for active in PLAYER_IDS {
                let s = board(audit, "core-071", active);
                let card = must(held(&s, PlayerId::P1, audit), audit);
                let (state, self_) = guarded(s.state(), &card.id);
                let hook = must(preview_hook(audit, FaceKind::Radiant), &format!("{audit}'s hook"));
                let answer = hook(ConditionContext {
                    state: &state,
                    self_: &self_,
                    controller: PlayerId::P1,
                    radiant: true,
                    zone: ConditionZone::Hand,
                    your_turn: active == PlayerId::P1,
                });
                assert_eq!(
                    shown_of(&hand_card(&s.view(PlayerId::P1), &card.id)),
                    Some(answer.clone()),
                    "{audit} {active}"
                );
                assert_eq!(
                    answer
                        .iter()
                        .map(|entry| entry.label.clone())
                        .collect::<Vec<String>>(),
                    vec!["all permanents", "only your opponent's"]
                );
                // Each label sits in the Radiant text, and each value counts its ids.
                for entry in &answer {
                    assert!(text_of(audit, FaceKind::Radiant).contains(&entry.label));
                    assert_eq!(
                        entry.ids.as_ref().map(|ids| ids.len()),
                        Some(entry.value as usize)
                    );
                }
            }
        }
    }

    #[test]
    fn r177_r583_two_boards_that_differ_only_in_an_enemy_face_down_card_show_the_same_preview() {
        jackioh_cards::register_all();
        for audit in AUDITS {
            // Intern Stimmy's loc is below both Audits', Bear Honeypot's above: one of them is always a target.
            let low = board(audit, "core-071", PlayerId::P1);
            let high = board(audit, "core-060", PlayerId::P1);
            let of = |s: &Scenario| -> Option<Vec<PreviewValue>> {
                let first = must(s.hand(PlayerId::P1).first().map(|card| card.id.clone()), audit);
                shown_of(&hand_card(&s.view(PlayerId::P1), &first))
            };
            assert!(of(&low).is_some());
            assert_eq!(of(&low), of(&high));
        }
    }
}

// =============================================================================================
// C+ #74 Twice Forward One Step Backwards: the opponent's plays since it was set, its controller's alone
// =============================================================================================

mod c_74_twice_forward_previews_the_plays_it_has_counted_to_its_controller_alone_r280_r33 {
    use super::*;

    const LABEL: &str = "your opponent plays";
    const TIMMY: &str = "core-011"; // (1) Unit

    fn counted(face: FaceKind) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": {
                "hand": [STOCKPILE],
                "backrow": [{ "def": TWICE_FORWARD, "radiant": radiant_of(face), "faceUp": false }],
                "library": [MENACE],
            },
            "p2": { "hand": [TIMMY, STOCKPILE], "library": [MENACE, MENACE] },
        }))
    }

    /// TS `const at = () => shown(s.view("p1").you.backrow[0])`.
    fn at(s: &Scenario) -> Option<Vec<PreviewValue>> {
        shown_of(&s.view(PlayerId::P1).you.backrow[0])
    }

    /// TS's `for (const face of FACES)` loop: one `it` per face.
    fn counts_the_plays(face: FaceKind) {
        jackioh_cards::register_all();
        let mut s = counted(face);
        let trap = s.card(TWICE_FORWARD).id.clone();
        assert!(text_of(TWICE_FORWARD, face).contains(LABEL));
        assert_eq!(json_of(&at(&s)), json!([{ "label": LABEL, "value": 0 }]));
        s.play(TIMMY, json!({}));
        assert_eq!(json_of(&at(&s)), json!([{ "label": LABEL, "value": 1 }]));
        assert_ne!(s.card(trap.as_str()).face_up, Some(true));
    }

    #[test]
    fn r280_base_the_label_sits_in_the_faces_text_and_the_value_is_the_plays_counted_so_far() {
        counts_the_plays(FaceKind::Base);
    }

    #[test]
    fn r280_radiant_the_label_sits_in_the_faces_text_and_the_value_is_the_plays_counted_so_far() {
        counts_the_plays(FaceKind::Radiant);
    }

    #[test]
    fn r33_r177_face_down_the_opponents_view_carries_neither_the_preview_nor_the_count() {
        jackioh_cards::register_all();
        let mut s = counted(FaceKind::Base);
        s.play(TIMMY, json!({}));
        let theirs = json_of(&s.view(PlayerId::P2).opponent.backrow[0]);
        assert_eq!(theirs.get("faceDown"), Some(&json!(true)));
        assert!(!theirs.to_string().contains(LABEL));
        assert!(!json_of(&s.view(PlayerId::P2)).to_string().contains(LABEL));
    }

    #[test]
    fn r280_its_hook_is_a_pure_read_no_write_and_no_library_hand_or_state_active() {
        jackioh_cards::register_all();
        for face in FACES {
            let mut s = counted(face);
            s.play(TIMMY, json!({}));
            let card = s.card(TWICE_FORWARD).clone();
            let hook = must(preview_hook(TWICE_FORWARD, face), "the hook");
            let (state, self_) = guarded(s.state(), &card.id);
            let answer = hook(ConditionContext {
                state: &state,
                self_: &self_,
                controller: PlayerId::P1,
                radiant: card.radiant,
                zone: ConditionZone::Field,
                your_turn: false,
            });
            assert_eq!(shown_of(&s.view(PlayerId::P1).you.backrow[0]), Some(answer));
            let in_hand = hook(ConditionContext {
                state: &state,
                self_: &self_,
                controller: PlayerId::P1,
                radiant: card.radiant,
                zone: ConditionZone::Hand,
                your_turn: true,
            });
            assert_eq!(in_hand, Vec::<PreviewValue>::new());
        }
    }
}

// =============================================================================================
// C #59 Plague Doctor: N, every Plague Counter on the field (and the Radiant face's own placement)
// =============================================================================================

mod c_59_plague_doctor_previews_n_the_hit_its_cry_deals_r280 {
    use super::*;

    const DOCTOR_LABEL: &str = "the number of Plague Counters on the field";
    const PAWN: &str = "core-096";

    /// Tokens on both sides, a face-down trap's included: 2 + 1 + 3 = 6 on the field.
    fn plagued(face: FaceKind, active: PlayerId) -> Scenario {
        scenario(json!({
            "active": active,
            "p1": {
                "hand": [{ "def": PLAGUE_DOCTOR, "radiant": radiant_of(face) }, RAPID_REPLENISH],
                "field": [{ "def": TEMPO_TIMMY, "counters": { "plague": 2 } }],
                "library": [MENACE],
            },
            "p2": {
                "hand": [STOCKPILE],
                "field": [{ "def": MENACE, "counters": { "plague": 1 } }],
                "backrow": [{ "def": PAWN, "faceUp": false, "counters": { "plague": 3 } }],
                "health": 30,
            },
        }))
    }

    #[test]
    fn r280_the_label_is_the_phrase_both_faces_print_with_no_placeholder_in_it() {
        jackioh_cards::register_all();
        for face in FACES {
            let s = plagued(face, PlayerId::P1);
            let list = must(
                shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(PLAGUE_DOCTOR).id)),
                &format!("{face} preview"),
            );
            assert_eq!(
                list.iter()
                    .map(|entry| entry.label.clone())
                    .collect::<Vec<String>>(),
                vec![DOCTOR_LABEL]
            );
            assert!(text_of(PLAGUE_DOCTOR, face).contains(DOCTOR_LABEL));
            assert!(!DOCTOR_LABEL.contains(['{', '}']));
        }
    }

    /// TS's `for (const [face, expected] of …)` loop: one `it` per face.
    fn in_hand_previews_the_hit(face: FaceKind, expected: i32) {
        jackioh_cards::register_all();
        let mut s = plagued(face, PlayerId::P1);
        let value = value_of(&hand_card(&s.view(PlayerId::P1), &s.card(PLAGUE_DOCTOR).id));
        assert_eq!(value, expected);

        s.play(PLAGUE_DOCTOR, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![value]);
    }

    #[test]
    fn r280_base_in_hand_it_previews_6_and_its_cry_then_deals_exactly_that() {
        in_hand_previews_the_hit(FaceKind::Base, 6);
    }

    #[test]
    fn r280_radiant_in_hand_it_previews_8_and_its_cry_then_deals_exactly_that() {
        in_hand_previews_the_hit(FaceKind::Radiant, 8);
    }

    #[test]
    fn r280_radiant_on_the_field_the_preview_counts_its_own_tokens_plus_the_2_another_cry_would_place() {
        jackioh_cards::register_all();
        let mut s = plagued(FaceKind::Radiant, PlayerId::P1);
        s.play(PLAGUE_DOCTOR, json!({ "targets": [at_enemy_hero()] }));
        // 6 on the field, 2 more on the Doctor: 8 tokens now, and a Cry now would add 2 more.
        let lane = match &s.card(PLAGUE_DOCTOR).zone {
            Zone::Field { lane, .. } => Some(*lane),
            _ => None,
        };
        let lane = must(lane, "the Doctor's lane");
        let at = usize::try_from(lane - 1).expect("a lane is 1-based");
        assert_eq!(value_of(&s.view(PlayerId::P1).you.units[at]), 10);
        // A unit is public: the other seat sees the same number.
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.units[at]), 10);
    }

    #[test]
    fn r280_base_on_the_field_it_previews_the_tokens_there_now_for_both_seats() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "p1": { "field": [{ "def": PLAGUE_DOCTOR, "counters": { "plague": 1 } }], "hand": [RAPID_REPLENISH] },
            "p2": { "field": [{ "def": MENACE, "counters": { "plague": 2 } }], "hand": [STOCKPILE] },
        }));
        assert_eq!(value_of(&s.view(PlayerId::P1).you.units[0]), 3);
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.units[0]), 3);
    }

    #[test]
    fn r280_an_empty_field_previews_0_on_the_base_face_and_the_cry_then_deals_nothing_r63() {
        jackioh_cards::register_all();
        let mut s = scenario(
            json!({ "p1": { "hand": [PLAGUE_DOCTOR, RAPID_REPLENISH] }, "p2": { "hand": [STOCKPILE] } }),
        );
        assert_eq!(
            value_of(&hand_card(&s.view(PlayerId::P1), &s.card(PLAGUE_DOCTOR).id)),
            0
        );

        s.play(PLAGUE_DOCTOR, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), Vec::<i32>::new());
    }

    #[test]
    fn r280_r386_radiant_an_upgrade_of_its_tokens_moves_the_preview_with_the_hit() {
        jackioh_cards::register_all();
        let mut s = plagued(FaceKind::Radiant, PlayerId::P1);
        let card = s.card(PLAGUE_DOCTOR).id.clone();
        step_param(s.card_mut(&card), "tokens", 1);
        let value = value_of(&hand_card(&s.view(PlayerId::P1), &card));
        assert_eq!(value, 9);

        s.play(PLAGUE_DOCTOR, json!({ "targets": [at_enemy_hero()] }));

        assert_eq!(hits_on(&s, PlayerId::P2), vec![9]);
    }

    #[test]
    fn r280_the_hook_writes_nothing_and_reads_no_library_no_hand_and_not_state_active() {
        jackioh_cards::register_all();
        for face in FACES {
            for active in PLAYER_IDS {
                let s = plagued(face, active);
                let card = s.card(PLAGUE_DOCTOR).clone();
                let hook = must(preview_hook(PLAGUE_DOCTOR, face), "the Doctor's hook");
                let (state, self_) = guarded(s.state(), &card.id);
                let answer = hook(ConditionContext {
                    state: &state,
                    self_: &self_,
                    controller: PlayerId::P1,
                    radiant: card.radiant,
                    zone: ConditionZone::Hand,
                    your_turn: active == PlayerId::P1,
                });
                assert_eq!(
                    shown_of(&hand_card(&s.view(PlayerId::P1), &card.id)),
                    Some(answer)
                );
            }
        }
    }

    #[test]
    fn r280_the_opponents_doctor_in_hand_carries_no_preview_for_you_10_8() {
        jackioh_cards::register_all();
        let s = plagued(FaceKind::Base, PlayerId::P1);
        let theirs = s.view(PlayerId::P2).opponent.hand;
        assert!(!json_of(&theirs).to_string().contains(DOCTOR_LABEL));
    }
}

// =============================================================================================
// C #88 Siphon Squad: X, twice the Units its controller's opponent controls (base face only)
// =============================================================================================

mod c_88_siphon_squad_previews_x_the_attack_its_aura_takes_off_each_enemy_unit_r280 {
    use super::*;

    const SIPHON_LABEL: &str = "−X Attack";

    fn siphon_board(face_up: Option<bool>, radiant: bool, enemies: i32) -> Scenario {
        let enemies: Vec<Value> = (0..enemies)
            .map(|at| json!({ "def": MENACE, "lane": at + 1 }))
            .collect();
        let mut entry = json!({ "def": SIPHON_SQUAD, "radiant": radiant });
        if let Some(face_up) = face_up {
            entry["faceUp"] = json!(face_up);
        }
        scenario(json!({
            "p1": { "hand": [RAPID_REPLENISH], "backrow": [entry] },
            "p2": { "hand": [STOCKPILE], "field": enemies },
        }))
    }

    #[test]
    fn r280_the_label_is_in_the_base_faces_text_with_no_placeholder_in_it() {
        jackioh_cards::register_all();
        let s = siphon_board(Some(false), false, 2);
        let list = must(
            shown_of(&s.view(PlayerId::P1).you.backrow[0]),
            "the controller's preview",
        );
        assert_eq!(
            list.iter()
                .map(|entry| entry.label.clone())
                .collect::<Vec<String>>(),
            vec![SIPHON_LABEL]
        );
        assert!(text_of(SIPHON_SQUAD, FaceKind::Base).contains(SIPHON_LABEL));
        assert!(!SIPHON_LABEL.contains(['{', '}']));
    }

    #[test]
    fn r280_face_down_its_controller_sees_x_and_it_is_what_the_aura_takes_off_each_enemy_unit() {
        jackioh_cards::register_all();
        let s = siphon_board(Some(false), false, 2);
        let value = value_of(&s.view(PlayerId::P1).you.backrow[0]);

        assert_eq!(value, 4);
        let menace = must(s.unit(PlayerId::P2, 1).map(|card| card.id.clone()), "p2's Menace");
        assert_eq!(s.stats(menace.as_str()).attack, 9 - value);
    }

    #[test]
    fn r280_10_8_face_down_the_other_players_view_of_it_carries_no_preview() {
        jackioh_cards::register_all();
        let s = siphon_board(Some(false), false, 2);
        let theirs = json_of(&s.view(PlayerId::P2).opponent.backrow[0]);

        assert_eq!(theirs.get("faceDown"), Some(&json!(true)));
        assert!(!theirs.to_string().contains(SIPHON_LABEL));
        assert!(!json_of(&s.view(PlayerId::P2)).to_string().contains("preview"));
    }

    #[test]
    fn r280_face_up_both_players_see_x() {
        jackioh_cards::register_all();
        let s = siphon_board(Some(true), false, 3);

        assert_eq!(value_of(&s.view(PlayerId::P1).you.backrow[0]), 6);
        assert_eq!(value_of(&s.view(PlayerId::P2).opponent.backrow[0]), 6);
    }

    #[test]
    fn r280_in_hand_it_previews_the_x_it_would_take_now() {
        jackioh_cards::register_all();
        let s = scenario(
            json!({ "p1": { "hand": [SIPHON_SQUAD, RAPID_REPLENISH] }, "p2": { "field": [MENACE], "hand": [STOCKPILE] } }),
        );
        assert_eq!(
            value_of(&hand_card(&s.view(PlayerId::P1), &s.card(SIPHON_SQUAD).id)),
            2
        );
    }

    #[test]
    fn r280_r386_an_upgrade_of_its_multiplier_moves_the_preview_with_the_aura() {
        jackioh_cards::register_all();
        let mut s = siphon_board(Some(false), false, 2);
        let siphon = must(
            s.backrow(PlayerId::P1, 1).map(|card| card.id.clone()),
            "the Siphon",
        );
        step_param(s.card_mut(&siphon), "multiplier", 1);

        let value = value_of(&s.view(PlayerId::P1).you.backrow[0]);
        assert_eq!(value, 6);
        let menace = must(s.unit(PlayerId::P2, 1).map(|card| card.id.clone()), "p2's Menace");
        assert_eq!(s.stats(menace.as_str()).attack, 9 - value);
    }

    #[test]
    fn r280_the_radiant_face_has_no_x_and_no_preview() {
        jackioh_cards::register_all();
        let s = siphon_board(Some(true), true, 2);
        assert_eq!(shown_of(&s.view(PlayerId::P1).you.backrow[0]), None);
        assert_eq!(shown_of(&s.view(PlayerId::P2).opponent.backrow[0]), None);
    }

    #[test]
    fn r280_the_hook_writes_nothing_and_reads_no_library_no_hand_and_not_state_active() {
        jackioh_cards::register_all();
        for active in PLAYER_IDS {
            let s = scenario(json!({
                "active": active,
                "p1": { "hand": [RAPID_REPLENISH], "backrow": [{ "def": SIPHON_SQUAD, "faceUp": false }], "library": [MENACE] },
                "p2": { "hand": [STOCKPILE], "field": [MENACE], "library": [MENACE] },
            }));
            let card = must(s.backrow(PlayerId::P1, 1), "the Siphon");
            let hook = must(preview_hook(SIPHON_SQUAD, FaceKind::Base), "its hook");
            let (state, self_) = guarded(s.state(), &card.id);
            let answer = hook(ConditionContext {
                state: &state,
                self_: &self_,
                controller: PlayerId::P1,
                radiant: false,
                zone: ConditionZone::Field,
                your_turn: active == PlayerId::P1,
            });
            assert_eq!(shown_of(&s.view(PlayerId::P1).you.backrow[0]), Some(answer));
        }
    }
}

// =============================================================================================
// C #46 Divine Favor: the draws it asks for now (SPEC §8.6 row 46)
// =============================================================================================

mod c_46_divine_favor_previews_how_many_cards_it_would_draw_now_r280 {
    use super::*;

    // "Draw until you have {multiplier}× as many cards in hand as your opponent": the preview is the
    // draws it asks for, the opponent's hand times the multiplier less yours — without this card in
    // hand, which will have left it when it resolves. It reads both hands' sizes, which are public
    // (§10.8), and nothing of what is in them.
    const LABEL: &str = "Draw";

    fn drawn_by_p1(events: &[GameEvent]) -> usize {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    GameEvent::Drawn {
                        player: PlayerId::P1,
                        ..
                    }
                )
            })
            .count()
    }

    #[test]
    fn r280_the_label_is_printed_on_both_faces() {
        jackioh_cards::register_all();
        for face in FACES {
            assert!(text_of(DIVINE_FAVOR, face).contains(LABEL));
        }
    }

    /// TS's `for (const [face, opponentHand, yours, expected] of …)` loop: one `it` per row.
    fn previews_and_draws(face: FaceKind, opponent_hand: usize, yours: usize, expected: i32) {
        jackioh_cards::register_all();
        let mut hand = vec![json!({ "def": DIVINE_FAVOR, "radiant": radiant_of(face) })];
        hand.extend((0..yours).map(|_| json!(RAPID_REPLENISH)));
        let mut s = scenario(json!({
            "p1": { "hand": hand, "library": vec![MENACE; 8] },
            "p2": { "hand": vec![STOCKPILE; opponent_hand] },
        }));
        let card = s.card(DIVINE_FAVOR).id.clone();
        assert_eq!(
            json_of(&shown_of(&hand_card(&s.view(PlayerId::P1), &card))),
            json!([{ "label": LABEL, "value": expected }])
        );
        s.play(card.as_str(), json!({}));
        assert_eq!(drawn_by_p1(s.last_events()), expected as usize);
    }

    #[test]
    fn r280_base_2_other_cards_against_5_previews_3_and_it_draws_exactly_that() {
        previews_and_draws(FaceKind::Base, 5, 2, 3);
    }

    #[test]
    fn r280_radiant_1_other_cards_against_3_previews_5_and_it_draws_exactly_that() {
        previews_and_draws(FaceKind::Radiant, 3, 1, 5);
    }

    #[test]
    fn r280_at_or_past_the_mark_it_previews_0_and_draws_nothing() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [DIVINE_FAVOR, RAPID_REPLENISH, RAPID_REPLENISH], "library": [MENACE] },
            "p2": { "hand": [STOCKPILE] },
        }));
        let card = s.card(DIVINE_FAVOR).id.clone();
        assert_eq!(value_of(&hand_card(&s.view(PlayerId::P1), &card)), 0);
        s.play(card.as_str(), json!({}));
        assert!(
            !s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Drawn { .. }))
        );
    }

    #[test]
    fn r280_in_the_opponents_hand_it_is_hidden_p2s_view_carries_no_preview_of_it() {
        jackioh_cards::register_all();
        let s =
            scenario(json!({ "p1": { "hand": [DIVINE_FAVOR] }, "p2": { "hand": [STOCKPILE, STOCKPILE] } }));
        assert_eq!(s.view(PlayerId::P2).opponent.hand, HandView::Count { count: 1 });
        assert_eq!(
            json_of(&shown_of(&hand_card(
                &s.view(PlayerId::P1),
                &s.card(DIVINE_FAVOR).id
            ))),
            json!([{ "label": LABEL, "value": 2 }])
        );
    }

    #[test]
    fn r280_its_hook_reads_the_hands_sizes_and_nothing_in_them_a_hand_whose_cards_throw_on_access_still_answers()
     {
        jackioh_cards::register_all();
        let s = scenario(
            json!({ "p1": { "hand": [DIVINE_FAVOR, RAPID_REPLENISH] }, "p2": { "hand": [STOCKPILE, STOCKPILE, STOCKPILE] } }),
        );
        let card = s.card(DIVINE_FAVOR).id.clone();
        // Each hand keeps its size and none of its cards; each library is walled off whole.
        let (copy, self_) = fenced(s.state(), &card, Fence::Whole, Fence::SizeOnly, false);
        let hook = must(preview_hook(DIVINE_FAVOR, FaceKind::Base), "Divine Favor's hook");
        let ctx = ConditionContext {
            state: &copy,
            self_: &self_,
            controller: PlayerId::P1,
            radiant: false,
            zone: ConditionZone::Hand,
            your_turn: true,
        };
        assert_eq!(json_of(&hook(ctx)), json!([{ "label": LABEL, "value": 2 }]));
    }

    #[test]
    fn r280_two_games_that_differ_only_in_the_cards_hidden_from_a_viewer_show_the_same_preview() {
        jackioh_cards::register_all();
        let one = scenario(
            json!({ "p1": { "hand": [DIVINE_FAVOR, RAPID_REPLENISH] }, "p2": { "hand": [STOCKPILE, MENACE, MENACE] } }),
        );
        let two = scenario(json!({
            "p1": { "hand": [DIVINE_FAVOR, RAPID_REPLENISH] },
            "p2": { "hand": [MATH_EQUATION, STOCKPILE, BIG_D_FENDER] },
        }));
        let of = |s: &Scenario| -> Option<Vec<PreviewValue>> {
            shown_of(&hand_card(&s.view(PlayerId::P1), &s.card(DIVINE_FAVOR).id))
        };
        assert_eq!(of(&one), of(&two));
    }

    #[test]
    fn r280_a_c_57_echo_copying_it_previews_the_same_formula_on_the_copied_face() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": DIVINE_FAVOR, "radiant": true }, "classic-057", RAPID_REPLENISH],
                "library": [MENACE, MENACE, MENACE, MENACE],
            },
            "p2": { "hand": [STOCKPILE, STOCKPILE] },
        }));
        s.play(DIVINE_FAVOR, json!({}));
        let echo = s.card("classic-057").id.clone();
        // Radiant (2×): the opponent's 2 cards make a mark of 4. Divine Favor drew 2 (Echo and the Filler
        // left made 2), so p1 holds 3 cards besides Echo, which leaves when it resolves: 1 more draw.
        assert_eq!(s.hand(PlayerId::P1).len(), 4);
        assert_eq!(
            json_of(&shown_of(&hand_card(&s.view(PlayerId::P1), &echo))),
            json!([{ "label": LABEL, "value": 1 }])
        );
        s.play(echo.as_str(), json!({}));
        assert_eq!(drawn_by_p1(s.last_events()), 1);
    }
}
