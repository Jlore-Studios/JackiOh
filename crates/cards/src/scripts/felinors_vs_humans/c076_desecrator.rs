//! SPEC §8.14 FvH #76 Desecrator ((1) Unit, Human, Common, 1/1 → 2/2), a card of the held Felinors vs
//! Humans set. Built by S10 (#5260).
//!   Base:    "Crater / Cry: Exile your graveyard. Gain +{stats}/+{stats} for each card exiled." (stats 1)
//!   Radiant: the same text. (stats 2)
//!
//! Crater is catalog data (R783010). Cry (R1): the cards in its controller's graveyard counted as the Cry
//! runs and every one of them exiled (`exile_matching` over that graveyard, Classic #50 Voidwalker's
//! shape; R135), then a permanent `buff` of +stats/+stats for each on itself (R386). An empty graveyard
//! gains nothing. A summoned Desecrator runs no Cry. Tunes: stats 1 ↑ (Radiant 2).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-076";

fn desecrator() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let exiled = zone_count(ctx.state, ctx.controller, OffFieldZone::Graveyard);
            let mut effects =
                vec![exile_matching(json_as(json!({ "zones": ["graveyard"], "player": "self" })))];
            if exiled > 0 {
                let gain = exiled * param(&*ctx, "stats");
                effects.push(buff(json_as(json!({
                    "target": { "of": "self" },
                    "attack": gain,
                    "health": gain,
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The faces differ only in the declared number and the stats: catalog data.
    let base = desecrator();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #76 Desecrator — SPEC §8.14 row 76 (R1, R135, R386, R783010).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless card lying in a graveyard.
    const DEAD: &str = "stand-in (1) Spell";

    /// The printed stats and the gain per card exiled, base and Radiant.
    const STATS: i32 = 1;
    const RADIANT_STATS: i32 = 2;

    /// p1 plays Desecrator with `mine` cards in its graveyard and p2 two in theirs.
    fn desecrated(radiant: bool, mine: usize) -> Scenario {
        let mut s = weapon_board(
            "fvh076",
            json!({
                "hand": [{ "def": ID, "radiant": radiant }],
                "graveyard": vec![DEAD; mine],
                "library": filler(4),
            }),
            json!({ "graveyard": [DEAD, DEAD] }),
        );
        s.play(ID, json!({}));
        s
    }

    fn grows(radiant: bool, printed: i32, per_card: i32) {
        let mut s = desecrated(radiant, 3);
        assert!(s.pile(P1, "graveyard").is_empty());
        assert_eq!(s.pile(P1, "exile").len(), 3);
        assert_eq!(s.pile(P2, "graveyard").len(), 2, "only your own graveyard");
        let grown = printed + 3 * per_card;
        s.expect_stats(ID, json!({ "attack": grown, "health": grown }));
    }

    fn crater_reaches_play(radiant: bool) {
        let s = desecrated(radiant, 0);
        assert!(unit_has(s.state(), &s.unit_at(P1, 1), KeywordKind::Crater));
    }

    mod base {
        use super::*;

        #[test]
        fn exiles_your_graveyard_and_gains_1_1_for_each_card_exiled() {
            grows(false, STATS, STATS);
        }

        #[test]
        fn an_empty_graveyard_gains_nothing() {
            let mut s = desecrated(false, 0);
            s.expect_stats(ID, json!({ "attack": STATS, "health": STATS }));
        }

        #[test]
        fn its_crater_reaches_play() {
            crater_reaches_play(false);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn exiles_your_graveyard_and_gains_2_2_for_each_card_exiled() {
            grows(true, RADIANT_STATS, RADIANT_STATS);
        }

        #[test]
        fn its_crater_reaches_play() {
            crater_reaches_play(true);
        }
    }
}
