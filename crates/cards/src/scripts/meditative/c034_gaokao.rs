//! M #34 高考 (SPEC §8.8 row 34): (4) Spell, CN, KY, Rare.
//!
//! Base:    "Destroy every permanent that is neither CN nor KY. Buff each CN or KY permanent
//!           {times|time|times}."
//! Radiant: "Destroy every enemy permanent that is neither CN nor KY. Buff each of your CN or KY
//!           permanents {times|time|times}."
//! Engine:
//! - **Destroy:** `destroy_all({ side, rows: [units, backrow], notTags: [CN, KY] })` (§6.3
//!   Destroy). Indestructible cards, cards Immune to Spells (E35) and #25's immunity (ME-TRIBAL)
//!   survive it.
//! - **Buff:** `upgrade({ scope: { zones: [field], side, tags: [CN, KY] }, times })` (B3.4). A card
//!   scope's `tags` means any of them, so the scope takes permanents with either tag (MD-B14).
//! - The destroyed cards leave at the state check after the whole list (R59), and the two sets
//!   never overlap. Tags are read with the tags effects have granted (#35, `tags_of`, MD-B15).
//! - On the Radiant face the destroy is `side: enemy` and the Buffs `side: self`.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-034";

fn gaokao(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let times = param(&*ctx, "times");
            let (destroy_side, buff_side) = if radiant { ("enemy", "self") } else { ("any", "any") };
            vec![
                destroy_all(json_as(json!({
                    "side": destroy_side,
                    "rows": ["units", "backrow"],
                    "notTags": ["CN", "KY"],
                }))),
                upgrade(json_as(json!({
                    "scope": { "side": buff_side, "zones": ["field"], "tags": ["CN", "KY"] },
                    "times": times,
                }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face destroys enemy permanents only and Buffs only yours, three times.
    let base = gaokao(false);
    let radiant = gaokao(true);
    CardScripts { base, radiant }
}

// M #34 高考 — SPEC §8.8 row 34, BUILD M10 row M 34: "Destroys every permanent on both sides and
// rows that has neither CN nor KY (face-down ones included), Indestructible cards, cards Immune to
// Spells and M 25 surviving (MD-B1); then Buffs each permanent with either tag twice, on both
// sides (MD-B14), a CN tag M 35 granted counting (MD-B15); one state check after both; the Buffs on
// face-down cards are reported per R440; times reads through `param()`; radiant destroys enemy
// permanents only and Buffs only yours, three times".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GAOKAO: &str = "meditative-034";
    const RCTA: &str = "meditative-035";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008"; // Human: neither CN nor KY.
    const BROTHER: &str = "classicplus-076"; // CN Unit.
    const WALL: &str = "classic-041"; // Untagged Indestructible: survives, unbuffed.
    const HONEYPOT: &str = "core-060"; // Untagged Trap, set face-down.

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// Both sides hold a Vanilla and a Brother Lar; p1 also holds the Wall and a face-down
    /// Honeypot. p1 holds Gaokao (base unless `radiant_face`); both sides keep cards in hand so no
    /// turn auto-ends.
    fn casting(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": GAOKAO, "radiant": radiant_face }, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": BROTHER, "lane": 2 }, { "def": WALL, "lane": 3 }],
                "backrow": [{ "def": HONEYPOT, "lane": 1 }],
                "library": filler(4),
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": BROTHER, "lane": 2 }],
                "library": filler(4),
            },
        }))
    }

    fn upgraded_for(events: &[GameEvent], id: &str) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .count()
    }

    /// Brother Lar's id on `player`'s field (lane 2), read before the play: CN is never
    /// destroyed, so the id still names the card after it.
    fn brother(s: &Scenario, player: PlayerId) -> String {
        s.unit(player, 2).expect("Brother Lar stands").id
    }

    mod m34_gaokao {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r922_destroys_neither_tag_buffs_either_tag_twice() {
                let mut s = casting("gaokao", false);
                let p1_brother = brother(&s, P1);
                let p2_brother = brother(&s, P2);
                s.play(GAOKAO, json!({}));
                // Neither tag on either side: both Vanillas and the face-down Honeypot are gone
                // (besides the resolved Spell itself).
                for card in s.state().players.p1.graveyard.iter().chain(s.state().players.p2.graveyard.iter()) {
                    assert!(
                        ["core-008", "core-060", GAOKAO].contains(&card.def_id.as_str()),
                        "{}",
                        card.def_id
                    );
                }
                // Either tag: both Brothers stand, Buffed twice each; the Wall stands, unbuffed.
                assert_eq!(upgraded_for(s.events(), &p1_brother), 2);
                assert_eq!(upgraded_for(s.events(), &p2_brother), 2);
                let wall = s.unit(P1, 3).expect("the Wall stands");
                assert_eq!(wall.def_id, WALL);
                assert_eq!(upgraded_for(s.events(), &wall.id), 0);
            }

            #[test]
            fn r923_a_granted_cn_unit_survives_and_is_buffed() {
                // RCTA grants the Vanilla CN; Gaokao then spares it and Buffs it.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "gaokao-granted",
                    "p1": {
                        "mana": 10,
                        "hand": [{ "def": RCTA }, { "def": GAOKAO }],
                        "field": [{ "def": VANILLA, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let victim = s.unit(P1, 1).expect("the Vanilla");
                s.play(RCTA, json!({ "targets": [{ "pick": "instance", "instanceId": victim.id }] }));
                s.play(GAOKAO, json!({}));
                let standing = s.unit(P1, 1).expect("the granted unit stands");
                assert_eq!(standing.id, victim.id);
                assert_eq!(standing.granted_tags, Some(vec![Tag::Cn]));
                // Once from RCTA, twice from Gaokao.
                assert_eq!(upgraded_for(s.events(), &victim.id), 3);
            }

            #[test]
            fn times_reads_through_param() {
                let mut s = casting("gaokao-param", false);
                let p1_brother = brother(&s, P1);
                let p2_brother = brother(&s, P2);
                set_param(s.card_mut(GAOKAO), "times", 1);
                s.play(GAOKAO, json!({}));
                assert_eq!(upgraded_for(s.events(), &p1_brother), 1);
                assert_eq!(upgraded_for(s.events(), &p2_brother), 1);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn destroys_enemies_only_and_buffs_yours_thrice() {
                let mut s = casting("gaokao-radiant", true);
                let p1_brother = brother(&s, P1);
                let p2_brother = brother(&s, P2);
                s.play(GAOKAO, json!({}));
                // Yours stand: your Vanilla, your Brother, the Wall, the Honeypot.
                assert!(s.unit(P1, 1).is_some());
                assert!(s.unit(P1, 2).is_some());
                assert!(s.unit(P1, 3).is_some());
                // The enemy Vanilla is gone; the enemy Brother stands, Buffed not at all.
                assert!(s.unit(P2, 1).is_none());
                assert_eq!(upgraded_for(s.events(), &p2_brother), 0);
                // Yours Buffed three times.
                assert_eq!(upgraded_for(s.events(), &p1_brother), 3);
            }
        }
    }
}
