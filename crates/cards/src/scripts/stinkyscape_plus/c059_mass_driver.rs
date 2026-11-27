//! SPEC §8.12 S+ #59 Mass Driver (R3556007; R62, R1181000, R1420, R1840002, R3549001, R3828000,
//! R4992000):
//! (2) Field Spell, Mindustrial, Epic, a card of the Stinkyscape+ set.
//!   Base:    "Activate: In {count|turn|turns} add two Minerals that cost (0) and a Resource to your hand."
//!   Radiant: "Activate: In {count|turn|turns} add two Radiant Minerals that cost (0) and a Radiant
//!            Resource to your hand."
//!
//! It printed Flexible and 0/6 (0/12 Radiant) until issue #3828, which made Flexible a Unit's keyword
//! alone (R3828000): it keeps to its backrow and has no stats. The Activate queues an unwatched
//! `delay` (R1181000): at the start of
//! its controller's Nth own turn it delivers two random S+ Minerals at (0)
//! (`add_random_from_catalog`, as S+ #53 Laser Drill's) and a Resource (`gain_resources`,
//! R1840002) — Radiant ones on the Radiant face (R3549001). It lands if the Driver is gone
//! (R3556007); a full hand burns what does not fit (§2.4). Its badge names the effect and leaves the
//! count to the view, which adds " in N turns" (R4992000).

use jackioh_engine::prelude::*;
use jackioh_engine::prompts::RESUME_HOOK;

pub const ID: &str = "stinkyscapeplus-059";

/// "two Minerals".
const MINERALS: i32 = 2;
/// "a Resource".
const RESOURCES: i32 = 1;
/// "Minerals that cost (0)".
const MINERAL_COST: i32 = 0;
/// The delivery's resume step.
const DELIVER: &str = "deliver";
/// The Activate's id.
const DRIVE: &str = "drive";

/// R3549001: "a random Mineral": the held set's, tagged Mineral (as S+ #53's).
fn minerals() -> CatalogQueryArgs {
    json_as(json!({ "set": SetName::StinkyscapePlus, "tags": [Tag::Mineral] }))
}

fn mass_driver(radiant: bool, count_in_label: bool) -> Script {
    let label = if radiant {
        "In 2 turns: add two Radiant Minerals at (0) and a Radiant Resource"
    } else {
        "In 2 turns: add two Minerals at (0) and a Resource"
    };
    Script {
        activations: vec![ActivationDecl {
            id: DRIVE.to_string(),
            label: label.to_string(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(move |ctx| {
                let name = def_of(Some(&*ctx.state), ID).name.clone();
                let turns = param(&*ctx, "count");
                let label = if count_in_label {
                    // The form kept for the versions before issue #4992 (R4992000, R767000): the view
                    // adds " in N turns" too (R1181000), so that badge said it twice.
                    format!("{name}: two Minerals and a Resource in {turns} turns")
                } else {
                    format!("{name}: two Minerals and a Resource")
                };
                vec![delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "turns": turns,
                    "step": DELIVER,
                    "hook": RESUME_HOOK,
                    "label": label,
                })))]
            }),
        }],
        resume: IndexMap::from([(
            DELIVER,
            hook(move |_ctx| {
                vec![
                    add_random_from_catalog(json_as(json!({
                        "query": minerals(),
                        "count": MINERALS,
                        "costOverride": MINERAL_COST,
                        "radiant": radiant,
                    }))),
                    gain_resources(json_as(json!({ "count": RESOURCES, "radiant": radiant }))),
                ]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mass_driver(false, false),
        radiant: mass_driver(true, false),
    }
}

/// R767000: before the patch of issue #4992 the badge held the count the view adds ("… in 2 turns in
/// 2 turns"); `KEPT_SCRIPTS` folds those replays under it, since the label is hashed state.
pub fn script_v0_6_14i() -> CardScripts {
    CardScripts {
        base: mass_driver(false, true),
        radiant: mass_driver(true, true),
    }
}

// S+ #59 Mass Driver — SPEC §8.12 row S+ 59 (R3556007; R62, R1181000, R1840002, R3549001, R3828000,
// R4992000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005";

    /// p1 holds the Driver; `"mana": 10` plays and activates it from hand.
    fn driver(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": filler(6),
            }),
            json!({
                "hand": [FILLER],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s.activate(ID, json!({ "ability": DRIVE }));
        s
    }

    /// What the delivery added: Minerals and Resources gained since.
    fn delivered(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| {
                crate::card_def(&card.def_id).tags.contains(&Tag::Mineral)
                    || crate::resources_in_hand(s, P1).iter().any(|held| held.id == card.id)
            })
            .collect()
    }

    fn is_mineral(card: &CardInstance) -> bool {
        crate::card_def(&card.def_id).tags.contains(&Tag::Mineral)
    }

    /// The Driver's badges on `viewer`'s view of the board (both seats see the same ones).
    fn badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let seat = if viewer == P1 { view.you } else { view.opponent };
        seat.modifiers
            .into_iter()
            .map(|modifier| modifier.label)
            .filter(|label| label.starts_with("Mass Driver"))
            .collect()
    }

    /// R4992000: the badge states its countdown once, on both seats, then counts it down.
    fn assert_badge_counts_down_once(s: &mut Scenario) {
        for viewer in [P1, PlayerId::P2] {
            assert_eq!(
                badges(s, viewer),
                vec!["Mass Driver: two Minerals and a Resource in 2 turns"],
                "{viewer:?} reads the countdown once"
            );
        }
        next_turn_of(s, P1);
        for viewer in [P1, PlayerId::P2] {
            let shown = badges(s, viewer);
            assert_eq!(
                shown,
                vec!["Mass Driver: two Minerals and a Resource in 1 turn"],
                "{viewer:?} reads the next turn's countdown once"
            );
            assert!(shown.iter().all(|label| !label.contains("turns in")), "no doubled count");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3556007_the_activate_delivers_two_free_minerals_and_a_resource_in_two_turns() {
            let mut s = driver(false, "s59-deliver");
            // Not at the first own start: "in 2 turns" (R1181000).
            next_turn_of(&mut s, P1);
            assert!(delivered(&s).is_empty(), "nothing at the first own start");
            next_turn_of(&mut s, P1);
            let cards = delivered(&s);
            let minerals: Vec<&CardInstance> = cards.iter().filter(|card| is_mineral(card)).collect();
            let resources = cards.len() - minerals.len();
            assert_eq!(minerals.len(), MINERALS as usize, "two Minerals");
            assert_eq!(resources, RESOURCES as usize, "and a Resource");
            for mineral in minerals {
                assert!(!mineral.radiant, "no Radiant face's Radiant card");
                assert_eq!(effective_cost(s.state(), mineral, Default::default()), 0, "at (0)");
            }
        }

        #[test]
        fn r4992000_the_badge_states_the_countdown_once_and_counts_it_down() {
            let mut s = driver(false, "s59-badge");
            assert_badge_counts_down_once(&mut s);
        }

        #[test]
        fn r3556007_the_delivery_lands_after_the_driver_is_destroyed() {
            let mut s = driver(false, "s59-gone");
            let id = s.card(ID).id.clone();
            crate::run_as(
                &mut s,
                jackioh_engine::effects::destroy(json_as(json!({
                    "target": { "of": "instance", "instanceId": id },
                }))),
                P1,
            );
            // The mark collects at the turn's state checks; the unwatched delay stands regardless.
            next_turn_of(&mut s, P1);
            assert!(s.card(ID).zone.z() != ZoneName::Field, "the Driver is gone");
            assert!(delivered(&s).is_empty(), "nothing at the first own start");
            next_turn_of(&mut s, P1);
            assert_eq!(delivered(&s).len(), (MINERALS + RESOURCES) as usize, "still delivered");
        }

        #[test]
        fn r3828000_it_is_offered_no_unit_zone_and_keeps_to_its_backrow() {
            let mut s = weapon_board(
                "s59-backrow",
                json!({
                    "mana": 10,
                    "hand": [ID, FILLER],
                    "library": filler(6),
                }),
                json!({
                    "hand": [FILLER],
                    "library": filler(6),
                }),
            );
            // No Flexible, no stats: a unit zone is refused it (R3828000).
            s.expect_refused_with(
                |s| s.play(ID, json!({ "zone": 1, "row": "units" })),
                "goes in the backrow row",
            );
            s.play(ID, json!({ "zone": 1 }));
            let played = s.card(ID);
            assert!(
                matches!(played.zone, Zone::Field { row: Row::Backrow, lane: 1, .. }),
                "it lands in backrow lane 1: {:?}",
                played.zone
            );
            assert_eq!(
                jackioh_engine::faces::card_type_of(s.state(), played),
                CardType::FieldSpell,
                "a Field Spell there"
            );
            assert!(
                !jackioh_engine::layers::has_stat_line(s.state(), played),
                "with no stat line"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556007_radiant_all_radiant() {
            let mut s = driver(true, "s59-radiant");
            assert!(
                !jackioh_engine::layers::has_stat_line(s.state(), s.card(ID)),
                "no 0/12 since R3828000"
            );
            next_turn_of(&mut s, P1);
            next_turn_of(&mut s, P1);
            let cards = delivered(&s);
            assert_eq!(cards.len(), (MINERALS + RESOURCES) as usize);
            for card in &cards {
                assert!(card.radiant, "Radiant {}", card.def_id);
            }
            let minerals: Vec<&CardInstance> = cards.iter().filter(|card| is_mineral(card)).collect();
            assert_eq!(minerals.len(), MINERALS as usize);
            for mineral in minerals {
                assert_eq!(effective_cost(s.state(), mineral, Default::default()), 0, "at (0)");
            }
        }

        #[test]
        fn r4992000_radiant_the_badge_states_the_countdown_once() {
            let mut s = driver(true, "s59-badge-radiant");
            assert_badge_counts_down_once(&mut s);
        }
    }

    mod v0_6_14i {
        use super::*;
        use crate::scripts::stinkyscape_plus::c059_mass_driver::{ID, script_v0_6_14i};

        /// R767000: a game on v0.6.14i, before the countdown's single statement, reads it twice.
        #[test]
        fn r767000_a_game_on_v0_6_14i_states_its_count_twice() {
            crate::under_kept(ID, script_v0_6_14i(), || {
                let s = driver(false, "s59-kept");
                assert_eq!(
                    badges(&s, P1),
                    vec!["Mass Driver: two Minerals and a Resource in 2 turns in 2 turns"]
                );
            });
        }
    }
}
