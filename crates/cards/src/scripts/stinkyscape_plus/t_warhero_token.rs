//! SPEC §6.3, §8.12 S+ #T-warhero-token Warhero Token (R3548002, R3546000; Stinkyscape+ (issue #3534)): a
//! Stinkyscape Resource, Rare, (0), "Yeah, I'm a war hero." Every Warhero Token is worth one game-wide
//! value in Credits: 0 once the first Token has been printed, growing by a random 0 to 10 at the end
//! of every turn after one has (R3548002), doubled on the Radiant face. "They do nothing when played
//! other than give credits": the engine pays the worth through the Resource's own rules, so the script
//! stays `Script::default()`. It previews that worth in every zone (R3572002): "the Warhero Tokens'
//! worth" on the base face, "twice the Warhero Tokens' worth" on the Radiant one, each with the
//! number it pays (R280).
//!
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-warhero-token";

/// The base text's label for the game-wide worth (R3572002).
const WORTH_LABEL: &str = "the Warhero Tokens' worth";
/// The Radiant text's label for its doubled worth (R3572002).
const RADIANT_WORTH_LABEL: &str = "twice the Warhero Tokens' worth";

/// What the Token pays now in Credits (R3548002), in whichever zone it is read: the game-wide worth
/// times `multiplier`, the Radiant face's doubling.
fn worth_preview(label: &'static str, multiplier: i32) -> PreviewHook {
    condition_hook(move |ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        vec![count_preview(label, multiplier * warhero_worth(ctx.state))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        preview: Some(worth_preview(WORTH_LABEL, 1)),
        ..Script::default()
    };
    let radiant = Script {
        preview: Some(worth_preview(RADIANT_WORTH_LABEL, WARHERO_RADIANT_MULTIPLIER)),
        ..Script::default()
    };
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

    /// What the Token's own hook answers in `zone` (the hook runs as the card does, R280).
    fn read(s: &Scenario, card: &CardInstance, zone: ConditionZone) -> Vec<PreviewValue> {
        preview_of(s.state(), card, PlayerId::P1, zone).expect("a previewed card")
    }

    /// R3572002: the Token reads its worth in the hand: the game-wide value
    /// on its base face under "the Warhero Tokens' worth", twice it on its Radiant face under
    /// "twice the Warhero Tokens' worth", and 0 before any Token has been printed.
    #[test]
    fn r3572002_the_token_previews_its_worth_and_twice_it() {
        for radiant in [false, true] {
            let label = if radiant { RADIANT_WORTH_LABEL } else { WORTH_LABEL };
            let mut s = setup(radiant);
            let card = s.card(ID).clone();
            // A Spell is read in the hand alone: its hook answers whichever zone it is asked in.
            let zone = ConditionZone::Hand;
            // The hand card also quotes its sell price, after the worth (R3548002).
            s.state_mut().warhero_worth = None;
            assert_eq!(
                read(&s, &card, zone).first(),
                Some(&count_preview(label, 0)),
                "radiant {radiant}"
            );
            s.state_mut().warhero_worth = Some(7);
            assert_eq!(
                read(&s, &card, zone).first(),
                Some(&count_preview(label, if radiant { 14 } else { 7 })),
                "radiant {radiant}"
            );
        }
    }
}
