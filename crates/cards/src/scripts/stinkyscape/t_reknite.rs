//! SPEC §7 Stinkyscape T-reknite Reknite, a Resource token of the Stinkyscape set (§7's token table).
//!   Base:    "Your hero gains {armor} Armor." (1)
//!   Radiant: the same (2).
//!
//! A Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521): gained into its owner's
//! hand, played from there at (0) for its Credits (R2250000, R2250001). Its resolution is Armor the
//! hero keeps (`gain_hero_armor`, R3857000), which each hit it meets wears (R5183000), since #5183
//! (R5183002). Before it, M+ #12 Living Armor Potion's Armor: Core #98 Armor Up's `heroArmor` player
//! modifier (R757), gone at the start of its caster's next turn (`script_kept_5183`, kept, R767000).
//! The number is the declared `armor`, read as it resolves (R386).

use jackioh_engine::effects::add_player_modifier;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-reknite";

fn face() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![gain_hero_armor(json_as(json!({ "amount": param(&*ctx, "armor") })))]
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

/// R767000, R5183002: the face before #5183, kept for its replays: Core #98 Armor Up's `heroArmor`
/// modifier (R757), gone when its caster's next turn begins.
fn face_kept_5183() -> Script {
    Script {
        cry: Some(hook(|ctx| {
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

/// R767000, R5183002: `KEPT_SCRIPTS` folds the games before #5183 under it.
pub fn script_kept_5183() -> CardScripts {
    CardScripts {
        base: face_kept_5183(),
        radiant: face_kept_5183(),
    }
}

// S T-reknite — SPEC §7, BUILD M12 row S T-reknite. Tagged Resource and Token (issue #2521).
#[cfg(test)]
mod tests {
    use jackioh_engine::damage::hero_armor_of;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{play_resource, scenario};

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

    /// R5183002: Reknite's Armor is the hero's to keep (which a hit wears, R5183000): it stays through
    /// the opponent's turn and past its caster's next start of turn.
    #[test]
    fn r5183002_its_armor_is_the_heros_to_keep() {
        let mut s = played(false);
        s.expect_mana(P1, 4);
        assert_eq!(armor(&s), 1, "+1 Armor at once, for (0)");
        assert_eq!(s.state().players.p1.hero.armor, 1, "the hero's own Armor, no modifier");
        s.end_turn();
        assert_eq!(armor(&s), 1, "kept through the opponent's turn");
        s.end_turn();
        assert_eq!(armor(&s), 1, "and still there at its caster's next turn");
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
    fn r5183002_radiant_keeps_2() {
        let mut s = played(true);
        assert_eq!(armor(&s), 2);
        s.end_turn();
        s.end_turn();
        assert_eq!(armor(&s), 2);
    }

    /// R767000, R5183002: the kept face's Armor lasts through the opponent's turn and is gone at its
    /// caster's next.
    #[test]
    fn r5183002_the_kept_face_is_gone_at_the_casters_next_turn() {
        crate::under_kept(ID, super::script_kept_5183(), || {
            let mut s = played(false);
            assert_eq!(armor(&s), 1);
            assert_eq!(s.state().players.p1.hero.armor, 0, "a modifier, not the hero's own");
            s.end_turn();
            assert_eq!(armor(&s), 1, "kept through the opponent's turn");
            s.end_turn();
            assert_eq!(armor(&s), 0, "gone at its caster's next start of turn");
        });
    }
}
