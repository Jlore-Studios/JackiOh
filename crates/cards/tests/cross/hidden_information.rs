//! What `viewFor` hands each seat about cards it may not read (SPEC §9.1, §10.8, R33, R35, R97,
//! R177, R222, R223). Found by the polish-4 edge-case hunt (docs/polish/4-edge-cases.md, lens L10,
//! rounds 1 to 6); every case here failed before its fix. Where a leak is a difference between two
//! games that differ only in hidden cards, the test builds both and asserts the viewer cannot tell
//! them apart. Round 6 found #97 Zephyrs' offer reading a face-down trap and the library's order
//! (R222), a library-wide discount whose events spelled out the library's order once a card read
//! openly (R177), and instance ids numbered in the order the store sorts a deck in (R223). Round 7
//! found a random Make Radiant cueing only the cards it changed, which counted a hidden hand's Radiant
//! cards (R177), and #83's library replacements numbered top down, which located a revealed library
//! card by its id (R223). Round 8 found a hidden cue's zone saying where #28's pick landed, and #23
//! rolling only for a hand that held a base-face card, both of which told p2 about p1's hidden faces
//! (R177). The last case, R119's, is the one that did not fail first: it pins the strip `viewFor`
//! already made of `cardResolved.arrivedDuring`, which would name a face-down trap if it went out.
//!
//! Round 9 found three more: a card the mulligan returned, waiting in no pile while a replacement
//! draw's cast asks, read as public, so the deal's events named it to the other seat (R224); and #28's
//! cues trailed its real picks and landed on the owner's own hand first, so their order told the
//! other seat, and their place told the owner, which hidden faces were base-face (R177).
//!
//! Round 10 found two more in #28, and three things the view left out. R60's pick over the
//! non-Radiant cards alone made the chance that #28 passed over p1's public unit hang on how many of
//! p1's hidden cards were base-face, and its picks in the zones' order put a face-down trap's after
//! the public unit's and a hand card's before it (R242). And a card's own owner could not read what
//! it is made of beyond its printed face: a Corpse Eater's meals in hand, a Heroic Power's rolled
//! power, a crafted card's definition (R243).
//!
//! Port of `packages/cards/test/hidden-information.test.ts` (SURFACE §4.1, §8).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{choose_mode, remember};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::subsystems::hero_power::power_of;
use jackioh_engine::testkit::*;

/// viewFor's R97 sentinel.
const HIDDEN: &str = "hidden";

const GARY: &str = "core-004";
const STOCKPILE: &str = "core-005";
const HIT_JOB: &str = "core-016";
const MENACE: &str = "core-019";
const MATH_EQUATION: &str = "core-031";
const SHEEPISH: &str = "core-041";
const SPEK: &str = "core-048";
const MIND_CONTROL: &str = "core-049";
const TWINSPELL: &str = "core-079";
const SEVEN_SEVEN: &str = "core-025";
const LUNAR_ECLIPSE: &str = "core-035";
const MAGIC_JAMMED: &str = "core-036";
const EUGENICS: &str = "core-042";
const MR_VANILLA: &str = "core-008";
const BIGOT: &str = "core-002";
const TRANSMOGULATE: &str = "core-083";
const UNLICENSED: &str = "core-085";
const COLLATERAL: &str = "core-034";
const CORPSE_EATER: &str = "core-089";
const COMBO_INDEX: &str = "core-093";
const CALL_TO_CHAOS: &str = "core-095";
const MY_PAWN: &str = "core-096";
const GLOWY: &str = "core-026";
const GIGA: &str = "core-029";
const GIFTED: &str = "core-064";
const SORCERER: &str = "core-068";
const MASK: &str = "core-065";
const HONEYPOT: &str = "core-060";
/// Classic #60 Pile On: "Recruit every permanent in your deck".
const PILE_ON: &str = "classic-060";
const CALL_TO_ARMS: &str = "core-069";
const MOTHS: &str = "core-009";
const BLOOD_RIDDEN: &str = "core-027";
const ZEPHYRS: &str = "core-097";
const HEROIC_POWER: &str = "core-098";
const DREAM: &str = "core-023";
const KNOCKOFF: &str = "core-028";

fn events_of(view: &PlayerView, type_: GameEventType) -> Vec<GameEvent> {
    view.events
        .iter()
        .filter(|event| event.event_type() == type_)
        .cloned()
        .collect()
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("missing {what}"))
}

/// The viewer's two views are identical, event list first so a failure names the extra event.
fn indistinguishable(viewer: PlayerId, a: &Scenario, b: &Scenario) {
    let types = |s: &Scenario| -> Vec<GameEventType> {
        s.view(viewer).events.iter().map(GameEvent::event_type).collect()
    };
    assert_eq!(types(b), types(a));
    assert_eq!(b.view(viewer), a.view(viewer));
}

/// A value as TS's `toEqual` compares it: its JSON, absent fields absent.
fn json_of(value: &impl serde::Serialize) -> Value {
    serde_json::to_value(value).expect("a view serialises")
}

/// TS's `JSON.stringify`.
fn json_text(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("a view serialises")
}

/// TS's `expect(text).not.toMatch(/a|b|c/)`, negated: whether any alternative occurs.
fn matches_any(text: &str, alternatives: &[&str]) -> bool {
    alternatives.iter().any(|alternative| text.contains(alternative))
}

/// The prompt a view says the viewer must answer.
fn prompt_for_viewer(pending: Option<PendingView>, what: &str, refusal: &str) -> PendingPromptView {
    match must(pending, what) {
        PendingView::ForYou(prompt) => prompt,
        PendingView::Elsewhere(_) => panic!("{refusal}"),
    }
}

mod r177_a_prompt_option_that_offers_a_face_down_card {
    use super::*;

    #[test]
    fn r177_r33_an_echo_repeats_target_prompt_names_an_enemy_face_down_trap_by_id_only_no_def_id_and_no_name_in_its_label_or_key()
     {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-l10-echo",
            "p1": { "hand": [TWINSPELL, MIND_CONTROL, STOCKPILE], "mana": 10, "library": [HIT_JOB, HIT_JOB] },
            "p2": {
                "field": [MENACE],
                "backrow": [
                    { "def": MY_PAWN, "lane": 1 },
                    { "def": SHEEPISH, "lane": 2 },
                ],
                "hand": [STOCKPILE],
                "library": [HIT_JOB],
            },
        }));
        let pawn = must(s.backrow("p2", 1), "p2's lane-1 trap");
        let sheep = must(s.backrow("p2", 2), "p2's lane-2 trap");
        let menace = must(s.unit("p2", 1), "p2's unit");

        s.play(TWINSPELL, json!({}));
        s.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));

        // The Echo repeat asks again (§10.5 step 6, R81) and offers p2's two face-down traps.
        let view = s.view("p1");
        assert_eq!(json_of(&view.opponent.backrow[0]), json!({ "faceDown": true, "cost": 1 }));
        let pending = prompt_for_viewer(view.pending.clone(), "p1's Echo prompt", "the Echo prompt should be p1's");
        let trap_options: Vec<&PendingOption> = pending
            .options
            .iter()
            .filter(|option| {
                option.instance_id.as_deref() == Some(pawn.id.as_str())
                    || option.instance_id.as_deref() == Some(sheep.id.as_str())
            })
            .collect();
        assert_eq!(trap_options.len(), 2);
        for option in &trap_options {
            assert!(option.def_id.is_none());
            assert!(!matches_any(&option.label, &["Pawn", "Sheepish", "core-0"]));
            assert!(!matches_any(&option.key, &["Pawn", "Sheepish", "core-0"]));
        }
        // The chooser still answers with the id it was given.
        s.answer(json!([{ "pick": "instance", "instanceId": sheep.id }]));
        assert_eq!(s.card(&sheep).controller, PlayerId::P1);
    }
}

mod r177_a_card_replaced_where_the_viewer_cannot_see_it {
    use super::*;

    #[test]
    fn r177_r35_transmogulates_transformed_events_name_no_replaced_library_card_or_face_down_trap() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-l10-transmogulate",
            "p1": {
                "hand": [TRANSMOGULATE, STOCKPILE],
                "backrow": [{ "def": SHEEPISH, "lane": 1 }],
                "library": ["core-025", "core-002", "core-030"],
            },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));

        s.play(TRANSMOGULATE, json!({}));

        // p1's view: the three library replacements are hidden. p2's view: those three, the trap, and
        // the Stockpile in p1's hand, which Transmogulate replaces too (R365).
        let expect_hidden = PerPlayer { p1: 3usize, p2: 5usize };
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let into_hidden: Vec<(String, String)> = events_of(&s.view(viewer), GameEventType::Transformed)
                .into_iter()
                .filter_map(|event| match event {
                    GameEvent::Transformed {
                        instance_id,
                        from_def_id,
                        new_instance_id,
                        ..
                    } if new_instance_id == HIDDEN => Some((instance_id, from_def_id)),
                    _ => None,
                })
                .collect();
            assert_eq!(into_hidden.len(), expect_hidden[viewer]);
            for (instance_id, from_def_id) in &into_hidden {
                assert_eq!((instance_id.as_str(), from_def_id.as_str()), (HIDDEN, HIDDEN));
            }
        }
        // p1 controls its own trap, so it reads that replacement in full.
        let own = events_of(&s.view("p1"), GameEventType::Transformed)
            .into_iter()
            .filter(|event| matches!(event, GameEvent::Transformed { from_def_id, .. } if from_def_id == SHEEPISH))
            .count();
        assert_eq!(own, 1);
    }
}

mod r177_a_cost_change_on_a_card_the_viewer_may_not_read {
    use super::*;

    /// #93 at E, one card played, so end of turn runs E and D: two random hand cards cost 1 less.
    fn combo_index_game(held: &[&str]) -> Scenario {
        let mut hand = vec![SPEK];
        hand.extend_from_slice(held);
        let mut s = scenario(json!({
            "seed": "hunt-l10-combo",
            "p1": { "backrow": [COMBO_INDEX], "hand": hand, "library": [HIT_JOB, HIT_JOB] },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
        }));
        s.play(SPEK, json!({})).end_turn();
        s
    }

    #[test]
    fn r177_grade_ds_discount_does_not_tell_the_opponent_what_p1s_hidden_hand_cards_cost() {
        jackioh_cards::register_all();
        // Two games that differ only in which (hidden) cards p1 holds; every position differs in cost.
        let a = combo_index_game(&["core-025", "core-030"]);
        let b = combo_index_game(&["core-020", "core-011"]);

        assert_eq!(events_of(&a.view("p2"), GameEventType::CostChanged).len(), 2);
        assert_eq!(events_of(&b.view("p2"), GameEventType::CostChanged).len(), 2);
        assert_eq!(b.view("p2"), a.view("p2"));
        // The owner still reads its own discounts.
        assert!(
            events_of(&a.view("p1"), GameEventType::CostChanged)
                .iter()
                .all(|event| matches!(event, GameEvent::CostChanged { instance_id, cost, .. } if instance_id != HIDDEN && *cost >= 0))
        );
    }

    /// "chaos-2" is a seed whose #95 rolls "every card in your hand and library costs 2 less".
    fn chaos_discount_game(library: &[&str]) -> Scenario {
        let mut s = scenario(json!({
            "seed": "chaos-2",
            "p1": { "hand": [CALL_TO_CHAOS, STOCKPILE], "mana": 10, "library": library },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(CALL_TO_CHAOS, json!({}));
        s
    }

    #[test]
    fn r177_call_to_chaoss_library_discount_does_not_reveal_the_librarys_order_to_either_player() {
        jackioh_cards::register_all();
        let a = chaos_discount_game(&["core-025", "core-002"]);
        let b = chaos_discount_game(&["core-002", "core-025"]);

        // The discount rolled: the hand card and both library cards changed cost.
        assert_eq!(events_of(&a.view("p1"), GameEventType::CostChanged).len(), 3);
        assert_eq!(b.view("p1"), a.view("p1"));
        assert_eq!(b.view("p2"), a.view("p2"));
    }
}

mod r177_identities_an_event_carries_outside_its_redacted_fields {
    use super::*;

    #[test]
    fn r177_r97_destroyed_killer_id_does_not_name_a_killer_that_now_sits_in_the_opponents_hand() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-r2-hidden-killer",
            "p1": { "hand": [MATH_EQUATION, STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
            "p2": { "field": [GARY], "hand": [STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
        }));
        let gary = must(s.unit("p2", 1), "p2's Gary");
        let math = must(
            s.hand("p1").into_iter().find(|card| card.def_id == MATH_EQUATION),
            "p1's Math Equation",
        );

        // Fib(1 + 1) = 1 damage kills the 1/1; at end of turn the Spell returns to p1's hand.
        s.play(&math, json!({ "targets": [{ "pick": "instance", "instanceId": gary.id }] }));
        s.expect_in_zone(&gary, "graveyard");
        s.end_turn();
        s.expect_in_zone(&math, "hand");

        let view = s.view("p2");
        // The damage event already hides the Spell, since it now sits in p1's hand (R97)...
        let hit = must(
            events_of(&view, GameEventType::Damage).into_iter().find_map(|event| match event {
                GameEvent::Damage {
                    source_id, target_id, ..
                } if target_id == gary.id => Some(source_id),
                _ => None,
            }),
            "the damage event",
        );
        assert_eq!(hit.as_deref(), Some(HIDDEN));
        // ...so the destroyed event must not name it either.
        let death = must(
            events_of(&view, GameEventType::Destroyed).into_iter().find_map(|event| match event {
                GameEvent::Destroyed {
                    instance_id, killer_id, ..
                } if instance_id == gary.id => Some(killer_id),
                _ => None,
            }),
            "Gary's destroyed event",
        );
        assert_eq!(death.as_deref(), Some(HIDDEN));
    }
}

mod r177_a_card_that_ceased_to_exist_in_a_hidden_zone_stays_hidden_in_earlier_events {
    use super::*;

    #[test]
    fn r177_r33_r35_r97_after_transmogulate_replaces_a_face_down_trap_its_earlier_card_played_does_not_name_it_to_the_opponent()
     {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-r2-transmogulated-trap",
            "p1": { "hand": [SHEEPISH, TRANSMOGULATE, STOCKPILE], "mana": 10, "library": [HIT_JOB, HIT_JOB] },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        let sheep = must(
            s.hand("p1").into_iter().find(|card| card.def_id == SHEEPISH),
            "p1's Sheepish",
        );

        s.play(&sheep, json!({ "zone": 1 }));
        // Before the transform p2 reads nothing of it: a bare face-down zone and a redacted play.
        assert_eq!(
            json_of(&s.view("p2").opponent.backrow[0]),
            json!({ "faceDown": true, "cost": 1 })
        );
        assert!(!json_text(&s.view("p2")).contains(SHEEPISH));

        s.play(TRANSMOGULATE, json!({}));
        s.expect_in_zone(&sheep, "gone");

        // Sheepish was never revealed to p2: it never fired and never reached a public pile.
        let view = s.view("p2");
        let played = events_of(&view, GameEventType::CardPlayed)
            .into_iter()
            .filter(|event| matches!(event, GameEvent::CardPlayed { player: PlayerId::P1, .. }))
            .count();
        assert!(played >= 2);
        assert!(!json_text(&view).contains(SHEEPISH));
        assert!(!json_text(&view.events).contains(&format!("\"{}\"", sheep.id)));
    }

    /// "chaos-2" is a seed whose #95 rolls "every card in your hand and library costs 2 less".
    fn chaos_then_transmogulate(library: &[&str]) -> Scenario {
        let mut s = scenario(json!({
            "seed": "chaos-2",
            "p1": { "hand": [CALL_TO_CHAOS, TRANSMOGULATE, STOCKPILE], "mana": 10, "library": library },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(CALL_TO_CHAOS, json!({}));
        s.play(TRANSMOGULATE, json!({}));
        s
    }

    #[test]
    fn r177_after_transmogulate_replaces_the_library_call_to_chaoss_cost_changes_still_do_not_spell_out_its_old_order() {
        jackioh_cards::register_all();
        let a = chaos_then_transmogulate(&["core-025", "core-002"]);
        let b = chaos_then_transmogulate(&["core-002", "core-025"]);

        // The discount rolled for both library cards (and the hand), and both games transformed alike.
        assert!(events_of(&a.view("p1"), GameEventType::CostChanged).len() >= 3);
        assert!(events_of(&a.view("p1"), GameEventType::Transformed).len() >= 2);
        // Two games that differ only in the (hidden) order of p1's library look the same to both seats.
        assert_eq!(b.view("p2"), a.view("p2"));
        assert_eq!(b.view("p1"), a.view("p1"));
    }
}

mod r177_a_hidden_hand_cards_buff {
    use super::*;

    /// p1 Hit Jobs p2's Mr. Vanilla (3/3); p2 holds a Corpse Eater, base or Radiant.
    fn eater_game(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "hunt-r3-hidden-eater",
            "p1": { "hand": [HIT_JOB, STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
            "p2": {
                "field": [MR_VANILLA],
                "hand": [{ "def": CORPSE_EATER, "radiant": radiant }, STOCKPILE],
                "library": [HIT_JOB, HIT_JOB],
            },
        }));
        let vanilla = must(s.unit("p2", 1), "p2's Mr. Vanilla");
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": vanilla.id }] }));
        s.expect_in_zone(&vanilla, "graveyard");
        s
    }

    fn eater_of(s: &Scenario) -> CardInstance {
        must(
            s.hand("p2").into_iter().find(|card| card.def_id == CORPSE_EATER),
            "p2's Eater",
        )
    }

    #[test]
    fn r177_r97_a_corpse_eaters_gain_in_hand_does_not_tell_the_opponent_whether_it_is_radiant() {
        jackioh_cards::register_all();
        let base = eater_game(false);
        let radiant = eater_game(true);

        // Both Eaters fed on a 4/4: +4/+4 on the base face, double on the radiant one (§8 #89).
        assert_eq!(eater_of(&base).buffs, AttackHealth { attack: 4, health: 4 });
        assert_eq!(eater_of(&radiant).buffs, AttackHealth { attack: 8, health: 8 });

        // p2's hand is a count to p1 (§10.8), and the size of a hidden card's buff is the card's: the
        // event still plays its cue, and p1 cannot tell the two games apart.
        assert_eq!(
            json_of(&events_of(&base.view("p1"), GameEventType::Buffed)),
            json!([{ "type": "buffed", "instanceId": HIDDEN, "attack": 0, "health": 0 }])
        );
        assert_eq!(radiant.view("p1"), base.view("p1"));
        // p2 reads its own card's buff in full.
        let attacks: Vec<i32> = events_of(&radiant.view("p2"), GameEventType::Buffed)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::Buffed { attack, .. } => Some(attack),
                _ => None,
            })
            .collect();
        assert_eq!(attacks, vec![8]);
    }
}

mod r177_a_card_that_ceased_to_exist_where_the_viewer_could_not_read_it_stays_unread {
    use super::*;

    #[test]
    fn r177_r33_r35_a_face_down_trap_transmogulate_replaced_stays_hidden_after_its_replacement_reaches_the_graveyard() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "hunt-r3-transmog-trap",
            "p1": {
                "hand": [TRANSMOGULATE],
                "backrow": [{ "def": SHEEPISH, "lane": 1 }],
                "mana": 10,
                "library": [HIT_JOB],
            },
            "p2": { "hand": [MAGIC_JAMMED, STOCKPILE], "library": [HIT_JOB] },
        }));
        let sheep = must(s.backrow("p1", 1), "p1's Sheepish");

        s.play(TRANSMOGULATE, json!({}));
        s.expect_in_zone(&sheep, "gone");
        let replacement = must(s.backrow("p1", 1), "the replacement trap");
        assert_eq!(replacement.def_id, UNLICENSED);

        // p2 destroys p1's face-down replacement on its own turn (a Magic Jammed in p1's hand would have
        // been replaced too, R365), and it reaches p1's public graveyard.
        if s.state().active == PlayerId::P1 {
            s.end_turn();
        }
        s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": replacement.id }] }));
        s.expect_in_zone(&replacement, "graveyard");

        // Sheepish never fired and never reached a public pile, so p2 still reads nothing of it, while
        // p1, its controller, still reads what it replaced.
        let view = s.view("p2");
        assert!(!events_of(&view, GameEventType::Transformed).is_empty());
        assert!(!json_text(&view).contains(SHEEPISH));
        assert!(!json_text(&view.events).contains(&format!("\"{}\"", sheep.id)));
        assert!(
            events_of(&s.view("p1"), GameEventType::Transformed)
                .iter()
                .any(|event| matches!(
                    event,
                    GameEvent::Transformed { instance_id, from_def_id, .. }
                        if *instance_id == sheep.id && from_def_id == SHEEPISH
                ))
        );
        // The record the view reads is never forwarded to either seat.
        assert!(!json_text(&s.view("p1").events).contains("hiddenFrom"));
    }

    /// p1's one library card is replaced, then p2's Collateral Damage exiles the replacement into public
    /// view (a card in p1's own hand would have been replaced too, R365).
    fn library_game(card: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": "hunt-r3-transmog-library",
            "p1": { "hand": [TRANSMOGULATE], "mana": 10, "library": [card] },
            "p2": { "hand": [COLLATERAL, STOCKPILE], "field": [{ "def": GARY, "lane": 1 }], "library": [HIT_JOB] },
        }));
        s.play(TRANSMOGULATE, json!({}));
        if s.state().active == PlayerId::P1 {
            s.end_turn();
        }
        let gary = must(s.unit("p2", 1), "p2's Gary");
        s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": gary.id }] }));
        s
    }

    #[test]
    fn r177_r35_a_library_card_transmogulate_replaced_stays_hidden_after_its_replacement_is_exiled() {
        jackioh_cards::register_all();
        let a = library_game(SEVEN_SEVEN);
        let b = library_game(BIGOT);

        // The replacement is in p1's exile, a public pile, in both games.
        assert_eq!(a.pile("p1", "exile").len(), 1);
        assert_eq!(
            a.pile("p1", "exile").first().map(|card| card.def_id.clone()),
            b.pile("p1", "exile").first().map(|card| card.def_id.clone())
        );

        // p1's library held a card neither player saw and that has ceased to exist (R35): two games that
        // differ only in what it was look the same to both seats.
        assert_eq!(b.view("p2"), a.view("p2"));
        assert_eq!(b.view("p1"), a.view("p1"));
    }

    /// p1's two-card library, replaced by Transmogulate; p2 watches. `first` is a def id or a
    /// `{ def, radiant }` entry, as TS's `string | { def; radiant }`.
    fn immutable_library_game(first: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": "hunt-r3-transmog-immutable",
            "p1": { "hand": [TRANSMOGULATE, STOCKPILE], "mana": 10, "library": [first, HIT_JOB] },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(TRANSMOGULATE, json!({}));
        s
    }

    #[test]
    fn r35_r23_transmogulate_replaces_an_immutable_library_card_too_so_the_opponent_cannot_count_the_librarys_immutable_cards()
     {
        jackioh_cards::register_all();
        let with_immutable = immutable_library_game(json!({ "def": MENACE, "radiant": true }));
        let without = immutable_library_game(json!(STOCKPILE));

        // "Other zones: any card from the pool, same counts": Immutable stays only on the board, where
        // the Replace is a Transform (R23, §8 #83), so the library is replaced whole.
        let library: Vec<String> = with_immutable
            .pile("p1", "library")
            .into_iter()
            .map(|card| card.def_id)
            .collect();
        assert!(!library.contains(&MENACE.to_string()));
        assert_eq!(with_immutable.view("p2"), without.view("p2"));
    }
}

mod r177_a_number_the_view_carries_is_never_taken_by_a_hidden_card {
    use super::*;

    /// A Rush Token dies (no Eater gain, R11), then p1 installs a modifier with Lunar Eclipse.
    fn modifier_game(p2_hand: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": "hunt-r3-seq",
            "p1": { "hand": [HIT_JOB, LUNAR_ECLIPSE, STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
            "p2": { "field": ["T-rush"], "hand": [p2_hand, STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
        }));
        let token = must(s.unit("p2", 1), "p2's Rush Token");
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": token.id }] }));
        s.expect_in_zone(&token, "gone");
        s.play(LUNAR_ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
        s
    }

    #[test]
    fn r177_r169_a_modifiers_id_does_not_tell_the_opponent_that_p2_holds_a_corpse_eater() {
        jackioh_cards::register_all();
        let eater = modifier_game(CORPSE_EATER);
        let plain = modifier_game(SEVEN_SEVEN);

        // The token fed nothing (R11), so no `buffed` went out, and the hand is a count (§10.8).
        assert!(events_of(&eater.view("p1"), GameEventType::Buffed).is_empty());
        assert_eq!(eater.view("p1").you.modifiers.len(), 1);
        assert_eq!(eater.view("p1"), plain.view("p1"));
    }
}

mod r177_make_radiant_on_a_hidden_card_that_is_already_radiant {
    use super::*;

    #[test]
    fn r177_r97_29_giga_and_26_glowy_jelly_bean_do_not_tell_the_opponent_which_hidden_hand_cards_were_already_radiant() {
        jackioh_cards::register_all();
        // #29: "Every card in your hand becomes Radiant". Afterwards both hands are wholly Radiant, so
        // the games differ only in the face p1's 4-mana 7/7 had while p2 could not read it.
        fn giga(radiant: bool) -> Scenario {
            let mut s = scenario(json!({
                "seed": "hunt-r4-giga",
                "p1": { "hand": [GIGA, { "def": SEVEN_SEVEN, "radiant": radiant }, BIGOT], "mana": 10, "library": [HIT_JOB] },
                "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
            }));
            s.play(GIGA, json!({}));
            s
        }
        let giga_base = giga(false);
        let giga_radiant = giga(true);
        assert!(giga_base.hand("p1").iter().all(|card| card.radiant));
        assert!(giga_radiant.hand("p1").iter().all(|card| card.radiant));
        // A cue only for the cards that changed would count, for p2, how many were Radiant before.
        indistinguishable(PlayerId::P2, &giga_base, &giga_radiant);

        // #26: "Choose a card in your hand; it becomes Radiant". The same no-op on the chosen card.
        fn glowy(radiant: bool) -> Scenario {
            let mut s = scenario(json!({
                "seed": "hunt-r4-glowy",
                "p1": { "hand": [GLOWY, { "def": SEVEN_SEVEN, "radiant": radiant }, BIGOT], "mana": 10, "library": [HIT_JOB] },
                "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
            }));
            let chosen = must(
                s.hand("p1").into_iter().find(|card| card.def_id == SEVEN_SEVEN),
                "p1's 7/7",
            );
            s.play(GLOWY, json!({ "targets": [{ "pick": "instance", "instanceId": chosen.id }] }));
            assert!(s.card(&chosen).radiant);
            s
        }
        indistinguishable(PlayerId::P2, &glowy(false), &glowy(true));
    }

    #[test]
    fn r177_r33_r97_r213_64_gifted_program_does_not_tell_the_opponent_that_a_face_down_trap_was_already_radiant() {
        jackioh_cards::register_all();
        // The trap is p1's first card costing 1 or less this turn, so §10.5 step 3 makes it Radiant
        // (R213). It lands face-down and Radiant in both games; only its face in hand differed.
        fn game(radiant: bool) -> Scenario {
            let mut s = scenario(json!({
                "seed": "hunt-r4-gifted",
                "p1": {
                    "backrow": [GIFTED],
                    "hand": [{ "def": SHEEPISH, "radiant": radiant }, STOCKPILE],
                    "mana": 4,
                    "library": [HIT_JOB],
                },
                "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
            }));
            s.play(SHEEPISH, json!({ "zone": 2 }));
            s
        }
        let base = game(false);
        let radiant = game(true);
        assert_eq!(base.backrow("p1", 2).map(|card| card.radiant), Some(true));
        assert_eq!(radiant.backrow("p1", 2).map(|card| card.radiant), Some(true));
        assert_eq!(
            json_of(&base.view("p2").opponent.backrow[1]),
            json!({ "faceDown": true, "cost": 1 })
        );
        indistinguishable(PlayerId::P2, &base, &radiant);
    }
}

mod r177_eugenics_radiant_roll_over_a_hidden_library {
    use super::*;

    /// p1's library is eight 4-mana 7/7s. Eugenics exiles 7 at random, and with this seed the card left
    /// is library[5]; each remaining library card then has a 30% chance to become Radiant (§8 #42),
    /// and with this seed that card's roll comes up. The two games differ only in whether that one
    /// library card, which neither player can read (§9.1), was Radiant already.
    fn eugenics_game(left_card_radiant: bool) -> Scenario {
        let library: Vec<Value> = (0..8)
            .map(|at| json!({ "def": SEVEN_SEVEN, "radiant": left_card_radiant && at == 5 }))
            .collect();
        let mut s = scenario(json!({
            "seed": "r5-eugenics-7",
            "p1": { "hand": [EUGENICS, STOCKPILE], "mana": 10, "library": library },
            "p2": { "hand": [STOCKPILE], "library": [STOCKPILE] },
        }));
        s.play(EUGENICS, json!({}));
        s
    }

    #[test]
    fn r177_r97_42_rolls_every_remaining_library_card_so_its_cues_do_not_count_the_ones_already_radiant() {
        jackioh_cards::register_all();
        let base = eugenics_game(false);
        let radiant = eugenics_game(true);

        // The same seven cards went to the (public) exile pile in both games, all of them base-face.
        let ids = |s: &Scenario| -> Vec<String> { s.pile("p1", "exile").into_iter().map(|card| card.id).collect() };
        assert_eq!(ids(&base), ids(&radiant));
        assert!(base.pile("p1", "exile").iter().all(|card| !card.radiant));
        // One card is left, and it ends Radiant in both games: rolled into it, or already there.
        let faces = |s: &Scenario| -> Vec<bool> { s.pile("p1", "library").iter().map(|card| card.radiant).collect() };
        assert_eq!(faces(&base), vec![true]);
        assert_eq!(faces(&radiant), vec![true]);

        // "Each remaining library card has a 30% chance" (§8 #42): the already-Radiant card is rolled
        // like any other, and a success on a card nobody may read is cued whether or not its flag
        // changed (R177). A cue for the changed cards only lets both seats count the Radiant ones.
        indistinguishable(PlayerId::P2, &base, &radiant);
        indistinguishable(PlayerId::P1, &base, &radiant);
    }
}

mod r177_a_number_taken_by_a_face_down_trap_owed_an_event {
    use super::*;

    /// p1's Twisted Sorcerer swings for lethal at p2 (5 health). p2's lane-1 My Pawn cancels it and
    /// the AI plays out p1's turn (R44); p2's turn starts inside that playout and Masochism Mask asks
    /// p2 (§8 #65), so the declaration's trap window stops with a prompt open. p2's lane-2 card is
    /// face-down, and the games differ only in what it is. p2 answers, then plays Lunar Eclipse,
    /// whose "next Spell costs 1 less" is a modifier both seats read with its id (R169).
    fn pawn_game(lane_two: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r5-owed-window",
            "p1": { "field": [SORCERER], "library": [GIGA, GIGA, GIGA] },
            "p2": {
                "health": 5,
                "hand": [LUNAR_ECLIPSE, STOCKPILE],
                "backrow": [
                    { "def": MY_PAWN, "lane": 1, "faceUp": false },
                    { "def": lane_two, "lane": 2, "faceUp": false },
                    { "def": MASK, "lane": 3 },
                ],
                "library": [GIGA, GIGA, GIGA],
            },
        }));
        s.attack(SORCERER, "hero");
        s.answer(json!("lose 3"));
        s.play(LUNAR_ECLIPSE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
        s
    }

    #[test]
    fn r177_r33_r169_p1_cannot_tell_from_a_modifiers_id_that_p2s_other_face_down_trap_is_a_second_my_pawn() {
        jackioh_cards::register_all();
        let two_pawns = pawn_game(MY_PAWN);
        let pawn_and_sheep = pawn_game(SHEEPISH);
        let pawn_and_honeypot = pawn_game(HONEYPOT);

        // Every game played the same public course: the attack cancelled, p1's turn handed over and
        // ended, p2 on turn 10 with Lunar Eclipse's discount live, and the lane-2 card still face-down.
        for s in [&two_pawns, &pawn_and_sheep, &pawn_and_honeypot] {
            assert_eq!(s.state().turn, 10);
            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(
                json_of(&s.view("p1").opponent.backrow[1]),
                json!({ "faceDown": true, "cost": 1 })
            );
            assert_eq!(s.view("p1").opponent.modifiers.len(), 1);
        }

        // What p2's lane-2 face-down card is must not reach p1 (R33), not even through the number the
        // counter hands the next modifier: R177's "no number the view carries is taken by a hidden card".
        indistinguishable(PlayerId::P1, &pawn_and_sheep, &two_pawns);
        indistinguishable(PlayerId::P1, &pawn_and_honeypot, &two_pawns);
    }
}

/// The def ids a Discover prompt offers the viewer, sorted.
fn offered_to(s: &Scenario, viewer: PlayerId) -> Vec<String> {
    let Some(PendingView::ForYou(pending)) = s.view(viewer).pending else {
        panic!("no prompt open for {viewer}");
    };
    let mut offered: Vec<String> = pending
        .options
        .iter()
        .map(|option| option.def_id.clone().unwrap_or_else(|| option.key.clone()))
        .collect();
    offered.sort();
    offered
}

mod r222_97_zephyrs_dry_run_and_hidden_information {
    use super::*;

    /// p1 casts Zephyrs against p2's 1/14 Moths, which no affordable printed attack kills, so a
    /// candidate "clears the enemy board" (§10.7) only through what its text does. p2's lane-1 card is
    /// face-down, and the two games differ only in what it is: Sheepish, or My Pawn (which answers a
    /// declared attack and never a play).
    fn zephyrs_against(trap: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r6-zephyrs-trap",
            "p1": { "hand": [ZEPHYRS, STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
            "p2": { "field": [MOTHS], "backrow": [{ "def": trap, "lane": 1 }], "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(ZEPHYRS, json!({}));
        s
    }

    #[test]
    fn r222_r33_the_cards_97_offers_do_not_tell_p1_what_p2s_face_down_trap_is() {
        jackioh_cards::register_all();
        let sheep = zephyrs_against(SHEEPISH);
        let pawn = zephyrs_against(MY_PAWN);

        // p1 reads a bare face-down zone in both games, and both have the Discover open.
        assert_eq!(
            json_of(&sheep.view("p1").opponent.backrow[0]),
            json!({ "faceDown": true, "cost": 1 })
        );
        assert_eq!(
            json_of(&pawn.view("p1").opponent.backrow[0]),
            json!({ "faceDown": true, "cost": 1 })
        );
        assert_eq!(offered_to(&sheep, PlayerId::P1).len(), 3);

        // The scorer ranks for p1 (§10.7, R29), and what p1 may read is the same in both games, so the
        // three cards it offers must be too: an offer that moves with p2's face-down card names it.
        assert_eq!(offered_to(&sheep, PlayerId::P1), offered_to(&pawn, PlayerId::P1));
        indistinguishable(PlayerId::P1, &pawn, &sheep);
    }

    /// p1 is at 5 health, so §10.7's heal priority applies, and p2 has no units. The two games differ
    /// only in the order of p1's own library, which nobody may read (§3, §9.1, §10.8): Blood Ridden
    /// Glowy Jelly Bean (cast on draw: lose 5) is on top, or at the bottom.
    fn zephyrs_over_library(library: &[&str]) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r6-zephyrs-library",
            "p1": { "hand": [ZEPHYRS, STOCKPILE], "health": 5, "library": library },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(ZEPHYRS, json!({}));
        s
    }

    #[test]
    fn r222_the_cards_97_offers_do_not_tell_p1_the_order_of_its_own_library() {
        jackioh_cards::register_all();
        let on_top = zephyrs_over_library(&[BLOOD_RIDDEN, HIT_JOB, HIT_JOB]);
        let at_bottom = zephyrs_over_library(&[HIT_JOB, HIT_JOB, BLOOD_RIDDEN]);

        assert_eq!(offered_to(&on_top, PlayerId::P1).len(), 3);
        assert_eq!(on_top.view("p1").you.library_count, 3);

        // "The rest of the library stays hidden from both" (§10.8): an offer that moves with the order
        // of p1's library tells p1 what is on top of it.
        assert_eq!(offered_to(&on_top, PlayerId::P1), offered_to(&at_bottom, PlayerId::P1));
        assert!(events_of(&on_top.view("p1"), GameEventType::Drawn).is_empty());
        indistinguishable(PlayerId::P1, &at_bottom, &on_top);
    }
}

mod r177_a_library_cards_place_in_a_library_wide_event_sequence {
    use super::*;

    /// "chaos-2" is a seed whose #95 rolls "every card in your hand and library costs 2 less", which
    /// emits one `costChanged` per hand card and then one per library card, top first. #69 Call to
    /// Arms then recruits p1's one Unit (Gary, 1 cost) out of the library. The games differ only in
    /// where Gary was: on top of the two Hit Jobs, or between them.
    fn chaos_then_recruit(library: &[&str]) -> Scenario {
        let mut s = scenario(json!({
            "seed": "chaos-2",
            "p1": { "hand": [CALL_TO_CHAOS, CALL_TO_ARMS, STOCKPILE], "mana": 10, "library": library },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(CALL_TO_CHAOS, json!({}));
        s.play(CALL_TO_ARMS, json!({}));
        s
    }

    /// Where in the view's `costChanged` sequence the recruited card's own event sits (-1: nowhere).
    fn place_of_recruited(s: &Scenario, viewer: PlayerId) -> i64 {
        let gary = match s.unit("p1", 1) {
            Some(gary) if gary.def_id == GARY => gary,
            _ => panic!("Call to Arms recruited no Gary"),
        };
        events_of(&s.view(viewer), GameEventType::CostChanged)
            .iter()
            .position(|event| matches!(event, GameEvent::CostChanged { instance_id, .. } if *instance_id == gary.id))
            .map_or(-1, |at| at as i64)
    }

    /// The view's `costChanged` events, in order.
    fn discounts(s: &Scenario, viewer: PlayerId) -> Vec<GameEvent> {
        events_of(&s.view(viewer), GameEventType::CostChanged)
    }

    #[test]
    fn r177_r97_a_discount_made_while_a_card_lay_in_the_library_stays_unread_once_the_card_reads_openly_so_its_place_says_nothing()
     {
        jackioh_cards::register_all();
        let on_top = chaos_then_recruit(&[GARY, HIT_JOB, HIT_JOB]);
        let second = chaos_then_recruit(&[HIT_JOB, GARY, HIT_JOB]);

        // The discount rolled for the two hand cards and the three library cards, and Gary is on the
        // field in both games, the two Hit Jobs still in the library.
        for s in [&on_top, &second] {
            assert!(events_of(&s.view("p2"), GameEventType::CostChanged).len() >= 5);
            let library: Vec<String> = s.pile("p1", "library").into_iter().map(|card| card.def_id).collect();
            assert_eq!(library, vec![HIT_JOB.to_string(), HIT_JOB.to_string()]);
        }

        // §9.1 hides library order from both players, which is why R97 blanks `shuffledIn.position`
        // even for a card the viewer may read, and why R177 hides a library card's cost. The discount
        // went out one event per card in library order, so read openly once Gary did, its place among
        // them would say how deep Gary lay — here, whether p1's next draw was a 1-cost Unit. The finder
        // asked for Gary's event to read openly at the same place in both games; what SPEC asks is that
        // neither seat learns the order, and the event was made where nobody could read it (§3), so it
        // stays unread for good (R177) and both seats read the same batch in both games.
        for viewer in [PlayerId::P1, PlayerId::P2] {
            assert_eq!(place_of_recruited(&on_top, viewer), -1);
            assert_eq!(place_of_recruited(&second, viewer), -1);
            assert_eq!(discounts(&second, viewer), discounts(&on_top, viewer));
        }
    }
}

mod r223_instance_ids_and_the_order_a_deck_was_submitted_in {
    use super::*;

    // `apps/server`'s store hands `createGame` each deck ordered by card id (`app.resolve_deck`,
    // "a deck saved in one order comes back sorted"), and `createGame` numbers every card in that
    // order before §2.1 shuffles the library. This test cannot use `scenario()`, which numbers its
    // cards in its own setup order: it plays the engine's own path, `createGame` → `beginGame` →
    // mulligans → turns, as `replay.fold` and the server do.
    //
    // p2's deck is 18 cards costing 1 and #98 Heroic Power (a Quickdraw card, so it starts in the
    // opening hand whatever the shuffle does), plus one card p2 never shows: #1 Big D-fender, which
    // sorts before every other card of the deck, or #100 Ceaseless Void, which sorts after them all.
    // p1 only ever ends its turn; p2 plays Heroic Power as soon as it can pay its X, and nothing else.
    const P1_DECK: [&str; 20] = [
        "core-003", "core-004", "core-005", "core-007", "core-008", "core-010", "core-011", "core-015",
        "core-018", "core-023", "core-031", "core-035", "core-036", "core-039", "core-041", "core-044",
        "core-048", "core-050", "core-060", "core-062",
    ];
    const P2_SHARED: [&str; 19] = [
        "core-003", "core-004", "core-005", "core-007", "core-008", "core-011", "core-015", "core-023",
        "core-031", "core-035", "core-036", "core-044", "core-050", "core-062", "core-063", "core-081",
        "core-082", "core-086", HEROIC_POWER,
    ];

    /// The order the server's store returns a deck in: by card id.
    fn sorted_deck(ids: &[&str]) -> Vec<String> {
        let mut deck: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        deck.sort();
        deck
    }

    /// The two decks, as the store returns them, with `hidden` as p2's one card that never shows.
    fn deck_options(seed: &str, hidden: &str) -> CreateGameOptions {
        let mut p2 = P2_SHARED.to_vec();
        p2.push(hidden);
        json_as(json!({ "seed": seed, "decks": [sorted_deck(&P1_DECK), sorted_deck(&p2)] }))
    }

    fn heroic_power_of(state: &GameState) -> Option<String> {
        state
            .players
            .p2
            .backrow
            .iter()
            .flatten()
            .find(|card| card.def_id == HEROIC_POWER)
            .map(|card| card.id.clone())
    }

    fn act(state: &mut GameState, nonce: &mut u32, player: PlayerId, body: ActionBody) {
        let result = reduce(state, &Action::new(body.clone(), player, format!("r6-{nonce}")));
        *nonce += 1;
        if let Some(error) = result.error {
            panic!("{player} {}: {error}", body.action_type());
        }
        *state = result.state;
    }

    fn game_with_hidden(hidden: &str) -> GameState {
        let mut state = begin_game(&create_game(&deck_options("r6-deck-order", hidden))).state;
        let mut nonce = 0;
        let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
        act(&mut state, &mut nonce, PlayerId::P1, ActionBody::Mulligan { keep });
        let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
        act(&mut state, &mut nonce, PlayerId::P2, ActionBody::Mulligan { keep });
        for _guard in 0..16 {
            if heroic_power_of(&state).is_some() {
                break;
            }
            if state.result.is_some() || state.pending.is_some() {
                break;
            }
            if state.active == PlayerId::P1 {
                act(&mut state, &mut nonce, PlayerId::P1, ActionBody::EndTurn);
                continue;
            }
            let power = state
                .players
                .p2
                .hand
                .iter()
                .find(|card| card.def_id == HEROIC_POWER)
                .map(|card| card.id.clone());
            let play = legal_actions(&state, PlayerId::P2).into_iter().find(|body| {
                matches!(body, ActionBody::Play { instance_id, .. } if Some(instance_id) == power.as_ref())
            });
            act(&mut state, &mut nonce, PlayerId::P2, play.unwrap_or(ActionBody::EndTurn));
        }
        state
    }

    /// The id `createGame` gives p2's Heroic Power, with `hidden` as p2's one card that never shows.
    fn power_id_at(seed: &str, hidden: &str) -> Option<String> {
        create_game(&deck_options(seed, hidden))
            .players
            .p2
            .library
            .iter()
            .find(|card| card.def_id == HEROIC_POWER)
            .map(|card| card.id.clone())
    }

    fn seeds() -> Vec<String> {
        (0..200).map(|at| format!("r6-deck-order-{at}")).collect()
    }

    #[test]
    fn r223_r97_a_cards_instance_id_does_not_tell_the_opponent_where_it_sorts_in_its_owners_deck() {
        jackioh_cards::register_all();
        // The id reaches the opponent: p2 plays Heroic Power and p1's view names it (§10.8, R97).
        let game = game_with_hidden("core-100");
        let id = must(heroic_power_of(&game), "p2's Heroic Power on the field");
        assert_eq!(
            view_for(&game, PlayerId::P1)
                .opponent
                .hero
                .powers
                .first()
                .map(|power| power.instance_id.clone()),
            Some(id)
        );

        // Numbered in the order the store sorts a deck in, the power was c40 when p2's hidden card sorts
        // before it (#1) and c39 when it sorts after it (#100): the id was its rank. The finder asked for
        // one id in both games under the same seed, which no numbering can give — any order a card takes
        // among its deck's shifts with the cards around it. What §9.1 asks is that the id tell p1 nothing,
        // and the seed that orders the numbers is as hidden as the one that shuffles the library (§2.1):
        // whatever id p1 reads, the other game shows it under some seed, and the id is not the rank.
        let seeds = seeds();
        for hidden in ["core-001", "core-100"] {
            let other = if hidden == "core-001" { "core-100" } else { "core-001" };
            let seen = power_id_at(seeds.first().map_or("", String::as_str), hidden);
            assert!(
                seeds.iter().any(|seed| power_id_at(seed, other) == seen),
                "{} is possible either way",
                seen.as_deref().unwrap_or("undefined")
            );
            let ids: IndexSet<Option<String>> = seeds.iter().map(|seed| power_id_at(seed, hidden)).collect();
            assert!(ids.len() > 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lens L10): a random Make Radiant's cue count, and the ids a Replace mints in a library.
// ---------------------------------------------------------------------------

const RAPID: &str = "core-010";
const TUTOR: &str = "core-051";

fn id_number(id: &str) -> i64 {
    let digits = id.strip_prefix('c').unwrap_or(id);
    digits
        .parse::<i64>()
        .unwrap_or_else(|_| panic!("not an instance id: {id}"))
}

mod r177_a_random_make_radiant_over_a_hidden_hand {
    use super::*;

    /// p1 plays Stockpile; its first draw is #27 Blood Ridden Glowy Jelly Bean, which casts itself and
    /// makes a random non-Radiant card of p1's hand Radiant (R60). p1's hand then holds two cards, both
    /// Radiant or both not — hidden from p2 (§9.1, §10.8).
    fn blood_ridden_game(hand_radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "hunt-r7-blood-ridden",
            "p1": {
                "hand": [STOCKPILE, { "def": HIT_JOB, "radiant": hand_radiant }, { "def": RAPID, "radiant": hand_radiant }],
                "library": [BLOOD_RIDDEN, SEVEN_SEVEN, SEVEN_SEVEN, SEVEN_SEVEN],
            },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
        }));
        s.play(STOCKPILE, json!({}));
        s
    }

    #[test]
    fn r177_r60_27s_cue_does_not_tell_the_opponent_whether_p1s_hidden_hand_was_already_all_radiant() {
        jackioh_cards::register_all();
        let plain = blood_ridden_game(false);
        let radiant = blood_ridden_game(true);

        // The cast happened in both games (a public play), and p1's hand ends up the same size.
        for s in [&plain, &radiant] {
            assert!(
                events_of(&s.view("p2"), GameEventType::CardPlayed)
                    .iter()
                    .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == BLOOD_RIDDEN))
            );
        }
        assert_eq!(radiant.view("p2").opponent.hand, plain.view("p2").opponent.hand);

        // R177: "a cue for the changed cards only would count the Radiant ones". R60 narrows the random
        // pick to non-Radiant cards, so a cue only for a card that changed tells p2 whether any of p1's
        // hidden hand cards was still non-Radiant. p2 must not be able to tell the two games apart.
        indistinguishable(PlayerId::P2, &plain, &radiant);
    }
}

mod r223_instance_ids_transmogulate_gives_a_library {
    use super::*;

    #[test]
    fn r223_the_ids_transmogulate_mints_for_p1s_library_do_not_tell_p1_where_a_revealed_library_card_lies() {
        jackioh_cards::register_all();
        let sixteen_hit_jobs = [HIT_JOB; 16];
        let mut s = scenario(json!({
            "seed": "hunt-r7-transmog-ids",
            "p1": {
                "hand": [TRANSMOGULATE, RAPID],
                // A unit (replaced by a Legendary one) that can still switch keeps §2.5's auto-end away.
                "field": [SEVEN_SEVEN],
                "library": sixteen_hit_jobs,
                "graveyard": [HIT_JOB],
            },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB, HIT_JOB] },
        }));

        s.play(TRANSMOGULATE, json!({}));
        // The Tutor comes to hand only now: one held while Transmogulate resolved would have been
        // replaced with the rest of the hand (R365).
        let tutor = new_instance(s.state_mut(), TUTOR, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        s.state_mut().players.p1.hand.push(tutor);

        // What p1 reads after the Replace: the graveyard card's replacement is public, and the library is
        // a count. R35 walks the hand, the library and then the graveyard (R365), so if the replacements
        // were numbered in that walk the library's ids are the block just below the graveyard one's.
        let after_replace = s.view("p1");
        let gy_replacement = must(
            events_of(&after_replace, GameEventType::Transformed)
                .into_iter()
                .find_map(|event| match event {
                    GameEvent::Transformed { new_instance_id, .. } if new_instance_id != HIDDEN => Some(new_instance_id),
                    _ => None,
                }),
            "the graveyard card's public replacement",
        );
        let library_count = after_replace.you.library_count;
        let first_library_id = id_number(&gy_replacement) - i64::from(library_count);

        // #51 Private Tutor reveals library cards to p1 as prompt options (§10.8). Pick the type and
        // bracket with the most matches so the reveal shows as many cards as it can.
        s.play(TUTOR, json!({}));
        let library = s.pile("p1", "library");
        let type_prompt = must(s.state().pending.clone(), "the type prompt");
        let best_type = most_matching(&type_prompt, |want| {
            library
                .iter()
                .filter(|card| type_matches(s.state(), &card.def_id, want))
                .count()
        });
        s.answer(json!(best_type));
        let bracket_prompt = must(s.state().pending.clone(), "the bracket prompt");
        let best_bracket = most_matching(&bracket_prompt, |want| {
            library
                .iter()
                .filter(|card| {
                    type_matches(s.state(), &card.def_id, &best_type)
                        && in_bracket(effective_cost(s.state(), card, Default::default()), want)
                })
                .count()
        });
        s.answer(json!(best_bracket));

        let reveal = prompt_for_viewer(s.view("p1").pending, "p1's reveal prompt", "the reveal should be p1's");
        let revealed: Vec<String> = reveal
            .options
            .iter()
            .filter_map(|option| option.instance_id.clone())
            .collect();
        assert!(revealed.len() >= 2);

        // §9.1 and §10.8: "the rest of the library stays hidden from both" — its order included, for its
        // own player too (§3's "Nobody"). R223: an instance id says nothing of where its card came from.
        // The option's id is the answer's handle and is p1's to read; what it must not do is give p1 the
        // card's place. Reading each revealed card's place off its id must not give its real place — the
        // two cards p1 does not take stay in the library, where p1 would know when each comes up.
        let ids: Vec<String> = s.pile("p1", "library").into_iter().map(|card| card.id).collect();
        let read_off_the_id: Vec<i64> = revealed.iter().map(|id| id_number(id) - first_library_id).collect();
        let actual: Vec<i64> = revealed
            .iter()
            .map(|id| ids.iter().position(|at| at == id).map_or(-1, |at| at as i64))
            .collect();
        assert_ne!(read_off_the_id, actual);
    }
}

/// `defOf(s.state, defId).type` against a Tutor type option ("Trap" covers Field Traps too).
fn type_matches(state: &GameState, def_id: &str, want: &str) -> bool {
    let type_ = def_of(Some(state), def_id).type_;
    if want == "Trap" {
        type_ == CardType::Trap || type_ == CardType::FieldTrap
    } else {
        type_.as_str() == want
    }
}

/// The mode option with the most library matches.
fn most_matching(pending: &PendingChoice, count: impl Fn(&str) -> usize) -> String {
    let mut options: Vec<String> = pending
        .options
        .iter()
        .filter_map(|option| match &option.selection {
            Selection::Mode { option } => Some(option.clone()),
            _ => None,
        })
        .collect();
    options.sort_by(|a, b| count(b.as_str()).cmp(&count(a.as_str())));
    must(options.into_iter().next(), "a mode option")
}

fn in_bracket(cost: i32, bracket: &str) -> bool {
    if bracket == "0-1" {
        return cost <= 1;
    }
    if bracket == "4+" {
        return cost >= 4;
    }
    bracket.parse::<i32>().is_ok_and(|n| cost == n)
}

// ---------------------------------------------------------------------------
// #28 Knockoff Temu Glowy Jelly Bean: where a hidden Make Radiant landed
// ---------------------------------------------------------------------------

mod r177_where_a_random_make_radiant_over_hidden_zones_landed {
    use super::*;

    /// p1 plays #28 ("2 random cards among your library, hand and field become Radiant", R60). p1's
    /// hand then holds one Hit Job, Radiant already or not, and the library three 4-mana 7/7s, none
    /// Radiant; p1 has nothing on the field. Neither the hand nor the library is p2's to read (§9.1).
    /// With this seed the base-face Hit Job is one of the two picks.
    fn knockoff_game(hand_radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r8-l10-knockoff-0",
            "p1": {
                "hand": [KNOCKOFF, { "def": HIT_JOB, "radiant": hand_radiant }],
                "library": [SEVEN_SEVEN, SEVEN_SEVEN, SEVEN_SEVEN],
            },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(KNOCKOFF, json!({}));
        s
    }

    #[test]
    fn r177_r60_28s_cues_do_not_tell_p2_whether_p1s_hidden_hand_card_was_already_radiant() {
        jackioh_cards::register_all();
        let plain = knockoff_game(false);
        let radiant = knockoff_game(true);

        // Two picks in both games, none of them on a card p2 may read, and the Hit Job ends Radiant in
        // both: picked in one game, already Radiant in the other.
        for s in [&plain, &radiant] {
            let cues = events_of(&s.view("p2"), GameEventType::RadiantSet);
            assert_eq!(cues.len(), 2);
            assert!(must(s.hand("p1").into_iter().next(), "p1's Hit Job").radiant);
        }

        // R177: a cue on a card p2 may not read must not count p1's hidden Radiant cards. The zone a
        // redacted `radiantSet` carries says whether the pick landed in the hand or the library, so a
        // "hand" cue tells p2 that p1's hand still held a non-Radiant card.
        indistinguishable(PlayerId::P2, &plain, &radiant);
    }

    /// The same pick over "field" (§8 #28): p1's lane-2 Sheepish is face-down, so only p1 reads it
    /// (R33), and it is Radiant already or not. With this seed the base-face trap is one of the picks
    /// (R242 draws the owner's hidden cards' share of the pick by the groups' sizes alone).
    fn trap_game(trap_radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r8-l10-knockoff-trap-5",
            "p1": {
                "hand": [KNOCKOFF, STOCKPILE],
                "backrow": [{ "def": SHEEPISH, "lane": 2, "radiant": trap_radiant }],
                "library": [SEVEN_SEVEN, SEVEN_SEVEN, SEVEN_SEVEN],
            },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(KNOCKOFF, json!({}));
        s
    }

    #[test]
    fn r177_r33_r60_28s_cues_do_not_tell_p2_whether_p1s_face_down_trap_was_already_radiant() {
        jackioh_cards::register_all();
        let plain = trap_game(false);
        let radiant = trap_game(true);

        for s in [&plain, &radiant] {
            assert_eq!(
                json_of(&s.view("p2").opponent.backrow[1]),
                json!({ "faceDown": true, "cost": 1 })
            );
            assert_eq!(events_of(&s.view("p2"), GameEventType::RadiantSet).len(), 2);
            assert!(must(s.backrow("p1", 2), "p1's trap").radiant);
        }

        // Which of p1's hidden cards the owner's share of the pick lands on hangs on their faces (R60
        // picks the non-Radiant ones, R242), so a cue whose zone named the face-down trap's lane would
        // tell p2 the trap was base-face, the face R33 keeps from p2: the view says only whose it was.
        indistinguishable(PlayerId::P2, &plain, &radiant);
    }
}

// ---------------------------------------------------------------------------
// #23 Reoccurring Dream: the roll on an all-Radiant hand
// ---------------------------------------------------------------------------

mod r177_23s_chance_on_a_hidden_hand {
    use super::*;

    /// p1 plays #23 with one other card in hand, Radiant or not. With this seed the 30% succeeds.
    fn dream_game(hand_radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r8-l10-dream-8",
            "p1": { "hand": [DREAM, { "def": HIT_JOB, "radiant": hand_radiant }], "library": [SEVEN_SEVEN] },
            "p2": { "hand": [STOCKPILE], "library": [HIT_JOB] },
        }));
        s.play(DREAM, json!({}));
        s
    }

    #[test]
    fn r177_r60_r129_23s_cue_does_not_tell_p2_whether_p1s_hidden_hand_was_already_all_radiant() {
        jackioh_cards::register_all();
        let plain = dream_game(false);
        let radiant = dream_game(true);

        // The roll succeeded in the game with a base-face Hit Job, which became Radiant.
        assert!(
            must(
                plain.hand("p1").into_iter().find(|card| card.def_id == HIT_JOB),
                "p1's Hit Job"
            )
            .radiant
        );
        assert!(
            plain
                .view("p2")
                .events
                .iter()
                .any(|event| event.event_type() == GameEventType::RadiantSet)
        );

        // R177 lists #23 among the random picks whose unmade picks are cued on the zone's Radiant cards,
        // "so an all-Radiant hand … is cued as a hand the pick changed". #23 skips its roll when the
        // hand holds nothing it could change, so it never cues then, and a cue tells p2 the hand held a
        // non-Radiant card.
        indistinguishable(PlayerId::P2, &plain, &radiant);
    }
}

// ---------------------------------------------------------------------------
// R119's bookkeeping on `cardResolved`
// ---------------------------------------------------------------------------

mod r119_the_arrivals_a_plays_card_resolved_names_stay_the_engines {
    use super::*;

    #[test]
    fn r119_r33_r97_card_resolveds_arrived_during_which_can_name_a_face_down_trap_the_plays_recruit_set_reaches_neither_seats_view()
     {
        jackioh_cards::register_all();
        // Classic #60 Pile On's Radiant face, "Recruit every permanent in your deck": a play whose Recruit
        // sets a trap as it resolves.
        let mut s = scenario(json!({
            "seed": "edge-r8-hp-honeypot",
            "p1": {
                "hand": [{ "def": PILE_ON, "radiant": true }, MR_VANILLA],
                "library": [HONEYPOT, STOCKPILE, STOCKPILE],
                "mana": 8,
            },
            "p2": {
                "hand": [MR_VANILLA],
                "library": [MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA],
            },
        }));
        let power = must(s.hand("p1").into_iter().next(), "p1's Pile On");

        s.play(&power, json!({}));

        // The Recruit set the Bear Honeypot face-down on p1's backrow while the play resolved, so the
        // raw cardResolved names it among the play's arrivals.
        let honeypot = must(
            (1..=5)
                .filter_map(|lane| s.backrow("p1", lane))
                .find(|card| card.def_id == HONEYPOT),
            "the recruited Bear Honeypot",
        );
        assert_ne!(honeypot.face_up, Some(true));
        let resolved: Vec<Option<Vec<String>>> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardResolved {
                    instance_id,
                    arrived_during,
                    ..
                } if *instance_id == power.id => Some(arrived_during.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(resolved, vec![Some(vec![honeypot.id.clone()])]);

        // Neither seat's view carries the field, and p2's does not name p1's face-down trap through it.
        for seat in [PlayerId::P1, PlayerId::P2] {
            assert!(!events_of(&s.view(seat), GameEventType::CardResolved).is_empty());
            assert!(!json_text(&s.view(seat).events).contains("arrivedDuring"));
        }
        assert!(!json_text(&s.view("p2").events).contains(&format!("\"{}\"", honeypot.id)));
    }
}

// ---------------------------------------------------------------------------
// Round 9: the mulligan's returned cards and #28's cues (R224, R177)
// ---------------------------------------------------------------------------

fn fixture_def(id: &str, type_: CardType) -> CardDef {
    let face = if type_ == CardType::Unit {
        json!({ "attack": 2, "health": 2, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": type_.as_str(),
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

/// TS's module `let setupNonce`.
static SETUP_NONCE: AtomicU32 = AtomicU32::new(0);

fn act_as(state: &GameState, player: PlayerId, body: ActionBody) -> GameState {
    let nonce = SETUP_NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let result = reduce(state, &Action::new(body, player, format!("edge-r9-view-{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Every id or def id an event names, for "does this view name X" checks.
fn named(event: &GameEvent) -> Vec<String> {
    let mut out = Vec::new();
    if let Value::Object(fields) = json_of(event) {
        for (key, value) in fields {
            if let Value::String(text) = &value
                && (key.ends_with("Id") || key.ends_with("Ids"))
            {
                out.push(text.clone());
            }
            if let Value::Array(items) = &value
                && key.ends_with("Ids")
            {
                out.extend(items.iter().filter_map(|item| item.as_str().map(str::to_string)));
            }
        }
    }
    out
}

/// TS `registeredScripts()`: the registry as `registerScripts` last set it — this thread's testkit
/// override once a fixture is in (SURFACE §8), the production registry before.
fn registered_scripts_now() -> IndexMap<String, CardScripts> {
    match scripts_override() {
        Some(scripts) => scripts.clone(),
        None => registered_scripts().clone(),
    }
}

// ---------------------------------------------------------------------------
// The mulligan's returned cards while setup waits on a question (R224, R97)
// ---------------------------------------------------------------------------

const ASKING: &str = "edge-r9-view-asks";

/// A card whose start-of-game clause asks as it arrives in a hand (R151), once: §2.1 step 4 runs it
/// again. Setup draws no cast-on-draw card (R635), so this is what can still make it wait (R224).
fn asking(state: &mut GameState) {
    state
        .transient_defs
        .insert(ASKING.to_string(), fixture_def(ASKING, CardType::Spell));
    let script = Script {
        start_of_game: Some(hook(|ctx| {
            let asked = ctx
                .live_self()
                .and_then(|card| card.memory.get("asked"))
                .and_then(Value::as_bool)
                == Some(true);
            if asked {
                vec![]
            } else {
                vec![
                    remember(json_as(json!({ "key": "asked", "value": true }))),
                    choose_mode(json_as(json!({
                        "options": ["ok"],
                        "step": "ok",
                        "prompt": "the clause's question",
                    }))),
                ]
            }
        })),
        resume: [("ok", hook(|_ctx| vec![]))].into_iter().collect(),
        ..Script::default()
    };
    let mut scripts = registered_scripts_now();
    scripts.insert(
        ASKING.to_string(),
        CardScripts {
            base: script.clone(),
            radiant: script,
        },
    );
    register_scripts(scripts);
}

fn setup_p1_deck() -> Vec<String> {
    (0..20).map(|at| format!("core-{:03}", at + 1)).collect()
}

fn setup_p2_deck() -> Vec<String> {
    (0..20).map(|at| format!("core-{:03}", at + 30)).collect()
}

mod r224_r97_a_card_the_mulligan_returned_while_setup_waits {
    use super::*;

    #[test]
    fn r224_r97_p2s_returned_opening_card_stays_unread_by_p1_while_p2s_replacements_clause_asks() {
        jackioh_cards::register_all();
        // p1's opening draw hits an asking card, so p2's opening deal happens inside p1's answer: a
        // recorded action, whose `drawn` events for p2's cards are in p1's view, redacted (R97).
        let mut begun: Option<GameState> = None;
        for at in 0..300 {
            let candidate = format!("edge-r9-view-deal-{at}");
            let mut game: GameState =
                create_game(&json_as(json!({ "seed": candidate, "decks": [setup_p1_deck(), setup_p2_deck()] })));
            asking(&mut game);
            let asker = new_instance(&mut game, ASKING, PlayerId::P1, Zone::Library { player: PlayerId::P1 });
            game.players.p1.library[0] = asker;
            let state = begin_game(&game).state;
            if state
                .pending
                .as_ref()
                .is_some_and(|pending| pending.kind == PromptKind::Mode && pending.player_id == PlayerId::P1)
            {
                begun = Some(state);
                break;
            }
        }
        let mut state = must(begun, "a seed whose opening draw reaches the asking card");
        let first = must(state.pending.clone(), "p1's question");
        state = act_as(
            &state,
            PlayerId::P1,
            ActionBody::Answer {
                choice_id: first.id.clone(),
                selection: vec![Selection::Mode { option: "ok".to_string() }],
            },
        );
        assert_eq!(mulligan_owed(&state), vec![PlayerId::P1, PlayerId::P2]);
        assert!(!state.players.p2.hand.is_empty());

        // p1 keeps its hand, sealed until p2 answers (R265).
        let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
        state = act_as(&state, PlayerId::P1, ActionBody::Mulligan { keep });
        assert_eq!(mulligan_owed(&state), vec![PlayerId::P2]);

        // p2 returns one card, and its replacement draw is the asking card, so setup waits (R224) with
        // the returned card in no pile until it goes back.
        let returned = must(state.players.p2.hand.first().cloned(), "a card for p2 to return");
        let cod = new_instance(&mut state, ASKING, PlayerId::P2, Zone::Library { player: PlayerId::P2 });
        state.players.p2.library.insert(0, cod);
        let keep: Vec<String> = state.players.p2.hand.iter().skip(1).map(|card| card.id.clone()).collect();
        state = act_as(&state, PlayerId::P2, ActionBody::Mulligan { keep });
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P2));

        // The deal's event naming the returned card is still in p1's window.
        let view = view_for(&state, PlayerId::P1);
        let deal = view
            .events
            .iter()
            .filter(|event| matches!(event, GameEvent::Drawn { player: PlayerId::P2, .. }))
            .count();
        assert!(deal > 0);

        // §9.1: p2's hand is hidden from p1, and the card is on its way back to p2's library, hidden
        // from both. Nothing in p1's view may name it.
        let leaks: Vec<&GameEvent> = view
            .events
            .iter()
            .filter(|event| {
                let names = named(event);
                names.contains(&returned.id) || names.contains(&returned.def_id)
            })
            .collect();
        assert!(leaks.is_empty(), "{}", json_text(&leaks));
    }
}

// ---------------------------------------------------------------------------
// #28's picks and its cues, in the order the other seat sees them (R177, R60, §9.1)
// ---------------------------------------------------------------------------

/// p2's view of the `radiantSet` events #28's play made, redacted as p2 reads them: TS's
/// `{ order, events }` as a pair.
fn knockoff_cues(seed: &str, hand_radiant: bool) -> (Vec<String>, Vec<GameEvent>) {
    let mut s = scenario(json!({
        "seed": seed,
        "p1": {
            "mana": 2,
            // The one card left in p1's hand once #28 is played, Radiant or not; the library is empty.
            "hand": ["core-028", { "def": "core-016", "radiant": hand_radiant }],
            "library": [],
            "field": ["core-025"],
        },
    }));
    s.play("core-028", json!({}));
    let unit_id = must(s.unit("p1", 1), "p1's unit").id;
    let events = events_of(&s.view("p2"), GameEventType::RadiantSet);
    let order = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::RadiantSet { instance_id, .. } => Some(if *instance_id == unit_id {
                "unit".to_string()
            } else {
                instance_id.clone()
            }),
            _ => None,
        })
        .collect();
    (order, events)
}

mod r177_r60_28s_cues_keep_the_hidden_faces_hidden {
    use super::*;

    #[test]
    fn r177_r60_28s_cue_on_an_all_radiant_hand_can_come_in_any_order_a_pick_could() {
        jackioh_cards::register_all();
        // Two worlds p2 cannot tell apart by what changed: p1's one hidden hand card is base-face (A)
        // or already Radiant (B). Either way #28's two picks make p1's public unit Radiant and cue one
        // hidden card in p1's hand, since R177 cues the pick R60 could not make on the Radiant card, "so
        // an all-Radiant hand ... is cued as a hand the pick changed". The order the two arrive in must
        // not tell the worlds apart either: every order p2 can see in A must be one B can produce.
        let seeds: Vec<String> = (0..40).map(|at| format!("edge-r9-view-28-{at}")).collect();
        let orders = |radiant: bool| -> IndexSet<String> {
            seeds
                .iter()
                .map(|seed| json_text(&knockoff_cues(seed, radiant).0))
                .collect()
        };
        let in_a = orders(false);
        let in_b = orders(true);
        // Sanity: both worlds show p2 the same outcome, the unit and one hidden cue.
        for order in in_a.iter().chain(in_b.iter()) {
            let mut parsed: Vec<String> = serde_json::from_str(order).expect("an order is a JSON list");
            parsed.sort();
            assert_eq!(parsed, vec![HIDDEN_ID.to_string(), "unit".to_string()]);
        }
        // A cue that always trailed the public pick said "the hand was all Radiant" whenever a pick led;
        // R242 sends the public card's event first and the hand's after it, in both worlds alike.
        let only_in_a: Vec<&String> = in_a.iter().filter(|order| !in_b.contains(*order)).collect();
        assert!(
            only_in_a.is_empty(),
            "A {} B {}",
            json_text(&in_a.iter().collect::<Vec<_>>()),
            json_text(&in_b.iter().collect::<Vec<_>>())
        );
    }

    #[test]
    fn r177_r60_28s_cue_for_a_pick_it_could_not_make_lands_where_its_owner_cannot_read_it_either() {
        jackioh_cards::register_all();
        // p1's hand holds two cards that are already Radiant, and its library one card: base-face in
        // world A, Radiant in world B. §3 and §9.1: a library is read by nobody, p1 included, so p1
        // must not learn which world it is in. #28 wants two picks and R177 cues the ones R60 could not
        // make; if the cues go to p1's own hand cards, which p1 reads, the number of them spells out
        // how many of p1's library cards were base-face.
        fn cues_for(library_radiant: bool) -> Vec<GameEvent> {
            let mut s = scenario(json!({
                "seed": "edge-r9-view-28-owner",
                "p1": {
                    "mana": 2,
                    "hand": ["core-028", { "def": "core-016", "radiant": true }, { "def": "core-005", "radiant": true }],
                    "library": [{ "def": "core-010", "radiant": library_radiant }],
                    "field": [],
                },
            }));
            s.play("core-028", json!({}));
            events_of(&s.view("p1"), GameEventType::RadiantSet)
        }
        let world_a = cues_for(false);
        let world_b = cues_for(true);
        fn shape(events: &[GameEvent]) -> Vec<String> {
            let mut shape: Vec<String> = events
                .iter()
                .map(|event| match event {
                    GameEvent::RadiantSet { instance_id, zone, .. } => format!(
                        "{}:{}",
                        zone.z(),
                        if instance_id == HIDDEN_ID { "unread" } else { "read" }
                    ),
                    _ => String::new(),
                })
                .collect();
            shape.sort();
            shape
        }
        assert_eq!(world_a.len(), 2);
        assert_eq!(
            shape(&world_b),
            shape(&world_a),
            "A {}\nB {}",
            json_text(&world_a),
            json_text(&world_b)
        );
    }
}

// ---------------------------------------------------------------------------
// Round 10: a random pick over public and hidden cards (R242), and what the view carries (R243)
// ---------------------------------------------------------------------------

const TEMPO_TIMMY: &str = "core-011";

mod r242_28s_pick_over_public_and_hidden_cards {
    use super::*;

    /// p1 plays #28 with one card left in hand and two in the library, all base-face in world A and all
    /// Radiant in world B, and one public unit on the field. Neither the hand nor the library is p2's to
    /// read (§9.1).
    fn odds_game(seed: &str, hidden_radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 2,
                "hand": [KNOCKOFF, { "def": HIT_JOB, "radiant": hidden_radiant }],
                "library": [
                    { "def": SEVEN_SEVEN, "radiant": hidden_radiant },
                    { "def": SEVEN_SEVEN, "radiant": hidden_radiant },
                ],
                "field": [TEMPO_TIMMY],
            },
            "p2": { "hand": [HIT_JOB], "library": [HIT_JOB] },
        }));
        s.play(KNOCKOFF, json!({}));
        s
    }

    /// What p2 can see #28 did: whether p1's public unit turned Radiant, and the cues' shape.
    fn outcome(s: &Scenario) -> String {
        let unit = must(s.view("p2").opponent.units.first().cloned().flatten(), "p1's unit");
        let cues: Vec<String> = events_of(&s.view("p2"), GameEventType::RadiantSet)
            .iter()
            .filter_map(|event| match event {
                GameEvent::RadiantSet { instance_id, zone, .. } => Some(if instance_id == HIDDEN_ID {
                    format!("hidden@{}", zone.z())
                } else {
                    "unit".to_string()
                }),
                _ => None,
            })
            .collect();
        json_text(&json!({ "unitRadiant": unit.radiant, "cues": cues }))
    }

    #[test]
    fn r242_r60_r177_28_picking_p1s_public_unit_or_not_does_not_tell_p2_whether_p1s_hidden_cards_were_radiant() {
        jackioh_cards::register_all();
        let seeds: Vec<String> = (0..24).map(|at| format!("r10-l10-knockoff-odds-{at}")).collect();
        let world_a: IndexSet<String> = seeds.iter().map(|seed| outcome(&odds_game(seed, false))).collect();
        let world_b: IndexSet<String> = seeds.iter().map(|seed| outcome(&odds_game(seed, true))).collect();
        // R177's cues make an all-Radiant hidden pile look like a pile the pick changed: every outcome p2
        // can see in the base-face world must be one the all-Radiant world can produce too. Drawn from
        // the non-Radiant cards alone, the two picks always took the public unit when every hidden card
        // was Radiant, so a game where it was passed over told p2 that p1 held a base-face hidden card.
        let only_in_a: Vec<&String> = world_a.iter().filter(|seen| !world_b.contains(*seen)).collect();
        assert!(
            only_in_a.is_empty(),
            "A {}\nB {}",
            json_text(&world_a.iter().collect::<Vec<_>>()),
            json_text(&world_b.iter().collect::<Vec<_>>())
        );
        // And the pick is still random: some seeds pass the public unit over.
        assert!(world_a.iter().any(|seen| {
            serde_json::from_str::<Value>(seen).expect("an outcome is JSON")["unitRadiant"] == json!(false)
        }));
    }

    /// p1 holds a Hit Job and has Sheepish face-down in backrow lane 2 and Tempo Timmy in unit lane 1,
    /// public. Exactly one of the two hidden cards is base-face: the Hit Job in world A, the trap in
    /// world B. #28's two picks take that card and the unit in both worlds, so both end with the same
    /// faces everywhere.
    fn order_game(trap_base: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "r10-l10-knockoff-order",
            "p1": {
                "mana": 2,
                "hand": [KNOCKOFF, { "def": HIT_JOB, "radiant": trap_base }],
                "field": [TEMPO_TIMMY],
                "backrow": [{ "def": SHEEPISH, "lane": 2, "radiant": !trap_base }],
                "library": [],
            },
            "p2": { "hand": [HIT_JOB], "library": [HIT_JOB] },
        }));
        s.play(KNOCKOFF, json!({}));
        s
    }

    #[test]
    fn r242_r33_r177_28s_event_order_does_not_tell_p2_whether_its_hidden_pick_was_p1s_hand_card_or_p1s_face_down_trap() {
        jackioh_cards::register_all();
        let hand_pick = order_game(false);
        let trap_pick = order_game(true);
        for s in [&hand_pick, &trap_pick] {
            assert!(must(s.unit("p1", 1), "Timmy").radiant);
            assert!(must(s.backrow("p1", 2), "Sheepish").radiant);
            assert!(must(s.hand("p1").into_iter().next(), "Hit Job").radiant);
            assert_eq!(
                json_of(&s.view("p2").opponent.backrow[1]),
                json!({ "faceDown": true, "cost": 1 })
            );
        }
        // Both worlds end with every card Radiant; which hidden card was base-face is the face R33 and
        // §9.1 keep from p2. In the zones' own order the backrow comes after the unit row, so a hidden
        // pick after the public unit's could only have been the face-down trap; R242 sends the public
        // cards' events first and then p1's hidden ones.
        indistinguishable(PlayerId::P2, &hand_pick, &trap_pick);
    }
}

mod r243_what_the_view_carries_of_a_card_beyond_its_printed_face {
    use super::*;

    /// The viewer's own hand card `id` as its JSON, so a test reads its fields by name.
    fn hand_entry(view: &PlayerView, id: &str) -> Value {
        let HandView::Cards(hand) = &view.you.hand else {
            panic!("expected the viewer's own hand in full");
        };
        json_of(must(
            hand.iter().find(|card| card.instance_id == id),
            &format!("{id} in the viewer's hand"),
        ))
    }

    #[test]
    fn r243_a_corpse_eater_that_fed_in_its_owners_hand_shows_its_owner_the_stats_it_now_has() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": {
                "hand": [CORPSE_EATER, "core-021"],
                "field": [{ "def": SEVEN_SEVEN, "lane": 1 }],
                "library": [MR_VANILLA, MR_VANILLA],
            },
            "p2": { "hand": ["core-021"], "field": [{ "def": TEMPO_TIMMY, "lane": 1 }], "library": [MR_VANILLA] },
        }));
        let eater = g.card(CORPSE_EATER).clone();
        // The 7/7's Armor 7 eats Timmy's First Strike 3 whole; its 7 kills the 3/3 Timmy.
        g.attack(SEVEN_SEVEN, TEMPO_TIMMY);
        g.expect_in_zone(TEMPO_TIMMY, "graveyard");
        // §8 #89: in hand it gains the dead unit's attack and max health, 2/2 + 3/3.
        assert_eq!(g.card(&eater).buffs, AttackHealth { attack: 3, health: 3 });

        let entry = hand_entry(&g.view("p1"), &eater.id);
        assert_eq!(entry["attack"], json!(5), "the hand card's attack in p1's view");
        assert_eq!(entry["health"], json!(5), "the hand card's health in p1's view");
        // p2 sees a count of p1's hand, as before (§10.8).
        assert_eq!(g.view("p2").opponent.hand, HandView::Count { count: 2 });
    }

    #[test]
    fn r243_r43_r151_a_heroic_power_in_its_owners_hand_shows_the_power_it_rolled_which_its_cost_alone_does_not_name() {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": ["core-021"], "library": [HEROIC_POWER, MR_VANILLA, MR_VANILLA] },
            "p2": { "hand": ["core-021"], "library": [MR_VANILLA] },
        }));
        // The turn's draw puts the Heroic Power in hand, and R151 rolls its power as it arrives.
        g.start_turn();
        let power = must(
            g.hand("p1").into_iter().find(|card| card.def_id == HEROIC_POWER),
            "the drawn Heroic Power",
        );
        let rolled = must(power_of(&power), "a rolled power on the drawn Heroic Power");
        let entry = hand_entry(&g.view("p1"), &power.id);
        // The card costs (0) whatever it rolled (R752), so the cost in the view does not say which it is.
        assert_eq!(entry["cost"], json!(0));
        assert_eq!(
            entry["power"],
            json!(rolled.name.to_string()),
            "p1's view of the card names its power"
        );
    }

    #[test]
    fn r243_r77_r179_a_card_craft_a_card_fused_into_its_owners_hand_can_be_read_from_the_view_its_name_and_summed_stats_and_still_not_by_the_other_seat()
     {
        jackioh_cards::register_all();
        let mut g = scenario(json!({
            "p1": { "hand": ["core-092", "core-066", "core-021"], "library": [MR_VANILLA, MR_VANILLA] },
            "p2": { "hand": ["core-021"], "library": [MR_VANILLA] },
        }));
        let ingredients: Vec<CardInstance> = vec![g.card("core-092").clone(), g.card("core-066").clone()];
        let mut rng = create_rng(&g.state().seed, g.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        let crafted = {
            let mut sink = EngineSink::new(g.state_mut(), &mut events, &mut rng);
            must(
                fuse(
                    &mut sink,
                    FuseArgs {
                        ingredients,
                        to_hand: Some(PlayerId::P1),
                        ..Default::default()
                    },
                ),
                "the crafted card",
            )
        };
        let def = must(
            g.state().transient_defs.get(&crafted.def_id).cloned(),
            "the fused definition in match state",
        );
        assert_eq!(def.name, "Felinor Fiender + The Rock");
        assert_eq!([def.base.attack, def.base.health], [Some(15), Some(17)]);

        // The definition exists only in match state (R179): no catalog a client holds has it, so the
        // view is the only place its owner can read what they crafted.
        let own = g.view("p1");
        assert_eq!(
            own.defs
                .as_ref()
                .and_then(|defs| defs.get(&crafted.def_id))
                .map(|crafted_def| crafted_def.name.clone()),
            Some(def.name.clone())
        );
        assert_eq!(hand_entry(&own, &crafted.id)["attack"], json!(15));
        // It is still a hidden hand card for the other seat (§10.8): nothing names it there.
        assert!(!json_text(&g.view("p2")).contains(&def.name));
        assert!(
            g.view("p2")
                .defs
                .as_ref()
                .and_then(|defs| defs.get(&crafted.def_id))
                .is_none()
        );
    }
}

mod r227_r177_a_card_set_face_down_takes_a_fresh_id {
    use super::*;

    const REMINISCE: &str = "core-072";
    const RENO: &str = "core-053";

    /// p2 returns a Sheepish p1 watched go to the graveyard, and sets it again: TS's
    /// `{ s, oldId, newId }`.
    fn reset_sheepish() -> (Scenario, String, String) {
        let mut s = scenario(json!({
            "seed": "r227-reset-sheepish",
            "active": "p2",
            "p1": { "hand": [MR_VANILLA, RENO], "mana": 4, "library": [MR_VANILLA, MR_VANILLA, MR_VANILLA] },
            "p2": { "hand": [REMINISCE, RENO], "graveyard": [SHEEPISH], "mana": 4, "library": [MR_VANILLA, MR_VANILLA] },
        }));
        let old_id = must(
            s.state().players.p2.graveyard.first().map(|card| card.id.clone()),
            "Sheepish in p2's graveyard",
        );
        s.play(REMINISCE, json!({}));
        s.answer(json!(SHEEPISH));
        let back = must(
            s.state().players.p2.hand.iter().find(|card| card.id == old_id).cloned(),
            "Sheepish back in p2's hand",
        );
        s.play(back, json!({}));
        let new_id = must(s.backrow("p2", 1), "Sheepish set face-down").id;
        (s, old_id, new_id)
    }

    #[test]
    fn r227_the_setters_own_events_carry_the_old_id_as_former_id_so_its_client_can_find_the_hand_card() {
        jackioh_cards::register_all();
        let (s, old_id, new_id) = reset_sheepish();
        assert_ne!(new_id, old_id);

        let played: Vec<(String, Option<String>)> = events_of(&s.view("p2"), GameEventType::CardPlayed)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed {
                    instance_id,
                    def_id,
                    former_id,
                    ..
                } if def_id == SHEEPISH => Some((instance_id, former_id)),
                _ => None,
            })
            .collect();
        assert_eq!(played, vec![(new_id.clone(), Some(old_id.clone()))]);
        let summoned: Vec<(String, Option<String>)> = events_of(&s.view("p2"), GameEventType::Summoned)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    instance_id,
                    def_id,
                    former_id,
                    ..
                } if def_id == SHEEPISH => Some((instance_id, former_id)),
                _ => None,
            })
            .collect();
        assert_eq!(summoned, vec![(new_id, Some(old_id))]);
    }

    #[test]
    fn r227_the_other_seat_sees_neither_id_while_the_card_is_face_down_and_no_former_id_at_all() {
        jackioh_cards::register_all();
        let (s, old_id, new_id) = reset_sheepish();
        let view = json_text(&s.view("p1"));
        assert!(!view.contains(&format!("\"{old_id}\"")));
        assert!(!view.contains(&format!("\"{new_id}\"")));
        assert!(!view.contains("formerId"));
        // The Sheepish's return to hand named the old id, and follows the card to its face-down zone.
        let returned: Vec<GameEvent> = s
            .view("p1")
            .events
            .into_iter()
            .filter(|event| json_of(event).get("defId").and_then(Value::as_str) == Some(SHEEPISH))
            .collect();
        assert!(returned.is_empty(), "{returned:?}");
    }

    #[test]
    fn r227_r97_once_the_trap_fires_and_is_public_the_events_that_named_its_old_id_read_openly_again() {
        jackioh_cards::register_all();
        let (mut s, old_id, _new_id) = reset_sheepish();
        s.end_turn();
        assert_eq!(s.state().active, PlayerId::P1);
        s.play(MR_VANILLA, json!({}));
        assert!(
            s.state()
                .players
                .p2
                .graveyard
                .iter()
                .any(|card| card.def_id == SHEEPISH)
        );

        let naming: Vec<GameEvent> = s
            .view("p1")
            .events
            .into_iter()
            .filter(|event| json_text(event).contains(&format!("\"{old_id}\"")))
            .collect();
        assert!(naming.iter().any(|event| json_text(event).contains(SHEEPISH)));
    }
}
