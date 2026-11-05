//! What a card's "you" follows once it changes sides (SPEC §6.2, §8 Conventions: "'Your' means the
//! controller", R171, R384, R3044000). Found by the polish-4 edge-case hunt (docs/polish/4-edge-cases.md,
//! lens L1).
//!  - #79 Twinspell's "Activate: The next Spell you play this turn gains Echo +1" is its controller's to
//!    use: stolen, it is the thief who activates it and whose next Spell echoes (until #3044 its grant was
//!    a modifier the permanent installed, which moved with it, R30).
//!  - §6.2: a start-of-turn hook fires on its controller's own turn start. One queued for that
//!    controller does not fire for the player who took the card while the queue was running.

use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MOTHS: &str = "core-009";
const POINTMASTER: &str = "core-020";
const SEVEN_SEVEN: &str = "core-025";
const GRAVEDIGGER: &str = "core-037";
const KPOP: &str = "core-050";
const TWINSPELL: &str = "core-079";
const MROW: &str = "core-086";
const LIBRARY: [&str; 8] = [
    VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA,
];

fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn backrow_at(g: &Scenario, player: &'static str, lane: i32) -> CardInstance {
    g.backrow(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a backrow card in lane {lane}"))
}

fn echo_next_spell_mods(g: &Scenario, player: PlayerId) -> usize {
    g.state().players[player]
        .mods
        .iter()
        .filter(|m| matches!(m.kind, ModifierKind::EchoNextSpell { .. }))
        .count()
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

/// R384, R3044000: a stolen Twinspell's Activate is its new controller's
mod r3044000_a_stolen_twinspells_activate_is_its_new_controllers {
    use super::*;

    #[test]
    fn r3044000_r12_r171_r2618000_a_stolen_twinspell_grants_its_echo_to_the_thiefs_next_spell_and_goes_to_the_thiefs_graveyard_s8_c79()
     {
        let mut g = setup(json!({
            "p1": { "hand": [TWINSPELL, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [KPOP, STOCKPILE, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
        }));
        g.play(TWINSPELL, json!({ "zone": 3 }));
        let twinspell = backrow_at(&g, "p1", 3);
        g.end_turn();

        g.play(KPOP, json!({ "targets": pick::instance(&twinspell) }));
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        g.end_turn();
        // The steal fired at the start of p2's turn. Twinspell installs nothing standing on the field, so
        // there was no modifier to carry: p2 activates it (R384), and the grant is p2's.
        assert_eq!(g.state().active, PlayerId::P2);
        assert_eq!(g.card(&twinspell).controller, PlayerId::P2);
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P1), 0);
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P2), 0);
        g.activate(&twinspell, json!({}));
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P1), 0);
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P2), 1);

        // Twinspell has gone to p2's graveyard, the side it was on, as p2's card (R2618000), and p2's
        // Stockpile resolves twice (draw 2, twice).
        g.expect_in_zone(&twinspell, "graveyard");
        let before = g.hand("p2").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(g.hand("p2").len(), before - 1 + 4);
        assert!(
            g.pile("p2", "graveyard")
                .iter()
                .any(|card| card.id == twinspell.id)
        );
        assert_eq!(g.card(&twinspell).owner, PlayerId::P2);
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P2), 0);
    }
}

/// §6.2: a start-of-turn hook belongs to its controller's turn
mod s6_2_a_start_of_turn_hook_belongs_to_its_controllers_turn {
    use super::*;

    #[test]
    fn r62_r153_s6_2_a_start_of_turn_queue_belongs_to_its_controller_the_moths_mrows_death_takes_mid_queue_changes_sides_and_the_gravedigger_behind_it_still_fires_for_p1()
     {
        let mut g = setup(json!({
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": MOTHS, "lane": 1 }, { "def": GRAVEDIGGER, "lane": 2 }],
                "graveyard": [POINTMASTER],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": MROW, "lane": 1 }], "graveyard": [SEVEN_SEVEN], "library": LIBRARY },
        }));
        let moths = g.unit_at("p1", 1);
        let digger = g.unit_at("p1", 2);
        let mrow = g.unit_at("p2", 1);
        let p2_hand = g.hand("p2").len();

        // Moths compels Mrow, which dies to the strike back; its Death takes control of the unit that
        // destroyed it (R361): Moths.
        g.start_turn();
        g.expect_in_zone(&mrow, "graveyard");
        assert_eq!(g.card(&moths).controller, PlayerId::P2);
        assert_eq!(g.card(&digger).controller, PlayerId::P1);

        assert!(def_ids(&g.hand("p1")).contains(&POINTMASTER.to_string()));
        assert_eq!(g.hand("p2").len(), p2_hand);
        assert!(def_ids(&g.pile("p2", "graveyard")).contains(&SEVEN_SEVEN.to_string()));
    }
}
