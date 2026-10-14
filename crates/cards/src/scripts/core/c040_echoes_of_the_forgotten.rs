//! #40 Echoes of the Forgotten (SPEC §8.2): 2-cost Field Spell, "Start of your turn: deal damage to
//! the enemy hero equal to the cards in your exile; then exile the bottom card of your library",
//! radiant "… equal to twice the cards in your exile; …" (R275): only that multiple changes.
//!
//! R280: its `preview` is the damage it would deal if its controller's turn started now (`damage_now`,
//! the hook's own); the exile count is public (§3). R72: "cards in exile" is YOUR OWN pile, never the
//! game-wide `counters.exiled` (R55) or the opponent's.
//!
//! ORDER is load-bearing: the damage is counted BEFORE the new card enters exile, so this turn's exile
//! pays out next turn. The hook reads `ctx.state` (CLAUDE.md rule 5 bans writing it) and freezes the
//! number into the `damage` effect. The bottom card goes through `exileBottomOfLibrary` (shared with #65):
//! `exile` takes a `TargetSpec`, which cannot name a library card, and the clause is not a choice (R81).

use jackioh_engine::effects::{damage, exile_bottom_of_library};
use jackioh_engine::prelude::*;
use jackioh_engine::query::zone_count;
use jackioh_engine::zones::OffFieldZone;

pub const ID: &str = "core-040";

/// One value per face (SURFACE §4.2's constant object).
#[derive(Clone, Copy)]
struct PerFace<T: Copy> {
    base: T,
    radiant: T,
}

impl<T: Copy> PerFace<T> {
    fn of(&self, face: FaceKind) -> T {
        match face {
            FaceKind::Base => self.base,
            FaceKind::Radiant => self.radiant,
        }
    }
}

/// How many damage each card in your exile is worth: "equal to the cards", radiant "twice the cards".
const PER_EXILED_CARD: PerFace<i32> = PerFace { base: 1, radiant: 2 };

/// R280: the formula as each face prints it, which the preview labels its number with.
const FORMULA: PerFace<&str> = PerFace {
    base: "the cards in your exile",
    radiant: "twice the cards in your exile",
};

/// The damage the hook deals if it runs now. R72: your own exile pile, read before anything new
/// enters it, through the engine's read-only `zoneCount` rather than off `PlayerState` (BUILD M3-T1).
fn damage_now(state: &GameState, controller: PlayerId, per_card: i32) -> i32 {
    per_card * zone_count(state, controller, OffFieldZone::Exile)
}

/// The two faces differ only by what each card in the exile is worth.
fn echoes_of_the_forgotten(face: FaceKind) -> Script {
    let per_card = PER_EXILED_CARD.of(face);
    let formula = FORMULA.of(face);
    Script {
        start_of_turn: Some(hook(move |ctx| {
            vec![
                damage(json_as(json!({
                    "to": { "of": "enemyHero" },
                    "amount": damage_now(&*ctx.state, ctx.controller, per_card),
                }))),
                // "then exile the bottom card of your library" — after the count, so it pays out next turn.
                exile_bottom_of_library(json_as(json!({ "player": "self" }))),
            ]
        })),
        preview: Some(condition_hook(move |ctx| {
            vec![PreviewValue {
                label: formula.to_string(),
                value: damage_now(ctx.state, ctx.controller, per_card),
                display: None,
                ids: None,
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: echoes_of_the_forgotten(FaceKind::Base),
        radiant: echoes_of_the_forgotten(FaceKind::Radiant),
    }
}

// #40 Echoes of the Forgotten — SPEC §8.2 row 40, BUILD M4-T4 row 40: "Start of turn: damage = exile
// count, then bottom card exiled; empty library → no exile, no fatigue"; Radiant: twice the count (R275).
// R280's `preview` is proved in test/preview.test.ts. R72: your own exile pile only. R62: the bottom
// card is exiled before the draw. R63: a hit of 0 is no damage instance, so an empty pile emits none
// (twice 0 is 0). §2.4/R3: fatigue is the draw's, so an empty library shows exactly one instance.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const ECHOES: &str = "core-040"; // Field Spell, 2
    const MENACE: &str = "core-019"; // library filler — not cast-on-draw, so a draw is just a draw
    const TIMMY: &str = "core-011";
    const POSTDOC: &str = "core-061";
    const BIG_D: &str = "core-001";
    const STOCKPILE: &str = "core-005";

    /// `library[0]` is the top, so the LAST entry is the bottom card this card exiles.
    const LIBRARY: [&str; 3] = [MENACE, TIMMY, POSTDOC];

    /// Three cards in p1's exile, so the base face deals 3 and the radiant face 6.
    const EXILE_THREE: [&str; 3] = [STOCKPILE, TIMMY, MENACE];

    /// The harness's import-time `registerAll()`: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn damage_to(s: &Scenario, target: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn defs_in(s: &Scenario, player: &str, zone: &str) -> Vec<String> {
        s.pile(player, zone).into_iter().map(|card| card.def_id).collect()
    }

    fn exiled_events(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Exiled { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn deals_damage_to_the_enemy_hero_equal_to_your_exile_count_then_exiles_the_bottom_card() {
            let mut s = scn(json!({
                "p1": { "backrow": [ECHOES], "exile": EXILE_THREE, "library": LIBRARY },
                "p2": {},
            }));

            s.start_turn();

            s.expect_health("p2", 27);
            assert_eq!(damage_to(&s, "hero-p2"), vec![3]);
            // The bottom card (#61) left the library for the exile pile; the top card (#19) was drawn.
            assert_eq!(defs_in(&s, "p1", "exile"), strings(&[STOCKPILE, TIMMY, MENACE, POSTDOC]));
            assert_eq!(defs_in(&s, "p1", "library"), strings(&[TIMMY]));
            assert_eq!(exiled_events(&s), strings(&[POSTDOC]));
        }

        #[test]
        fn the_count_is_read_before_the_new_card_enters_exile_so_an_empty_pile_deals_nothing_r63() {
            let mut s = scn(json!({
                "p1": { "backrow": [ECHOES], "library": LIBRARY },
                "p2": {},
            }));

            s.start_turn();

            // The card this turn exiles brings the pile to 1, and yet the hit was 0: the order is the rule.
            assert!(damage_to(&s, "hero-p2").is_empty());
            s.expect_health("p2", 30);
            assert_eq!(defs_in(&s, "p1", "exile"), strings(&[POSTDOC]));
        }

        #[test]
        fn the_exile_this_turn_pays_out_next_turn_0_damage_then_1() {
            let mut s = scn(json!({
                "p1": { "backrow": [ECHOES], "hand": [STOCKPILE, TIMMY], "library": [MENACE, TIMMY, POSTDOC, BIG_D] },
                "p2": { "hand": [STOCKPILE, TIMMY], "library": [MENACE, TIMMY] },
            }));

            s.start_turn(); // exile 0 → no damage; #1 (the bottom) is exiled
            s.expect_health("p2", 30);

            s.end_turn(); // p2's turn starts
            s.end_turn(); // back to p1: exile 1 → 1 damage, and #61 (the new bottom) is exiled

            s.expect_health("p2", 29);
            assert_eq!(defs_in(&s, "p1", "exile"), strings(&[BIG_D, POSTDOC]));
        }

        #[test]
        fn r72_it_counts_your_own_exile_pile_not_the_opponents() {
            let mut s = scn(json!({
                "p1": { "backrow": [ECHOES], "exile": [STOCKPILE], "library": LIBRARY },
                "p2": { "exile": [STOCKPILE, TIMMY, MENACE, POSTDOC, BIG_D] },
            }));

            s.start_turn();

            s.expect_health("p2", 29); // 1, your own pile — not 5 and not 6
            assert_eq!(damage_to(&s, "hero-p2"), vec![1]);
        }

        #[test]
        fn sec8_2_an_empty_library_exiles_nothing_and_adds_no_fatigue_of_its_own() {
            let mut s = scn(json!({
                "p1": { "backrow": [ECHOES], "exile": [STOCKPILE, TIMMY] },
                "p2": {},
            }));

            s.start_turn();

            s.expect_health("p2", 28);
            assert!(exiled_events(&s).is_empty());
            assert_eq!(s.pile("p1", "exile").len(), 2);
            // §2.4/R3: the turn's own draw fatigues for FATIGUE_DAMAGE(1) = 1 and nothing else does, so a
            // second fatigue instance from this card's exile clause would show as 30 − 1 − 2 = 27.
            s.expect_health("p1", 29);
            assert_eq!(damage_to(&s, "hero-p1"), vec![1]);
        }

        #[test]
        fn sec6_2_start_of_your_turn_the_opponents_turn_start_does_not_fire_it() {
            let mut s = scn(json!({
                "active": "p2",
                "p1": { "backrow": [ECHOES], "exile": EXILE_THREE, "library": LIBRARY },
                "p2": { "library": [MENACE, TIMMY] },
            }));

            s.start_turn(); // p2's turn

            s.expect_health("p2", 30);
            s.expect_health("p1", 30);
            assert_eq!(s.pile("p1", "exile").len(), 3);
            assert_eq!(defs_in(&s, "p1", "library"), strings(&LIBRARY));
        }

        #[test]
        fn it_stays_on_the_board_and_fires_again_every_one_of_your_turns() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [ECHOES],
                    "exile": EXILE_THREE,
                    "hand": [STOCKPILE, TIMMY],
                    "library": [MENACE, TIMMY, POSTDOC, BIG_D],
                },
                "p2": { "hand": [STOCKPILE, TIMMY], "library": [MENACE, TIMMY] },
            }));

            s.start_turn(); // 3 in exile → 3 damage, pile becomes 4
            s.end_turn();
            s.end_turn(); // 4 in exile → 4 damage

            s.expect_health("p2", 23);
            assert_eq!(damage_to(&s, "hero-p2"), vec![3, 4]);
            s.expect_in_zone(ECHOES, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn twice_the_cards_in_your_exile_three_exiled_cards_deal_6() {
            let mut s = scn(json!({
                "p1": { "backrow": [{ "def": ECHOES, "radiant": true }], "exile": EXILE_THREE, "library": LIBRARY },
                "p2": {},
            }));

            s.start_turn();

            s.expect_health("p2", 24); // 2 × 3
            assert_eq!(damage_to(&s, "hero-p2"), vec![6]);
        }

        #[test]
        fn r63_an_empty_exile_pile_deals_nothing_on_the_radiant_face_either_twice_0_is_0() {
            let mut s = scn(json!({
                "p1": { "backrow": [{ "def": ECHOES, "radiant": true }], "library": LIBRARY },
                "p2": {},
            }));

            s.start_turn();

            s.expect_health("p2", 30);
            assert!(damage_to(&s, "hero-p2").is_empty());
            // The exile clause still runs, so next turn pays out 2.
            assert_eq!(defs_in(&s, "p1", "exile"), strings(&[POSTDOC]));
        }

        #[test]
        fn the_radiant_face_changes_only_the_multiple_so_the_bottom_card_is_still_exiled() {
            let mut s = scn(json!({
                "p1": { "backrow": [{ "def": ECHOES, "radiant": true }], "exile": [STOCKPILE], "library": LIBRARY },
                "p2": {},
            }));

            s.start_turn();

            s.expect_health("p2", 28); // 2 × 1
            assert_eq!(defs_in(&s, "p1", "exile"), strings(&[STOCKPILE, POSTDOC]));
            assert_eq!(exiled_events(&s), strings(&[POSTDOC]));
        }

        #[test]
        fn the_exile_this_turn_pays_out_twice_next_turn_0_damage_then_2() {
            let mut s = scn(json!({
                "p1": {
                    "backrow": [{ "def": ECHOES, "radiant": true }],
                    "hand": [STOCKPILE, TIMMY],
                    "library": [MENACE, TIMMY, POSTDOC, BIG_D],
                },
                "p2": { "hand": [STOCKPILE, TIMMY], "library": [MENACE, TIMMY] },
            }));

            s.start_turn();
            s.expect_health("p2", 30);

            s.end_turn();
            s.end_turn();

            s.expect_health("p2", 28);
            assert_eq!(damage_to(&s, "hero-p2"), vec![2]);
        }

        #[test]
        fn r72_the_radiant_face_counts_your_own_pile_too() {
            let mut s = scn(json!({
                "p1": { "backrow": [{ "def": ECHOES, "radiant": true }], "exile": [STOCKPILE], "library": LIBRARY },
                "p2": { "exile": [STOCKPILE, TIMMY, MENACE] },
            }));

            s.start_turn();

            s.expect_health("p2", 28); // 2 × 1, not 2 × 3 or 2 × 4
        }

        #[test]
        fn sec8_2_an_empty_library_still_exiles_nothing_on_the_radiant_face() {
            let mut s = scn(json!({
                "p1": { "backrow": [{ "def": ECHOES, "radiant": true }], "exile": [STOCKPILE, TIMMY] },
                "p2": {},
            }));

            s.start_turn();

            s.expect_health("p2", 26); // 2 × 2
            assert!(exiled_events(&s).is_empty());
            assert_eq!(s.pile("p1", "exile").len(), 2);
            s.expect_health("p1", 29);
        }
    }
}
