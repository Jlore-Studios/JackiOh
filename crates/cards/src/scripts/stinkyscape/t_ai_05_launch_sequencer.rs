//! S T-AI-5 Launch Sequencer (SPEC §7, §8.10 row T-AI-5; §2.4, R482, R1852006, R1881003): (1) Spell, AI,
//! Token (no printed rarity), an AI generated token of the Stinkyscape set, built by S21.
//!   Base:    "Your Starship launch costs (1). Draw 1."
//!   Radiant: "Your Starship launch costs (0). Draw 2."
//! Engine: `starship_launch_costs` (R1852006): its caster's next launch by the Activate costs (1) in place
//!   of (3), a public badge the launch spends; with no Starship yet, the price waits for the next launch
//!   (R1881003). Then a draw (§2.4). Radiant: (0), and draw 2. Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-05";

/// R1852006's "(1)" and "(0)": what the next launch costs, base and Radiant.
const LAUNCH_COST: i32 = 1;
const RADIANT_LAUNCH_COST: i32 = 0;
/// "Draw 1", "Draw 2".
const DRAWS: i32 = 1;
const RADIANT_DRAWS: i32 = 2;

fn face(cost: i32, draws: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                starship_launch_costs(StarshipLaunchCostArgs { cost }),
                draw(DrawArgs { count: draws, player: None }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(LAUNCH_COST, DRAWS),
        radiant: face(RADIANT_LAUNCH_COST, RADIANT_DRAWS),
    }
}

// S T-AI-5 Launch Sequencer — SPEC §8.10 row T-AI-5, BUILD M12 row S T-AI-5: "its caster's next launch costs
// (1), held with no Starship yet (R1881003), then a draw; radiant (0) and two draws".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SEQUENCER: &str = "stinkyscape-t-ai-05";
    const FILLER: &str = "core-005";

    fn cast(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "launch-sequencer",
            "p1": { "hand": [{ "def": SEQUENCER, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(SEQUENCER, json!({}));
        s
    }

    #[test]
    fn r1881003_with_no_starship_yet_the_next_launch_costs_1_and_it_draws_1() {
        let s = cast(false);
        assert!(dormant_starship_of(s.state(), P1).is_none());
        assert_eq!(launch_cost(s.state(), P1), 1);
        assert_eq!(launch_cost(s.state(), P2), STARSHIP_LAUNCH_COST, "its caster's alone");
        assert_eq!(s.hand(P1).len(), 2, "the filler and one draw");
    }

    #[test]
    fn radiant_the_launch_costs_0_and_it_draws_2() {
        let s = cast(true);
        assert_eq!(launch_cost(s.state(), P1), 0);
        assert_eq!(s.hand(P1).len(), 3, "the filler and two draws");
    }

    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce.
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const COMMANDER: &str = "stinkyscape-137"; // Cry: launches your Dormant Starship free.

    #[test]
    fn r1881003_cast_before_any_starship_the_later_launch_costs_1_and_spends_it() {
        let mut s = scenario(json!({
            "seed": "launch-sequencer-later",
            "p1": {
                "hand": [SEQUENCER, HIT_JOB, FILLER],
                "field": [AP_ROUNDS],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.play(SEQUENCER, json!({}));
        assert!(dormant_starship_of(s.state(), P1).is_none());
        let piece = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
        let ship = dormant_starship_of(s.state(), P1)
            .expect("the piece's death built one")
            .id
            .clone();
        assert_eq!(
            launch_cost(s.state(), P1),
            1,
            "the price waited for this Starship"
        );
        let mana = s.state().players.p1.mana.current;
        crate::launch_starship(&mut s, &ship);
        assert!(is_launched_starship(s.card(ship.as_str())));
        assert_eq!(s.state().players.p1.mana.current, mana - 1, "the launch cost (1)");
        assert_eq!(
            launch_cost(s.state(), P1),
            STARSHIP_LAUNCH_COST,
            "and spent the price"
        );
    }

    #[test]
    fn r1881003_a_launch_by_an_effect_spends_none() {
        let (mut s, ship) = crate::built_starship(
            "launch-sequencer-effect",
            json!([{ "def": AP_ROUNDS, "lane": 1 }]),
            json!({ "hand": [SEQUENCER, COMMANDER] }),
            json!({}),
        );
        s.play(SEQUENCER, json!({}));
        assert_eq!(launch_cost(s.state(), P1), 1);
        s.play(COMMANDER, json!({ "zone": 4 }));
        assert!(
            is_launched_starship(s.card(ship.as_str())),
            "Starfleet Commander's Cry launched it"
        );
        assert_eq!(
            launch_cost(s.state(), P1),
            1,
            "the price is still held for a launch by the Activate"
        );
    }
}
