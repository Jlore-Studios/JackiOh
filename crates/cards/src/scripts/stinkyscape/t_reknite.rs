//! SPEC §7 Stinkyscape T-reknite Reknite, a Resource token of the Stinkyscape set (§7's token table).
//!   Base:    "Your hero has +{armor} Armor until the start of your next turn." (1)
//!   Radiant: the same (2).
//!
//! A Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521): gained into its owner's
//! hand, played from there at (0) for its Credits (R2250000, R2250001). Its resolution is M+ #12
//! Living Armor Potion's Armor: Core #98 Armor Up's `heroArmor` player modifier (R757), gone at the
//! start of its caster's next turn, so it lasts through the opponent's turn. The number is the
//! declared `armor`, read as it resolves (R386).

use jackioh_engine::effects::add_player_modifier;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-reknite";

fn face() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            // Exactly Core #98 Armor Up's modifier (R757): gone when the next own turn begins.
            let opponent = opponent_of(ctx.controller);
            vec![add_player_modifier(json_as(json!({
                "mod": {
                    "kind": "heroArmor",
                    "amount": param(&*ctx, "armor"),
                    "expiry": { "until": "nextTurnOf", "player": opponent, "fromTurn": ctx.sink.state.turn },
                },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(),
        radiant: face(),
    }
}

// S T-reknite — SPEC §7, BUILD M12 row S T-reknite. Tagged Resource and Token (issue #2521).
#[cfg(test)]
mod tests {
    use jackioh_engine::damage::hero_armor_of;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn in_hand(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn played(radiant: bool) -> Scenario {
        let mut s = in_hand(radiant);
        play_resource(&mut s, P1, ID, Value::Null);
        s
    }

    fn armor(s: &Scenario) -> i32 {
        hero_armor_of(s.state(), P1)
    }

    #[test]
    fn tagged_resource_and_token() {
        assert_eq!(card_def(ID).tags, vec![Tag::Resource, Tag::Token]);
    }

    #[test]
    fn armor_through_the_opponents_turn() {
        let mut s = played(false);
        s.expect_mana(P1, 4);
        assert_eq!(armor(&s), 1, "+1 Armor at once, for (0)");
        s.end_turn();
        assert_eq!(armor(&s), 1, "kept through the opponent's turn");
        s.end_turn();
        assert_eq!(armor(&s), 0, "gone at its caster's next start of turn");
    }

    #[test]
    fn armor_reads_through_param() {
        let mut s = in_hand(false);
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "armor", 3);
        play_resource(&mut s, P1, ID, Value::Null);
        assert_eq!(armor(&s), 3);
    }

    #[test]
    fn radiant_plus_2() {
        let mut s = played(true);
        assert_eq!(armor(&s), 2);
        s.end_turn();
        assert_eq!(armor(&s), 2);
        s.end_turn();
        assert_eq!(armor(&s), 0);
    }
}
