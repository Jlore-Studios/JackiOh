//! S #114 Arctic Citadel (SPEC §8.10 row 114): (4) Unit, Human, Epic, 4/4 → 8/8, of the Stinkyscape set.
//!   Base:    "Cry: Fuse every card in your exile into this."
//!   Radiant: "Cry: Fuse every card in both exiles into this."
//! Engine: Cry (R1): `fuse_cards` (R77, R102): every card in its controller's exile pile, oldest first,
//! fused into this card in one Fuse, this card kept: summed stats, the union of keywords, the scripts
//! concatenated, its cost R77's. The exiled cards cease to exist (R77), the exile counter unchanged (R55).
//! An empty exile, nothing. The Radiant face reads both exiles, its controller's first.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-114";

fn citadel(both: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some(me) = ctx.self_.as_ref().map(|me| me.id.clone()) else {
                return Vec::new();
            };
            let mut sides = vec![ctx.controller];
            if both {
                sides.push(opponent_of(ctx.controller));
            }
            let ingredients: Vec<String> = sides
                .into_iter()
                .flat_map(|player| zone_cards(ctx.state, player, OffFieldZone::Exile))
                .map(|card| card.id)
                .collect();
            if ingredients.is_empty() {
                return Vec::new();
            }
            vec![fuse_cards(FuseCardsArgs {
                instance_ids: Some(ingredients),
                target_instance_id: Some(me),
                ..FuseCardsArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: citadel(false),
        radiant: citadel(true),
    }
}

// S #114 Arctic Citadel — SPEC §8.10 row 114, BUILD M12 row S 114 (R1, R77).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const TIMMY: &str = "core-011"; // (1) 3/3, Rush and First Strike.
    const MENACE: &str = "core-019"; // (3) 9/9, Taunt.
    const FILLER: &str = "core-005";

    fn exiles(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "exile": mine,
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "exile": theirs, "library": filler(4) },
        }))
    }

    fn citadel(s: &Scenario) -> CardInstance {
        s.unit_at(P1, 1)
    }

    #[test]
    fn r77_its_cry_fuses_your_exile_into_it_summing_stats_and_joining_keywords() {
        let mut s = exiles(false, json!([VANILLA, TIMMY]), json!([MENACE]));
        s.play(ID, json!({ "zone": 1 }));
        let fused = citadel(&s);
        s.expect_stats(&fused, json!({ "attack": 11, "health": 11 }));
        assert!(unit_has(s.state(), &fused, KeywordKind::FirstStrike));
        assert!(s.pile(P1, "exile").is_empty(), "the exiled cards cease to exist");
        assert_eq!(s.pile(P2, "exile").len(), 1, "the opponent's exile stays");
    }

    #[test]
    fn r77_an_empty_exile_changes_nothing() {
        let mut s = exiles(false, json!([]), json!([MENACE]));
        s.play(ID, json!({ "zone": 1 }));
        let me = citadel(&s);
        assert_eq!(me.def_id, ID);
        s.expect_stats(&me, json!({ "attack": 4, "health": 4 }));
        assert_eq!(s.pile(P2, "exile").len(), 1);
    }

    #[test]
    fn r275_radiant_8_8_fuses_both_exiles() {
        let mut s = exiles(true, json!([VANILLA]), json!([MENACE]));
        s.play(ID, json!({ "zone": 1 }));
        let fused = citadel(&s);
        // R77: a Radiant kept card wears the fused Radiant face, every ingredient's Radiant stats summed:
        s.expect_stats(&fused, json!({ "attack": 38, "health": 38 }));
        assert!(unit_has(s.state(), &fused, KeywordKind::Taunt));
        assert!(s.pile(P1, "exile").is_empty());
        assert!(s.pile(P2, "exile").is_empty());
    }
}
