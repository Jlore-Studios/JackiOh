//! SPEC §8.13 IA #124 Firewall Breach ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Destroy an enemy face-down card. Draw a card."
//!   Radiant: "Destroy two enemy face-down cards. Draw a card."
//!
//! A declared enemy backrow pick only a face-down card satisfies (S #9 Ticketed's shape, R1850003:
//! a face-up Field Spell is never offered), destroyed where it stands (§6.3 Destroy); then a draw.

use jackioh_engine::effects::{destroy, draw};
use jackioh_engine::preview::is_face_down;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-124";

/// "An enemy face-down card": the check is data the script's `target_checks` reads (R1850003).
const FACE_DOWN: &str = "faceDown";

/// "Draw a card" on both faces.
const DRAWS: i32 = 1;

fn targets(count: i32) -> Vec<TargetDecl> {
    vec![TargetDecl::target(count, count, json!({ "side": "enemy", "of": ["backrow"], "check": FACE_DOWN }))]
}

fn breach(count: i32) -> Script {
    Script {
        targets: targets(count),
        target_checks: IndexMap::from([(
            FACE_DOWN,
            target_check(|args| args.candidate.is_some_and(|card| is_face_down(args.state, card))),
        )]),
        cry: Some(hook(move |_ctx| {
            let mut effects: Vec<Effect> = (0..count)
                .map(|index| {
                    destroy(json_as(json!({
                        "target": { "of": "chosen", "index": index },
                    })))
                })
                .collect();
            effects.push(draw(json_as(json!({ "count": DRAWS }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: breach(1),
        radiant: breach(2),
    }
}

// IA #124 Firewall Breach — SPEC §8.13 row 124.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TRAP: &str = "core-041"; // Sheepish: a Trap, to sit face-down
    const ARMOR: &str = "core-073"; // Anti-oneshot Armor: a face-up Field Spell
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool, backrow: Value) -> Scenario {
        scenario(json!({
            "seed": "firewall-breach",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(3) },
            "p2": { "hand": [FILLER], "backrow": backrow, "library": crate::filler(3) },
        }))
    }

    fn offered(s: &Scenario) -> Vec<String> {
        pick::instance_ids(&s.play_targets(ID))
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_a_face_down_card_and_draws_a_card() {
            let mut s = board(
                false,
                json!([
                    { "def": TRAP, "faceUp": false, "lane": 1 },
                    { "def": ARMOR, "faceUp": true, "lane": 2 },
                ]),
            );
            let trap = s.card(TRAP).id.clone();
            assert_eq!(offered(&s), vec![trap.clone()], "the face-up Field Spell is not offered");
            s.play(ID, pick::targets(pick::id(&trap)));
            s.expect_in_zone(trap.as_str(), "graveyard");
            assert_eq!(s.hand(P1).len(), 2, "the played Breach drew one");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroys_two_face_down_cards_and_draws_a_card() {
            let mut s = board(
                true,
                json!([
                    { "def": TRAP, "faceUp": false, "lane": 1 },
                    { "def": FILLER, "faceUp": false, "lane": 2 },
                    { "def": ARMOR, "faceUp": true, "lane": 3 },
                ]),
            );
            let targets = offered(&s);
            assert_eq!(targets.len(), 2, "the face-up Field Spell is not offered");
            s.play(
                ID,
                json!({ "targets": [
                    { "pick": "instance", "instanceId": targets[0] },
                    { "pick": "instance", "instanceId": targets[1] },
                ] }),
            );
            for target in &targets {
                s.expect_in_zone(target.as_str(), "graveyard");
            }
            assert_eq!(s.hand(P1).len(), 2, "the played Breach drew one");
        }

        #[test]
        fn with_one_face_down_card_it_destroys_that_one() {
            let mut s = board(true, json!([{ "def": TRAP, "faceUp": false, "lane": 1 }]));
            let trap = s.card(TRAP).id.clone();
            assert_eq!(offered(&s), vec![trap.clone()]);
            s.play(ID, pick::targets(pick::id(&trap)));
            s.expect_in_zone(trap.as_str(), "graveyard");
            assert_eq!(s.hand(P1).len(), 2, "the played Breach drew one");
        }
    }
}
