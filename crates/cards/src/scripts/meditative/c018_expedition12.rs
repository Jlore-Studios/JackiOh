//! Meditative #18 Expedition12 (SPEC §8.8 row 18). (1) Spell, Quickdraw, Epic.
//!   Base:    "Lose ({loss}) mana next turn. Make a card in your hand Radiant."
//!   Radiant: "Cleave. Lose ({loss}) mana next turn. Make a card in your hand Radiant."
//!
//! The loss is a next-turn rider (§6.3 Mana, `next_turn_mana`): your next refresh gives `loss` less,
//! never below 0, as #21 Hinder's does to the opponent's (#1563; it was ME-TURN's lost refresh, R844,
//! "Lose all mana next turn", until then). The pick is a declared hand pick (R81, as C+ #41's): your
//! other non-Radiant hand cards, made Radiant (§5.2). With no other non-Radiant hand card the pick has
//! nothing and the Spell still resolves (§8 Conventions). The Radiant face's Cleave (R1563001), Echo 1
//! until #1563, also makes the cards beside the pick in your hand Radiant: the one before it and the one
//! after it, in hand order as the Spell resolves.
//!
//! `script_v0_4_0f` keeps the card as it ran through v0.4.0f for its replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-018";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::hand(
        1,
        1,
        json!({ "of": ["hand"], "excludeSelf": true, "check": "notRadiant" }),
    )]
}

fn target_checks() -> IndexMap<&'static str, TargetCheck> {
    IndexMap::from([(
        "notRadiant",
        target_check(|a| a.candidate.is_some_and(|candidate| !candidate.radiant)),
    )])
}

/// The picked hand card's id, off the play's declared targets.
fn picked(ctx: &EffectContext<'_>) -> Option<String> {
    ctx.targets.iter().find_map(|selection| match selection {
        Selection::Instance { instance_id } => Some(instance_id.clone()),
        _ => None,
    })
}

/// R1563001: the hand cards on either side of the pick, in hand order as the Spell resolves (the
/// Spell itself is resolving, out of the hand).
fn beside_the_pick(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(pick) = picked(ctx) else {
        return vec![];
    };
    let hand: Vec<String> = zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
        .iter()
        .map(|card| card.id.clone())
        .collect();
    let Some(at) = hand.iter().position(|id| *id == pick) else {
        return vec![];
    };
    [at.checked_sub(1), Some(at + 1)]
        .into_iter()
        .flatten()
        .filter_map(|index| hand.get(index).cloned())
        .collect()
}

fn cry(ctx: &mut EffectContext<'_>, cleave: bool) -> Vec<Effect> {
    let mut effects = vec![
        next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "loss") }))),
        set_radiant(json_as(json!({ "target": { "of": "chosen" } }))),
    ];
    if cleave {
        effects.extend(
            beside_the_pick(&*ctx)
                .iter()
                .map(|id| set_radiant(json_as(json!({ "instanceId": id })))),
        );
    }
    effects
}

fn face(cleave: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        targets: targets(),
        target_checks: target_checks(),
        cry: Some(hook(move |ctx| cry(ctx, cleave))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// R767000: Expedition12 through v0.4.0f: "Lose all mana next turn" (R844's lost refresh), and Echo 1
/// on the Radiant face.
fn face_v0_4_0f(echo: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            echo: echo.then_some(1),
            ..StaticFlags::default()
        }),
        targets: targets(),
        target_checks: target_checks(),
        cry: Some(hook(|_| {
            vec![
                lose_refreshes(json_as(json!({ "turns": 1 }))),
                set_radiant(json_as(json!({ "target": { "of": "chosen" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script_v0_4_0f() -> CardScripts {
    CardScripts {
        base: face_v0_4_0f(false),
        radiant: face_v0_4_0f(true),
    }
}

// Meditative #18 Expedition12 — SPEC §8.8 row 18, BUILD M10 row M 18: Quickdraw; your next turn's
// refresh gives 2 less, never below 0, while mana gained during that turn still adds; the turn after
// is normal; an extra turn taken next is the one that loses it; the loss reads through `param()`; the
// declared pick is another non-Radiant card in your hand, made Radiant; with none it still resolves;
// radiant Cleave: the cards beside the pick in your hand become Radiant too (R1563001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const EXPEDITION: &str = "meditative-018";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const TIMMY: &str = "core-011"; // (1) Unit 3/3.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const COIN: &str = "core-t-coin"; // (0) Spell: gain 1 mana.

    /// A board where p1 can play the expedition and neither side auto-ends its turn, on p1's turn 9,
    /// so its next turn's max mana is 4.
    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "expedition12",
            "p1": {
                "hand": hand,
                "field": [VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
        }))
    }

    fn pick(s: &Scenario, card: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": s.card(card).id }] })
    }

    /// The hand cards `legalActions` offers as the expedition's pick.
    fn offered(s: &Scenario) -> Vec<String> {
        let expedition_id = s.card(EXPEDITION).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == expedition_id => {
                    targets
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|target| match target {
                            Selection::Instance { instance_id } => Some(instance_id),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                }
                _ => vec![],
            })
            .collect()
    }

    fn radiant_hand(state: &GameState) -> Vec<String> {
        state.players.p1.hand.iter().filter(|card| card.radiant).map(|card| card.id.clone()).collect()
    }

    /// p1's next turn: the opponent's turn passes, and p1's refresh has run.
    fn next_turn(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
    }

    mod base {
        use super::*;

        #[test]
        fn next_turn_gives_2_less_and_the_turn_after_is_normal() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER]));
            s.play(EXPEDITION, pick(&s, FILLER));
            assert_eq!(radiant_hand(s.state()), vec![s.card(FILLER).id.clone()]);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -2);

            next_turn(&mut s);
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, side.mana.max - 2);
            assert_eq!(side.mana.next_turn_mod, 0);
            assert_eq!(side.lost_refresh_through, None);

            next_turn(&mut s);
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, side.mana.max);
        }

        #[test]
        fn the_loss_never_takes_the_refresh_below_0() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "expedition12-floor",
                "turn": 1,
                "p1": { "hand": [EXPEDITION, FILLER], "field": [VANILLA], "library": [FILLER, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
            }));
            s.play(EXPEDITION, pick(&s, FILLER));
            s.state_mut().players.p1.mana.next_turn_mod -= 5;
            next_turn(&mut s);
            assert_eq!(s.state().players.p1.mana.current, 0);
        }

        #[test]
        fn mana_gained_during_that_turn_still_adds() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER, COIN]));
            s.play(EXPEDITION, pick(&s, FILLER));
            next_turn(&mut s);
            let before = s.state().players.p1.mana.current;
            // The Coin's gain is an effect, not the refresh: it adds.
            s.play(COIN, json!({}));
            assert_eq!(s.state().players.p1.mana.current, before + 1);
        }

        #[test]
        fn r845_an_extra_turn_taken_next_is_the_one_that_loses_it() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER]));
            s.play(EXPEDITION, pick(&s, FILLER));
            s.state_mut().players.p1.extra_turns = Some(1);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, side.mana.max - 2);
            next_turn(&mut s);
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, side.mana.max);
        }

        #[test]
        fn r386_the_loss_reads_through_param_a_buff_makes_it_1_a_nerf_3() {
            // A step down is the Buff's direction for a loss (`better: down`).
            for (steps, loss) in [(-1, 1), (1, 3)] {
                crate::register_all();
                let mut s = board(json!([EXPEDITION, FILLER]));
                step_param(s.card_mut(EXPEDITION), "loss", steps);
                s.play(EXPEDITION, pick(&s, FILLER));
                assert_eq!(s.state().players.p1.mana.next_turn_mod, -loss);
            }
        }

        #[test]
        fn the_pick_becomes_radiant_and_offers_no_radiant_card() {
            crate::register_all();
            let mut s = board(json!([
                EXPEDITION,
                FILLER,
                { "def": VANILLA, "radiant": true },
            ]));
            let offered_ids = offered(&s);
            assert!(!offered_ids.contains(&s.card(EXPEDITION).id.clone()));
            let shiny = s.hand(P1).into_iter().find(|held| held.radiant).expect("the Radiant card").id.clone();
            assert!(!offered_ids.contains(&shiny), "no Radiant card is offered");
            s.play(EXPEDITION, pick(&s, FILLER));
            assert!(s.card(FILLER).radiant, "the pick becomes Radiant");
        }

        #[test]
        fn base_face_has_no_cleave_only_the_pick_becomes_radiant() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, VANILLA, FILLER, TIMMY]));
            s.play(EXPEDITION, pick(&s, FILLER));
            assert_eq!(radiant_hand(s.state()), vec![s.card(FILLER).id.clone()]);
        }

        #[test]
        fn r640_it_is_quickdraw_on_both_faces() {
            crate::register_all();
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(face.static_flags.as_ref().and_then(|flags| flags.quickdraw), Some(true));
                assert_eq!(face.static_flags.as_ref().and_then(|flags| flags.echo), None);
            }
            assert!(crate::card_def(EXPEDITION).tags.contains(&Tag::Quickdraw));
        }

        #[test]
        fn with_no_other_card_it_still_resolves() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            s.expect_in_zone(EXPEDITION, "graveyard");
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1563001_cleave_makes_the_cards_beside_the_pick_radiant_too() {
            crate::register_all();
            let mut s = board(json!([
                { "def": EXPEDITION, "radiant": true },
                VANILLA,
                FILLER,
                TIMMY,
                MENACE,
            ]));
            s.play(EXPEDITION, pick(&s, FILLER));
            assert!(s.card(FILLER).radiant, "the pick");
            assert!(s.card(VANILLA).radiant, "the card before it");
            assert!(s.card(TIMMY).radiant, "the card after it");
            assert!(!s.card(MENACE).radiant, "two away stays");
            // No Echo: no second pick opens, and the loss is the base face's (2).
            assert!(s.state().pending.is_none());
            assert_eq!(s.state().players.p1.mana.next_turn_mod, -2);
        }

        #[test]
        fn r1563001_a_pick_at_the_end_of_the_hand_has_one_card_beside_it() {
            crate::register_all();
            let mut s = board(json!([{ "def": EXPEDITION, "radiant": true }, MENACE, VANILLA, FILLER]));
            s.play(EXPEDITION, pick(&s, FILLER));
            assert!(s.card(FILLER).radiant);
            assert!(s.card(VANILLA).radiant);
            assert!(!s.card(MENACE).radiant);
        }
    }
}
