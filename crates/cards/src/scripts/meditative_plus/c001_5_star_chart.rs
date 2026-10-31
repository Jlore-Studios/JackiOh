//! Meditative+ #1.5 Star Chart (SPEC §8.9 row 1.5, §7, R2521008, R1847000, R1847001, R1847002, R386).
//! (0) Spell, AI, Token: an AI generated card M+ #1 Bob's Plan adds after S #148 Alteran Reach (#2521).
//!   Base:    "Travel {nodes|node|nodes} along your starscape map, gaining each node's reward." — 2
//!   Radiant: the same words — 4
//!
//! Each node is one `travel_starscape`, Alteran Reach's own travel (R1847001): its caster's map, the
//! player's and no card's, charted from the match rng at their first travel when they have none yet
//! (R1847000); the node's heal, draw, mana or damage; and at the 20th node, the edge, S #148.1 Edge of
//! the Universe to the hand, a full hand burning it (§2.4). Nothing travels past the edge, so the
//! travels left when it is reached do nothing (R2521008). The travels are free: none of Alteran
//! Reach's (1) is paid and no Alteran Reach need be on the field; each is a travel, so it spends a
//! Radiant Alteran Reach's free first travel of the turn as any travel does (R1847002). `nodes` is
//! declared (R386) and read through `param`; both faces run this script.

use jackioh_engine::effects::travel_starscape;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001-5";

/// S #148.1 Edge of the Universe, the card the map's edge adds (R1847001), as Alteran Reach's travel
/// gives it.
const EDGE_OF_THE_UNIVERSE: &str = "stinkyscape-148-1";

/// R2521008: `nodes` travels, one after another, each stopping at the edge.
fn star_chart() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            (0..param(&*ctx, "nodes").max(0))
                .map(|_| travel_starscape(EDGE_OF_THE_UNIVERSE))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = star_chart();
    // The same script: the Radiant face's 4 is its declared `nodes`, which `param` reads off the face.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative+ #1.5 Star Chart — SPEC §8.9 row 1.5, BUILD M11 row M+ 1.5: "its caster travels their
// starscape map 2 nodes, gaining each node's reward (R1847001), charting it first when they have none
// (R1847000); at the edge it adds Edge of the Universe and the travels left do nothing (R2521008); no
// mana is paid and no Alteran Reach is needed; nodes reads through `param()`; radiant 4 nodes".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::starscape::{starscape_amount, starscape_traveled_this_turn};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    /// The hero's health the heals are measured from.
    const HEALTH: i32 = 10;

    /// 19 Heals and the edge: a map whose every node but the last gives the same thing.
    fn heal_path() -> Vec<StarscapeReward> {
        let mut nodes = vec![StarscapeReward::Heal; (STARSCAPE_EDGE_TRAVELS - 1) as usize];
        nodes.push(StarscapeReward::Edge);
        nodes
    }

    /// p1 holds the Star Chart (Radiant when `radiant`), p1's hero at `HEALTH`; p1's map is
    /// `heal_path` standing on `at`, or none charted yet with no `at`. p2's hero at 100, so no reward
    /// ends the game.
    fn charting(seed: &str, radiant: bool, at: Option<i32>) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(10),
                "health": HEALTH,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4), "health": 100 },
        }));
        if let Some(at) = at {
            s.state_mut().players.p1.starscape = Some(StarscapeMap {
                nodes: heal_path(),
                at,
                travelled_on: 0,
            });
        }
        s
    }

    fn at(s: &Scenario) -> i32 {
        s.state().players.p1.starscape.as_ref().map_or(0, |map| map.at)
    }

    /// The nodes the play traveled to, in order.
    fn traveled(s: &Scenario) -> Vec<i32> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::StarscapeTravelled { player: P1, node, .. } => Some(*node),
                _ => None,
            })
            .collect()
    }

    fn edges_in_hand(s: &Scenario) -> usize {
        s.hand(P1)
            .iter()
            .filter(|card| card.def_id == EDGE_OF_THE_UNIVERSE)
            .count()
    }

    fn heal() -> i32 {
        starscape_amount(StarscapeReward::Heal).unwrap_or(0)
    }

    mod base {
        use super::*;

        #[test]
        fn r2521008_travels_2_nodes_gaining_each_reward() {
            let mut s = charting("star-chart", false, Some(0));
            let mana = s.state().players.p1.mana.current;
            s.play(ID, json!({}));
            assert_eq!(traveled(&s), vec![1, 2]);
            assert_eq!(at(&s), 2);
            assert_eq!(s.state().players.p1.hero.health, HEALTH + 2 * heal(), "two Heals");
            assert_eq!(s.state().players.p1.mana.current, mana, "no travel price is paid");
        }

        #[test]
        fn r2521008_with_no_map_it_charts_one_from_the_match_rng_and_travels_it() {
            let mut s = charting("star-chart-uncharted", false, None);
            let cursor = s.state().rng_cursor;
            s.play(ID, json!({}));
            let map = s.state().players.p1.starscape.clone().expect("the first travel charts a map");
            assert_eq!(map.nodes.len() as i32, STARSCAPE_EDGE_TRAVELS);
            assert_eq!(map.at, 2);
            assert!(s.state().rng_cursor > cursor, "the chart draws from the match rng");
            assert_eq!(traveled(&s), vec![1, 2]);
        }

        #[test]
        fn r2521008_it_stops_at_the_edge_adding_edge_of_the_universe() {
            let mut s = charting("star-chart-edge", false, Some(STARSCAPE_EDGE_TRAVELS - 1));
            s.play(ID, json!({}));
            assert_eq!(traveled(&s), vec![STARSCAPE_EDGE_TRAVELS], "one travel, then the edge");
            assert_eq!(at(&s), STARSCAPE_EDGE_TRAVELS);
            assert_eq!(edges_in_hand(&s), 1);

            // Standing on the edge already, it travels nowhere and adds nothing.
            let mut s = charting("star-chart-past", false, Some(STARSCAPE_EDGE_TRAVELS));
            s.play(ID, json!({}));
            assert!(traveled(&s).is_empty());
            assert_eq!(at(&s), STARSCAPE_EDGE_TRAVELS);
            assert_eq!(edges_in_hand(&s), 0);
        }

        #[test]
        fn r2521008_its_travels_spend_a_radiant_alteran_reachs_free_travel_this_turn() {
            let mut s = charting("star-chart-free", false, Some(0));
            assert!(!starscape_traveled_this_turn(s.state(), P1));
            s.play(ID, json!({}));
            assert!(starscape_traveled_this_turn(s.state(), P1));
        }

        #[test]
        fn r386_nodes_reads_through_param() {
            let mut s = charting("star-chart-param", false, Some(0));
            assert_eq!(crate::upgrade_number(&mut s, ID, "nodes"), 3);
            s.play(ID, json!({}));
            assert_eq!(traveled(&s), vec![1, 2, 3]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2521008_travels_4_nodes_gaining_each_reward() {
            let mut s = charting("star-chart-radiant", true, Some(0));
            s.play(ID, json!({}));
            assert_eq!(traveled(&s), vec![1, 2, 3, 4]);
            assert_eq!(at(&s), 4);
            assert_eq!(s.state().players.p1.hero.health, HEALTH + 4 * heal(), "four Heals");
        }

        #[test]
        fn r2521008_it_stops_at_the_edge_the_travels_left_doing_nothing() {
            let mut s = charting("star-chart-radiant-edge", true, Some(STARSCAPE_EDGE_TRAVELS - 2));
            s.play(ID, json!({}));
            assert_eq!(
                traveled(&s),
                vec![STARSCAPE_EDGE_TRAVELS - 1, STARSCAPE_EDGE_TRAVELS],
                "two travels of four, the second to the edge"
            );
            assert_eq!(at(&s), STARSCAPE_EDGE_TRAVELS);
            assert_eq!(edges_in_hand(&s), 1, "one Edge of the Universe, no more");
        }
    }
}
