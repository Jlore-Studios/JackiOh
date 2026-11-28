//! FvH #79 Post Human Aesthetist (SPEC §8.14 row 79): (4) Unit, Human, Mythic, 2/8 → 4/16. Built by S08
//! (#5258).
//!   Base:    "Can't attack\nCry: Give a Human the Felinor tag and "End of turn: Allure a random enemy
//!            Unit.""
//!   Radiant: "Can't attack\nCry: Give a Human the Felinor tag and "End of turn: Steal a random enemy
//!            Unit.""
//!
//! Can't attack is catalog data. Cry (R1): one declared Human Unit of either side but itself (R81), a
//! helpful pick (it gives); with none it enters and the Cry does nothing (R90). The Unit gains the
//! Felinor tag (`grant_tag`, R923) and the quoted ability, a `grant_ability` with `on: EndOfTurn`
//! (R5237006) named `felinorsvshumans-079#endOfTurn`, whose text is the face's `grants` entry and whose
//! hook is this file's `grants["endOfTurn"]` on that face. It runs at its holder's controller's end of
//! turn as the holder's own (FV2 #79), so on an enemy Human it acts against the Aesthetist's side.
//! Base: `allure_unit` of one random enemy Unit of the holder, stolen at the start of the holder's
//! controller's next turn (R5238008, R963). Radiant: `steal` of one random enemy Unit, drawn from the
//! match rng as the effect applies (C #40 MC Tech's shape), destroyed with no room (R4205000). A copy
//! keeps the grant (R57) and the holder loses it leaving the field (R78). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-079";

/// The granted ability's key, in this file's `grants` and in each face's catalog `grants`.
const GRANT_KEY: &str = "endOfTurn";

/// One enemy Unit of the running card's controller, drawn as the effect applies (R60).
fn random_enemy_unit(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let units: Vec<CardInstance> = active_units_of(&*ctx.state, opponent_of(ctx.controller))
        .into_iter()
        .cloned()
        .collect();
    ctx.rng.shuffle(&units).into_iter().take(1).map(|card| card.id).collect()
}

/// Base: "End of turn: Allure a random enemy Unit."
fn allure_one() -> Hook {
    hook(|_ctx| {
        vec![allure_unit(AllureUnitArgs {
            instance_id: None,
            random_enemy: Some(true),
        })]
    })
}

/// Radiant: "End of turn: Steal a random enemy Unit."
fn steal_one() -> Hook {
    hook(|_ctx| {
        vec![for_each_card(ForEachCardArgs {
            cards: Arc::new(random_enemy_unit),
            each: Arc::new(|instance_id: &str| steal(json_as(json!({ "instanceId": instance_id })))),
        })]
    })
}

fn aesthetist(radiant: bool, at_end: Hook) -> Script {
    let mut human = TargetDecl::target(
        1,
        1,
        json!({ "side": "any", "of": ["unit"], "tags": ["Human"], "excludeSelf": true }),
    );
    human.aim = Some(TargetAim::Help);
    Script {
        targets: vec![human],
        cry: Some(hook(move |_ctx| {
            let mut grant: GrantAbilityArgs = json_as(json!({
                "target": { "of": "chosen" },
                "grant": format!("{ID}#{GRANT_KEY}"),
                "radiant": radiant,
            }));
            grant.on = Some(GrantOn::EndOfTurn);
            vec![
                grant_tag(json_as(json!({ "target": { "of": "chosen" }, "tag": "Felinor" }))),
                grant_ability(grant),
            ]
        })),
        grants: IndexMap::from([(GRANT_KEY, at_end)]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: aesthetist(false, allure_one()),
        radiant: aesthetist(true, steal_one()),
    }
}

// FvH #79 Post Human Aesthetist — SPEC §8.14 row 79.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const OTHER: &str = "stand-in (1) 3/3";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    /// The Aesthetist in p1's hand beside p1's Human (lane 1), facing one non-Human of p2's (lane 1).
    fn gallery(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [HUMAN] },
            "p2": { "hand": [FILLER], "field": [OTHER] },
        }))
    }

    /// Plays the Aesthetist onto p1's Human: the Human's id and p2's Unit's.
    fn gift_own_human(s: &mut Scenario) -> (String, String) {
        let human = s.unit_at(P1, 1).id;
        let theirs = s.unit_at(P2, 1).id;
        s.play(ID, pick::targets(pick::id(&human)));
        (human, theirs)
    }

    fn holds_end_of_turn_grant(s: &Scenario, card: &str) -> bool {
        s.card(card)
            .grants
            .iter()
            .flatten()
            .any(|grant| grant.on == Some(GrantOn::EndOfTurn))
    }

    fn is_felinor(s: &Scenario, card: &str) -> bool {
        tags_of(s.state(), s.card(card)).contains(&Tag::Felinor)
    }

    mod base {
        use super::*;

        #[test]
        fn it_cannot_attack() {
            let s = scenario(json!({
                "p1": { "field": [ID], "hand": [FILLER] },
                "p2": { "field": [OTHER], "hand": [FILLER] },
            }));
            assert!(s.attacks_of(ID).is_empty());
        }

        #[test]
        fn the_human_gains_the_felinor_tag_and_the_quoted_end_of_turn() {
            let mut s = gallery(false);
            let (human, _theirs) = gift_own_human(&mut s);
            assert!(is_felinor(&s, &human));
            assert!(holds_end_of_turn_grant(&s, &human));
            assert_eq!(
                grant_texts(s.state(), s.card(human.as_str())),
                Some(vec!["End of turn: Allure a random enemy Unit.".to_string()])
            );
        }

        #[test]
        fn at_its_controllers_end_of_turn_it_allures_an_enemy_unit_stolen_at_their_next_start() {
            let mut s = gallery(false);
            let (_human, theirs) = gift_own_human(&mut s);
            s.end_turn();
            assert_eq!(s.card(theirs.as_str()).controller, P2, "an Allure waits for p1's next turn");
            s.end_turn();
            assert_eq!(s.card(theirs.as_str()).controller, P1);
        }

        #[test]
        fn a_non_human_cannot_be_chosen() {
            let mut s = gallery(false);
            let theirs = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.play(ID, pick::targets(pick::id(&theirs))));
            s.expect_in_zone(ID, "hand");
        }

        #[test]
        fn with_no_human_to_choose_it_enters_and_gives_nothing() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER], "field": [OTHER] },
                "p2": { "hand": [FILLER], "field": [OTHER] },
            }));
            let mine = s.unit_at(P1, 1).id;
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "field");
            assert!(!is_felinor(&s, &mine));
            assert!(!holds_end_of_turn_grant(&s, &mine));
        }

        #[test]
        fn on_an_enemy_human_it_acts_for_the_enemy_at_their_end_of_turn() {
            let mut s = scenario(json!({
                "p1": { "hand": [ID, FILLER], "field": [OTHER] },
                "p2": { "hand": [FILLER], "field": [HUMAN] },
            }));
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&theirs)));
            assert!(is_felinor(&s, &theirs));
            let mine: Vec<String> =
                active_units_of(s.state(), P1).into_iter().map(|unit| unit.id.clone()).collect();
            s.end_turn();
            s.end_turn();
            s.end_turn();
            let taken = mine.iter().filter(|id| s.card(id.as_str()).controller == P2).count();
            assert_eq!(taken, 1, "one of p1's Units, stolen at p2's next start");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_cannot_attack() {
            let s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [FILLER] },
                "p2": { "field": [OTHER], "hand": [FILLER] },
            }));
            assert!(s.attacks_of(ID).is_empty());
        }

        #[test]
        fn the_human_gains_the_felinor_tag_and_the_quoted_steal() {
            let mut s = gallery(true);
            let (human, _theirs) = gift_own_human(&mut s);
            assert!(is_felinor(&s, &human));
            assert_eq!(
                grant_texts(s.state(), s.card(human.as_str())),
                Some(vec!["End of turn: Steal a random enemy Unit.".to_string()])
            );
        }

        #[test]
        fn at_its_controllers_end_of_turn_it_steals_an_enemy_unit_at_once() {
            let mut s = gallery(true);
            let (_human, theirs) = gift_own_human(&mut s);
            s.end_turn();
            assert_eq!(s.card(theirs.as_str()).controller, P1);
        }

        #[test]
        fn with_no_room_the_steal_destroys_it() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ID, "radiant": true }, FILLER],
                    "field": [HUMAN, OTHER, OTHER, OTHER],
                },
                "p2": { "hand": [FILLER], "field": [OTHER] },
            }));
            let human = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&human)));
            s.end_turn();
            s.expect_in_zone(theirs.as_str(), "graveyard");
        }
    }
}
