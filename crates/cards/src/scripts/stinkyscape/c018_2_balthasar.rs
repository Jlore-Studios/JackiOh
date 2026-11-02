//! S #18.2 BALTHASAR (SPEC §8.10 row S 18.2, §7; §2.4, §6.1, R57, R62, R70, R180, R290, R384, R451,
//! R1300, R1843000, R1866002): (2) Field Spell, Tech, Token (printed Legendary), summoned by S #18
//! Imitated Instrumentality.
//!   Base:    "Indestructible / Start of turn: Vote to remove you if your hero has less than half its
//!            starting health. / Activate: Add a copy of the last card you played to your hand. It costs
//!            ({surcharge}) more." (1)
//!   Radiant: "… / Activate: Add a Radiant copy of the last card you played to your hand. It costs
//!            ({surcharge}) more." (1)
//!
//! Engine: Indestructible is printed in `catalog.json` (§6.1). Start of turn (R62): `vote_out` when twice
//! its controller's hero health is below the health that hero started with, its seat's handicap
//! `heroHealth` else HERO_HEALTH (`starting_health_of`, R290, R180) (R1843000). Activate (R384, once a
//! turn): a copy (R57) of the last face-up card its controller played (`last_face_up_played`, R451: casts
//! count, R70; a face-down Trap never; the AI generated cards passed over), on the face it was played,
//! a Chinese one Chinese (R1300), added to their hand with `add_to_hand`'s `costMod` of `surcharge`, not
//! capped; a full hand burns it (§2.4); nothing before a play.

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-018-2";

/// "Less than half": twice the hero's health below its starting health (R1866002).
const HALVES: i32 = 2;

/// R1866002: "votes to remove you if your hero has less than half its starting health".
fn vote_if_below_half(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let me = ctx.controller;
    if HALVES * hero_of(ctx.state, me).health < starting_health_of(ctx.state, me) {
        vec![vote_out(Default::default())]
    } else {
        vec![]
    }
}

/// The copy of the last card its controller played, Radiant on `radiant` or when that card was.
fn copy_last_play(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let Some(last) = last_face_up_played(ctx.state, ctx.controller) else {
        return vec![];
    };
    let mut args = json!({
        "defId": last.def_id,
        "radiant": radiant || last.radiant,
        "costMod": param(ctx, "surcharge"),
    });
    // R1300: a copy of a Chinese card is Chinese.
    if last.chinese == Some(true) {
        args["chinese"] = json!(true);
    }
    vec![add_to_hand(json_as(args))]
}

/// R2476000: after "the last card you played", the card its Activate would copy, by name as it was
/// played, or the dash before a play. On the field, to both seats: the record keeps face-up plays
/// alone (Traps and Field Traps, set face-down, never count, R451), so it names nothing hidden.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let last = last_face_up_played(ctx.state, ctx.controller);
        vec![card_preview(
            ctx.state,
            "the last card you played",
            last.as_ref().map(|last| (last.def_id.as_str(), last.radiant)),
        )]
    })
}

fn balthasar(radiant: bool) -> Script {
    let copy = ActivationDecl {
        id: "copy".to_string(),
        label: if radiant {
            "Add a Radiant copy of the last card you played to your hand. It costs more".to_string()
        } else {
            "Add a copy of the last card you played to your hand. It costs more".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| copy_last_play(ctx, radiant)),
    };
    Script {
        start_of_turn: Some(hook(vote_if_below_half)),
        activations: vec![copy],
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: balthasar(false),
        radiant: balthasar(true),
    }
}

// S #18.2 BALTHASAR — SPEC §8.10 row S 18.2, BUILD M12 row S 18.2 (R1866002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const BALTHASAR: &str = "stinkyscape-018-2";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-016"; // Hit Job: inert in a hand.
    const BEAR: &str = "core-060"; // (1) Trap, set face-down.

    fn table(radiant: bool, hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": hand, "backrow": [{ "def": BALTHASAR, "radiant": radiant }], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn votes_at_start(health: i32, handicap: Option<Handicap>) -> usize {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "backrow": [BALTHASAR], "health": health, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.state_mut().players.p1.handicap = handicap;
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::VoteCast { def_id, .. } if def_id == BALTHASAR))
            .count()
    }

    /// R2476000: after "the last card you played", the card its Activate would copy, to both seats: the
    /// dash, then Mr. Vanilla once played; a face-down Trap set after it changes nothing.
    #[test]
    fn r2476000_it_names_the_last_card_you_played() {
        crate::register_all();
        for radiant in [false, true] {
            let mut s = table(radiant, json!([VANILLA, BEAR, FILLER]));
            let balthasar = s.card(BALTHASAR).id.clone();
            let label = "the last card you played".to_string();
            assert_eq!(crate::preview_seen(&s, P1, &balthasar), vec![(label.clone(), 0, Some("—".to_string()))]);
            s.play(VANILLA, json!({}));
            s.play(BEAR, json!({}));
            let named = vec![(label, 1, Some("Mr. Vanilla".to_string()))];
            assert_eq!(crate::preview_seen(&s, P1, &balthasar), named, "radiant {radiant}");
            assert_eq!(crate::preview_seen(&s, PlayerId::P2, &balthasar), named, "radiant {radiant}");
        }
    }

    fn copy(s: &mut Scenario) {
        let balthasar = s.card(BALTHASAR).id.clone();
        s.activate(balthasar.as_str(), json!({ "ability": "copy" }));
    }

    fn held(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == def_id).collect()
    }

    fn copied_after_a_play(face: bool, played_radiant: bool) -> (Scenario, CardInstance) {
        let mut s = table(face, json!([{ "def": VANILLA, "radiant": played_radiant }, FILLER]));
        s.play(VANILLA, json!({}));
        assert!(held(&s, VANILLA).is_empty());
        copy(&mut s);
        let copies = held(&s, VANILLA);
        assert_eq!(copies.len(), 1);
        let copy = copies[0].clone();
        (s, copy)
    }

    mod base {
        use super::*;

        #[test]
        fn r1866002_votes_only_below_half_its_starting_health() {
            assert_eq!(votes_at_start(14, None), 1);
            assert_eq!(votes_at_start(15, None), 0);
        }

        #[test]
        fn r1866002_a_handicapped_seat_reads_its_own_starting_health() {
            let twenty = Some(Handicap { hero_health: Some(20), ..HUMAN_HANDICAP });
            assert_eq!(votes_at_start(9, twenty), 1);
            assert_eq!(votes_at_start(10, twenty), 0);
        }

        #[test]
        fn r1866002_activate_adds_a_copy_of_your_last_face_up_play_costing_1_more() {
            let (s, copy) = copied_after_a_play(false, false);
            assert!(!copy.radiant);
            assert_eq!(play_cost(s.state(), &copy), 2);
        }

        #[test]
        fn r1866002_a_radiant_play_is_copied_radiant() {
            let (_, copy) = copied_after_a_play(false, true);
            assert!(copy.radiant);
        }

        #[test]
        fn r1866002_a_trap_set_face_down_is_not_the_last_play() {
            let mut s = table(false, json!([VANILLA, BEAR, FILLER]));
            s.play(VANILLA, json!({}));
            s.play(BEAR, json!({ "zone": 2 }));
            copy(&mut s);
            assert_eq!(held(&s, VANILLA).len(), 1);
            assert!(held(&s, BEAR).is_empty());
        }

        #[test]
        fn r1866002_nothing_before_a_play() {
            let mut s = table(false, json!([FILLER]));
            let before = s.hand(P1).len();
            copy(&mut s);
            assert_eq!(s.hand(P1).len(), before);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Burned));
        }

        #[test]
        fn r1866002_a_full_hand_burns_the_copy() {
            let mut hand = vec![json!(VANILLA)];
            hand.extend((0..9).map(|_| json!(FILLER)));
            let mut s = table(false, json!(hand));
            s.play(VANILLA, json!({}));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.hand(P1).len() as i32, HAND_CAP);

            copy(&mut s);
            assert!(held(&s, VANILLA).is_empty());
            assert!(s.last_events().iter().any(|event| {
                matches!(event, GameEvent::Burned { def_id, .. } if def_id == VANILLA)
            }));
        }

        #[test]
        fn r1866002_activates_once_a_turn() {
            let (mut s, _) = copied_after_a_play(false, false);
            let balthasar = s.card(BALTHASAR).id.clone();
            s.expect_refused(|s| s.activate(balthasar.as_str(), json!({ "ability": "copy" })));
        }

        #[test]
        fn surcharge_reads_through_param() {
            let mut s = table(false, json!([VANILLA, FILLER]));
            assert_eq!(crate::degrade_number(&mut s, BALTHASAR, "surcharge"), 2);
            s.play(VANILLA, json!({}));
            copy(&mut s);
            let copies = held(&s, VANILLA);
            assert_eq!(play_cost(s.state(), &copies[0]), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1866002_radiant_the_copy_is_radiant() {
            let (s, copy) = copied_after_a_play(true, false);
            assert!(copy.radiant);
            assert_eq!(play_cost(s.state(), &copy), 2);
        }
    }
}
