//! SPEC §8.13 IA #108 Second Draft ((1) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Discard your hand. Draw that many cards, plus one."
//!   Radiant: "Discard your hand. Draw that many cards, plus two."
//!
//! `n` is the controller's hand count as the Spell resolves — the Spell itself has left it — or 0
//! under a discard guard (`discard_guarded`, R800, R4589007); then `discard_hand` and a draw of
//! `n + EXTRA` (§2.4's pipeline). The Radiant face's extra is two.

use jackioh_engine::draw::discard_guarded;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-108";

/// "plus one", the base face's.
const EXTRA: i32 = 1;
/// "plus two", the Radiant face's.
const RADIANT_EXTRA: i32 = 2;

fn draft(extra: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            let held = if discard_guarded(view.state, view.controller) {
                0
            } else {
                zone_count(view.state, view.controller, OffFieldZone::Hand)
            };
            vec![
                discard_hand(json_as(json!({}))),
                draw(json_as(json!({ "count": held + extra }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: draft(EXTRA),
        radiant: draft(RADIANT_EXTRA),
    }
}

// IA #108 Second Draft — SPEC §8.13 row 108 (R4589007).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";

    /// p1 casts Second Draft with `held` other cards in hand.
    fn drafted(radiant: bool, held: usize) -> Scenario {
        let mut hand: Vec<Value> = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..held).map(|_| json!(FILLER)));
        let mut s = scenario(json!({
            "p1": { "hand": hand, "library": crate::filler(8) },
            "p2": { "library": crate::filler(3) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r4589007_draws_one_more_than_discarded() {
            let s = drafted(false, 3);
            assert_eq!(s.hand(P1).len(), 4, "three discarded, four drawn");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589007_radiant_draws_two_more_than_discarded() {
            let s = drafted(true, 3);
            assert_eq!(s.hand(P1).len(), 5, "three discarded, five drawn");
        }
    }
}
