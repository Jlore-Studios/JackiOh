//! SPEC §8.12 S+ #55 Thorium Reactor (R3556005; R62, R1420):
//! (2) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Start of turn: Gain {mana} mana & draw {card|card|cards}.
//!            End of turn: {chance}% chance to blow up dealing {damage} damage to your hero.
//!            Activate: Discard a Thorium to skip the blow up chance this turn."
//!   Radiant: "Lucky 1
//!            Start of turn: Gain {mana} mana & draw {card|card|cards}.
//!            End of turn: {chance}% chance to blow up dealing {damage} damage to your hero.
//!            Activate: Discard a Thorium to skip the blow up chance this turn."
//!
//! Start of its controller's turn (R62): temporary mana and a draw (as S T-red-narcor's). End of its
//! controller's turn: a `chance`% roll from the game's Rng — Lucky and Luck rerolling toward a miss
//! (R3556005) — and on a hit `damage` to its controller's hero, then it destroys itself ("blow up").
//! The Activate discards a chosen #48 Thorium from hand (an existing `choose_from_hand` prompt, as
//! S+ #29 Grinder Veteran's) to skip this turn's roll. With no Thorium in hand it is not offered.

use std::sync::Arc;

use jackioh_engine::effects::{choose_from_hand, discard, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-055";

/// S+ #48 Thorium, the card the Activate discards.
const THORIUM: &str = "stinkyscapeplus-048";
/// What the Activate remembers: the turn it skipped the blow up on.
const SKIPPED: &str = "skipBlowUp";
/// The resume step answering the Thorium pick.
const DISCARD: &str = "discard";
/// The Activate's id.
const THORIUM_ACTIVATE: &str = "thorium";
/// A percent chance is out of a hundred.
const PERCENT: f64 = 100.0;

/// R3556005: "a Thorium" is a #48 Thorium.
fn is_thorium(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    card.def_id == THORIUM && card.zone.z() == ZoneName::Hand && card.controller == ctx.controller
}

/// R3556005: this turn's roll is skipped when the Activate ran this turn.
fn skipped_this_turn(ctx: &EffectContext<'_>) -> bool {
    recalled(ctx, SKIPPED).and_then(|value| value.as_i64()) == Some(i64::from(ctx.state.turn))
}

fn reactor() -> Script {
    let label = "Discard a Thorium: skip this turn's blow up chance";
    Script {
        start_of_turn: Some(hook(|ctx| {
            vec![
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
                draw(json_as(json!({ "count": param(&*ctx, "card") }))),
            ]
        })),
        end_of_turn: Some(hook(|ctx| {
            if skipped_this_turn(ctx) {
                return vec![];
            }
            // R3556005: Lucky and Luck reroll toward a miss.
            let lucky = ctx.self_.as_ref().map_or(0, |own| {
                lucky_on(ctx.sink.state, own) + luck_of(ctx.sink.state, ctx.controller)
            });
            let chance = f64::from(param(&*ctx, "chance")) / PERCENT;
            if !ctx.rng.lucky(lucky, |rng| rng.chance(chance), |a, b| a && b) {
                return vec![];
            }
            vec![
                damage(json_as(
                    json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "damage") }),
                )),
                destroy(json_as(json!({ "target": { "of": "self" } }))),
            ]
        })),
        activations: vec![ActivationDecl {
            id: THORIUM_ACTIVATE.to_string(),
            label: label.to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: Some(condition_hook(|c| {
                hand_holds_any(c.state, c.controller, &[THORIUM])
            })),
            has: None,
            run: hook(move |_ctx| {
                vec![choose_from_hand(ChooseFromHandArgs {
                    step: DISCARD.to_string(),
                    count: Some(1),
                    prompt: Some(label.to_string()),
                    where_: Some(Arc::new(is_thorium) as HandFilter),
                    ..Default::default()
                })]
            }),
        }],
        resume: IndexMap::from([(
            DISCARD,
            hook(|ctx| {
                vec![
                    discard(json_as(json!({ "target": { "of": "chosen", "index": 0 } }))),
                    remember(json_as(
                        json!({ "key": SKIPPED, "value": ctx.state.turn }),
                    )),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: reactor(),
        radiant: reactor(),
    }
}

// S+ #55 Thorium Reactor — SPEC §8.12 row S+ 55 (R3556005; R62).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005";

    /// p1 holds the hand; `"mana": 10` plays the Reactor from hand.
    fn reactor_board(seed: &str, hand: Value) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": hand,
                "library": filler(6),
            }),
            json!({
                "hand": [FILLER],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn reactor_only(radiant: bool, seed: &str) -> Scenario {
        reactor_board(seed, json!([{ "def": ID, "radiant": radiant }, FILLER]))
    }

    /// Whether the Reactor blew up: it is in p1's graveyard, off the field.
    fn blew_up(s: &Scenario) -> bool {
        s.pile(P1, "graveyard").iter().any(|card| card.def_id == ID)
    }

    /// The Thorium in p1's hand.
    fn thorium_in_hand(s: &Scenario) -> String {
        s.hand(P1)
            .iter()
            .find(|card| card.def_id == THORIUM)
            .expect("a Thorium")
            .id
            .clone()
    }

    /// Answers the open Thorium pick with p1's Thorium.
    fn answer_thorium(s: &mut Scenario) {
        let id = thorium_in_hand(s);
        s.answer(json!(id));
    }

    mod base {
        use super::*;

        #[test]
        fn r3556005_start_of_turn_gains_mana_and_draws() {
            // The first end's roll may blow the Reactor up; take the first seed that survives it.
            let mut found = None;
            for n in 0..20 {
                let mut s = reactor_only(false, &format!("s55-mana-{n}"));
                s.end_turn();
                if blew_up(&s) {
                    continue;
                }
                next_turn_of(&mut s, P1);
                found = Some(s);
                break;
            }
            let s = found.expect("some seed survives the first roll");
            // Start of turn: the refresh, then 3 mana gained, a natural draw and the Reactor's draw.
            let mana = &s.state().players[P1].mana;
            assert_eq!(mana.current, mana.max + 3, "3 mana gained past the refresh");
            assert_eq!(s.hand(P1).len(), 3, "the natural draw and the Reactor's draw");
        }

        #[test]
        fn r3556005_about_fifteen_percent_of_ends_blow_up() {
            let mut blown = 0;
            for n in 0..200 {
                let mut s = reactor_only(false, &format!("s55-blow-{n}"));
                let health = s.state().players[P1].hero.health;
                s.end_turn();
                if blew_up(&s) {
                    blown += 1;
                    // Each blow up is a 30 hit on p1's hero, and the Reactor is gone.
                    assert_eq!(s.state().players[P1].hero.health, health - 30);
                    assert!(s.card(ID).zone.z() != ZoneName::Field, "the Reactor destroyed itself");
                } else {
                    assert_eq!(s.state().players[P1].hero.health, health);
                }
            }
            assert!((10..=60).contains(&blown), "{blown} of 200 ends blew up");
        }

        #[test]
        fn r3556005_the_activate_discards_a_chosen_thorium_and_skips_this_turns_roll() {
            // The premise: some unskipped ends blow up.
            let mut unskipped = 0;
            for n in 0..100 {
                let mut s = reactor_only(false, &format!("s55-skip-{n}"));
                s.end_turn();
                unskipped += blew_up(&s) as i32;
            }
            assert!(unskipped > 0, "the premise: some unskipped ends blow up");
            // With the skip none of the same-shape ends do.
            for n in 0..100 {
                let mut s = reactor_board(&format!("s55-skipped-{n}"), json!([ID, THORIUM]));
                s.activate(ID, json!({ "ability": THORIUM_ACTIVATE }));
                answer_thorium(&mut s);
                assert!(
                    s.pile(P1, "graveyard").iter().any(|card| card.def_id == THORIUM),
                    "the chosen Thorium reached the graveyard"
                );
                s.end_turn();
                assert!(!blew_up(&s), "the skipped end never blows up");
                assert_eq!(s.state().players[P1].hero.health, HERO_HEALTH);
            }
        }

        #[test]
        fn r3556005_the_roll_returns_next_turn() {
            // Skip one end, then play the next: over enough seeds it blows up again.
            let mut blown = 0;
            for n in 0..60 {
                let mut s = reactor_board(&format!("s55-return-{n}"), json!([ID, THORIUM]));
                s.activate(ID, json!({ "ability": THORIUM_ACTIVATE }));
                answer_thorium(&mut s);
                s.end_turn();
                assert!(!blew_up(&s), "skipped this turn");
                next_turn_of(&mut s, P1);
                s.end_turn();
                blown += blew_up(&s) as i32;
            }
            assert!(blown > 0, "the roll returns next turn");
        }

        #[test]
        fn r3556005_not_offered_without_a_thorium() {
            let mut s = reactor_only(false, "s55-none");
            s.expect_refused(|s| s.activate(ID, json!({ "ability": THORIUM_ACTIVATE })));
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn the_random_policy_answers_with_a_thorium() {
            let mut s = reactor_board("s55-policy", json!([ID, THORIUM, FILLER]));
            s.activate(ID, json!({ "ability": THORIUM_ACTIVATE }));
            let state = s.state().clone();
            let mut rng = Rng::new("s55-policy", 0);
            let body = choose_action(&state, P1, &mut rng).expect("the policy answers");
            assert!(matches!(body, ActionBody::Answer { .. }), "{body:?}");
            let next = reduce(&state, &Action::new(body, P1, "s55-policy-answer".to_string()));
            assert!(next.error.is_none(), "{:?}", next.error);
            assert!(next.state.pending.is_none());
            assert!(
                next.state.players[P1].graveyard.iter().any(|card| card.def_id == THORIUM),
                "one Thorium discarded"
            );
        }

        #[test]
        fn a_timeout_answers_with_a_thorium() {
            let mut s = reactor_board("s55-timeout", json!([ID, THORIUM, FILLER]));
            s.activate(ID, json!({ "ability": THORIUM_ACTIVATE }));
            s.act(ActionBody::Timeout, P1, "the controller's clock runs out");
            assert!(s.state().pending.is_none());
            assert!(
                s.pile(P1, "graveyard").iter().any(|card| card.def_id == THORIUM),
                "one Thorium discarded"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556005_lucky_1_blows_up_less_often() {
            let mut base_blown = 0;
            let mut radiant_blown = 0;
            for n in 0..200 {
                let mut s = reactor_only(false, &format!("s55-lucky-{n}"));
                s.end_turn();
                base_blown += blew_up(&s) as i32;
                let mut s =
                    reactor_board(&format!("s55-lucky-{n}"), json!([{ "def": ID, "radiant": true }, FILLER]));
                s.end_turn();
                radiant_blown += blew_up(&s) as i32;
            }
            assert!((10..=60).contains(&base_blown), "{base_blown} of 200 base ends blew up");
            assert!(
                radiant_blown < base_blown,
                "Lucky 1 blows up less often: {radiant_blown} vs {base_blown}"
            );
        }
    }
}
