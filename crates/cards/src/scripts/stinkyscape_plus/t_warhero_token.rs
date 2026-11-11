//! SPEC §6.3, §8.12 S+ #T-warhero-token Warhero Token (R3548002, R3546000; Stinkyscape+ (issue #3534)): a
//! Stinkyscape Resource, Rare, (0), "Yeah, I'm a war hero." Every Warhero Token is worth one game-wide
//! value in Credits: 0 once the first Token has been printed, growing by a random 0 to 10 at the end
//! of every turn after one has (R3548002), doubled on the Radiant face. "They do nothing when played
//! other than give credits": the engine pays the worth through the Resource's own rules, so the script
//! stays `Script::default()`.
//!
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-warhero-token";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SEED: &str = "warhero-token-pays";

    fn setup(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": SEED,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": {},
        }))
    }

    /// R3548002: playing a Warhero Token pays its worth in Credits — twice on its Radiant face.
    #[test]
    fn warhero_token_pays_its_worth_and_its_radiant_face_twice() {
        for radiant in [false, true] {
            let mut s = setup(radiant);
            s.state_mut().warhero_worth = Some(7);
            s.play(ID, json_as(json!({})));
            assert_eq!(
                market_credits_of(s.state(), PlayerId::P1),
                if radiant { 14 } else { 7 }
            );
        }
    }
}
