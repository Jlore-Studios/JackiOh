//! #39 Recycling Initiative (SPEC §8.2; R4, R57, R65, R71, R78, R86, R126, R127, R133, R215, R275).
//! Spell, cost 0.
//!   Base:    "Exile this on play. End of turn: add a copy of every other card you played this
//!            turn to your hand"
//!   Radiant: "Exile this on play. End of turn: add a Radiant copy of every other card you played
//!            this turn to your hand; the copies cost 1 less" — §8's cell "Copies are Radiant and
//!            cost 1 less" (R275's raise: the copies' face and their price). The cell restates only
//!            what a copy is and costs, so the exile and the end-of-turn clause are kept unchanged
//!            (§8 Conventions).
//! R71: the log is read when the delayed effect RUNS, so cards played AFTER this one are copied too;
//! a delayed effect, not an `endOfTurn` hook, since the card is in exile by then and
//! `turn.triggerOrder` walks only units and the backrow. R86: an id whose instance ceased to exist (a
//! unit token that left the field, R11) is skipped; one that merely changed zone is still copied, so
//! nothing filters on `zone`. R133: the set of cards played, not the list of plays, so a card played,
//! bounced and replayed (#24) is copied ONCE. A flag is only ever set, never taken away (§5.2).
//! R126, R127: the continuation is registered ONCE, in the `resume` step table (`hook: RESUME_HOOK`),
//! and named by stored def id (the shape R76 gives #50 K-Pop Fanatic), so it comes back with the card
//! gone from play, which is why the id it needs travels in `data`.
//! The discount is R65's `costMod`, not a `costOverride`, which would make an X-cost copy free and
//! erase an embiggen price; it is `addToHand`'s `costMod` (the copy does not exist yet for
//! `setCostMod` to name), which R78 keeps in every zone. A copy a full hand burns reaches the
//! graveyard with its radiant flag and without the discount (§2.4, R4, R215).

use indexmap::IndexSet;
use jackioh_engine::effects::{add_to_hand, delay, exile};
use jackioh_engine::prelude::*;
use jackioh_engine::prompts::RESUME_HOOK;
use jackioh_engine::query::played_ids_this_turn;
use jackioh_engine::state::find_instance;

pub const ID: &str = "core-039";

/// What a face does to each copy: whether it is Radiant regardless and costs less. Base: a plain copy
/// at its printed price, radiant flag kept (R57). Radiant: a Radiant copy that costs (1) less, the
/// declared number `discount` (R386).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CopyTerms {
    radiant: bool,
}

const BASE_TERMS: CopyTerms = CopyTerms { radiant: false };

const RADIANT_TERMS: CopyTerms = CopyTerms { radiant: true };

/// The step the end-of-turn delayed effect re-enters (§10.6: `script.resume[step]`).
const COPY_STEP: &str = "copies";

/// The one thing the continuation captures: which play was this card's own (R71's "every other").
const SELF_KEY: &str = "selfId";

/// The copies' discount: none on the base face; on the Radiant face the card's `discount` as it stands
/// in exile, where the continuation finds it (R386 keeps its tuning there), or the printed one when the
/// card has ceased to exist (R127: the continuation still names its definition).
fn discount_of(ctx: &EffectContext<'_>, terms: CopyTerms) -> i32 {
    if terms.radiant { param(ctx, "discount") } else { 0 }
}

/// The captured id, narrowed rather than cast: `data` is JSON that crossed a phase boundary.
fn excluded_id(ctx: &EffectContext<'_>) -> Option<String> {
    if let Some(Value::String(captured)) = ctx.data.get(SELF_KEY) {
        return Some(captured.clone());
    }
    ctx.self_.as_ref().map(|card| card.id.clone())
}

/// R71, R86, R133: the other cards `player` played this turn that still exist, each once, in play order:
/// the cards the end of turn copies, and the count its preview shows (R2476000).
fn other_plays(state: &GameState, player: PlayerId, self_id: Option<&str>) -> Vec<CardInstance> {
    let mut seen: IndexSet<String> = IndexSet::new();
    let mut out = Vec::new();
    for id in played_ids_this_turn(state, player) {
        let id: String = id.to_string();
        if self_id == Some(id.as_str()) || !seen.insert(id.clone()) {
            continue;
        }
        if let Some(card) = find_instance(state, &id) {
            out.push(card.clone());
        }
    }
    out
}

/// R2476000: the words the count follows on both faces.
const OTHER_PLAYS: &str = "each other card you played this turn";

/// R2476000: in its owner's hand, the copies its end of turn would add were it played now: the other
/// cards played this turn so far. A floor, as cards played after it are copied too (R71). Plays are
/// public. Once played it is in exile, where no preview is asked.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Hand {
            return vec![];
        }
        let count = other_plays(ctx.state, ctx.controller, Some(ctx.self_.id.as_str())).len();
        vec![count_preview(OTHER_PLAYS, count as i32)]
    })
}

/// R71: the log is read here, when the delayed effect runs, so later plays are in it (the engine's
/// read-only `playedIdsThisTurn`, ids in play order). R86: an id whose instance no longer exists is
/// skipped rather than fizzled on. R133: a card played, bounced and replayed is two log entries and one
/// card, and "every other card you played this turn" is the set of cards, so each id yields one copy;
/// deduping by id leaves R86's skip untouched.
fn copies_of_other_plays(ctx: &EffectContext<'_>, terms: CopyTerms) -> Vec<Effect> {
    let discount = discount_of(ctx, terms);
    let mut out: Vec<Effect> = Vec::new();
    for card in other_plays(&*ctx.state, ctx.controller, excluded_id(ctx).as_deref()) {
        let mut args = json!({
            "defId": card.def_id,
            "player": "self",
            // R57: a fresh copy carries the radiant flag and nothing else; the radiant face sets it.
            "radiant": terms.radiant || card.radiant,
        });
        if discount != 0 {
            args["costMod"] = json!(-discount);
        }
        // R1300: a copy of a Chinese card is Chinese.
        if card.chinese == Some(true) {
            args["chinese"] = json!(true);
        }
        out.push(add_to_hand(json_as(args)));
    }

    out
}

/// The two faces differ only in what a copy is and costs.
fn recycling_initiative(terms: CopyTerms) -> Script {
    // R62's continuation, registered once: the `resume` step table the `delay` below names.
    let copy_step: Hook = hook(move |ctx| copies_of_other_plays(ctx, terms));

    Script {
        cry: Some(hook(|ctx| {
            let mut args = json!({
                "at": { "phase": "end", "player": "self" },
                "step": COPY_STEP,
                "hook": RESUME_HOOK,
            });
            if let Some(self_) = ctx.self_.as_ref() {
                args["data"] = json!({});
                args["data"][SELF_KEY] = json!(self_.id);
            }
            vec![
                // Armed before the exile, so the continuation records this instance while it still exists.
                delay(json_as(args)),
                exile(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        resume: IndexMap::from([(COPY_STEP, copy_step)]),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: recycling_initiative(BASE_TERMS),
        radiant: recycling_initiative(RADIANT_TERMS),
    }
}

// #39 Recycling Initiative — SPEC §8.2 row 39, BUILD M4-T4 row 39, R71, R62, R65.
// Exiled on play; end-of-turn adds copies of cards played this turn; radiant copies cost 1 less.
#[cfg(test)]
mod tests {
    use crate::js;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    const RECYCLING: &str = "core-039"; // Spell, 0
    const TIMMY: &str = "core-011"; // Unit, 1 — 3/3 Rush, First Strike, no Cry
    const MR_TOKEN: &str = "core-015"; // Unit, 1 — Cry: summon a Rush Token
    const BIG_D: &str = "core-001"; // Unit, 2 — 0/8, an aura and no Cry: nothing to fire on a replay
    const BREAD: &str = "core-018"; // Field Trap, 1 — fires in the end-of-turn trap window (R62)
    const FELINORS: &str = "core-012"; // Unit, 2 — 3/4, the body that kills a Rush Token on the strike-back
    const RUSH_TOKEN: &str = "core-t-rush"; // the unit-token CARD, playable from a hand (#75), 3/3 Rush
    const STOCKPILE: &str = "core-005"; // Spell, 1 — the spare card that keeps a turn meaningful (§2.5)
    const MENACE: &str = "core-019"; // library filler
    const POSTDOC: &str = "core-061"; // library filler

    /// Both sides keep a card in hand and cards in the library, so no turn auto-ends (§2.5, R82).
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE, TIMMY], "library": [MENACE, POSTDOC] })
    }

    fn with_spare(mut side: Value) -> Value {
        if let Value::Object(spare) = spare() {
            for (key, value) in spare {
                side[key.as_str()] = value;
            }
        }
        side
    }

    /// `register_all` first: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn count_in(s: &Scenario, player: &str, zone: &str, def_id: &str) -> usize {
        def_ids(&s.pile(player, zone)).iter().filter(|id| *id == def_id).count()
    }

    /// The index in the cumulative log of the first event matching, or -1.
    fn index_of(s: &Scenario, matches: impl Fn(&GameEvent) -> bool) -> i64 {
        s.events().iter().position(matches).map_or(-1, |at| at as i64)
    }

    fn added_to_hand_of(player: PlayerId, def_id: &'static str) -> impl Fn(&GameEvent) -> bool {
        move |event: &GameEvent| {
            matches!(
                event,
                GameEvent::AddedToHand { player: who, def_id: def, .. } if *who == player && def == def_id
            )
        }
    }

    /// What the client is told a hand card costs (§10.8): `effectiveCost`, so R65's `costMod` is in it.
    fn view_cost(s: &Scenario, player: &str, instance_id: &str) -> i64 {
        let view = js(&s.view(player));
        let hand = view["you"]["hand"]
            .as_array()
            .expect("viewCost: the viewer's own hand should be cards, not a count");
        let card = hand
            .iter()
            .find(|entry| entry["instanceId"] == instance_id)
            .unwrap_or_else(|| panic!("viewCost: {instance_id} is not in {player}'s hand view"));
        card["cost"].as_i64().expect("a hand card's cost is a number")
    }

    fn copy_in_hand(s: &Scenario, def_id: &str) -> CardInstance {
        s.pile("p1", "hand")
            .into_iter()
            .find(|card| card.def_id == def_id)
            .unwrap_or_else(|| panic!("no copy of {def_id} in p1's hand"))
    }

    /// R2476000: on either face, in hand, the copies its end of turn would add, after "each other card you
    /// played this turn": 0, then 1 once Timmy is played, and the end of turn adds exactly that one copy.
    #[test]
    fn r2476000_in_hand_it_previews_the_other_cards_played_this_turn_and_copies_that_many() {
        for radiant in [false, true] {
            let mut s = scn(json!({
                "p1": {
                    "hand": [TIMMY, { "def": RECYCLING, "radiant": radiant }, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                    "mana": 10,
                },
                "p2": spare(),
            }));
            let spell = s.card(RECYCLING).id.clone();
            let seen = |s: &Scenario| crate::preview_seen(s, PlayerId::P1, &spell);
            let label = "each other card you played this turn".to_string();
            assert_eq!(seen(&s), vec![(label.clone(), 0, None)], "radiant {radiant}");
            s.play(TIMMY, json!({ "zone": 1 }));
            assert_eq!(seen(&s), vec![(label.clone(), 1, None)], "radiant {radiant}");
            s.play(spell.as_str(), json!({}));
            assert!(seen(&s).is_empty(), "radiant {radiant}: no preview in exile");
            s.end_turn();
            assert_eq!(count_in(&s, "p1", "hand", TIMMY), 1, "radiant {radiant}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn sec8_2_exile_this_on_play_the_spell_goes_to_exile_never_to_the_graveyard() {
            let mut s = scn(json!({
                "p1": { "hand": [RECYCLING, TIMMY], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));
            let spell = s.hand("p1").into_iter().next().expect("p1 holds the spell");

            s.play(&spell, json!({}));

            s.expect_in_zone(&spell, "exile").expect_events(json!(["cardPlayed", "exiled"]));
            assert!(def_ids(&s.pile("p1", "graveyard")).is_empty());
        }

        #[test]
        fn r71_at_end_of_turn_it_copies_every_other_card_played_this_turn_including_one_played_after_it() {
            let mut s = scn(json!({
                "p1": { "hand": [TIMMY, RECYCLING, MR_TOKEN, STOCKPILE], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.play(MR_TOKEN, json!({ "zone": 2 })); // played AFTER the spell, and still copied (R71)
            s.end_turn();

            // The originals are on the field, so a copy is the only way either def reaches the hand.
            assert_eq!(count_in(&s, "p1", "hand", TIMMY), 1);
            assert_eq!(count_in(&s, "p1", "hand", MR_TOKEN), 1);
        }

        #[test]
        fn sec8_2_every_other_card_it_never_copies_itself() {
            let mut s = scn(json!({
                "p1": { "hand": [TIMMY, RECYCLING, STOCKPILE], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.end_turn();

            assert_eq!(count_in(&s, "p1", "hand", RECYCLING), 0);
            assert_eq!(count_in(&s, "p1", "hand", TIMMY), 1);
        }

        #[test]
        fn r57_the_copy_is_a_fresh_instance_so_the_original_stays_where_it_is() {
            let mut s = scn(json!({
                "p1": { "hand": [TIMMY, RECYCLING, STOCKPILE], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            let original = s.unit("p1", 1).expect("Timmy in lane 1");
            s.play(RECYCLING, json!({}));
            s.end_turn();

            s.expect_in_zone(&original, "field");
            let copy = s.pile("p1", "hand").into_iter().find(|card| card.def_id == TIMMY);
            let copy = copy.expect("a copy of Timmy in hand");
            assert_ne!(copy.id, original.id);
            assert!(!copy.radiant);
        }

        #[test]
        fn r62_the_copies_arrive_during_the_turn_that_ends_before_the_next_turn_starts() {
            let mut s = scn(json!({
                "p1": { "hand": [TIMMY, RECYCLING, STOCKPILE], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let copied = index_of(&s, added_to_hand_of(PlayerId::P1, TIMMY));
            let next_turn = index_of(&s, |event| matches!(event, GameEvent::TurnStarted { .. }));
            assert!(copied >= 0);
            assert!(next_turn >= 0);
            assert!(copied < next_turn);
        }

        #[test]
        fn r62_the_end_of_turn_trap_window_fires_first_then_this_delayed_effect() {
            let mut s = scn(json!({
                "p1": { "hand": [TIMMY, RECYCLING, STOCKPILE], "backrow": [BREAD], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 })); // 1 of 4 mana, so the turn ends with 3 unspent for #18 to read
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let fired = index_of(&s, |event| matches!(event, GameEvent::TrapFired { .. }));
            let copied = index_of(&s, added_to_hand_of(PlayerId::P1, TIMMY));
            assert!(fired >= 0);
            assert!(copied >= 0);
            assert!(fired < copied);
        }

        #[test]
        fn r86_an_instance_that_has_ceased_to_exist_is_skipped_and_the_rest_of_the_pool_still_arrives() {
            let mut s = scn(json!({
                "p1": { "hand": [RUSH_TOKEN, TIMMY, RECYCLING, STOCKPILE], "library": [MENACE, POSTDOC] },
                "p2": with_spare(json!({ "field": [FELINORS] })),
            }));

            s.play(RUSH_TOKEN, json!({ "zone": 1 })); // a unit-token card played from hand (§5, #75)
            s.play(TIMMY, json!({ "zone": 2 }));
            s.play(RECYCLING, json!({}));

            let token = s.unit("p1", 1).expect("the Rush Token in lane 1");
            // Rush lets it attack a unit the turn it arrives; the 3/4 body kills it on the strike-back, and
            // R11 makes a unit token that leaves the field cease to exist.
            let felinors = s.unit("p2", 1).expect("Felinors in p2's lane 1");
            s.attack(&token, &felinors);
            s.expect_in_zone(&token, "gone");

            s.end_turn();

            assert_eq!(count_in(&s, "p1", "hand", RUSH_TOKEN), 0);
            assert_eq!(count_in(&s, "p1", "hand", TIMMY), 1);
        }

        #[test]
        fn r57_the_base_face_keeps_the_played_cards_flag_a_radiant_cards_copy_is_radiant() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [{ "def": TIMMY, "radiant": true }, RECYCLING, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let copy = copy_in_hand(&s, TIMMY);
            assert!(copy.radiant);
            assert_eq!(copy.cost_mod, 0);
        }

        #[test]
        fn r1300_a_copy_of_a_chinese_card_is_chinese() {
            for chinese in [true, false] {
                let mut s = scn(json!({
                    "p1": { "hand": [TIMMY, RECYCLING, STOCKPILE], "library": [MENACE, POSTDOC] },
                    "p2": spare(),
                }));
                let played = s.card(TIMMY).id.clone();
                if chinese && let Some(card) = find_instance_mut(s.state_mut(), &played) {
                    card.chinese = Some(true);
                }

                s.play(TIMMY, json!({ "zone": 1 }));
                s.play(RECYCLING, json!({}));
                s.end_turn();

                assert_eq!(copy_in_hand(&s, TIMMY).chinese, chinese.then_some(true));
            }
        }

        #[test]
        fn the_base_face_gives_no_discount_the_copy_costs_the_printed_price() {
            let mut s = scn(json!({
                "p1": { "hand": [BIG_D, RECYCLING, STOCKPILE], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));

            s.play(BIG_D, json!({ "zone": 1 })); // #1 Big D-fender, printed cost 2
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let copy = copy_in_hand(&s, BIG_D);
            assert_eq!(copy.cost_mod, 0);
            assert_eq!(view_cost(&s, "p1", &copy.id), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn sec8_conventions_the_radiant_cell_restates_only_the_price_so_it_is_still_exiled_on_play() {
            let mut s = scn(json!({
                "p1": { "hand": [{ "def": RECYCLING, "radiant": true }, TIMMY], "library": [MENACE, POSTDOC] },
                "p2": spare(),
            }));
            let spell = s.hand("p1").into_iter().next().expect("p1 holds the spell");

            s.play(&spell, json!({}));

            s.expect_in_zone(&spell, "exile");
            assert!(def_ids(&s.pile("p1", "graveyard")).is_empty());
        }

        #[test]
        fn sec8_conventions_it_still_copies_every_other_card_played_this_turn() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [TIMMY, { "def": RECYCLING, "radiant": true }, MR_TOKEN, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.play(MR_TOKEN, json!({ "zone": 2 }));
            s.end_turn();

            assert_eq!(count_in(&s, "p1", "hand", TIMMY), 1);
            assert_eq!(count_in(&s, "p1", "hand", MR_TOKEN), 1);
            assert_eq!(count_in(&s, "p1", "hand", RECYCLING), 0);
        }

        #[test]
        fn r275_the_copy_of_a_non_radiant_card_is_radiant_and_one_cheaper() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [TIMMY, { "def": RECYCLING, "radiant": true }, MR_TOKEN, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 })); // printed cost 1, played as the base 3/3
            s.play(RECYCLING, json!({}));
            s.play(MR_TOKEN, json!({ "zone": 2 })); // printed cost 1, played after it (R71)
            s.end_turn();

            for def_id in [TIMMY, MR_TOKEN] {
                let copy = copy_in_hand(&s, def_id);
                assert!(copy.radiant);
                assert_eq!(copy.cost_mod, -1);
                assert_eq!(view_cost(&s, "p1", &copy.id), 0);
            }
            assert_eq!(s.unit("p1", 1).map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r386_an_upgrade_makes_the_copies_cost_2_less_kept_in_exile_and_a_degrade_finds_the_discount_at_its_floor() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [BIG_D, { "def": RECYCLING, "radiant": true }, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));
            assert!(!crate::can_degrade_number(&s, RECYCLING, "discount"));
            assert_eq!(crate::upgrade_number(&mut s, RECYCLING, "discount"), 2);

            s.play(BIG_D, json!({ "zone": 1 })); // printed cost 2
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let copy = copy_in_hand(&s, BIG_D);
            assert_eq!(copy.cost_mod, -2);
            assert_eq!(view_cost(&s, "p1", &copy.id), 0);
        }

        #[test]
        fn r275_the_radiant_copy_plays_as_its_radiant_face() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [TIMMY, { "def": RECYCLING, "radiant": true }, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(TIMMY, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.end_turn();
            let copy = copy_in_hand(&s, TIMMY);
            s.end_turn(); // back to p1

            s.play(&copy, json!({ "zone": 2 }));
            // #11 Tempo Timmy's Radiant face is a 6/6 (§8.1 row 11), for 0 mana.
            s.expect_stats(&copy, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
            s.expect_mana("p1", 4);
        }

        #[test]
        fn r65_the_copies_cost_1_less_is_a_cost_mod_so_the_copy_is_offered_at_one_less_than_its_price() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [BIG_D, { "def": RECYCLING, "radiant": true }, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(BIG_D, json!({ "zone": 1 })); // printed cost 2
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let copy = copy_in_hand(&s, BIG_D);
            assert_eq!(copy.cost_mod, -1);
            assert_eq!(view_cost(&s, "p1", &copy.id), 1);
        }

        #[test]
        fn r65_the_discount_is_what_the_player_actually_pays_on_a_later_turn() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [BIG_D, { "def": RECYCLING, "radiant": true }, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(BIG_D, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.end_turn(); // p1's copies arrive; p2's turn starts
            let copy = copy_in_hand(&s, BIG_D);
            s.end_turn(); // p2 ends; p1's turn starts, mana back to MAX_MANA 4

            s.expect_mana("p1", 4);
            s.play(&copy, json!({ "zone": 2 }));
            s.expect_mana("p1", 3); // 2 printed − 1 = 1 paid (R65, R78: the costMod survived the zone change)
        }

        #[test]
        fn r57_the_copy_of_a_radiant_card_is_radiant_too_and_the_flag_is_all_it_carries() {
            let mut s = scn(json!({
                "p1": {
                    "hand": [{ "def": BIG_D, "radiant": true }, { "def": RECYCLING, "radiant": true }, STOCKPILE],
                    "library": [MENACE, POSTDOC],
                },
                "p2": spare(),
            }));

            s.play(BIG_D, json!({ "zone": 1 }));
            s.play(RECYCLING, json!({}));
            s.end_turn();

            let copy = copy_in_hand(&s, BIG_D);
            assert!(copy.radiant);
            assert_eq!(copy.damage, 0);
            assert_eq!(js(&copy.buffs), json!({ "attack": 0, "health": 0 }));
        }
    }
}
